use std::sync::Arc;

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs,
        ChatCompletionTool, CreateChatCompletionRequestArgs,
    },
    Client,
};
use dioxus::{
    logger::tracing::{info, warn},
    prelude::*,
};
use futures::StreamExt;

use crate::{
    chat_input::ChatInput,
    settings::SETTINGS,
    utils::{call_tools, tools_to_openai_objects},
};
use crate::{mcp::Host, message::Message};

#[component]
pub fn Home() -> Element {
    let client = use_resource(|| async {
        let settings = SETTINGS();
        let api_base = settings.api_url;
        let api_key = settings.api_key;

        let client = Client::with_config(
            OpenAIConfig::new()
                .with_api_key(api_key)
                .with_api_base(api_base),
        );
        client
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
    let mut chat: Signal<Vec<ChatCompletionRequestMessage>> = use_signal(|| {
        vec![ChatCompletionRequestSystemMessageArgs::default()
            .content(
                "You are a helpful assistant. 
                You have access to tools which you can call to help the user in the user's task.",
            )
            .build()
            .unwrap()
            .into()]
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
        let mut count = 0u8;
        loop {
            let host = consume_context::<Arc<Host>>();
            let tools = host.list_tools();
            let tools: Vec<ChatCompletionTool> = tools_to_openai_objects(tools);
            let request = CreateChatCompletionRequestArgs::default()
                .max_tokens(2048u32)
                .model(model.clone())
                .messages(chat.cloned())
                .tools(tools)
                .build()?;

            let mut stream = client.chat().create_stream(request).await?;
            let mut text = "".to_string();
            let mut tool_calls = vec![];
            while let Some(result) = stream.next().await {
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
            if tool_calls.is_empty() {
                return Ok(());
            }
            let new_messages = call_tools(tool_calls, host.clone()).await?;
            new_chat.extend(new_messages.into_iter());
            chat.set(new_chat);
            streaming_msg.set(None);

            count += 1;
            if count > 10 {
                tool_count_warning.set(true);
                return Ok(());
            }
        }
    };

    let send_msg = move |s: String| async move {
        let mut new_chat = { chat.cloned() };
        new_chat.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(s.clone())
                .build()?
                .into(),
        );
        chat.set(new_chat);
        
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
                    Message { msg: (*c).clone() }
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
