use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage, ChatCompletionRequestToolMessageArgs, ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContent, CreateChatCompletionRequestArgs
    },
    Client,
};
use dioxus::{logger::tracing::info, prelude::*};
use futures::StreamExt;
use pulldown_cmark::Parser;

use crate::markdown::StreamMdToHtml;

#[component]
pub fn Home() -> Element {
    let client = use_resource(|| async {
        let api_base = "http://192.168.29.3:1234/v1";
        // Required but ignored
        let api_key = "ollama";

        let client = Client::with_config(
            OpenAIConfig::new()
                .with_api_key(api_key)
                .with_api_base(api_base),
        );
        client
    });
    let mut busy = use_signal(|| false);
    let mut streaming_msg: Signal<Option<String>> = use_signal(|| None);
    let mut chat = use_signal(|| Vec::<ChatCompletionRequestMessage>::new());
    let send_msg = move |s: String| async move {
        if let Some(client) = client() {
            let model = "llama3.2:1b";
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
            let mut streamer = StreamMdToHtml::new(vec![]);
            let mut tool_calls = vec![];
            while let Some(result) = stream.next().await {
                match result {
                    Ok(r) => {
                        let Some(choice) = r.choices.first() else {continue};
                        let delta = &choice.delta;
                        if let Some(t) = &delta.content {
                            // info!("{}", t);
                            text = format!("{}{}", &text, t);
                            let mut lines: Vec<_> = text.lines().collect();
                            let remaining = if lines.len() > 1 {
                                lines.remove(lines.len() - 1).to_string()
                            } else {
                                "".to_string()
                            };
                            for l in lines {
                                streamer.handle_line(l.to_string()).await?;
                            }
                            text = remaining;
                            streaming_msg.set(Some(String::from_utf8_lossy(&streamer.out).into()));
                        }
                        if let Some(tools) = &delta.tool_calls {
                            info!("{:?}", tools);
                            tool_calls.extend_from_slice(tools);
                        }
                    },
                    Err(e) => {
                        eprintln!("{}", e);
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
                    .content(ChatCompletionRequestToolMessageContent::Text(format!("Calling {:?}()", tc.function)))
                    .build()?
                    .into();
                new_chat.push(tcm);
            }

            chat.set(new_chat);
            streaming_msg.set(None);
        }

        anyhow::Ok(())
    };
    let stream_output: Option<Element> = streaming_msg().map(move |m| rsx! {
        div {
            class: "message ai-message",
            dangerous_inner_html: m,
        }
    });

    rsx! {
        div {
            class: "content",
            div {
                style: "flex-grow: 1",
                for c in chat.iter() {
                    Message {msg: (*c).clone()}
                }
            }
            {stream_output}
            div {
                style: "flex-grow: 0",
                InputBox {on_send: Callback::new(move |s: String| async move {
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
                })},
            }
        }
    }
}

#[component]
fn StreamingMessage() -> Element {
    rsx! {}
}

#[component]
fn Message(msg: ChatCompletionRequestMessage) -> Element {
    let (class, content) = match msg {
        ChatCompletionRequestMessage::Developer(m) => {
            let s = match m.content {
                async_openai::types::ChatCompletionRequestDeveloperMessageContent::Text(t) => t,
                async_openai::types::ChatCompletionRequestDeveloperMessageContent::Array(a) => 
                    a.into_iter().map(|m| m.text).collect::<Vec<_>>().join("\n"),
            };
            ("message system-message", s)
        },
        ChatCompletionRequestMessage::System(m) => {
            let s = match m.content {
                async_openai::types::ChatCompletionRequestSystemMessageContent::Text(t) => t,
                async_openai::types::ChatCompletionRequestSystemMessageContent::Array(a) => 
                    a.into_iter().map(|m| match m {
                        async_openai::types::ChatCompletionRequestSystemMessageContentPart::Text(t) => t.text,
                    }).collect::<Vec<_>>().join("\n"),
            };
            ("message system-message", s)
        },
        ChatCompletionRequestMessage::User(m) => {
            let s = match m.content {
                async_openai::types::ChatCompletionRequestUserMessageContent::Text(t) => t,
                async_openai::types::ChatCompletionRequestUserMessageContent::Array(a) => 
                    a.into_iter().map(|item| match item {
                        async_openai::types::ChatCompletionRequestUserMessageContentPart::Text(t) => t.text,
                        async_openai::types::ChatCompletionRequestUserMessageContentPart::ImageUrl(_) => "<Image>".to_string(),
                        async_openai::types::ChatCompletionRequestUserMessageContentPart::InputAudio(_) => "<Audio>".to_string(),
                    }).collect::<Vec<_>>().join("\n"),
            };
            ("message human-message", s)
        },
        ChatCompletionRequestMessage::Assistant(m) => {
            let s = match m.content {
                Some(v) => {
                    match v {
                        async_openai::types::ChatCompletionRequestAssistantMessageContent::Text(t) => t,
                        async_openai::types::ChatCompletionRequestAssistantMessageContent::Array(a) => 
                            a.into_iter().map(|item| match item {
                                async_openai::types::ChatCompletionRequestAssistantMessageContentPart::Text(t) => t.text,
                                async_openai::types::ChatCompletionRequestAssistantMessageContentPart::Refusal(r) => r.refusal,
                            }).collect::<Vec<_>>().join("\n"),
                    }
                },
                None => "".to_string(),
            };
            let parser = Parser::new(&s);
            let mut html_output = String::new();
            pulldown_cmark::html::push_html(&mut html_output, parser);
            ("message ai-message", html_output)
        },
        ChatCompletionRequestMessage::Tool(m) => {
            let s = match m.content {
                async_openai::types::ChatCompletionRequestToolMessageContent::Text(t) => t,
                async_openai::types::ChatCompletionRequestToolMessageContent::Array(a) => a.into_iter().map(|item| {
                    match item {
                        async_openai::types::ChatCompletionRequestToolMessageContentPart::Text(t) => t.text,
                    }
                }).collect::<Vec<_>>().join("\n"),
            };
            ("message tool-message", s)
        },
        ChatCompletionRequestMessage::Function(m) => {
            let s = match m.content {
                Some(v) => v,
                None => "".to_string(),
            };
            ("message tool-message", s)
        },
    };
    rsx! {
        div {
            class: class,
            dangerous_inner_html: content,
        }
    }
}

#[component]
fn InputBox(on_send: Callback<String, ()>) -> Element {
    let mut text = use_signal(|| "".to_string());
    let set_text = move |e: Event<FormData>| {
        text.set(e.value());
    };
    let send = move |_e: Event<MouseData>| {
        on_send(text.cloned());
        text.set("".to_string());
    };
    rsx! {
        div {
            style: "
            display: flex;
            flex-direction: row;
            padding: 1em;
            ",
            textarea {
                style: "flex-grow: 1",
                oninput: set_text,
                value: text,
            }
            button {
                onclick: send,
                "🖅"
            }
        }
    }
}
