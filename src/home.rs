use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestToolMessageArgs, ChatCompletionRequestToolMessageContent,
        ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageArgs,
        ChatCompletionRequestUserMessageContent, CreateChatCompletionRequestArgs,
    },
    Client,
};
use dioxus::{
    logger::tracing::{info, warn},
    prelude::*,
};
use futures::StreamExt;

use crate::message::Message;
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
        eprintln!("made client...");
        client
    });
    let model = use_resource(|| async {
        let settings = SETTINGS();
        let model = settings.model;
        eprintln!("made model... {model:?}");
        model
    });
    let is_configured = use_resource(move || async move {
        let settings = SETTINGS();
        let client_loaded = client().is_some();
        let model = model().flatten();
        let model_loaded = model.is_some();
        let configured = client_loaded && model_loaded && !settings.api_url.is_empty();
        eprintln!("configured: {configured}. Model {model:?}");
        configured
    });
    let mut busy = use_signal(|| false);
    let disabled = use_resource(move || async move {
        let disabled = !is_configured().unwrap_or_else(|| false) || busy();
        eprintln!("Disabled: {disabled}");
        disabled
    });
    let mut streaming_msg: Signal<Option<String>> = use_signal(|| None);
    let mut chat = use_signal(|| Vec::<ChatCompletionRequestMessage>::new());
    let send_msg = move |s: String| async move {
        if disabled().unwrap_or_else(|| true) {
            return Ok(());
        }
        // let model = model().unwrap(); //"qwen3-coder:30b";
        let Some(model) = model() else { return Ok(()) };
        let Some(model) = model else { return Ok(()) };
        if let Some(client) = client() {
            let mut chat_request = { chat.cloned() };
            chat_request.push(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(s)
                    .build()?
                    .into(),
            );

            let request = CreateChatCompletionRequestArgs::default()
                .max_tokens(2048u32)
                .model(model)
                .messages(chat_request.clone())
                .build()?;

            // let response = client.chat().create(request).await?;
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
                            text = format!("{}{}", &text, t);
                            streaming_msg.set(Some(text.clone()));
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
            let msg = ChatCompletionRequestAssistantMessageArgs::default()
                .content(text)
                .build()?
                .into();
            new_chat.push(msg);

            for tc in tool_calls.into_iter() {
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
        }

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
                            let b = busy.clone();
                            if *b.read() {
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
                            let res = send_msg(s.clone()).await;
                            eprintln!("{:#?}", res);
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
    let mut text = use_signal(|| "write a rust program to write a python program".to_string());
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
