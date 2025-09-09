//! MCP Host implementation for managing and communicating with MCP servers.
//!
//! This module provides the main Host struct that manages multiple MCP servers,
//! handles tool discovery and execution, and provides a unified interface for
//! interacting with various MCP servers. It includes both external MCP servers
//! and built-in functionality like web fetching.

use dioxus::logger::tracing::warn;
use serde_json::{Value, json};
use std::{collections::HashMap, time::Duration};
use tokio::sync::RwLock;

use crate::mcp::{
    McpTool, ServerSpec, ToolDescriptor, ToolResult, fetch::FetchMcpServer, server::_McpServer,
};

/// Trait defining the interface for MCP servers.
///
/// This trait abstracts the communication with MCP servers, allowing both
/// external process-based servers and built-in functionality to be treated
/// uniformly by the host.
#[async_trait::async_trait]
pub trait MCPServer: Send + Sync {
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
    async fn rpc(&mut self, method: &str, params: Value) -> anyhow::Result<serde_json::Value>;
}

/// Main MCP Host that manages multiple MCP servers and provides a unified interface.
///
/// The Host maintains a collection of MCP servers (both built-in and external),
/// handles tool discovery across all servers, and routes tool calls to the
/// appropriate server. It provides timeout configuration for server operations.
pub struct MCPHost {
    /// Map of server ID to server implementation, protected by RwLock for concurrent access
    servers: RwLock<HashMap<String, Box<dyn MCPServer>>>,
    /// Timeout for individual RPC requests to servers
    #[allow(unused)]
    pub request_timeout: Duration,
    /// Timeout for server startup and initialization
    #[allow(unused)]
    pub startup_timeout: Duration,
}

impl MCPHost {
    /// Creates a new MCP Host with defaults.
    ///
    /// # Returns
    /// A new Host instance ready to manage MCP servers
    pub fn new() -> Self {
        Self::new_with_timeouts(Duration::from_secs(2), Duration::from_secs(2))
    }

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
    pub fn new_with_timeouts(request_timeout: Duration, startup_timeout: Duration) -> Self {
        let mut servers: HashMap<String, Box<dyn MCPServer>> = HashMap::new();
        // Add the built-in fetch server
        servers.insert("builtin".into(), Box::new(FetchMcpServer {}));

        Self::new_with_tools(servers, request_timeout, startup_timeout)
    }

    /// Creates a new MCP Host with the specified tools and timeouts.
    ///
    /// Initializes the host with a built-in fetch server that provides web access
    /// functionality. Additional external MCP servers can be added later.
    ///
    /// # Arguments
    /// * `servers` - MCP servers
    /// * `request_timeout` - Timeout for individual RPC requests
    /// * `startup_timeout` - Timeout for server startup/initialization
    ///
    /// # Returns
    /// A new Host instance ready to manage MCP servers
    pub fn new_with_tools(
        servers: HashMap<String, Box<dyn MCPServer>>,
        request_timeout: Duration,
        startup_timeout: Duration,
    ) -> Self {
        Self {
            servers: RwLock::new(servers),
            request_timeout,
            startup_timeout,
        }
    }

    /// Syncs this host's servers with the list of servers in settings.
    ///
    /// # Arguments
    /// * `specs` - Server specifications including command, arguments, and ID
    ///
    /// # Returns
    /// Ok(()) if the servers was successfully synced, or an error if spawning failed
    pub async fn sync_servers(&self, specs: Vec<ServerSpec>) -> anyhow::Result<()> {
        // add any specs which are not running
        for spec in &specs {
            let exists = { self.servers.read().await.contains_key(&spec.id) };
            if exists {
                continue;
            }
            let server =
                _McpServer::spawn(spec.clone(), self.request_timeout, self.startup_timeout).await?;
            self.servers
                .write()
                .await
                .insert(spec.id.clone(), Box::new(server));
        }
        Ok(())
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
    pub async fn add_server(&self, spec: ServerSpec) -> anyhow::Result<()> {
        let server =
            _McpServer::spawn(spec.clone(), self.request_timeout, self.startup_timeout).await?;
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
        let servers = self.servers.read().await;
        // Query each server for its tools
        for (id, s) in servers.iter() {
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
    pub async fn invoke(
        &self,
        server_id: &str,
        method: &str,
        params: Value,
    ) -> anyhow::Result<Value> {
        let mut servers = self.servers.write().await;
        let s = servers
            .get_mut(server_id)
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
    ) -> anyhow::Result<ToolResult> {
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
