use std::sync::Arc;

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs, ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContent, ChatCompletionTool, CreateChatCompletionRequestArgs
    },
    Client,
};
use dioxus::{
    logger::tracing::{info, warn},
    prelude::*,
};
use futures::StreamExt;
use serde_json::{json, Value};

use crate::{mcp::{Host, ServerSpec}, message::Message};
use crate::settings::SETTINGS;

#[component]
pub fn Home() -> Element {
    let client = use_resource(|| async {
        let settings = SETTINGS();
        let api_base = settings.api_url; //"http://192.168.29.3:11434/v1";
                                         // Required but ignored
        let api_key = settings.api_key; // "ollama";

        let client = Client::with_config(
            OpenAIConfig::new()
                .with_api_key(api_key)
                .with_api_base(api_base),
        );
        // eprintln!("made client...");
        client
    });
    let model = use_resource(|| async {
        let settings = SETTINGS();
        let model = settings.model;
        // eprintln!("made model... {model:?}");
        model
    });
    let is_configured = use_resource(move || async move {
        let settings = SETTINGS();
        let client_loaded = client().is_some();
        let model = model().flatten();
        let model_loaded = model.is_some();
        let configured = client_loaded && model_loaded && !settings.api_url.is_empty();
        // eprintln!("configured: {configured}. Model {model:?}");
        configured
    });
    let mut busy = use_signal(|| false);
    let disabled = use_resource(move || async move {
        let disabled = !is_configured().unwrap_or_else(|| false) || busy();
        // eprintln!("Disabled: {disabled}");
        disabled
    });
    let mut streaming_msg: Signal<Option<String>> = use_signal(|| None);
    let mut chat: Signal<Vec<ChatCompletionRequestMessage>> = use_signal(|| vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content("You are a helpful assistant. 
            You have access to tools which you can call to help the user in the user's task.")
            .build()
            .unwrap()
            .into()
    ]);
    
    let send_msg = move |s: String| async move {
        // if busy() || s.is_empty() {
        //     return Ok(());
        // }
        // let model = model().unwrap(); //"qwen3-coder:30b";
        let Some(model) = model() else { return Ok(()) };
        let Some(model) = model else { return Ok(()) };
        let Some(client) = client() else { return Ok(()) };
        let mut chat_request = { chat.cloned() };
        chat_request.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(s)
                .build()?
                .into(),
        );

        let host = consume_context::<Arc<Host>>();
        let tools = host.list_tools();
        if tools.len() == 0 {
            let spec = ServerSpec {
                id: "fetch".into(),
                cmd: "npx".into(),
                args: vec!["@tokenizin/mcp-npx-fetch".into()],
            };
            let res = host.add_server(spec).await;
            if let Err(e) = res {
                eprintln!("failed to start server {e}");
            }
        }
        let tools = host.list_tools();
        // eprintln!("{tools:#?}");
        let tools: Vec<ChatCompletionTool> = tools.iter().map(move |t| {
            let t  = t.clone();
            // eprintln!("==={t:?}===");
            ChatCompletionTool { 
                r#type: async_openai::types::ChatCompletionToolType::Function, 
                function: async_openai::types::FunctionObject { 
                    name: format!("{}/{}", t.server_id, t.tool.name), 
                    description: t.tool.description, 
                    parameters: Some(t.tool.input_schema),
                    strict: Some(true),
                }
            }
        }).collect();
        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(2048u32)
            .model(model)
            .messages(chat_request.clone())
            .tools(tools)
            .build()?;

        // let req_json = serde_json::to_string(&request).unwrap();
        // eprintln!("{req_json:#?}");

        // let response = client.chat().create(request).await?;
        let mut stream = client.chat().create_stream(request).await?;
        let mut text = "".to_string();
        let mut tool_calls = vec![];
        while let Some(result) = stream.next().await {
            // eprintln!("{result:?}");
            match result {
                Ok(r) => {
                    let Some(choice) = r.choices.first() else {
                        continue;
                    };
                    let delta = &choice.delta;
                    if let Some(t) = &delta.content {
                        if !t.is_empty() {
                            text = format!("{}{}", &text, t);
                            streaming_msg.set(Some(text.clone()));
                        }
                    }
                    if let Some(tools) = &delta.tool_calls {
                        info!("{:?}", tools);
                        tool_calls.extend_from_slice(tools);
                    }
                }
                Err(e) => {
                    warn!("{}", e);
                }
            }
        }
        let mut new_chat = chat.cloned();
        let text = text.trim();
        if !text.is_empty() {
            let msg = ChatCompletionRequestAssistantMessageArgs::default()
                .content(text)
                .build()?
                .into();
            new_chat.push(msg);
        }

        for tc in tool_calls.into_iter() {
            let Some(f) = tc.function.as_ref() else { continue };
            let parts: Vec<_> = f.name.as_ref()
                .map(|s| s.as_str())
                .unwrap_or_else(|| "")
                .split("/")
                .collect();
            if parts.len() == 2 {
                let server_id = parts[0];
                let tool_name = parts[1];
                let params_str = f.arguments
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or_else(|| "{}");
                let arguments: Value = serde_json::from_str(params_str)?;
                // let params = json!({
                //     "name": tool_name,
                //     "arguments": params,
                // });
                // let method = "tools/call";

                eprintln!("Calling {server_id}/{tool_name}({arguments:?})");
                let result = host.tool_call(server_id, tool_name, arguments).await?;
                eprintln!("Result of tool call: {result:?}");
            }
            // host.invoke(server_id, method, params)
            let tcm = ChatCompletionRequestToolMessageArgs::default()
                .content(ChatCompletionRequestToolMessageContent::Text(format!(
                    "Calling {:?}()",
                    tc.function
                )))
                .build()?
                .into();
            new_chat.push(tcm);
        }

        chat.set(new_chat);
        streaming_msg.set(None);

        anyhow::Ok(())
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
                    Message { msg: (*c).clone() }
                }
                {stream_output}
            }

            div { style: "flex-grow: 0",
                InputBox {
                    disabled: disabled().unwrap_or_else(|| true),
                    on_send: Callback::new(move |s: String| async move {
                        {
                            // let b = busy.clone();
                            if busy() {
                                return;
                            }
                        }
                        {
                            busy.set(true);
                            let mut new_chat = chat.cloned();
                            let msg = ChatCompletionRequestUserMessage {
                                content: ChatCompletionRequestUserMessageContent::Text(s.clone()),
                                ..Default::default()
                            };
                            new_chat.push(msg.into());
                            chat.set(new_chat);
                            if let Err(e) = send_msg(s.clone()).await {
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

#[component]
fn InputBox(disabled: bool, on_send: Callback<String, ()>) -> Element {
    let mut text = use_signal(|| "fetch and summarize front page of hackernews".to_string());
    let set_text = move |e: Event<FormData>| {
        if disabled {
            return;
        }
        text.set(e.value());
    };
    let send = move |_e: Event<MouseData>| {
        if disabled {
            return;
        }
        on_send(text.cloned());
        text.set("".to_string());
    };
    let disabled = if disabled { Some(true) } else { None };
    let nav = navigator();
    rsx! {
        div { style: "
            display: flex;
            flex-direction: row;
            ",
            button {
                onclick: move |_e: Event<MouseData>| {
                    nav.replace(crate::Route::Settings {});
                },
                "⛭"
            }
            textarea {
                style: "flex-grow: 1",
                disabled,
                oninput: set_text,
                value: text,
            }
            button { onclick: send, disabled, "🖅" }
        }
    }
}
