use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_ollama_url")]
    pub ollama_url: String,
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_model() -> String {
    "mistral".to_string()
}

fn default_ollama_url() -> String {
    "http://localhost:11434".to_string()
}

fn default_language() -> String {
    "en".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: default_model(),
            ollama_url: default_ollama_url(),
            language: default_language(),
        }
    }
}

pub fn ensure_config_dir() {
    if let Ok(home) = std::env::var("HOME") {
        let dir = PathBuf::from(home).join(".config").join("ta");
        let _ = std::fs::create_dir_all(&dir);
    }
}

fn config_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".config").join("ta").join("config.toml"))
}

fn parse(content: &str) -> Config {
    toml::from_str(content).unwrap_or_default()
}

pub fn load() -> Config {
    let path = match config_path() {
        Some(p) => p,
        None => return Config::default(),
    };
    match std::fs::read_to_string(&path) {
        Ok(content) => parse(&content),
        Err(_) => Config::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.model, "mistral");
        assert_eq!(cfg.ollama_url, "http://localhost:11434");
        assert_eq!(cfg.language, "en");
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
model = "llama3"
ollama_url = "http://192.168.1.1:11434"
language = "ja"
"#;
        let cfg = parse(toml);
        assert_eq!(cfg.model, "llama3");
        assert_eq!(cfg.ollama_url, "http://192.168.1.1:11434");
        assert_eq!(cfg.language, "ja");
    }

    #[test]
    fn test_parse_partial_config_uses_defaults() {
        let toml = r#"model = "llama3""#;
        let cfg = parse(toml);
        assert_eq!(cfg.model, "llama3");
        assert_eq!(cfg.ollama_url, "http://localhost:11434");
        assert_eq!(cfg.language, "en");
    }

    #[test]
    fn test_parse_malformed_toml_uses_defaults() {
        let cfg = parse("this is not valid toml ][[[");
        assert_eq!(cfg.model, "mistral");
        assert_eq!(cfg.ollama_url, "http://localhost:11434");
    }

    #[test]
    fn test_load_nonexistent_home_returns_default() {
        std::env::set_var("HOME", "/nonexistent/path/that/does/not/exist/xyz");
        let cfg = load();
        assert_eq!(cfg.model, "mistral");
        assert_eq!(cfg.ollama_url, "http://localhost:11434");
    }
}
