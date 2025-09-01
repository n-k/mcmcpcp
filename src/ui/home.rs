//! Main chat interface component for MCMCPCP.
//! 
//! This module implements the primary chat interface where users interact with LLMs.
//! It handles message display, streaming responses, tool execution, and manages the
//! conversation flow between the user, LLM, and MCP tools.

use std::sync::Arc;

use dioxus::{
    logger::tracing::{info, warn},
    prelude::*,
};

use crate::{llm::{FunctionDelta, ToolCallDelta}, utils::{call_tools, tools_to_message_objects}};
use crate::{
    ui::{
        message::MessageEl,      // Component for displaying individual messages
        chat_input::ChatInput,   // Component for message input
        settings::SETTINGS,      // Global settings accessor
    },
    llm::{ContentPart, LlmClient, Message, Tool},  // LLM types and client
    mcp::host::Host,  // MCP host for tool execution
};

/// Main chat interface component.
/// 
/// This component provides the primary user interface for chatting with LLMs.
/// It manages the conversation state, handles streaming responses, executes tools,
/// and provides safety mechanisms to prevent runaway tool execution.
#[component]
pub fn Home() -> Element {
    // Initialize LLM client from settings
    let client = use_resource(|| async {
        let settings = SETTINGS();
        let api_base = settings.api_url;
        let api_key = settings.api_key;

        // Create LLM client with configured API settings
        let lmc = LlmClient::new(api_base, api_key);
        lmc
    });
    
    // Get selected model from settings
    let model = use_resource(|| async {
        let settings = SETTINGS();
        let model = settings.model;
        model
    });
    
    // Check if the application is properly configured
    let is_configured = use_resource(move || async move {
        let settings = SETTINGS();
        let client_loaded = client().is_some();
        let model = model().flatten();
        let model_loaded = model.is_some();
        let configured = client_loaded && model_loaded && !settings.api_url.is_empty();
        configured
    });
    
    // Track if the system is currently processing a request
    let mut busy = use_signal(|| false);
    
    // Determine if the chat input should be disabled
    let disabled = use_resource(move || async move {
        let disabled = !is_configured().unwrap_or_else(|| false) || busy();
        disabled
    });
    
    // Current streaming message content (for real-time display)
    let mut streaming_msg: Signal<Option<String>> = use_signal(|| None);
    
    // Chat conversation history
    let mut chat: Signal<Vec<Message>> = use_signal(|| {
        vec![Message::System {
            content: "You are a helpful assistant. 
                You have access to tools which you can call to help the user in the user's task."
                .into(),
        }]
    });
    
    // Flag to show warning when too many tool calls are made
    let mut tool_count_warning: Signal<bool> = use_signal(|| false);

    // Main loop for handling LLM responses and tool execution.
    // 
    // This function manages the conversation flow:
    // 1. Sends the current conversation to the LLM
    // 2. Processes streaming responses (text and tool calls)
    // 3. Executes any requested tools
    // 4. Continues the loop until no more tools are called
    // 5. Implements safety limits to prevent runaway tool execution
    let run_tools_loop = move || async move {
        // Ensure we have all required components
        let Some(model) = model() else {
            return anyhow::Ok(());
        };
        let Some(model) = model else { return Ok(()) };
        let Some(client) = client() else {
            return Ok(());
        };
        
        // Get MCP host and available tools
        let host = consume_context::<Arc<Host>>();
        let tools = host.list_tools().await;
        let tools: Vec<Tool> = tools_to_message_objects(tools);

        let mut count = 0u8; // Safety counter to prevent infinite loops
        loop {
            // Start streaming response from LLM
            let mut stream = client.stream(&model, &chat.read(), &tools).await?;
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
                if text.starts_with("[TOOL_CALLS]") {
                    let t = text.replace("[TOOL_CALLS]", "");
                    let parts: Vec<String> = t.split("<SPECIAL_32>")
                        .map(|s| s.into())
                        .collect();
                    tool_calls.push(ToolCallDelta { 
                        id: Some("...".into()), 
                        kind: Some("function".into()), 
                        function: Some(FunctionDelta { 
                            name: Some(parts[0].to_string()), 
                            arguments: Some(parts[1].clone()), 
                        }),
                    });
                } else {
                    // Regular assistant message
                    chat.push(Message::Assistant {
                        content: Some(text.to_string()),
                    });
                }
            }
            
            // If no tools were called, we're done
            if tool_calls.is_empty() {
                return Ok(());
            }
            
            // Execute the requested tools
            let new_messages = call_tools(tool_calls, host.clone()).await?;
            chat.extend(new_messages);

            // Safety check: prevent runaway tool execution
            count += 1;
            if count > 10 {
                tool_count_warning.set(true);
                return Ok(());
            }
        }
    };

    // Handles sending a new user message and starting the conversation loop.
    // 
    // Adds the user's message to the chat history and initiates the LLM
    // response and tool execution loop.
    let send_msg = move |s: String| async move {
        // Add user message to chat history
        chat.push(Message::User {
            content: vec![ContentPart::Text { text: s }],
        });

        // Start the LLM response and tool execution loop
        run_tools_loop().await
    };

    // Renders the currently streaming message if one exists.
    // 
    // Shows real-time LLM responses as they're being generated,
    // with proper Markdown rendering.
    let stream_output: Option<Element> = streaming_msg().map(move |m| {
        rsx! {
            div { class: "message ai-message", {crate::md2rsx::markdown_to_rsx(&m)} }
        }
    });

    // Render the main chat interface
    rsx! {
        div { class: "content",
            // Scrollable message area
            div { style: "flex-grow: 1; overflow: auto;",
                // Render all messages in the conversation
                for c in chat.iter() {
                    MessageEl { msg: (*c).clone() }
                }
                
                // Show streaming message if one is being generated
                {stream_output}
                
                // Show tool count warning if too many tools have been executed
                if tool_count_warning() {
                    div {
                        "10 tool calls have been made without user intervention."
                        button {
                            onclick: move |_| async move {
                                tool_count_warning.set(false);
                                // Continue with more tool execution
                                if let Err(e) = run_tools_loop().await {
                                    eprintln!("{e:?}");
                                }
                            },
                            "Continue"
                        }
                        button {
                            onclick: move |_| async move {
                                // Stop tool execution
                                tool_count_warning.set(false);
                            },
                            "Stop"
                        }
                    }
                }
            }

            // Fixed chat input area at the bottom
            div { style: "flex-grow: 0",
                ChatInput {
                    disabled: disabled().unwrap_or_else(|| true),
                    on_send: Callback::new(move |s: String| async move {
                        // Prevent multiple concurrent requests
                        {
                            if busy() {
                                return;
                            }
                        }
                        
                        // Process the message
                        {
                            busy.set(true);
                            if let Err(e) = send_msg(s).await {
                                warn!("{e:?}");
                            }
                            busy.set(false);
                        }
                    }),
                }
            }
        }
    }
}
