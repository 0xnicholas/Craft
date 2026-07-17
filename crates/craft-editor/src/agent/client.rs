use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;

use serde_json::Value;

use super::config::AgentConfig;
use super::types::{AgentStreamEvent, ChatMessage, LlmBackend, ToolCall, ToolDef};

pub struct LiveLlmBackend {
    pub http: reqwest::blocking::Client,
}

impl Default for LiveLlmBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveLlmBackend {
    pub fn new() -> Self {
        Self {
            http: reqwest::blocking::Client::new(),
        }
    }
}

impl LlmBackend for LiveLlmBackend {
    fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
        stream: bool,
        api_base: &str,
        api_key: &str,
        model: &str,
        event_tx: mpsc::Sender<AgentStreamEvent>,
    ) {
        let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "tools": tools,
            "stream": stream,
        });

        let request = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body);

        match request.send() {
            Ok(response) => {
                if !response.status().is_success() {
                    let _ = event_tx.send(AgentStreamEvent::Error(format!(
                        "HTTP {}",
                        response.status().as_u16()
                    )));
                    return;
                }
                if stream {
                    Self::parse_sse(response, &event_tx);
                } else {
                    Self::parse_non_streaming(response, &event_tx);
                }
            }
            Err(e) => {
                let _ = event_tx.send(AgentStreamEvent::Error(format!("HTTP error: {e}")));
            }
        }
    }
}

impl LiveLlmBackend {
    fn parse_non_streaming(
        response: reqwest::blocking::Response,
        event_tx: &mpsc::Sender<AgentStreamEvent>,
    ) {
        let full_text = match response.text() {
            Ok(t) => t,
            Err(e) => {
                let _ = event_tx.send(AgentStreamEvent::Error(format!("read error: {e}")));
                return;
            }
        };
        let parsed: Value = match serde_json::from_str(&full_text) {
            Ok(v) => v,
            Err(e) => {
                let _ = event_tx.send(AgentStreamEvent::Error(format!("parse error: {e}")));
                return;
            }
        };
        let choice = &parsed["choices"][0];
        let message = &choice["message"];
        let content = message["content"].as_str().unwrap_or("").to_string();
        let tool_calls: Vec<ToolCall> = message["tool_calls"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|tc| serde_json::from_value(tc.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        let _ = event_tx.send(AgentStreamEvent::Done {
            full_text: content,
            tool_calls,
        });
    }

    fn parse_sse(response: reqwest::blocking::Response, event_tx: &mpsc::Sender<AgentStreamEvent>) {
        let reader = std::io::BufReader::new(response);
        let mut full_text = String::new();
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    let _ = event_tx.send(AgentStreamEvent::Error(format!("read error: {e}")));
                    return;
                }
            };
            if line.is_empty() {
                continue;
            }
            if line == "data: [DONE]" {
                break;
            }
            if let Some(data) = line.strip_prefix("data: ") {
                let parsed: Value = match serde_json::from_str(data) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if let Some(delta) = parsed["choices"][0]["delta"]["content"].as_str() {
                    full_text.push_str(delta);
                    let _ = event_tx.send(AgentStreamEvent::Token(delta.to_string()));
                }
            }
        }
        let _ = event_tx.send(AgentStreamEvent::Done {
            full_text,
            tool_calls: Vec::new(),
        });
    }
}

pub struct AgentClient {
    inner: Arc<AgentClientInner>,
}

struct AgentClientInner {
    backend: Box<dyn LlmBackend>,
    config: AgentConfig,
    request_in_flight: AtomicBool,
}

impl AgentClient {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            inner: Arc::new(AgentClientInner {
                backend: Box::new(LiveLlmBackend::new()),
                config,
                request_in_flight: AtomicBool::new(false),
            }),
        }
    }

    pub fn chat(
        &self,
        messages: Vec<ChatMessage>,
        tools: &[ToolDef],
        stream: bool,
        event_tx: mpsc::Sender<AgentStreamEvent>,
    ) -> Option<JoinHandle<()>> {
        if self
            .inner
            .request_in_flight
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return None;
        }

        let inner = Arc::clone(&self.inner);
        let tools_owned: Vec<ToolDef> = tools.to_vec();
        let handle = std::thread::spawn(move || {
            inner.backend.chat(
                &messages,
                &tools_owned,
                stream,
                &inner.config.api_base,
                &inner.config.api_key,
                &inner.config.model,
                event_tx,
            );
            inner.request_in_flight.store(false, Ordering::SeqCst);
        });
        Some(handle)
    }

    pub fn shutdown(&self) {
        self.inner.request_in_flight.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    struct MockLlmBackend;

    impl LlmBackend for MockLlmBackend {
        fn chat(
            &self,
            _messages: &[ChatMessage],
            _tools: &[ToolDef],
            stream: bool,
            _api_base: &str,
            _api_key: &str,
            _model: &str,
            event_tx: mpsc::Sender<AgentStreamEvent>,
        ) {
            if stream {
                for chunk in &["Hello", " world", "!"] {
                    let _ = event_tx.send(AgentStreamEvent::Token(chunk.to_string()));
                }
                let _ = event_tx.send(AgentStreamEvent::Done {
                    full_text: "Hello world!".into(),
                    tool_calls: vec![],
                });
            } else {
                let _ = event_tx.send(AgentStreamEvent::Done {
                    full_text: "mock response".into(),
                    tool_calls: vec![],
                });
            }
        }
    }

    fn make_client() -> AgentClient {
        AgentClient {
            inner: Arc::new(AgentClientInner {
                backend: Box::new(MockLlmBackend),
                config: AgentConfig::default(),
                request_in_flight: AtomicBool::new(false),
            }),
        }
    }

    #[test]
    fn client_streaming_returns_tokens_then_done() {
        let client = make_client();
        let (tx, rx) = mpsc::channel();

        let handle = client.chat(vec![], &[], true, tx).expect("should start");
        handle.join().unwrap();

        let events: Vec<_> = rx.try_iter().collect();
        assert_eq!(events.len(), 4);
        assert!(matches!(events[0], AgentStreamEvent::Token(_)));
        assert!(matches!(events[3], AgentStreamEvent::Done { .. }));
    }

    #[test]
    fn client_prevents_concurrent_requests() {
        let client = make_client();
        let (tx1, _rx1) = mpsc::channel();
        let (tx2, _rx2) = mpsc::channel();

        let h1 = client.chat(vec![], &[], true, tx1);
        let h2 = client.chat(vec![], &[], true, tx2);

        assert!(h1.is_some());
        assert!(h2.is_none());
    }
}
