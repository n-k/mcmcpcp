use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub id: Option<u32>,
    pub provider: ProviderSettings,
    pub last_chat_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "id", rename_all = "lowercase")]
pub enum ProviderSettings {
    OpenRouter {
        api_key: String,
        model: Option<String>,
    },
    Ollama {
        api_url: String,
        model: Option<String>,
    },
}

impl ProviderSettings {
    pub fn is_configured(&self) -> bool {
        match &self {
            ProviderSettings::OpenRouter { api_key, model } => {
                !api_key.is_empty() && model.is_some()
            }
            ProviderSettings::Ollama { api_url, model } => !api_url.is_empty() && model.is_some(),
        }
    }

    pub fn get_api_url(&self) -> String {
        match &self {
            ProviderSettings::OpenRouter { .. } => "https://openrouter.ai/api/v1".to_string(),
            ProviderSettings::Ollama { api_url, .. } => api_url.clone(),
        }
    }

    pub fn get_api_key(&self) -> Option<String> {
        match &self {
            ProviderSettings::OpenRouter { api_key, .. } => Some(api_key.clone()),
            ProviderSettings::Ollama { .. } => None,
        }
    }

    pub fn get_model(&self) -> Option<String> {
        match &self {
            ProviderSettings::OpenRouter { model, .. } => model.clone(),
            ProviderSettings::Ollama { model, .. } => model.clone(),
        }
    }
}
