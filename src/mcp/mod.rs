// Copyright © 2025 Nipun Kumar

//! Model Context Protocol (MCP) implementation for MCMCPCP.
//!
//! This module provides a complete implementation of the Model Context Protocol,
//! allowing the application to communicate with external MCP servers that provide
//! tools and resources. The MCP enables LLMs to interact with external systems
//! in a standardized way.
//!
//! Key components:
//! - `host`: MCP host that manages server connections and tool execution
//! - `server`: Individual MCP server management and communication
//! - `transport`: Communication layer for server processes (native only)
//! - `jsonrpc`: JSON-RPC protocol implementation for MCP communication
//! - `config`: Configuration structures for MCP servers

// Module declarations
mod config; // Configuration structures and parsing
pub mod fetch;
pub mod host; // Main MCP host implementation (public for external access)
mod jsonrpc; // JSON-RPC protocol implementation
mod server; // Individual MCP server management
#[cfg(not(target_arch = "wasm32"))]
mod transport; // Process-based transport (native platforms only) // built-in fetch MCP server

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Specification for an MCP server configuration.
///
/// This defines how to start and identify an MCP server, including
/// the command to execute, any arguments needed, and environment variables.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerSpec {
    /// Unique identifier for this server instance
    pub id: String,
    /// Command to execute to start the server
    pub cmd: String,
    /// Command-line arguments to pass to the server
    pub args: Vec<String>,
    /// Environment variables to set for the server process
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    /// Whether this server is enabled (defaults to true for backward compatibility)
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// Default value for the enabled field (true for backward compatibility)
fn default_enabled() -> bool {
    true
}

/// Represents a tool provided by an MCP server.
///
/// Tools are functions that can be called by the LLM to perform actions
/// or retrieve information from external systems.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    /// Name of the tool (used for invocation)
    pub name: String,
    /// Optional human-readable description of what the tool does
    pub description: Option<String>,
    /// JSON Schema defining the expected input parameters
    pub input_schema: Value,
}

/// Associates a tool with its originating server.
///
/// This allows the system to route tool calls back to the correct
/// MCP server for execution.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ToolDescriptor {
    /// ID of the server that provides this tool
    pub server_id: String,
    /// The tool definition itself
    pub tool: McpTool,
}

/// Result returned from executing a tool on an MCP server.
///
/// Tool results can contain multiple content items and may indicate
/// whether an error occurred during execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    /// List of content items returned by the tool
    pub content: Vec<ToolResultContent>,
    /// Whether the tool execution resulted in an error
    pub is_error: Option<bool>,
}

/// Individual content item within a tool result.
///
/// Content can be text, binary data, or references to resources,
/// with optional MIME type information for proper handling.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultContent {
    /// Type of content (e.g., "text", "image", "resource")
    pub r#type: String,
    /// Text content (for text-based results)
    pub text: Option<String>,
    /// MIME type of the content for proper interpretation
    pub mime_type: Option<String>,
    /// Base64-encoded binary data (for non-text content)
    pub data: Option<String>,
    /// Reference to a resource (for resource-type content)
    pub resource: Option<Value>,
}
