use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub size: Option<String>,
    pub quantization: Option<String>,
    pub context_length: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String, // "user" or "assistant"
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
    pub options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: Option<Message>,
    pub done: bool,
    pub done_reason: Option<String>,
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn list_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error + Send + Sync>>;
    async fn chat_stream(&self, request: ChatRequest) -> Result<futures::stream::BoxStream<'static, Result<ChatResponse, Box<dyn std::error::Error + Send + Sync>>>, Box<dyn std::error::Error + Send + Sync>>;
    async fn cancel(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

impl From<crate::models::Message> for Message {
    fn from(message: crate::models::Message) -> Self {
        Self {
            role: message.role,
            content: message.content,
        }
    }
}

pub mod ollama;