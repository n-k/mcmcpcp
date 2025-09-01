use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};
use std::{collections::HashMap, time::Duration};
use tokio::sync::RwLock;

use crate::mcp::{
    McpTool, ServerSpec, ToolDescriptor, ToolResult, ToolResultContent, server::McpServer,
};

#[async_trait::async_trait]
pub trait _Server: Send + Sync {
    async fn list_tools(&self) -> Vec<McpTool>;

    async fn rpc(&self, method: &str, params: Value) -> anyhow::Result<serde_json::Value>;
}

struct FetchMcpServer {}

#[async_trait::async_trait]
impl _Server for FetchMcpServer {
    async fn list_tools(&self) -> Vec<McpTool> {
        vec![McpTool {
            name: "fetch".into(),
            description: Some("Fetch the contents of a URL.".into()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to fetch"
                    }
                },
                "required": ["url"]
            }),
        }]
    }

    async fn rpc(&self, method: &str, params: Value) -> anyhow::Result<serde_json::Value> {
        if method != "tools/call" {
            bail!("Error: unknown RPC method {method}");
        }
        let name = params
            .get("name")
            .map(|v| v.as_str())
            .flatten()
            .unwrap_or_else(|| "");
        if name != "fetch" {
            bail!("Unknown tool: {method}")
        };
        let params = params
            .get("arguments")
            .map(|v| v.clone())
            .unwrap_or_else(|| json!({}));
        if let Some(Value::String(url)) = params.get("url") {
            let text = match _fetch(url.to_string()).await {
                Ok(s) => s,
                Err(e) => format!("{e:?}"),
            };
            return Ok(serde_json::to_value(ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".into(),
                    text: Some(text),
                    mime_type: None,
                    data: None,
                    resource: None,
                }],
                is_error: None,
            })?);
        }
        Ok(Value::Null)
    }
}

#[cfg(target_arch = "wasm32")]
async fn _fetch(url: String) -> anyhow::Result<String> {
    use gloo_net::http::Request;
    use tokio::sync::oneshot;
    use dioxus::logger::tracing::warn;
    
    let (tx, rx) = oneshot::channel::<String>();
    wasm_bindgen_futures::spawn_local(async move {
        use dioxus::logger::tracing::warn;

        let _url = format!("https://api.allorigins.win/raw?url={url}");
        let req = Request::get(&_url)
            .send()
            .await;
        let text = match req {
            Ok(req) => {
                let response = req.text().await;
                match response {
                    Ok(s) => {
                        s
                    },
                    Err(e) => {
                        format!("Error in builtin/fetch: {e:?}")
                    }
                }
            }
            Err(e) => {
                format!("Error in builtin/fetch: {e:?}")
            }
        };
        let len = text.len();
        if tx.send(text).is_err() {
            warn!("Receiver dropped before message was sent");
        }
    });

    let s = match rx.await {
        Ok(val) => val,
        Err(e) => format!("Error fetching data during tool call!"),
    };
    Ok(s)
}

#[cfg(not(target_arch = "wasm32"))]
async fn _fetch(url: String) -> anyhow::Result<String> {
    reqwest::Client::new()
        .get(&url)
        .send()
        .await?
        .text()
        .await
        .map_err(|e| anyhow!("{e:?}"))
}

unsafe impl Send for FetchMcpServer {}
unsafe impl Sync for FetchMcpServer {}

pub struct Host {
    servers: RwLock<HashMap<String, Box<dyn _Server>>>,
    #[allow(unused)]
    pub request_timeout: Duration,
    #[allow(unused)]
    pub startup_timeout: Duration,
}

impl Host {
    pub fn new(request_timeout: Duration, startup_timeout: Duration) -> Self {
        let mut servers: HashMap<String, Box<dyn _Server>> = HashMap::new();
        servers.insert("builtin".into(), Box::new(FetchMcpServer {}));

        Self {
            servers: RwLock::new(servers),
            request_timeout,
            startup_timeout,
        }
    }

    pub async fn add_server(&self, spec: ServerSpec) -> Result<()> {
        let server =
            McpServer::spawn(spec.clone(), self.request_timeout, self.startup_timeout).await?;
        self.servers.write().await.insert(spec.id, Box::new(server));
        Ok(())
    }

    pub async fn list_tools(&self) -> Vec<ToolDescriptor> {
        let mut res = vec![];
        for (id, s) in self.servers.read().await.iter() {
            let tools = s.list_tools().await;
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

    // pub async fn invoke(&self, server_id: &str, method: &str, params: Value) -> Result<Value> {
    //     let servers = self.servers.read().await;
    //     let s = servers
    //         .get(server_id)
    //         .ok_or_else(|| anyhow::anyhow!("unknown server {server_id}"))?;
    //     s.rpc_call(method, params).await
    // }

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
        let result = s.rpc("tools/call", params).await?;
        serde_json::from_value(result).map_err(|e| e.into())
    }
}
