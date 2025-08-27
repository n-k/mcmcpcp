use std::{collections::HashMap, time::Duration};
use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::mcp::server::{McpServer, ServerSpec, Tool};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub server_id: String,
    pub tool: Tool, // opaque; server-defined
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
        let server = McpServer::spawn(spec.clone(), self.request_timeout, self.startup_timeout).await?;
        self.servers.write().insert(spec.id, server);
        Ok(())
    }

    pub fn list_tools(&self) -> Vec<ToolDescriptor> {
        self.servers.read().iter().flat_map(|(id, s)| {
            let id = id.clone();
            let tools = s.tool_cache.lock().clone();
            tools.into_iter().map(move |t| ToolDescriptor { server_id: id.clone(), tool: t })
        }).collect()
    }

    #[allow(unused)]
    pub async fn invoke(&self, server_id: &str, method: &str, params: Value) -> Result<Value> {
        let servers = self.servers.read();
        let s = servers.get(server_id).ok_or_else(|| anyhow::anyhow!("unknown server {server_id}"))?;
        s.rpc_call(method, params).await
    }
}
