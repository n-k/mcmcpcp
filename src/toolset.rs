use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::bail;
use dioxus::logger::tracing::warn;
use serde_json::{json, Value};

use crate::mcp::{
    fetch::FetchMcpServer, host::{MCPHost, MCPServer}, McpTool, ToolResult, ToolResultContent
};

#[async_trait::async_trait]
pub trait Toolset {
    #[allow(unused)]
    fn get_name(&self) -> &str;

    fn get_mcp_host(&self) -> Arc<MCPHost>;

    async fn get_state(&self) -> Value;
}

#[derive(Clone)]
pub struct ChatTools {
    pub host: Arc<MCPHost>,
}

impl ChatTools {
    #[allow(unused)]
    pub fn new() -> Self {
        let mut servers: HashMap<String, Box<dyn MCPServer>> = HashMap::new();
        servers.insert(
            "fetch".into(),
            Box::new(FetchMcpServer {}),
        );
        let host =
            MCPHost::new_with_tools(servers, Duration::from_secs(10), Duration::from_secs(10));
        Self { host: Arc::new(host) }
    }
}

#[async_trait::async_trait]
impl Toolset for ChatTools {
    fn get_name(&self) -> &str {
        "Chat Tools"
    }

    fn get_mcp_host(&self) -> Arc<MCPHost> {
        self.host.clone()
    }

    async fn get_state(&self) -> Value {
        Value::Null
    }
}

#[derive(Clone)]
pub struct StoryWriter {
    pub host: Arc<MCPHost>,
}

impl StoryWriter {
    pub fn new(story: String,) -> Self {
        let mut servers: HashMap<String, Box<dyn MCPServer>> = HashMap::new();
        servers.insert(
            "fetch".into(),
            Box::new(FetchMcpServer {}),
        );
        servers.insert(
            "writer".into(),
            Box::new(StoryWriterMcpServer { story }),
        );
        let host =
            MCPHost::new_with_tools(servers, Duration::from_secs(10), Duration::from_secs(10));
        Self { host: Arc::new(host) }
    }
}

#[async_trait::async_trait]
impl Toolset for StoryWriter {
    fn get_name(&self) -> &str {
        "Story Writer"
    }

    fn get_mcp_host(&self) -> Arc<MCPHost> {
        self.host.clone()
    }

    async fn get_state(&self) -> Value {
        let mut map = self.host.servers.write().await;
        let Some(server) = map.get_mut("writer") else {
            return json!({"story": ""});
        };
        server.rpc("get_value", json!({})).await
            .unwrap_or_else(|e| {
                warn!("Error getting value from MCP server: {e:?}");
                json!({})
            })
    }
}

pub struct StoryWriterMcpServer {
    pub story: String,
}

#[async_trait::async_trait]
impl MCPServer for StoryWriterMcpServer {
    /// Returns the fetch tool definition.
    ///
    /// Provides a single "fetch" tool that can retrieve content from URLs.
    async fn list_tools(&self) -> Vec<McpTool> {
        vec![
            McpTool {
                name: "count_paragraphs".into(),
                description: Some("Count number of paragraphs in story.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            McpTool {
                name: "get_paragraphs".into(),
                description: Some("Get the contents of one or more paragraphs of the story.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "paragraph_number": {
                            "type": "number",
                            "description": "Which paragraph to start from. Note that paragraphs start counting at 0"
                        },
                        "paragraph_count": {
                            "type": "number",
                            "description": "How many paragraphs to get."
                        }
                    },
                    "required": ["paragraph_number"]
                }),
            },
            McpTool {
                name: "rewrite_paragraph".into(),
                description: Some("Rewrite one paragraph completely.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "paragraph_number": {
                            "type": "number",
                            "description": "Which paragraph to rewrite. Note that paragraphs start counting at 0"
                        },
                        "new_contents": {
                            "type": "string",
                            "description": "New contents of the paragraph."
                        }
                    },
                    "required": ["paragraph_number", "new_contents"]
                }),
            },
            McpTool {
                name: "delete_paragraphs".into(),
                description: Some(
                    "Delete one or more paragraphs of the story. 
                Note that all paragraph numbers of the story will change. 
                You must reread the story."
                        .into(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "paragraph_number": {
                            "type": "number",
                            "description": "Which paragraph to start from. Note that paragraphs start counting at 0"
                        },
                        "paragraph_count": {
                            "type": "number",
                            "description": "How many paragraphs to delete. Default is 1"
                        }
                    },
                    "required": ["paragraph_number"]
                }),
            },
        ]
    }

    /// Handles RPC calls for the writer server.
    ///
    /// Currently only supports the "tools/call" method with the "fetch" tool.
    /// The fetch tool retrieves content from the specified URL and returns it as text.
    async fn rpc(&mut self, method: &str, params: Value) -> anyhow::Result<serde_json::Value> {
        // Special method to get value
        if method == "get_value" {
            return Ok(json!({"story": &self.story}));
        }

        // Only support tool calls for this built-in server
        if method != "tools/call" {
            bail!("Error: unknown RPC method {method}");
        }

        // Extract the tool name from parameters
        let name = params
            .get("name")
            .map(|v| v.as_str())
            .flatten()
            .unwrap_or_else(|| "");

        if ![
            "count_paragraphs",
            "get_paragraphs",
            "rewrite_paragraph",
            "delete_paragraphs",
        ]
        .contains(&name)
        {
            bail!("Unknown tool: {name}");
        }

        // Extract tool arguments
        let params = params
            .get("arguments")
            .map(|v| v.clone())
            .unwrap_or_else(|| json!({}));

        let result: ToolResult = if name == "count_paragraphs" {
            self.count_paragraphs()
        } else if name == "get_paragraphs" {
            let idx = params.get("paragraph_number")
                .map(|v| v.clone())
                .unwrap_or_else(|| Value::Number(0.into()))
                .as_number()
                .map(|v| v.clone())
                .unwrap_or_else(|| 0u32.into())
                .as_u64()
                .unwrap() as usize;
            let count = params.get("paragraph_count")
                .map(|v| v.clone())
                .unwrap_or_else(|| Value::Number(0.into()))
                .as_number()
                .map(|v| v.clone())
                .unwrap_or_else(|| 0u32.into())
                .as_u64()
                .unwrap() as usize;
            self.get_paragraphs(idx, count)
        } else if name == "rewrite_paragraph" {
            let idx = params.get("paragraph_number")
                .map(|v| v.clone())
                .unwrap_or_else(|| Value::Number(0.into()))
                .as_number()
                .map(|v| v.clone())
                .unwrap_or_else(|| 0u32.into())
                .as_u64()
                .unwrap() as usize;
            let new_contents = params
                .get("new_contents")
                .map(|v| v.as_str())
                .flatten()
                .unwrap_or_else(|| "");
            self.rewrite_paragraph(idx, new_contents)
        } else if name == "delete_paragraphs" {
            let idx = params.get("paragraph_number")
                .map(|v| v.clone())
                .unwrap_or_else(|| Value::Number(0.into()))
                .as_number()
                .map(|v| v.clone())
                .unwrap_or_else(|| 0u32.into())
                .as_u64()
                .unwrap() as usize;
            let count = params.get("paragraph_count")
                .map(|v| v.clone())
                .unwrap_or_else(|| Value::Number(0.into()))
                .as_number()
                .map(|v| v.clone())
                .unwrap_or_else(|| 0u32.into())
                .as_u64()
                .unwrap() as usize;
            self.delete_paragraphs(idx, count)
        } else {
            ToolResult { 
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some("".to_string()),
                    ..Default::default()
                }], 
                is_error: Some(false),
            }
        };

        Ok(serde_json::to_value(result)?)
    }
}

impl StoryWriterMcpServer {
    fn get_paras(&self) -> Vec<String> {
        self.story.lines().map(|s| s.to_string()).collect()
    }

    fn count_paragraphs(&self) -> ToolResult {
        let paras = self.get_paras();
        let count = paras.len();
        ToolResult { 
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(format!(" There are currently {count} paragraphs in the story.")),
                ..Default::default()
            }], 
            is_error: Some(false),
        }
    }

    fn get_paragraphs(&self, idx: usize, count: usize) -> ToolResult {
        let paras = self.get_paras();
        let paras = &paras.as_slice()[idx .. idx+count];
        ToolResult { 
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(paras.to_vec().join("\n")),
                ..Default::default()
            }], 
            is_error: Some(false),
        }
    }

    fn rewrite_paragraph(&mut self, idx: usize, c: &str) -> ToolResult {
        let mut paras = self.get_paras();
        if idx >= paras.len() {
            paras.push(c.to_string());
        } else {
            paras[idx] = c.to_string();
        }
        self.story = paras.join("\n");
        ToolResult { 
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(format!("OK, rewrote {idx}th paragraph.")),
                ..Default::default()
            }], 
            is_error: Some(false),
        }
    }

    fn delete_paragraphs(&mut self, idx: usize, count: usize) -> ToolResult {
        let mut paras = self.get_paras();
        paras.drain(idx..idx+count);
        self.story = paras.join("\n");
        ToolResult { 
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(format!("OK, deleted {idx}th paragraph.")),
                ..Default::default()
            }], 
            is_error: Some(false),
        }
    }
}
