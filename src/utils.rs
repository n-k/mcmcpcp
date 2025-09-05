//! Utility functions for handling tool calls and message conversion.
//! 
//! This module provides helper functions for converting between MCP tool descriptors
//! and LLM tool objects, as well as executing tool calls and formatting their results
//! for inclusion in chat conversations.

use std::sync::Arc;

use dioxus::logger::tracing::warn;
use serde_json::Value;

use crate::llm::Function;
use crate::llm::Message;
use crate::llm::Tool;
use crate::llm::ToolCallDelta;
use crate::mcp::host::MCPHost;
use crate::mcp::ToolDescriptor;

/// Converts MCP tool descriptors to LLM tool objects.
/// 
/// This function transforms tool descriptors from MCP servers into the format
/// expected by LLM APIs. Each tool is prefixed with its server ID to ensure
/// unique naming and proper routing when the tool is called.
/// 
/// # Arguments
/// * `tools` - Vector of tool descriptors from MCP servers
/// 
/// # Returns
/// Vector of `Tool` objects formatted for LLM API requests
pub fn tools_to_message_objects(tools: Vec<ToolDescriptor>) -> Vec<Tool> {
    tools
        .iter()
        .map(move |t| {
            let t = t.clone();
            Tool {
                r#type: "function".into(),
                function: Function {
                    // Prefix tool name with server ID for unique identification
                    name: format!("{}--{}", t.server_id, t.tool.name),
                    description: t.tool.description,
                    parameters: Some(t.tool.input_schema),
                    strict: Some(true), // Enable strict parameter validation
                },
            }
        })
        .collect()
}

/// Executes tool calls and converts results to chat messages.
/// 
/// This function processes tool call deltas from the LLM, extracts the server ID
/// and tool name, executes the tools on the appropriate MCP servers, and formats
/// the results as tool messages that can be added to the chat conversation.
/// 
/// # Arguments
/// * `tool_calls` - Vector of tool call deltas from the LLM response
/// * `host` - MCP host for executing tool calls
/// 
/// # Returns
/// Vector of tool result messages to add to the conversation, or an error
/// if any tool call fails
pub async fn call_tools(
    tool_calls: Vec<ToolCallDelta>,
    host: Arc<MCPHost>,
) -> anyhow::Result<Vec<Message>> {
    let mut new_chat: Vec<Message> = vec![];
    
    // Process each tool call from the LLM
    for tc in tool_calls.into_iter() {
        warn!("> Calling {tc:#?}");
        let Some(f) = tc.function.as_ref() else {
            continue; // Skip tool calls without function information
        };
        
        // Parse the tool name to extract server ID and tool name
        // Format is "server_id/tool_name"
        let parts: Vec<_> = f
            .name
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or_else(|| "")
            .split("--")
            .collect();
            
        if parts.len() == 2 {
            let server_id = parts[0];
            let tool_name = parts[1];
            
            // Parse the function arguments from JSON string
            let params_str = f
                .arguments
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or_else(|| "{}");
            let arguments: Value = serde_json::from_str(params_str)?;

            // Log the tool call for debugging
            eprintln!("Calling {server_id}/{tool_name}({arguments:?})");
            
            // Execute the tool call on the MCP server
            let result = host.tool_call(server_id, tool_name, arguments).await?;
            
            // Convert tool result to text messages
            // Filter for text content and combine into a single message
            let messages: Vec<String> = result
                .content
                .into_iter()
                .filter(|c| c.r#type == "text") // Only process text content
                .map(|c| c.text.unwrap_or_else(|| "".to_string()))
                .collect();
            let text = messages.join("\n");
            
            // Create a tool message with the result
            let tcm = Message::Tool { 
                tool_call_id: tc.id.unwrap_or_else(|| "".into()), 
                content: text 
            };
            new_chat.push(tcm);
        }
    }
    
    Ok(new_chat)
}
