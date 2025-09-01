//! MCP Host implementation for managing and communicating with MCP servers.
//! 
//! This module provides the main Host struct that manages multiple MCP servers,
//! handles tool discovery and execution, and provides a unified interface for
//! interacting with various MCP servers. It includes both external MCP servers
//! and built-in functionality like web fetching.

use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};
use std::{collections::HashMap, time::Duration};
use tokio::sync::RwLock;

use crate::mcp::{
    McpTool, ServerSpec, ToolDescriptor, ToolResult, ToolResultContent, server::McpServer,
};

/// Trait defining the interface for MCP servers.
/// 
/// This trait abstracts the communication with MCP servers, allowing both
/// external process-based servers and built-in functionality to be treated
/// uniformly by the host.
#[async_trait::async_trait]
pub trait _Server: Send + Sync {
    /// Lists all tools provided by this server.
    /// 
    /// # Returns
    /// Vector of `McpTool` definitions available from this server
    async fn list_tools(&self) -> Vec<McpTool>;

    /// Executes an RPC call on this server.
    /// 
    /// # Arguments
    /// * `method` - The RPC method name to call
    /// * `params` - Parameters for the RPC call
    /// 
    /// # Returns
    /// The result of the RPC call as a JSON value
    async fn rpc(&self, method: &str, params: Value) -> anyhow::Result<serde_json::Value>;
}

/// Built-in MCP server that provides web fetching functionality.
/// 
/// This server is always available and provides a "fetch" tool that can
/// retrieve content from URLs. It's implemented as a built-in server to
/// provide basic web access without requiring external MCP server setup.
struct FetchMcpServer {}

#[async_trait::async_trait]
impl _Server for FetchMcpServer {
    /// Returns the fetch tool definition.
    /// 
    /// Provides a single "fetch" tool that can retrieve content from URLs.
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

    /// Handles RPC calls for the fetch server.
    /// 
    /// Currently only supports the "tools/call" method with the "fetch" tool.
    /// The fetch tool retrieves content from the specified URL and returns it as text.
    async fn rpc(&self, method: &str, params: Value) -> anyhow::Result<serde_json::Value> {
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
            
        // Only support the "fetch" tool
        if name != "fetch" {
            bail!("Unknown tool: {name}")
        };
        
        // Extract tool arguments
        let params = params
            .get("arguments")
            .map(|v| v.clone())
            .unwrap_or_else(|| json!({}));
            
        // Execute the fetch if URL is provided
        if let Some(Value::String(url)) = params.get("url") {
            let text = match _fetch(url.to_string()).await {
                Ok(s) => s,
                Err(e) => format!("Fetch error: {e:?}"),
            };
            
            // Return the result in MCP tool result format
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

/// Fetches content from a URL (WASM version).
/// 
/// Uses a CORS proxy service to bypass browser CORS restrictions when running
/// in WASM. The fetch is performed in a spawned local task and the result is
/// communicated back through a oneshot channel.
/// 
/// # Arguments
/// * `url` - The URL to fetch content from
/// 
/// # Returns
/// The fetched content as a string, or an error message if the fetch fails
#[cfg(target_arch = "wasm32")]
async fn _fetch(url: String) -> anyhow::Result<String> {
    use gloo_net::http::Request;
    use tokio::sync::oneshot;
    use dioxus::logger::tracing::warn;
    
    // Create a channel to receive the result from the spawned task
    let (tx, rx) = oneshot::channel::<String>();
    
    // Spawn a local task to perform the fetch (required for WASM)
    wasm_bindgen_futures::spawn_local(async move {
        use dioxus::logger::tracing::warn;

        // Use CORS proxy to bypass browser restrictions
        let _url = format!("https://api.allorigins.win/raw?url={url}");
        let req = Request::get(&_url)
            .send()
            .await;
            
        let text = match req {
            Ok(req) => {
                let response = req.text().await;
                match response {
                    Ok(s) => s,
                    Err(e) => format!("Error in builtin/fetch: {e:?}")
                }
            }
            Err(e) => format!("Error in builtin/fetch: {e:?}")
        };
        
        // Send the result back through the channel
        if tx.send(text).is_err() {
            warn!("Receiver dropped before message was sent");
        }
    });

    // Wait for the result from the spawned task
    let s = match rx.await {
        Ok(val) => val,
        Err(_e) => "Error fetching data during tool call!".to_string(),
    };
    Ok(s)
}

/// Fetches content from a URL (native version).
/// 
/// Uses reqwest to directly fetch content from the URL without CORS restrictions.
/// This is simpler than the WASM version since native applications don't have
/// browser security restrictions.
/// 
/// # Arguments
/// * `url` - The URL to fetch content from
/// 
/// # Returns
/// The fetched content as a string, or an error if the fetch fails
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

/// Main MCP Host that manages multiple MCP servers and provides a unified interface.
/// 
/// The Host maintains a collection of MCP servers (both built-in and external),
/// handles tool discovery across all servers, and routes tool calls to the
/// appropriate server. It provides timeout configuration for server operations.
pub struct Host {
    /// Map of server ID to server implementation, protected by RwLock for concurrent access
    servers: RwLock<HashMap<String, Box<dyn _Server>>>,
    /// Timeout for individual RPC requests to servers
    #[allow(unused)]
    pub request_timeout: Duration,
    /// Timeout for server startup and initialization
    #[allow(unused)]
    pub startup_timeout: Duration,
}

impl Host {
    /// Creates a new MCP Host with the specified timeouts.
    /// 
    /// Initializes the host with a built-in fetch server that provides web access
    /// functionality. Additional external MCP servers can be added later.
    /// 
    /// # Arguments
    /// * `request_timeout` - Timeout for individual RPC requests
    /// * `startup_timeout` - Timeout for server startup/initialization
    /// 
    /// # Returns
    /// A new Host instance ready to manage MCP servers
    pub fn new(request_timeout: Duration, startup_timeout: Duration) -> Self {
        let mut servers: HashMap<String, Box<dyn _Server>> = HashMap::new();
        // Add the built-in fetch server
        servers.insert("builtin".into(), Box::new(FetchMcpServer {}));

        Self {
            servers: RwLock::new(servers),
            request_timeout,
            startup_timeout,
        }
    }

    /// Adds an external MCP server to the host.
    /// 
    /// Spawns a new MCP server process based on the provided specification
    /// and adds it to the host's server collection. The server will be
    /// available for tool discovery and execution after successful addition.
    /// 
    /// # Arguments
    /// * `spec` - Server specification including command, arguments, and ID
    /// 
    /// # Returns
    /// Ok(()) if the server was successfully added, or an error if spawning failed
    pub async fn add_server(&self, spec: ServerSpec) -> Result<()> {
        let server =
            McpServer::spawn(spec.clone(), self.request_timeout, self.startup_timeout).await?;
        self.servers.write().await.insert(spec.id, Box::new(server));
        Ok(())
    }

    /// Lists all available tools from all registered servers.
    /// 
    /// Queries each server for its available tools and returns a combined list
    /// with server ID information. This allows the LLM to see all available
    /// tools across all connected MCP servers.
    /// 
    /// # Returns
    /// Vector of tool descriptors with server ID and tool information
    pub async fn list_tools(&self) -> Vec<ToolDescriptor> {
        let mut res = vec![];
        
        // Query each server for its tools
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

    /// Invokes an RPC method on a specific server.
    /// 
    /// Routes the RPC call to the specified server and returns the result.
    /// This is a low-level method used by higher-level tool calling functions.
    /// 
    /// # Arguments
    /// * `server_id` - ID of the server to invoke the method on
    /// * `method` - RPC method name to call
    /// * `params` - Parameters for the RPC call
    /// 
    /// # Returns
    /// The result of the RPC call, or an error if the server is not found or the call fails
    pub async fn invoke(&self, server_id: &str, method: &str, params: Value) -> Result<Value> {
        let servers = self.servers.read().await;
        let s = servers
            .get(server_id)
            .ok_or_else(|| anyhow::anyhow!("unknown server {server_id}"))?;
        s.rpc(method, params).await
    }

    /// Executes a tool call on the specified server.
    /// 
    /// High-level method for calling tools on MCP servers. Formats the parameters
    /// appropriately and parses the result into a ToolResult structure.
    /// 
    /// # Arguments
    /// * `server_id` - ID of the server that provides the tool
    /// * `tool_name` - Name of the tool to execute
    /// * `arguments` - Arguments to pass to the tool
    /// 
    /// # Returns
    /// The tool execution result, or an error if the call fails
    pub async fn tool_call(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<ToolResult> {
        // Format parameters for the tools/call RPC method
        let params = json!({
            "name": tool_name,
            "arguments": arguments,
        });
        
        // Execute the RPC call and parse the result
        let result = self.invoke(server_id, "tools/call", params).await?;
        serde_json::from_value(result).map_err(|e| e.into())
    }
}
