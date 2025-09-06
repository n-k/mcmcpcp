//! Main chat interface component for MCMCPCP.
//!
//! This module implements the primary chat interface where users interact with LLMs.
//! It handles message display, streaming responses, tool execution, and manages the
//! conversation flow between the user, LLM, and MCP tools.

use dioxus::{
    logger::tracing::{info, warn},
    prelude::*,
};
use serde_json::{Value, json};

use crate::{
    app_settings::{Chat, Toolsets}, 
    llm::{FunctionDelta, ToolCallDelta}, 
    storage::{get_storage, Storage}, 
    toolset::{ChatTools, Story, StoryWriter, Toolset}, 
    utils::{call_tools, tools_to_message_objects}, 
    Route
};
use crate::{
    llm::{ContentPart, LlmClient, Message, Tool}, // LLM types and client
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
        let ts: Box<dyn Toolset> = match chat_type {
            Toolsets::Chat => {
                Box::new(ChatTools::new())
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
            let ts: Box<dyn Toolset> = if ch.chat_type == Toolsets::Story {
                let story: Story = serde_json::from_value(ch.value.clone())
                    .unwrap_or_else(|e| {
                        warn!("Invalid story metadta: {e:?}");
                        Default::default()
                    });
                Box::new(StoryWriter::new(story))
            } else {
                Box::new(ChatTools::new())
            };
            display.set(ts.get_markdown_repr().await);
            toolset.set(ts);
            chat.set(ch);
        }
    });
    let settings = use_resource(move || async move {
        let storage = match get_storage().await {
            Ok(s) => Some(s),
            Err(e) => {
                warn!("Could not get storage: {e:?}");
                None
            }
        };
        let Some(storage) = storage else {
            return None;
        };
        let settings = storage.load_settings().await.unwrap();
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

    let save_chat = move || async move {
        let storage = match get_storage().await {
            Ok(s) => Some(s),
            Err(e) => {
                warn!("Could not get storage: {e:?}");
                None
            }
        };
        let ts = &*toolset.read();
        let value = ts.get_state().await;
        chat.with_mut(move |c| c.value = value);
        let md = ts.get_markdown_repr().await;
        display.with_mut(|d| *d = md);
        let Some(stg) = storage else { return Ok(()) };
        let new_chat_id = stg.save_chat(&chat()).await?;
        chat.with_mut(|c| {
            c.id = Some(new_chat_id);
        });
        if id() != Some(new_chat_id) {
            nav.push(Route::ChatEl { id: new_chat_id });
        }
        anyhow::Ok(())
    };

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
        let Some(client) = client else {
            return Ok(());
        };

        // Get MCP host and available tools
        let ts = &*toolset.read();
        let host = ts.get_mcp_host();
        let tools = host.list_tools().await;
        // let host = consume_context::<Arc<MCPHost>>();
        // let tools = host.list_tools().await;
        let tools: Vec<Tool> = tools_to_message_objects(tools);

        let mut count = 0u8; // Safety counter to prevent infinite loops
        loop {
            // Start streaming response from LLM
            let mut stream = client.stream(&model, &chat.read().messages, &tools).await?;
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
                save_chat().await?;
                return Ok(());
            }

            // Execute the requested tools
            let new_messages = call_tools(tool_calls, host.clone()).await?;
            chat.with_mut(|c| {
                c.messages.extend(new_messages);
            });

            // Safety check: prevent runaway tool execution
            count += 1;
            if count > 10 {
                tool_count_warning.set(true);
                save_chat().await?;
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
        chat.with_mut(|c| {
            c.messages.push(Message::User {
                content: vec![ContentPart::Text { text: s }],
            });
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
    let display = display.cloned();

    // Render the main chat interface
    rsx! {
        div { 
            class: "content",
            div {
                style: "
                height: 100%;
                min-width: 50%;
                flex-grow: 1;
                overflow: hidden;
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
                                if let Err(e) = send_msg(s).await {
                                    warn!("{e:?}");
                                }
                                busy.set(false);
                            }
                        }),
                    }
                }
            }
            if chat().chat_type == Toolsets::Story {
                {
                    if let Some(d) = display {
                        rsx!{
                            div {
                                style: "
                                height: 100%;
                                overflow: auto;
                                ",
                                {crate::md2rsx::markdown_to_rsx(&d)}
                            }
                        }
                    } else {
                        rsx! {}
                    }
                }
            }
        }
    }
}

fn extract_wierd_tool_calls(text: &str) -> anyhow::Result<Option<ToolCallDelta>> {
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
