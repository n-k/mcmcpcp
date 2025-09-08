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
You have access to tools which you can call to help the user in the user's task.
====
TOOL USE

You have access to a set of tools that are executed upon the user's approval.
You can use one tool per message, and will receive the result of that tool use in the user's response. 
You use tools step-by-step to accomplish a given task, with each tool use informed by the result of the previous tool use.

# Tool Use Formatting

Tool use is formatted using XML-style tags. The tool name is enclosed in opening and closing tags, 
    and each parameter is similarly enclosed within its own set of tags. 
Here's the structure:

<tool_name>
<parameter1_name>value1</parameter1_name>
<parameter2_name>value2</parameter2_name>
...
</tool_name>

For example:

<fetch>
<url>src/main.js</url>
</url>

Always adhere to this format for the tool use to ensure proper parsing and execution.
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
