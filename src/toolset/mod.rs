use std::sync::Arc;

use serde_json::Value;

use crate::mcp::host::MCPHost;

pub mod chat;
pub mod story;

#[async_trait::async_trait]
pub trait Toolset {
    fn get_system_prompt(&self) -> String;

    fn get_mcp_host(&self) -> Arc<MCPHost>;

    async fn get_state(&self) -> Value;

    async fn get_markdown_repr(&self) -> Option<String>;
}
