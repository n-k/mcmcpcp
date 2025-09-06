use std::{collections::HashMap, sync::Arc, time::Duration};

use serde_json::Value;

use crate::{
    mcp::{
        fetch::FetchMcpServer,
        host::{MCPHost, MCPServer},
    },
    toolset::Toolset,
};

#[derive(Clone)]
pub struct ChatTools {
    pub host: Arc<MCPHost>,
}

impl ChatTools {
    #[allow(unused)]
    pub fn new() -> Self {
        let mut servers: HashMap<String, Box<dyn MCPServer>> = HashMap::new();
        servers.insert("fetch".into(), Box::new(FetchMcpServer {}));
        let host =
            MCPHost::new_with_tools(servers, Duration::from_secs(10), Duration::from_secs(10));
        Self {
            host: Arc::new(host),
        }
    }
}

#[async_trait::async_trait]
impl Toolset for ChatTools {
    fn get_system_prompt(&self) -> String {
        "You are a helpful assistant. 
        You have access to tools which you can call to help the user in the user's task."
            .into()
    }

    fn get_mcp_host(&self) -> Arc<MCPHost> {
        self.host.clone()
    }

    async fn get_state(&self) -> Value {
        Value::Null
    }

    async fn get_markdown_repr(&self) -> Option<String> {
        None
    }
}
