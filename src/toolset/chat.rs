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
    pub fn new(host: Arc<MCPHost>) -> Self {
        Self {
            host,
        }
    }
}

#[async_trait::async_trait]
impl Toolset for ChatTools {
    fn get_system_prompt(&self) -> String {
        "You are a helpful assistant. 
You have access to tools which you can call to help the user in the user's task.
====
TOOL USE

You have access to a set of tools that are executed upon the user's approval.
You can use one tool per message, and will receive the result of that tool use in the user's response. 
You use tools step-by-step to accomplish a given task, with each tool use informed by the result of the previous tool use.
        ".into()
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
