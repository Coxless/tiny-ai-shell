pub mod ollama;

use crate::context::ContextInfo;

pub struct LlmClient {
    pub model: String,
    pub base_url: String,
}

impl LlmClient {
    pub fn new(model: String, base_url: String) -> Self {
        Self { model, base_url }
    }

    pub async fn generate(&self, _input: &str, _context: &ContextInfo) -> anyhow::Result<String> {
        todo!("implement in step 2")
    }

    pub async fn explain(&self, _command: &str) -> anyhow::Result<String> {
        todo!("implement in step 8")
    }

    pub async fn rewrite(
        &self,
        _original_input: &str,
        _command: &str,
        _instruction: &str,
    ) -> anyhow::Result<String> {
        todo!("implement in step 9")
    }
}
