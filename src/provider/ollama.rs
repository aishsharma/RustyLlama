use super::*;
use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub base_url: String,
    pub timeout: u64,
}

#[derive(Debug, Deserialize)]
struct OllamaModelDetails {
    #[serde(rename = "quantization_level")]
    quantization_level: Option<String>,
    #[serde(rename = "context_length")]
    context_length: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
    size: Option<u64>,
    details: Option<OllamaModelDetails>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelsResponse {
    models: Vec<OllamaModel>,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            timeout: 30,
        }
    }
}

pub struct OllamaProvider {
    client: Client,
    config: OllamaConfig,
}

impl OllamaProvider {
    pub fn new(config: OllamaConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .build()
            .unwrap();
        Self { client, config }
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn list_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/api/models", self.config.base_url);
        let response = self.client.get(&url).send().await?;
        let data: OllamaModelsResponse = response.json().await?;
        let models = data
            .models
            .into_iter()
            .map(|m| ModelInfo {
                name: m.name,
                size: m.size.map(|s| format!("{} bytes", s)),
                quantization: m.details.and_then(|details| details.quantization_level),
                context_length: m.details.and_then(|details| details.context_length),
            })
            .collect();
        Ok(models)
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<BoxStream<'static, Result<ChatResponse, Box<dyn std::error::Error + Send + Sync>>>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/api/chat", self.config.base_url);
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        let body = response.text().await?;
        let chat_response = ChatResponse {
            message: Some(Message {
                role: "assistant".to_string(),
                content: body,
            }),
            done: true,
            done_reason: None,
        };

        Ok(stream::once(async move { Ok(chat_response) }).boxed())
    }

    async fn cancel(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Ollama doesn't have a direct cancel API yet.
        Ok(())
    }

    async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/api/models", self.config.base_url);
        let response = self.client.get(&url).send().await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err("Health check failed".into())
        }
    }
}