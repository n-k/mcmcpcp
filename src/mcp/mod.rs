// mod api;
mod config;
pub mod host;
mod jsonrpc;
mod server;
#[cfg(not(target_arch = "wasm32"))]
mod transport;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ServerSpec {
    pub id: String,
    pub cmd: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub server_id: String,
    pub tool: McpTool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    pub content: Vec<ToolResultContent>,
    pub is_error: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultContent {
    pub r#type: String,
    pub text: Option<String>,
    pub mime_type: Option<String>,
    pub data: Option<String>,
    pub resource: Option<Value>,
}
