//! Utility functions for handling tool calls and message conversion.
//! 
//! This module provides helper functions for converting between MCP tool descriptors
//! and LLM tool objects, as well as executing tool calls and formatting their results
//! for inclusion in chat conversations.

use std::sync::Arc;

use serde_json::Value;

use crate::llm::Function;
use crate::llm::Message;
use crate::llm::Tool;
use crate::llm::ToolCallDelta;
use crate::mcp::host::MCPHost;
use crate::mcp::ToolDescriptor;
use crate::app_settings::Chat;
use crate::storage::{get_storage, Storage};
use crate::toolset::Toolset;
use crate::llm::{FunctionDelta, LlmClient};
use dioxus::logger::tracing::{info, warn};
use dioxus::prelude::*;
use dioxus_router::Navigator;

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
            warn!("no function");
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

        warn!("function parts: {parts:?}");
            
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

            warn!("arguments: {arguments:?}");

            // Log the tool call for debugging
            warn!("Calling {server_id}/{tool_name}({arguments:?})");
            
            // Execute the tool call on the MCP server
            let result = host.tool_call(server_id, tool_name, arguments).await?;
            warn!("result: {result:?}");
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

/// Extracts tool calls from text that uses non-standard formats.
/// 
/// Some LLM models may return tool calls in custom formats rather than the
/// standard streaming format. This function attempts to parse these alternative
/// formats and convert them to standard ToolCallDelta objects.
/// 
/// # Arguments
/// * `text` - The text content to parse for tool calls
/// 
/// # Returns
/// An optional ToolCallDelta if a tool call was successfully extracted
pub fn extract_wierd_tool_calls(text: &str) -> anyhow::Result<Option<ToolCallDelta>> {
    if text.starts_with("[TOOL_CALLS]") {
        let t = text.replace("[TOOL_CALLS]", "");
        let parts: Vec<String> = t.split("<SPECIAL_32>").map(|s| s.into()).collect();
        if parts.len() < 2 { return Ok(None) }
        return Ok(Some(ToolCallDelta {
            id: Some("...".into()),
            kind: Some("function".into()),
            function: Some(FunctionDelta {
                name: Some(parts[0].to_string()),
                arguments: Some(parts[1].clone()),
            }),
        }));
    }

    if let Ok(Value::Object(m)) = serde_json::from_str(text) {
        if let Some(name) = m.get("name").map(|x| x.as_str()).flatten() {
            if let Some(args) = m.get("arguments") {
                let arguments = if let Some(s) = args.as_str() {
                    Some(s.to_string())
                } else if let Some(m) = args.as_object() {
                    let args_str = serde_json::to_string(&Value::Object(m.clone()))?;
                    Some(args_str)
                } else {
                    None
                };

                return Ok(Some(ToolCallDelta {
                    id: Some("...".into()),
                    kind: Some("function".into()),
                    function: Some(FunctionDelta {
                        name: Some(name.to_string()),
                        arguments,
                    }),
                }));
            }
        }
    }

    Ok(None)
}

/// Saves a chat to storage and updates its state.
/// 
/// This function persists the chat to storage, updates the toolset state,
/// and handles navigation to the saved chat if it's a new chat.
/// 
/// # Arguments
/// * `chat` - Mutable signal containing the chat to save
/// * `toolset` - Reference to the current toolset
/// * `display` - Mutable signal for the markdown display
/// * `id` - Signal containing the current chat ID
/// * `nav` - Navigator for routing
/// 
/// # Returns
/// Result indicating success or failure of the save operation
pub async fn save_chat_to_storage(
    chat: &mut Signal<Chat>,
    toolset: &Box<dyn Toolset>,
    display: &mut Signal<Option<String>>,
    id: Signal<Option<u32>>,
    nav: &Navigator,
) -> anyhow::Result<()> {
    let storage = match get_storage().await {
        Ok(s) => Some(s),
        Err(e) => {
            warn!("Could not get storage: {e:?}");
            None
        }
    };
    
    let value = toolset.get_state().await;
    chat.with_mut(move |c| c.value = value);
    let md = toolset.get_markdown_repr().await;
    display.with_mut(|d| *d = md);
    
    let Some(stg) = storage else { return Ok(()) };
    let new_chat_id = stg.save_chat(&chat()).await?;
    chat.with_mut(|c| {
        c.id = Some(new_chat_id);
    });
    
    if id() != Some(new_chat_id) {
        nav.push(crate::Route::ChatEl { id: new_chat_id });
    }
    
    Ok(())
}

/// Main loop for handling LLM responses and tool execution.
/// 
/// This function manages the conversation flow:
/// 1. Sends the current conversation to the LLM
/// 2. Processes streaming responses (text and tool calls)
/// 3. Executes any requested tools
/// 4. Continues the loop until no more tools are called
/// 5. Implements safety limits to prevent runaway tool execution
/// 
/// # Arguments
/// * `client` - LLM client for making API calls
/// * `model` - Model name to use for the conversation
/// * `chat` - Mutable signal containing the chat messages
/// * `toolset` - Reference to the current toolset for getting tools
/// * `streaming_msg` - Signal for displaying streaming responses
/// * `save_chat_fn` - Async closure for saving the chat
/// 
/// # Returns
/// Result indicating success or failure, and the number of tool calls made
pub async fn run_tools_loop<F, Fut>(
    client: &LlmClient,
    model: &str,
    chat: &mut Signal<Chat>,
    toolset: &Box<dyn Toolset>,
    streaming_msg: &mut Signal<Option<String>>,
    save_chat_fn: F,
) -> anyhow::Result<u8>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<()>>,
{
    // Get MCP host and available tools
    let host = toolset.get_mcp_host();
    let tools = host.list_tools().await;
    let tools: Vec<Tool> = tools_to_message_objects(tools);

    let mut count = 0u8; // Safety counter to prevent infinite loops
    loop {
        // Start streaming response from LLM
        let mut stream = client.stream(model, &chat.read().messages, &tools).await?;
        let mut text = "".to_string();
        let mut tool_calls = vec![];

        // Process streaming response chunks
        while let Some(e) = stream.recv().await {
            let Some(ch) = e.choices.first() else { break };

            // Handle text content (assistant response)
            if let Some(t) = &ch.delta.content {
                if !t.is_empty() {
                    text = format!("{}{}", &text, t);
                    // Update streaming display in real-time
                    streaming_msg.set(Some(text.clone()));
                }
            }

            // Handle tool calls
            if let Some(tools) = &ch.delta.tool_calls {
                info!("{:?}", tools);
                tool_calls.extend_from_slice(tools);
            }
        }

        // Clear streaming display once complete
        streaming_msg.set(None);
        let text = text.trim();

        // Process the final response
        if !text.is_empty() {
            // Handle special tool call format (fallback for some models)
            if let Ok(Some(tcd)) = extract_wierd_tool_calls(&text) {
                tool_calls.push(tcd);
            } else {
                // Regular assistant message
                chat.with_mut(|c| {
                    c.messages.push(Message::Assistant {
                        content: Some(text.to_string()),
                    });
                });
            }
        }

        // If no tools were called, we're done
        if tool_calls.is_empty() {
            save_chat_fn().await?;
            return Ok(count);
        }

        // Execute the requested tools
        let new_messages = call_tools(tool_calls, host.clone()).await?;
        chat.with_mut(|c| {
            c.messages.extend(new_messages);
        });

        // Safety check: prevent runaway tool execution
        count += 1;
        if count > 10 {
            save_chat_fn().await?;
            return Ok(count);
        }
    }
}
