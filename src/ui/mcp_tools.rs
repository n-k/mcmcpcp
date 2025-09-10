// Copyright © 2025 Nipun Kumar

use dioxus::prelude::*;
use std::sync::Arc;

use crate::mcp::ToolDescriptor;
use crate::mcp::host::MCPHost;

#[derive(Props, Clone, PartialEq)]
pub struct McpToolsProps {
    pub on_close: EventHandler<()>,
}

#[component]
pub fn McpTools(props: McpToolsProps) -> Element {
    let mut tools = use_signal(Vec::<ToolDescriptor>::new);

    // Load tools when component mounts
    use_effect(move || {
        let host = consume_context::<Arc<MCPHost>>();
        let host_clone = host.clone();
        spawn(async move {
            let tool_list = host_clone.list_tools().await;
            tools.set(tool_list);
        });
    });
    let tools = tools();
    let is_empty = tools.is_empty();

    rsx! {
        div { style: "
                padding: 2rem;
                height: 100%;
                overflow-y: auto;
                background: #fff;
            ",

            // Header
            div { style: "
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    margin-bottom: 2rem;
                    border-bottom: 1px solid #e0e0e0;
                    padding-bottom: 1rem;
                ",
                h2 { style: "
                        margin: 0;
                        color: #333;
                        font-size: 1.5rem;
                    ",
                    "Available MCP Tools"
                }
                button {
                    style: "
                        background: none;
                        border: none;
                        font-size: 1.5rem;
                        cursor: pointer;
                        color: #666;
                        padding: 0.5rem;
                        border-radius: 4px;
                    ",
                    onclick: move |_| props.on_close.call(()),
                    "×"
                }
            }

            // Tools list
            div { style: "
                    display: flex;
                    flex-direction: column;
                    gap: 1rem;
                ",

                if is_empty {
                    div { style: "
                            text-align: center;
                            color: #666;
                            padding: 2rem;
                            font-style: italic;
                        ",
                        "No MCP tools available"
                    }
                } else {
                    {
                        tools
                            .into_iter()
                            .map(|tool| {
                                rsx! {
                                    ToolCard { key: "{tool.server_id}-{tool.tool.name}", tool }
                                }
                            })
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ToolCardProps {
    tool: ToolDescriptor,
}

#[component]
fn ToolCard(props: ToolCardProps) -> Element {
    let mut expanded = use_signal(|| false);

    rsx! {
        div { style: "
                border: 1px solid #e0e0e0;
                border-radius: 8px;
                padding: 1rem;
                background: #f9f9f9;
            ",

            // Tool header
            div {
                style: "
                    display: flex;
                    justify-content: space-between;
                    align-items: flex-start;
                    cursor: pointer;
                ",
                onclick: move |_| expanded.toggle(),

                div { style: "flex: 1;",

                    // Tool name
                    h3 { style: "
                            margin: 0 0 0.5rem 0;
                            color: #2c3e50;
                            font-size: 1.1rem;
                        ",
                        "{props.tool.tool.name}"
                    }

                    // Server ID
                    div { style: "
                            font-size: 0.8rem;
                            color: #7f8c8d;
                            margin-bottom: 0.5rem;
                        ",
                        "Server: {props.tool.server_id}"
                    }

                    // Description (if available)
                    {
                        if let Some(description) = &props.tool.tool.description {
                            rsx! {
                                div { style: "
                                                                        color: #555;
                                                                        font-size: 0.9rem;
                                                                        line-height: 1.4;
                                                                    ",
                                    "{description}"
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    }
                }

                // Expand/collapse indicator
                div { style: "
                        color: #666;
                        font-size: 1.2rem;
                        margin-left: 1rem;
                    ",
                    if expanded() {
                        "−"
                    } else {
                        "+"
                    }
                }
            }

            // Expanded content (schema)
            if expanded() {
                div { style: "
                        margin-top: 1rem;
                        padding-top: 1rem;
                        border-top: 1px solid #e0e0e0;
                    ",

                    h4 { style: "
                            margin: 0 0 0.5rem 0;
                            color: #34495e;
                            font-size: 0.9rem;
                        ",
                        "Input Schema:"
                    }

                    pre { style: "
                            background: #2c3e50;
                            color: #ecf0f1;
                            padding: 1rem;
                            border-radius: 4px;
                            font-size: 0.8rem;
                            overflow-x: auto;
                            margin: 0;
                            white-space: pre-wrap;
                        ",
                        "{serde_json::to_string_pretty(&props.tool.tool.input_schema).unwrap_or_else(|_| \"Invalid JSON\".to_string())}"
                    }
                }
            }
        }
    }
}
