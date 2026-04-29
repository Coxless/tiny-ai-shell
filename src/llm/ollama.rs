use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const TIMEOUT_SECS: u64 = 30;

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Deserialize)]
struct GenerateChunk {
    response: Option<String>,
    done: Option<bool>,
    error: Option<String>,
}

pub struct OllamaClient {
    client: Client,
    base_url: String,
    pub model: String,
}

impl OllamaClient {
    pub fn new(base_url: String, model: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Self { client, base_url, model })
    }

    pub async fn check_connectivity(&self) -> Result<()> {
        self.client
            .get(&self.base_url)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    anyhow!("Ollama is not running. Start with: ollama serve")
                } else {
                    anyhow!("Failed to connect to Ollama: {}", e)
                }
            })?;
        Ok(())
    }

    pub async fn generate(&self, prompt: &str) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url);
        let request = GenerateRequest {
            model: &self.model,
            prompt,
            stream: true,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    anyhow!("Ollama is not running. Start with: ollama serve")
                } else {
                    anyhow!("Failed to connect to Ollama: {}", e)
                }
            })?;

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            let body: serde_json::Value =
                response.json().await.unwrap_or(serde_json::Value::Null);
            let msg = body
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("");
            if msg.contains("model") || msg.contains("not found") {
                return Err(anyhow!(
                    "Model '{}' not found. Pull with: ollama pull {}",
                    self.model,
                    self.model
                ));
            }
            return Err(anyhow!("Ollama API error ({})", status));
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Ollama API error ({}): {}", status, body));
        }

        let mut stream = response.bytes_stream();
        let mut result = String::new();
        let mut buffer = Vec::new();

        'outer: while let Some(chunk) = stream.next().await {
            let bytes = chunk.context("Failed to read response stream")?;
            buffer.extend_from_slice(&bytes);

            while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                let line_bytes: Vec<u8> = buffer.drain(..=pos).collect();
                let line = String::from_utf8_lossy(&line_bytes);
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Ok(chunk) = serde_json::from_str::<GenerateChunk>(line) {
                    if let Some(error) = chunk.error {
                        if error.contains("not found") || error.contains("model") {
                            return Err(anyhow!(
                                "Model '{}' not found. Pull with: ollama pull {}",
                                self.model,
                                self.model
                            ));
                        }
                        return Err(anyhow!("Ollama error: {}", error));
                    }
                    if let Some(resp) = chunk.response {
                        result.push_str(&resp);
                    }
                    if chunk.done == Some(true) {
                        break 'outer;
                    }
                }
            }
        }

        Ok(clean_command(&result))
    }
}

fn clean_command(s: &str) -> String {
    let s = s.trim();
    let s = if s.starts_with("```") {
        let without_fence = s.trim_start_matches('`');
        let body = match without_fence.find('\n') {
            Some(nl) => &without_fence[nl + 1..],
            None => without_fence,
        };
        body.trim_end_matches('`').trim()
    } else {
        s
    };
    s.trim_matches('`').trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_check_connectivity_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_body("Ollama is running")
            .create_async()
            .await;

        let client = OllamaClient::new(server.url(), "mistral".to_string()).unwrap();
        assert!(client.check_connectivity().await.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_check_connectivity_refused() {
        let client =
            OllamaClient::new("http://127.0.0.1:1".to_string(), "mistral".to_string()).unwrap();
        let err = client.check_connectivity().await.unwrap_err();
        assert!(
            err.to_string().contains("Ollama is not running"),
            "expected 'Ollama is not running' in: {}",
            err
        );
    }

    #[test]
    fn test_clean_command_plain() {
        assert_eq!(clean_command("ls -la"), "ls -la");
    }

    #[test]
    fn test_clean_command_inline_backticks() {
        assert_eq!(clean_command("`ls -la`"), "ls -la");
    }

    #[test]
    fn test_clean_command_code_block_with_lang() {
        assert_eq!(clean_command("```bash\nls -la\n```"), "ls -la");
    }

    #[test]
    fn test_clean_command_code_block_no_lang() {
        assert_eq!(clean_command("```\nls -la\n```"), "ls -la");
    }

    #[test]
    fn test_clean_command_trims_whitespace() {
        assert_eq!(clean_command("  ls -la  \n"), "ls -la");
    }

    #[tokio::test]
    async fn test_generate_success_streaming() {
        let mut server = Server::new_async().await;
        let body = concat!(
            "{\"model\":\"mistral\",\"response\":\"ls\",\"done\":false}\n",
            "{\"model\":\"mistral\",\"response\":\" -la\",\"done\":false}\n",
            "{\"model\":\"mistral\",\"response\":\"\",\"done\":true}\n",
        );
        let mock = server
            .mock("POST", "/api/generate")
            .with_status(200)
            .with_header("content-type", "application/x-ndjson")
            .with_body(body)
            .create_async()
            .await;

        let client = OllamaClient::new(server.url(), "mistral".to_string()).unwrap();
        let result = client.generate("list files").await.unwrap();
        assert_eq!(result, "ls -la");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_generate_strips_inline_backticks() {
        let mut server = Server::new_async().await;
        let body = "{\"model\":\"mistral\",\"response\":\"`ls -la`\",\"done\":true}\n";
        let mock = server
            .mock("POST", "/api/generate")
            .with_status(200)
            .with_body(body)
            .create_async()
            .await;

        let client = OllamaClient::new(server.url(), "mistral".to_string()).unwrap();
        let result = client.generate("list files").await.unwrap();
        assert_eq!(result, "ls -la");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_generate_model_not_found_404() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/generate")
            .with_status(404)
            .with_body("{\"error\":\"model 'bad-model' not found, try pulling it first\"}")
            .create_async()
            .await;

        let client = OllamaClient::new(server.url(), "bad-model".to_string()).unwrap();
        let err = client.generate("test").await.unwrap_err();
        assert!(
            err.to_string().contains("not found"),
            "expected 'not found' in: {}",
            err
        );
        assert!(
            err.to_string().contains("ollama pull"),
            "expected 'ollama pull' in: {}",
            err
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_generate_error_in_stream() {
        let mut server = Server::new_async().await;
        let body = "{\"error\":\"model 'x' not found\"}\n";
        let mock = server
            .mock("POST", "/api/generate")
            .with_status(200)
            .with_body(body)
            .create_async()
            .await;

        let client = OllamaClient::new(server.url(), "x".to_string()).unwrap();
        let err = client.generate("test").await.unwrap_err();
        assert!(
            err.to_string().contains("not found"),
            "expected 'not found' in: {}",
            err
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_generate_connection_refused() {
        let client =
            OllamaClient::new("http://127.0.0.1:1".to_string(), "mistral".to_string()).unwrap();
        let err = client.generate("test").await.unwrap_err();
        assert!(
            err.to_string().contains("Ollama is not running"),
            "expected 'Ollama is not running' in: {}",
            err
        );
    }
}
