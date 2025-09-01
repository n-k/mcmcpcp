use std::sync::Arc;

use dioxus::{
    logger::tracing::{info, warn},
    prelude::*,
};

use crate::{llm::{FunctionDelta, ToolCallDelta}, utils::{call_tools, tools_to_message_objects}};
use crate::{
    ui::{
        message::MessageEl,
        chat_input::ChatInput,
        settings::SETTINGS,
    },
    llm::{ContentPart, LlmClient, Message, Tool},
    mcp::host::Host,
};

#[component]
pub fn Home() -> Element {
    let client = use_resource(|| async {
        let settings = SETTINGS();
        let api_base = settings.api_url;
        let api_key = settings.api_key;

        // let client = Client::with_config(
        //     OpenAIConfig::new()
        //         .with_api_key(api_key)
        //         .with_api_base(api_base),
        // );
        let lmc = LlmClient::new(api_base, api_key);
        lmc
    });
    let model = use_resource(|| async {
        let settings = SETTINGS();
        let model = settings.model;
        model
    });
    let is_configured = use_resource(move || async move {
        let settings = SETTINGS();
        let client_loaded = client().is_some();
        let model = model().flatten();
        let model_loaded = model.is_some();
        let configured = client_loaded && model_loaded && !settings.api_url.is_empty();
        configured
    });
    let mut busy = use_signal(|| false);
    let disabled = use_resource(move || async move {
        let disabled = !is_configured().unwrap_or_else(|| false) || busy();
        disabled
    });
    let mut streaming_msg: Signal<Option<String>> = use_signal(|| None);
    let mut chat: Signal<Vec<Message>> = use_signal(|| {
        vec![Message::System {
            content: "You are a helpful assistant. 
                You have access to tools which you can call to help the user in the user's task."
                .into(),
        }]
    });
    let mut tool_count_warning: Signal<bool> = use_signal(|| false);

    let run_tools_loop = move || async move {
        let Some(model) = model() else {
            return anyhow::Ok(());
        };
        let Some(model) = model else { return Ok(()) };
        let Some(client) = client() else {
            return Ok(());
        };
        let host = consume_context::<Arc<Host>>();
        let tools = host.list_tools().await;
        let tools: Vec<Tool> = tools_to_message_objects(tools);

        let mut count = 0u8;
        loop {
            let mut stream = client.stream(&model, &chat.read(), &tools).await?;
            let mut text = "".to_string();
            let mut tool_calls = vec![];
            while let Some(e) = stream.recv().await {
                let Some(ch) = e.choices.first() else { break };
                if let Some(t) = &ch.delta.content {
                    if !t.is_empty() {
                        text = format!("{}{}", &text, t);
                        streaming_msg.set(Some(text.clone()));
                    }
                }
                if let Some(tools) = &ch.delta.tool_calls {
                    info!("{:?}", tools);
                    tool_calls.extend_from_slice(tools);
                }
            }
            streaming_msg.set(None);
            let text = text.trim();
            if !text.is_empty() {
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
                    chat.push(Message::Assistant {
                        content: Some(text.to_string()),
                    });
                }
            }
            if tool_calls.is_empty() {
                return Ok(());
            }
            let new_messages = call_tools(tool_calls, host.clone()).await?;
            chat.extend(new_messages);

            count += 1;
            if count > 10 {
                tool_count_warning.set(true);
                return Ok(());
            }
        }
    };

    let send_msg = move |s: String| async move {
        chat.push(Message::User {
            content: vec![ContentPart::Text { text: s }],
        });

        run_tools_loop().await
    };

    let stream_output: Option<Element> = streaming_msg().map(move |m| {
        rsx! {
            div { class: "message ai-message", {crate::md2rsx::markdown_to_rsx(&m)} }
        }
    });

    rsx! {
        div { class: "content",
            div { style: "flex-grow: 1; overflow: auto;",
                for c in chat.iter() {
                    MessageEl { msg: (*c).clone() }
                }
                {stream_output}
                if tool_count_warning() {
                    div {
                        "10 tool calls have been made without user intervention."
                        button {
                            onclick: move |_| async move {
                                tool_count_warning.set(false);
                                if let Err(e) = run_tools_loop().await {
                                    eprintln!("{e:?}");
                                }
                            },
                            "Continue"
                        }
                        button {
                            onclick: move |_| async move {
                                tool_count_warning.set(false);
                            },
                            "Stop"
                        }
                    }
                }
            }

            div { style: "flex-grow: 0",
                ChatInput {
                    disabled: disabled().unwrap_or_else(|| true),
                    on_send: Callback::new(move |s: String| async move {
                        {
                            if busy() {
                                return;
                            }
                        }
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
