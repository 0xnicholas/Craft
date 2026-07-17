use std::path::Path;

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub provider: String,
    pub api_base: String,
    pub model: String,
    pub api_key: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        let api_key = std::env::var("CRAFT_LLM_API_KEY").unwrap_or_default();
        Self {
            provider: "openai".into(),
            api_base: "https://api.openai.com/v1".into(),
            model: "gpt-4o".into(),
            api_key,
        }
    }
}

impl AgentConfig {
    pub fn load(root: &Path) -> Self {
        let mut config = Self::default();
        let toml_path = root.join("craft.toml");
        if let Ok(content) = std::fs::read_to_string(&toml_path) {
            if let Ok(parsed) = content.parse::<toml::Table>() {
                if let Some(agent) = parsed.get("agent") {
                    if let Some(v) = agent.get("provider").and_then(|v| v.as_str()) {
                        config.provider = v.to_string();
                    }
                    if let Some(v) = agent.get("api_base").and_then(|v| v.as_str()) {
                        config.api_base = v.to_string();
                    }
                    if let Some(v) = agent.get("model").and_then(|v| v.as_str()) {
                        config.model = v.to_string();
                    }
                    if let Some(env_var) = agent.get("api_key_env").and_then(|v| v.as_str()) {
                        if let Ok(key) = std::env::var(env_var) {
                            config.api_key = key;
                        }
                    }
                }
            }
        }
        if config.api_key.is_empty() {
            config.api_key = std::env::var("CRAFT_LLM_API_KEY").unwrap_or_default();
        }
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_no_config() {
        temp_env::with_var("CRAFT_LLM_API_KEY", None::<&str>, || {
            let config = AgentConfig::default();
            assert_eq!(config.api_base, "https://api.openai.com/v1");
            assert_eq!(config.model, "gpt-4o");
            assert!(config.api_key.is_empty());
        });
    }

    #[test]
    fn loads_from_env_var() {
        temp_env::with_var("CRAFT_LLM_API_KEY", Some("sk-test"), || {
            let config = AgentConfig::default();
            assert_eq!(config.api_key, "sk-test");
        });
    }
}
