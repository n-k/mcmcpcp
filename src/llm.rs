//! Language Learning Model (LLM) client implementation for MCMCPCP.
//! 
//! This module provides a client for communicating with OpenAI-compatible LLM APIs,
//! supporting both streaming and non-streaming responses. It handles message formatting,
//! tool calling, and response parsing according to the OpenAI API specification.
//! 
//! The client supports both native and WASM targets, with appropriate async runtime
//! handling for each platform.

use anyhow::bail;
use dioxus::logger::tracing::{info, warn};
use futures::StreamExt as _;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{
    spawn,
    sync::mpsc::{self, Receiver},
};

/// HTTP client for communicating with LLM APIs.
/// 
/// Supports OpenAI-compatible APIs and handles authentication, request formatting,
/// and response streaming. The client is designed to work with various LLM providers
/// that implement the OpenAI API specification.
#[derive(Clone)]
pub struct LlmClient {
    /// Base URL for the LLM API (e.g., "https://api.openai.com/v1")
    api_url: String,
    /// API key for authentication
    api_key: String,
    /// HTTP client for making requests
    client: Client,
}

impl LlmClient {
    /// Creates a new LLM client with the specified API URL and key.
    /// 
    /// # Arguments
    /// * `api_url` - Base URL for the LLM API
    /// * `api_key` - API key for authentication
    pub fn new(api_url: String, api_key: String) -> Self {
        Self {
            api_url,
            api_key,
            client: Client::new(),
        }
    }

    /// Retrieves the list of available models from the LLM API.
    /// 
    /// Makes a GET request to the `/models` endpoint to fetch all available
    /// models that can be used for chat completions.
    /// 
    /// # Returns
    /// A `ModelsResponse` containing the list of available models, or an error
    /// if the request fails or the API returns an error status.
    pub async fn models(&self) -> anyhow::Result<ModelsResponse> {
        let res = self
            .client
            .get(format!("{}/models", &self.api_url))
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .send()
            .await?;
            
        // Check for HTTP error status and provide detailed error information
        if !res.status().is_success() {
            let status = res.status().clone();
            let body = res.text().await?;
            bail!("Request failed: {} - {}", status, body);
        }

        Ok(res.json().await?)
    }

    /// Creates a streaming chat completion request (native platforms only).
    /// 
    /// Sends a chat completion request with streaming enabled, allowing real-time
    /// processing of the LLM's response as it's generated. This is useful for
    /// providing immediate feedback to users and handling tool calls as they occur.
    /// 
    /// # Arguments
    /// * `model` - The model ID to use for completion
    /// * `messages` - Conversation history and context
    /// * `tools` - Available tools that the LLM can call
    /// 
    /// # Returns
    /// A receiver channel that yields `StreamEvent`s as the response is generated,
    /// or an error if the request fails.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn stream(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> anyhow::Result<Receiver<StreamEvent>> {
        // Send the streaming chat completion request
        let res = self
            .client
            .post(format!("{}/chat/completions", &self.api_url))
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "stream": true,        // Enable streaming response
                "messages": messages,
                "tools": tools,
                "max_tokens": 2048,    // Limit response length
            }))
            .send()
            .await?;

        // Check for HTTP error status
        if !res.status().is_success() {
            let status = res.status().clone();
            let body = res.text().await?;
            bail!("Request failed: {} - {}", status, body);
        }

        // Create a channel for streaming events with a buffer of 32 items
        let (tx, rx) = mpsc::channel::<StreamEvent>(32);

        // Spawn a task to process the streaming response
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
                
                // Convert bytes to text and process line by line
                let text = String::from_utf8_lossy(&chunk);
                for line in text.lines() {
                    // Skip lines that don't start with "data: " (SSE format)
                    if !line.starts_with("data: ") {
                        continue;
                    }
                    let data = &line[6..]; // Remove "data: " prefix
                    
                    // Check for stream completion marker
                    if data == "[DONE]" {
                        info!("\n-- Stream complete --");
                        return;
                    }
                    
                    // Parse and send the stream event
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

    /// Creates a streaming chat completion request (WASM platforms only).
    /// 
    /// Similar to the native version but uses `spawn_local` for WASM compatibility.
    /// This version omits the `max_tokens` parameter as it may not be supported
    /// by all WASM-compatible LLM providers.
    /// 
    /// # Arguments
    /// * `model` - The model ID to use for completion
    /// * `messages` - Conversation history and context
    /// * `tools` - Available tools that the LLM can call
    /// 
    /// # Returns
    /// A receiver channel that yields `StreamEvent`s as the response is generated,
    /// or an error if the request fails.
    #[cfg(target_arch = "wasm32")]
    pub async fn stream(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> anyhow::Result<Receiver<StreamEvent>> {
        use wasm_bindgen_futures::spawn_local;
        
        // Send the streaming chat completion request
        let res = self
            .client
            .post(format!("{}/chat/completions", &self.api_url))
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "stream": true,        // Enable streaming response
                "messages": messages,
                "tools": tools,
                "max_tokens": 2048,
            }))
            .send()
            .await?;

        // Check for HTTP error status
        if !res.status().is_success() {
            let status = res.status().clone();
            let body = res.text().await?;
            bail!("Request failed: {} - {}", status, body);
        }

        // Create a channel for streaming events with a buffer of 32 items
        let (tx, rx) = mpsc::channel::<StreamEvent>(32);

        // Spawn a local task to process the streaming response (WASM-compatible)
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
                
                // Convert bytes to text and process line by line
                let text = String::from_utf8_lossy(&chunk);
                for line in text.lines() {
                    // Skip lines that don't start with "data: " (SSE format)
                    if !line.starts_with("data: ") {
                        continue;
                    }
                    let data = &line[6..]; // Remove "data: " prefix
                    
                    // Check for stream completion marker
                    if data == "[DONE]" {
                        info!("\n-- Stream complete --");
                        return;
                    }
                    
                    // Parse and send the stream event
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

/// Response structure for the models API endpoint.
/// 
/// Contains a list of available models that can be used for chat completions.
#[derive(Debug, Deserialize)]
pub struct ModelsResponse {
    /// List of available models
    pub data: Vec<Model>,
}

/// Represents a single model available from the LLM API.
/// 
/// Contains basic information about a model that can be used for completions.
#[derive(Debug, Deserialize)]
pub struct Model {
    /// Unique identifier for the model (e.g., "gpt-4", "claude-3-sonnet")
    pub id: String,
}

/// Represents a message in a conversation with an LLM.
/// 
/// Messages have different roles (system, user, assistant, tool) and contain
/// content appropriate for each role. This follows the OpenAI API message format.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    /// System message that sets context and instructions for the LLM
    System {
        /// The system prompt content
        content: String,
    },
    /// User message containing the human's input
    User {
        /// List of content parts (text, images, etc.)
        content: Vec<ContentPart>,
    },
    /// Assistant message containing the LLM's response
    Assistant {
        /// The assistant's response text (None if only tool calls)
        content: Option<String>,
    },
    /// Tool result message containing the output of a tool call
    Tool {
        /// ID of the tool call this result corresponds to
        tool_call_id: String,
        /// The result content from the tool execution
        content: String,
    },
}

/// Represents different types of content that can be included in a user message.
/// 
/// Supports text content and image URLs for multimodal interactions.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// Plain text content
    Text { 
        /// The text content
        text: String 
    },
    /// Image content referenced by URL
    ImageUrl { 
        /// Image URL and metadata
        image_url: ImageUrl 
    },
}

/// Represents an image URL for multimodal content.
/// 
/// Used when including images in user messages for vision-capable models.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ImageUrl {
    /// URL pointing to the image resource
    pub url: String,
}

/// Represents a tool that can be called by the LLM.
/// 
/// Tools allow the LLM to interact with external systems and perform actions
/// beyond text generation. Each tool has a function definition with parameters.
#[derive(Debug, Serialize)]
pub struct Tool {
    /// Type of tool (typically "function")
    pub r#type: String,
    /// Function definition for this tool
    pub function: Function,
}

/// Defines a function that can be called as a tool.
/// 
/// Contains the function name, description, and parameter schema that the LLM
/// uses to understand how to call the function properly.
#[derive(Debug, Serialize)]
pub struct Function {
    /// Name of the function
    pub name: String,
    /// Optional description of what the function does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema defining the function's parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    /// Whether to use strict mode for parameter validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Represents a single event in a streaming response.
/// 
/// Each event contains choices with delta updates that incrementally build
/// the complete response from the LLM.
#[derive(Debug, Deserialize)]
pub struct StreamEvent {
    /// Unique identifier for this stream
    pub id: String,
    /// Type of object (typically "chat.completion.chunk")
    pub object: String,
    /// List of choice deltas in this event
    pub choices: Vec<Choice>,
}

/// Represents a choice delta in a streaming response.
/// 
/// Contains incremental updates to the response content and metadata
/// about the completion status.
#[derive(Debug, Deserialize)]
pub struct Choice {
    /// Index of this choice (typically 0 for single responses)
    pub index: u32,
    /// Delta containing the incremental update
    pub delta: Delta,
    /// Reason why the response finished (if complete)
    pub finish_reason: Option<String>,
}

/// Contains incremental updates in a streaming response.
/// 
/// Each delta may contain partial content, role information, or tool call updates
/// that are combined to build the complete response.
#[derive(Debug, Deserialize)]
pub struct Delta {
    /// Role of the message (if this is the first delta)
    pub role: Option<String>,
    /// Incremental text content
    pub content: Option<String>,
    /// Incremental tool call information
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

/// Represents an incremental update to a tool call in a streaming response.
/// 
/// Tool calls may be streamed in parts, with the function name and arguments
/// being built up over multiple deltas.
#[derive(Debug, Deserialize, Clone)]
pub struct ToolCallDelta {
    /// Unique identifier for this tool call
    pub id: Option<String>,
    /// Type of tool call (typically "function")
    #[serde(rename = "type")]
    pub kind: Option<String>,
    /// Function call details
    pub function: Option<FunctionDelta>,
}

/// Represents incremental updates to a function call in a streaming response.
/// 
/// The function name and arguments may be streamed separately and need to be
/// accumulated to form the complete function call.
#[derive(Debug, Deserialize, Clone)]
pub struct FunctionDelta {
    /// Function name (if this is the first delta for this call)
    pub name: Option<String>,
    /// Incremental function arguments (JSON string)
    pub arguments: Option<String>,
}
