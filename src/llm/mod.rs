pub mod ollama;

use crate::context::ContextInfo;
use anyhow::Result;
use ollama::OllamaClient;

pub use ollama::resolve_base_url;

pub struct LlmClient {
    pub model: String,
    pub base_url: String,
}

impl LlmClient {
    pub fn new(model: String, base_url: String) -> Self {
        Self { model, base_url }
    }

    fn ollama_client(&self) -> Result<OllamaClient> {
        OllamaClient::new(self.base_url.clone(), self.model.clone())
    }

    fn build_generate_prompt(&self, input: &str, context: &ContextInfo) -> String {
        format!(
            "System:\n\
             You are a CLI assistant.\n\
             Rules:\n\
             - Output exactly ONE shell command\n\
             - No explanation, no markdown, no code blocks\n\
             - Avoid dangerous commands\n\
             - Prefer safe flags\n\
             - Consider the OS: {os}\n\
             - Current directory: {pwd}\n\
             \n\
             User:\n\
             {input}",
            os = context.os,
            pwd = context.pwd,
            input = input,
        )
    }

    pub async fn generate(&self, input: &str, context: &ContextInfo) -> Result<String> {
        let prompt = self.build_generate_prompt(input, context);
        self.ollama_client()?.generate(&prompt).await
    }

    fn detect_language() -> &'static str {
        let lang = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_default();
        if lang.starts_with("ja") {
            "Japanese"
        } else {
            "English"
        }
    }

    fn build_explain_prompt(&self, command: &str) -> String {
        let language = Self::detect_language();
        format!(
            "Explain this shell command concisely in {language}:\n\
             {command}\n\
             \n\
             Rules:\n\
             - One or two sentences maximum\n\
             - Focus on what it does, not how flags work in detail\n\
             - Use plain language",
            language = language,
            command = command,
        )
    }

    pub async fn explain(&self, command: &str) -> Result<String> {
        let prompt = self.build_explain_prompt(command);
        self.ollama_client()?.generate(&prompt).await
    }

    pub async fn rewrite(
        &self,
        _original_input: &str,
        _command: &str,
        _instruction: &str,
    ) -> Result<String> {
        todo!("implement in step 9")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextInfo;

    fn make_client() -> LlmClient {
        LlmClient::new("mistral".to_string(), "http://localhost:11434".to_string())
    }

    #[test]
    fn test_prompt_contains_os_and_pwd() {
        let client = make_client();
        let ctx = ContextInfo {
            os: "linux".to_string(),
            pwd: "/home/user/project".to_string(),
            ..Default::default()
        };
        let prompt = client.build_generate_prompt("list files", &ctx);
        assert!(prompt.contains("linux"));
        assert!(prompt.contains("/home/user/project"));
        assert!(prompt.contains("list files"));
    }

    #[test]
    fn test_prompt_contains_rules() {
        let client = make_client();
        let ctx = ContextInfo::default();
        let prompt = client.build_generate_prompt("test", &ctx);
        assert!(prompt.contains("ONE shell command"));
        assert!(prompt.contains("no markdown"));
        assert!(prompt.contains("no code blocks"));
    }

    #[test]
    fn test_explain_prompt_contains_command() {
        let client = make_client();
        let prompt = client.build_explain_prompt("ls -la");
        assert!(prompt.contains("ls -la"));
        assert!(prompt.contains("One or two sentences maximum"));
        assert!(prompt.contains("plain language"));
    }

    #[test]
    fn test_detect_language_japanese() {
        std::env::set_var("LC_ALL", "ja_JP.UTF-8");
        assert_eq!(LlmClient::detect_language(), "Japanese");
        std::env::remove_var("LC_ALL");
    }

    #[test]
    fn test_detect_language_english_fallback() {
        std::env::remove_var("LC_ALL");
        std::env::remove_var("LANG");
        assert_eq!(LlmClient::detect_language(), "English");
    }
}
