use anyhow::bail;
use dioxus::logger::tracing::{info, warn};
use futures::StreamExt as _;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{
    spawn,
    sync::mpsc::{self, Receiver},
};

#[derive(Clone)]
pub struct LlmClient {
    api_url: String,
    api_key: String,
    client: Client,
}

impl LlmClient {
    pub fn new(api_url: String, api_key: String) -> Self {
        Self {
            api_url,
            api_key,
            client: Client::new(),
        }
    }

    pub async fn models(&self) -> anyhow::Result<ModelsResponse> {
        let res = self
            .client
            .get(format!("{}/models", &self.api_url))
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .send()
            .await?;
        if !res.status().is_success() {
            let status = res.status().clone();
            let body = res.text().await?;
            bail!("Request failed: {} - {}", status, body);
        }

        Ok(res.json().await?)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn stream(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> anyhow::Result<Receiver<StreamEvent>> {
        let res = self
            .client
            .post(format!("{}/chat/completions", &self.api_url))
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "stream": true,
                "messages": messages,
                "tools": tools,
            }))
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status().clone();
            let body = res.text().await?;
            bail!("Request failed: {} - {}", status, body);
        }

        let (tx, rx) = mpsc::channel::<StreamEvent>(32);

        spawn(async move {
            let mut stream = res.bytes_stream();
            while let Some(item) = stream.next().await {
                let chunk = match item {
                    Ok(x) => x,
                    Err(e) => {
                        warn!("Response stream error: {e:?}");
                        return;
                    },
                };
                let text = String::from_utf8_lossy(&chunk);
                for line in text.lines() {
                    if !line.starts_with("data: ") {
                        continue;
                    }
                    let data = &line[6..];
                    if data == "[DONE]" {
                        info!("\n-- Stream complete --");
                        return;
                    }
                    if let Ok(event) = serde_json::from_str::<StreamEvent>(data) {
                        if let Err(e) = tx.send(event).await {
                            warn!("Could not send response event: {e:?}");
                            return;
                        }
                    }
                }
            }
        });

        Ok(rx)
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn stream(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> anyhow::Result<Receiver<StreamEvent>> {
        use wasm_bindgen_futures::spawn_local;
        let res = self
            .client
            .post(format!("{}/chat/completions", &self.api_url))
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "stream": true,
                "messages": messages,
                "tools": tools,
            }))
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status().clone();
            let body = res.text().await?;
            bail!("Request failed: {} - {}", status, body);
        }

        let (tx, rx) = mpsc::channel::<StreamEvent>(32);

        spawn_local(async move {
            let mut stream = res.bytes_stream();
            while let Some(item) = stream.next().await {
                let chunk = match item {
                    Ok(x) => x,
                    Err(e) => {
                        warn!("Response stream error: {e:?}");
                        return;
                    },
                };
                let text = String::from_utf8_lossy(&chunk);
                for line in text.lines() {
                    if !line.starts_with("data: ") {
                        continue;
                    }
                    let data = &line[6..];
                    if data == "[DONE]" {
                        info!("\n-- Stream complete --");
                        return;
                    }
                    if let Ok(event) = serde_json::from_str::<StreamEvent>(data) {
                        if let Err(e) = tx.send(event).await {
                            warn!("Could not send response event: {e:?}");
                            return;
                        }
                    }
                }
            }
        });

        Ok(rx)
    }
}

#[derive(Debug, Deserialize)]
pub struct ModelsResponse {
    pub data: Vec<Model>,
}

#[derive(Debug, Deserialize)]
pub struct Model {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    System {
        content: String,
    },
    User {
        content: Vec<ContentPart>,
    },
    Assistant {
        content: Option<String>,
    },
    Tool {
        tool_call_id: String,
        content: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct Tool {
    pub r#type: String,
    pub function: Function,
}

#[derive(Debug, Serialize)]
pub struct Function {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct StreamEvent {
    pub id: String,
    pub object: String,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolCallDelta {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub function: Option<FunctionDelta>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FunctionDelta {
    pub name: Option<String>,
    pub arguments: Option<String>,
}
