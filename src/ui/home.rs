//! Main chat interface component for MCMCPCP.
//!
//! This module implements the primary chat interface where users interact with LLMs.
//! It handles message display, streaming responses, tool execution, and manages the
//! conversation flow between the user, LLM, and MCP tools.

use std::sync::Arc;

use dioxus::{
    logger::tracing::warn,
    prelude::*,
};
use serde_json::json;

use crate::{
    app_settings::{AppSettings, Chat, Toolsets}, mcp::host::MCPHost, storage::{get_storage, Storage}, toolset::{chat::ChatTools, story::{Story, StoryWriter}, Toolset}, utils::{run_tools_loop, save_chat_to_storage}
};
use crate::{
    llm::{ContentPart, LlmClient, Message}, // LLM types and client
    ui::{
        chat_input::ChatInput, // Component for message input
        message::MessageEl,    // Component for displaying individual messages
    },
};

#[component]
pub fn ChatEl(id: u32) -> Element {
    rsx! {
        Home {
            id: Signal::new(Some(id)),
            chat_type: Toolsets::Chat,
        }
    }
}

#[component]
pub fn NewChat() -> Element {
    rsx! {
        Home {
            id: Signal::new(None),
            chat_type: Toolsets::Chat,
        }
    }
}

#[component]
pub fn NewStory() -> Element {
    rsx! {
        Home {
            id: Signal::new(None),
            chat_type: Toolsets::Story,
        }
    }
}

/// Main chat interface component.
///
/// This component provides the primary user interface for chatting with LLMs.
/// It manages the conversation state, handles streaming responses, executes tools,
/// and provides safety mechanisms to prevent runaway tool execution.
#[component]
pub fn Home(
    id: Signal<Option<u32>>,
    chat_type: Toolsets,
) -> Element {
    let nav = navigator();
    let mut toolset: Signal<Box<dyn Toolset>> = use_signal(|| {
        let host = consume_context::<Arc<MCPHost>>();
        let ts: Box<dyn Toolset> = match chat_type {
            Toolsets::Chat => {
                Box::new(ChatTools::new(host))
            },
            Toolsets::Story => {
                Box::new(StoryWriter::new(Default::default()))
            },
        };
        ts
    });
    let mut chat: Signal<Chat> = use_signal(|| {
        let ts = &*toolset.read();
        Chat {
            id: None,
            chat_type: chat_type,
            messages: vec![Message::System {
                content: ts.get_system_prompt(),
            }],
            value: match chat_type {
                Toolsets::Chat => {
                    json!({})
                },
                Toolsets::Story => {
                    serde_json::to_value(Story::default()).unwrap()
                },
            },
        }
    });
    let mut display: Signal<Option<String>> = use_signal(|| None);
    let _ = use_resource(move || async move {
        let Some(id) = id() else {
            let ts = &*toolset.read();
            let d = ts.get_markdown_repr().await;
            display.set(d);
            return;
        };
        let storage = match get_storage().await {
            Ok(s) => Some(s),
            Err(e) => {
                warn!("Could not get storage: {e:?}");
                None
            }
        };
        let Some(storage) = storage else {
            return;
        };
        if let Ok(Some(ch)) = storage.get_chat(id).await {
            let host = consume_context::<Arc<MCPHost>>();
            let ts: Box<dyn Toolset> = if ch.chat_type == Toolsets::Story {
                let story: Story = serde_json::from_value(ch.value.clone())
                    .unwrap_or_else(|e| {
                        warn!("Invalid story metadta: {e:?}");
                        Default::default()
                    });
                Box::new(StoryWriter::new(story))
            } else {
                Box::new(ChatTools::new(host))
            };
            display.set(ts.get_markdown_repr().await);
            toolset.set(ts);
            chat.set(ch);
        }
    });
    let settings = use_resource(move || async move {
        let settings_ctx = consume_context::<Signal<Option<AppSettings>>>();
        let settings = settings_ctx.read().clone();
        settings
    });
    // Initialize LLM client from settings
    let client = use_resource(move || async move {
        let Some(settings) = settings() else {
            return None;
        };
        let Some(settings) = settings else {
            return None;
        };
        let api_base = settings.provider.get_api_url();
        let api_key = settings
            .provider
            .get_api_key()
            .unwrap_or_else(|| "".to_string());

        // Create LLM client with configured API settings
        let lmc = LlmClient::new(api_base, api_key);
        Some(lmc)
    });

    // Get selected model from settings
    let model = use_resource(move || async move {
        let Some(settings) = settings() else {
            return None;
        };
        let Some(settings) = settings else {
            return None;
        };
        let model: Option<String> = settings.provider.get_model();
        model
    });

    // Check if the application is properly configured
    let is_configured = use_resource(move || async move {
        let Some(settings) = settings() else {
            return false;
        };
        let Some(settings) = settings else {
            return false;
        };
        let client_loaded = client().is_some();
        let model = model().flatten();
        let model_loaded = model.is_some();
        let configured = client_loaded && model_loaded && settings.provider.is_configured();
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

    // Use the extracted save_chat_to_storage utility function
    let save_chat = move || async move {
        let ts = &*toolset.read();
        save_chat_to_storage(&mut chat, ts, &mut display, id, &nav).await
    };

    // Flag to show warning when too many tool calls are made
    let mut tool_count_warning: Signal<bool> = use_signal(|| false);

    // Error state for handling run_tools_loop errors
    let mut error_state: Signal<Option<String>> = use_signal(|| None);

    // Main loop for handling LLM responses and tool execution using extracted utility
    let run_tools_loop_impl = move || async move {
        // Ensure we have all required components
        let Some(model) = model() else {
            return anyhow::Ok(0u8);
        };
        let Some(model) = model else { return Ok(0u8) };
        let Some(client) = client() else {
            return Ok(0u8);
        };
        let Some(client) = client else {
            return Ok(0u8);
        };

        let ts = &*toolset.read();
        
        error_state.set(None);
        let count = run_tools_loop(
            &client,
            &model,
            &mut chat,
            ts,
            &mut streaming_msg,
            save_chat,
        ).await?;

        // Handle tool count warning if too many tools were executed
        if count > 10 {
            tool_count_warning.set(true);
        }

        Ok(count)
    };

    // Handles sending a new user message and starting the conversation loop.
    //
    // Adds the user's message to the chat history and initiates the LLM
    // response and tool execution loop.
    let send_msg = move |s: String| async move {
        // Clear any previous errors
        error_state.set(None);
        
        // Add user message to chat history
        chat.with_mut(|c| {
            c.messages.push(Message::User {
                content: vec![ContentPart::Text { text: s }],
            });
        });

        // Start the LLM response and tool execution loop
        if let Err(e) = run_tools_loop_impl().await {
            error_state.set(Some(format!("Error during conversation: {}", e)));
        }  
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
    let display = display.cloned();
    let chat_class = if display.is_some() { "small" } else { "large" };

    // Render the main chat interface
    rsx! {
        div { 
            class: "content {chat_class}",
            div {
                class: "chat {chat_class}",
                style: "
                display: flex;
                flex-direction: column;
                ",
                div {
                    style: "
                    flex-grow: 1;
                    overflow: auto;
                    ",
                    // Render all messages in the conversation
                    for c in chat.read().messages.iter() {
                        MessageEl { msg: (*c).clone() }
                    }

                    // Show streaming message if one is being generated
                    {stream_output}

                    // Show tool count warning if too many tools have been executed
                    if tool_count_warning() {
                        div {
                            style: "
                            background-color: #fff3cd;
                            border: 1px solid #ffeaa7;
                            border-radius: 4px;
                            padding: 1em;
                            margin: 1em 0;
                            ",
                            "10 tool calls have been made without user intervention."
                            div {
                                style: "margin-top: 0.5em;",
                                button {
                                    style: "margin-right: 0.5em;",
                                    onclick: move |_| async move {
                                        tool_count_warning.set(false);
                                        error_state.set(None);
                                        // Continue with more tool execution
                                        if let Err(e) = run_tools_loop_impl().await {
                                            error_state.set(Some(format!("Error during tool execution: {}", e)));
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

                    // Show error message if there's an error
                    if let Some(error_msg) = error_state() {
                        div {
                            style: "
                            background-color: #f8d7da;
                            border: 1px solid #f5c6cb;
                            border-radius: 4px;
                            padding: 1em;
                            margin: 1em 0;
                            color: #721c24;
                            ",
                            div {
                                style: "font-weight: bold; margin-bottom: 0.5em;",
                                "Error occurred:"
                            }
                            div {
                                style: "margin-bottom: 1em; font-family: monospace; white-space: pre-wrap;",
                                "{error_msg}"
                            }
                            div {
                                button {
                                    style: "margin-right: 0.5em; background-color: #dc3545; color: white; border: none; padding: 0.5em 1em; border-radius: 4px; cursor: pointer;",
                                    onclick: move |_| async move {
                                        error_state.set(None);
                                        // Clear any previous errors and retry
                                        if let Err(e) = run_tools_loop_impl().await {
                                            error_state.set(Some(format!("Error during retry: {}", e)));
                                        }
                                    },
                                    "Retry"
                                }
                                button {
                                    style: "background-color: #6c757d; color: white; border: none; padding: 0.5em 1em; border-radius: 4px; cursor: pointer;",
                                    onclick: move |_| async move {
                                        error_state.set(None);
                                    },
                                    "Cancel"
                                }
                            }
                        }
                    }
                }
                // Fixed chat input area at the bottom
                div {
                    style: "
                    flex-grow: 0;
                    padding: 1.5em;
                    ",
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
                                send_msg(s).await;
                                busy.set(false);
                            }
                        }),
                    }
                }
            }
            if let Some(d) = display {
                div {
                    class: "tool-display",
                    style: "
                    overflow: auto;
                    ",
                    {crate::md2rsx::markdown_to_rsx(&d)}
                }
            }
        }
    }
}
