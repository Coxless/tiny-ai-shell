pub mod ollama;

use crate::context::ContextInfo;
use anyhow::Result;
use ollama::OllamaClient;

/// Strip newlines and control characters from context values to prevent prompt injection.
fn sanitize_context(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}

pub struct LlmClient {
    pub model: String,
    pub base_url: String,
    pub language: String,
}

/// Resolve display language for explanations.
/// Priority: config override > env var detection > "English"
pub fn resolve_language(config_lang: Option<&str>, env_lang: &str) -> &'static str {
    if let Some(lang) = config_lang {
        return if lang.starts_with("ja") { "Japanese" } else { "English" };
    }
    if env_lang.starts_with("ja") { "Japanese" } else { "English" }
}

impl LlmClient {
    pub fn new(model: String, base_url: String, language: String) -> Self {
        Self { model, base_url, language }
    }

    fn ollama_client(&self) -> Result<OllamaClient> {
        OllamaClient::new(self.base_url.clone(), self.model.clone())
    }

    fn build_generate_prompt(&self, input: &str, context: &ContextInfo) -> String {
        let mut prompt = format!(
            "System:\n\
             You are a CLI assistant.\n\
             Rules:\n\
             - Output exactly ONE shell command\n\
             - No explanation, no markdown, no code blocks\n\
             - Avoid dangerous commands\n\
             - Prefer safe flags\n\
             - Consider the OS: {os}\n\
             - Current directory: {pwd}",
            os = sanitize_context(&context.os),
            pwd = sanitize_context(&context.pwd),
        );

        if !context.files.is_empty() {
            let safe_files: Vec<String> = context.files.iter().map(|f| sanitize_context(f)).collect();
            prompt.push_str(&format!("\n- Files: {}", safe_files.join(", ")));
        }

        if let Some(branch) = &context.branch {
            prompt.push_str(&format!("\n- Git branch: {}", sanitize_context(branch)));
        }

        prompt.push_str(&format!("\n\nUser:\n{}", sanitize_context(input)));
        prompt
    }

    pub async fn check_connectivity(&self) -> Result<()> {
        self.ollama_client()?.check_connectivity().await
    }

    pub async fn generate(&self, input: &str, context: &ContextInfo) -> Result<String> {
        let prompt = self.build_generate_prompt(input, context);
        self.ollama_client()?.generate(&prompt).await
    }

    fn build_explain_prompt(&self, command: &str) -> String {
        format!(
            "Explain this shell command concisely in {language}:\n\
             {command}\n\
             \n\
             Rules:\n\
             - One or two sentences maximum\n\
             - Focus on what it does, not how flags work in detail\n\
             - Use plain language",
            language = sanitize_context(&self.language),
            command = sanitize_context(command),
        )
    }

    pub async fn explain(&self, command: &str) -> Result<String> {
        let prompt = self.build_explain_prompt(command);
        self.ollama_client()?.generate(&prompt).await
    }

    fn build_rewrite_prompt(&self, original_input: &str, command: &str, instruction: &str) -> String {
        format!(
            "Original request: {original_input}\n\
             Generated command: {command}\n\
             User feedback: {instruction}\n\
             \n\
             Generate an improved command based on the feedback.\n\
             Output exactly ONE shell command, no explanation.",
            original_input = sanitize_context(original_input),
            command = sanitize_context(command),
            instruction = sanitize_context(instruction),
        )
    }

    pub async fn rewrite(
        &self,
        original_input: &str,
        command: &str,
        instruction: &str,
    ) -> Result<String> {
        let prompt = self.build_rewrite_prompt(original_input, command, instruction);
        self.ollama_client()?.generate(&prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextInfo;

    fn make_client() -> LlmClient {
        LlmClient::new(
            "mistral".to_string(),
            "http://localhost:11434".to_string(),
            "English".to_string(),
        )
    }

    // --- resolve_language tests (pure function, no env mutation needed) ---

    #[test]
    fn test_resolve_language_config_ja() {
        assert_eq!(resolve_language(Some("ja"), ""), "Japanese");
        assert_eq!(resolve_language(Some("ja_JP.UTF-8"), ""), "Japanese");
    }

    #[test]
    fn test_resolve_language_config_en() {
        assert_eq!(resolve_language(Some("en"), ""), "English");
    }

    #[test]
    fn test_resolve_language_env_ja() {
        assert_eq!(resolve_language(None, "ja_JP.UTF-8"), "Japanese");
    }

    #[test]
    fn test_resolve_language_env_en_fallback() {
        assert_eq!(resolve_language(None, ""), "English");
        assert_eq!(resolve_language(None, "en_US.UTF-8"), "English");
    }

    #[test]
    fn test_resolve_language_config_overrides_env() {
        // config "ja" wins even when env says "en"
        assert_eq!(resolve_language(Some("ja"), "en_US.UTF-8"), "Japanese");
        // config "en" wins even when env says "ja"
        assert_eq!(resolve_language(Some("en"), "ja_JP.UTF-8"), "English");
    }

    // --- prompt tests ---

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
    fn test_prompt_includes_files() {
        let client = make_client();
        let ctx = ContextInfo {
            files: vec!["Cargo.toml".to_string(), "src".to_string()],
            ..Default::default()
        };
        let prompt = client.build_generate_prompt("test", &ctx);
        assert!(prompt.contains("Cargo.toml"));
        assert!(prompt.contains("src"));
    }

    #[test]
    fn test_prompt_includes_git_branch() {
        let client = make_client();
        let ctx = ContextInfo {
            branch: Some("main".to_string()),
            ..Default::default()
        };
        let prompt = client.build_generate_prompt("test", &ctx);
        assert!(prompt.contains("Git branch: main"));
    }

    #[test]
    fn test_prompt_omits_files_section_when_empty() {
        let client = make_client();
        let ctx = ContextInfo::default();
        let prompt = client.build_generate_prompt("test", &ctx);
        assert!(!prompt.contains("Files:"));
        assert!(!prompt.contains("Git branch:"));
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
    fn test_explain_prompt_uses_client_language() {
        let ja_client = LlmClient::new(
            "mistral".to_string(),
            "http://localhost:11434".to_string(),
            "Japanese".to_string(),
        );
        let prompt = ja_client.build_explain_prompt("ls -la");
        assert!(prompt.contains("Japanese"));
    }

    #[test]
    fn test_rewrite_prompt_contains_all_parts() {
        let client = make_client();
        let prompt = client.build_rewrite_prompt("list files", "ls -a", "no hidden files");
        assert!(prompt.contains("list files"));
        assert!(prompt.contains("ls -a"));
        assert!(prompt.contains("no hidden files"));
        assert!(prompt.contains("ONE shell command"));
    }

    #[test]
    fn test_rewrite_prompt_structure() {
        let client = make_client();
        let prompt = client.build_rewrite_prompt("req", "cmd", "fix");
        assert!(prompt.contains("Original request: req"));
        assert!(prompt.contains("Generated command: cmd"));
        assert!(prompt.contains("User feedback: fix"));
    }
}
