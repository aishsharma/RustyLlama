use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::provider::ollama::OllamaConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ollama: OllamaConfig,
    pub default_model: String,
    pub default_system_prompt: String,
    pub data_dir: PathBuf,
    pub theme: Theme,
    pub font_size: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
    System,
}

impl Default for AppConfig {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("RustyLlama");
        
        Self {
            ollama: OllamaConfig::default(),
            default_model: "llama2".to_string(),
            default_system_prompt: "You are a helpful assistant.".to_string(),
            data_dir,
            theme: Theme::Dark,
            font_size: 14.0,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let config_path = Self::default().data_dir.join("config.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }
    
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&self.data_dir)?;
        let config_path = self.data_dir.join("config.toml");
        let content = toml::to_string(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }
}