use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, time::Duration};
use tokio::sync::RwLock;

use crate::mcp::server::{McpServer, ServerSpec, Tool};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub server_id: String,
    pub tool: Tool,
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

pub struct Host {
    pub servers: RwLock<HashMap<String, McpServer>>,
    #[allow(unused)]
    pub request_timeout: Duration,
    #[allow(unused)]
    pub startup_timeout: Duration,
}

impl Host {
    pub fn new(request_timeout: Duration, startup_timeout: Duration) -> Self {
        Self {
            servers: RwLock::new(HashMap::new()),
            request_timeout,
            startup_timeout,
        }
    }

    #[allow(unused)]
    pub async fn add_server(&self, spec: ServerSpec) -> Result<()> {
        let server =
            McpServer::spawn(spec.clone(), self.request_timeout, self.startup_timeout).await?;
        self.servers.write().await.insert(spec.id, server);
        Ok(())
    }

    pub async fn list_tools(&self) -> Vec<ToolDescriptor> {
        let mut res = vec![];
        for (id, s) in self.servers.read().await.iter() {
            let tools = s.tool_cache.lock().await.clone();
            let ts: Vec<ToolDescriptor> = tools
                .into_iter()
                .map(move |t| ToolDescriptor {
                    server_id: id.clone(),
                    tool: t,
                })
                .collect();
            res.extend(ts);
        }
        res
    }

    #[allow(unused)]
    pub async fn invoke(&self, server_id: &str, method: &str, params: Value) -> Result<Value> {
        let servers = self.servers.read().await;
        let s = servers
            .get(server_id)
            .ok_or_else(|| anyhow::anyhow!("unknown server {server_id}"))?;
        s.rpc_call(method, params).await
    }

    pub async fn tool_call(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<ToolResult> {
        let servers = self.servers.read().await;
        let s = servers
            .get(server_id)
            .ok_or_else(|| anyhow::anyhow!("unknown server {server_id}"))?;
        let params = json!({
            "name": tool_name,
            "arguments": arguments,
        });
        let result = s.rpc_call("tools/call", params).await?;
        serde_json::from_value(result).map_err(|e| e.into())
    }
}
