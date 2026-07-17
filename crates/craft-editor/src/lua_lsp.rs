use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use crate::lsp::{LspMessage, frame_inbound};

pub fn find_lua_ls() -> Option<PathBuf> {
    if let Ok(p) = which::which("lua-language-server") {
        return Some(p);
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    [
        home.map(|h| h.join(".local/bin/lua-language-server")),
        Some(PathBuf::from("/opt/homebrew/bin/lua-language-server")),
        Some(PathBuf::from("/usr/local/bin/lua-language-server")),
    ]
    .into_iter()
    .flatten()
    .find(|candidate| candidate.exists())
}

pub struct LspClient {
    pub child: Child,
    pub stdin: std::process::ChildStdin,
    pub stdout_rx: mpsc::Receiver<LspMessage>,
    pub pending: Arc<Mutex<std::collections::HashMap<i64, mpsc::SyncSender<LspMessage>>>>,
    pub next_id: AtomicI64,
    pub workspace_root: PathBuf,
    pub capabilities: Option<serde_json::Value>,
}

pub fn spawn(workspace_root: &Path) -> Result<LspClient, LspError> {
    let Some(exe) = find_lua_ls() else {
        return Err(LspError::NotFound);
    };
    let mut child = Command::new(exe)
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| LspError::Spawn)?;

    let stdin = child.stdin.take().ok_or(LspError::Spawn)?;
    let stdout = child.stdout.take().ok_or(LspError::Spawn)?;

    let (tx, rx) = mpsc::channel();
    let pending: Arc<Mutex<std::collections::HashMap<i64, mpsc::SyncSender<LspMessage>>>> =
        Arc::new(Mutex::new(std::collections::HashMap::new()));

    let pending_for_reader = Arc::clone(&pending);
    std::thread::spawn(move || {
        use std::io::{BufReader, Read};
        let mut reader = BufReader::new(stdout);
        loop {
            match frame_inbound(reader.by_ref()) {
                Ok(Some(msg)) => {
                    if let Some(id) = msg.json.get("id").and_then(|i| i.as_i64()) {
                        if let Ok(mut map) = pending_for_reader.lock() {
                            if let Some(tx) = map.remove(&id) {
                                let _ = tx.send(msg);
                                continue;
                            }
                        }
                    }
                    let _ = tx.send(msg);
                }
                Ok(None) => break,
                Err(_) => break,
            }
        }
    });

    let mut client = LspClient {
        child,
        stdin,
        stdout_rx: rx,
        pending,
        next_id: AtomicI64::new(1),
        workspace_root: workspace_root.to_path_buf(),
        capabilities: None,
    };

    client.initialize_handshake().map_err(|_| LspError::Spawn)?;
    Ok(client)
}

impl LspClient {
    fn next_request_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    pub fn initialize_handshake(&mut self) -> std::io::Result<()> {
        use std::io::Write;
        let root_uri = format!("file://{}", self.workspace_root.display());
        let id = self.next_request_id();
        let init = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": root_uri,
                "capabilities": {
                    "textDocument": {
                        "synchronization": { "didSave": true },
                        "completion": { "completionItem": { "snippetSupport": false } }
                    }
                }
            }
        });
        let body = serde_json::to_vec(&init).map_err(std::io::Error::other)?;
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len())?;
        self.stdin.write_all(&body)?;
        self.stdin.flush()?;
        let (tx, rx) = mpsc::sync_channel(1);
        if let Ok(mut map) = self.pending.lock() {
            map.insert(id, tx);
        }
        let response = rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "initialize timeout"))?;
        self.capabilities = response.json.get("result").cloned();

        let initialized = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });
        let body = serde_json::to_vec(&initialized).map_err(std::io::Error::other)?;
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len())?;
        self.stdin.write_all(&body)?;
        self.stdin.flush()?;

        let did_change_config = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "workspace/didChangeConfiguration",
            "params": { "settings": {} }
        });
        let body = serde_json::to_vec(&did_change_config).map_err(std::io::Error::other)?;
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len())?;
        self.stdin.write_all(&body)?;
        self.stdin.flush()?;
        Ok(())
    }

    pub fn send_notification(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> std::io::Result<()> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        write_lsp_message(&mut self.stdin, &msg)
    }

    pub fn send_request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> std::io::Result<i64> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        write_lsp_message(&mut self.stdin, &msg)?;
        Ok(id)
    }

    pub fn shutdown(&mut self) -> std::io::Result<()> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": self.next_request_id(),
            "method": "shutdown",
            "params": null
        });
        write_lsp_message(&mut self.stdin, &msg)?;
        let exit = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        });
        write_lsp_message(&mut self.stdin, &exit)?;
        let _ = self.child.wait();
        Ok(())
    }
}

fn write_lsp_message<W: std::io::Write>(w: &mut W, msg: &serde_json::Value) -> std::io::Result<()> {
    let body = serde_json::to_vec(msg).map_err(std::io::Error::other)?;
    write!(w, "Content-Length: {}\r\n\r\n", body.len())?;
    w.write_all(&body)?;
    w.flush()?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct LspDiagnostic {
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub severity: crate::json_path::Severity,
    pub message: String,
}

#[derive(Debug)]
pub enum LspError {
    NotFound,
    Spawn,
}

impl std::fmt::Display for LspError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LspError::NotFound => write!(f, "lua-language-server not found on PATH"),
            LspError::Spawn => write!(f, "failed to spawn lua-language-server"),
        }
    }
}

impl std::error::Error for LspError {}
