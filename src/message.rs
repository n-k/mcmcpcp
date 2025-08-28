use dioxus::prelude::*;
use async_openai::{
    types::{
        ChatCompletionRequestMessage
    },
};

use crate::collapsible::Collapsible;

#[component]
pub fn Message(msg: ChatCompletionRequestMessage) -> Element {
    let (class, collapsed, content) = match msg {
        ChatCompletionRequestMessage::Developer(m) => {
            let s = match m.content {
                async_openai::types::ChatCompletionRequestDeveloperMessageContent::Text(t) => t,
                async_openai::types::ChatCompletionRequestDeveloperMessageContent::Array(a) => 
                    a.into_iter().map(|m| m.text).collect::<Vec<_>>().join("\n"),
            };
            ("message system-message", true, s)
        },
        ChatCompletionRequestMessage::System(m) => {
            let s = match m.content {
                async_openai::types::ChatCompletionRequestSystemMessageContent::Text(t) => t,
                async_openai::types::ChatCompletionRequestSystemMessageContent::Array(a) => 
                    a.into_iter().map(|m| match m {
                        async_openai::types::ChatCompletionRequestSystemMessageContentPart::Text(t) => t.text,
                    }).collect::<Vec<_>>().join("\n"),
            };
            ("message system-message", true, s)
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
            ("message human-message", false, s)
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
            ("message ai-message", false, s)
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
            ("message tool-message", true, s)
        },
        ChatCompletionRequestMessage::Function(m) => {
            let s = match m.content {
                Some(v) => v,
                None => "".to_string(),
            };
            ("message tool-message", true, s)
        },
    };
    let el = crate::md2rsx::markdown_to_rsx(&content)?;
    rsx! {
        div { class, 
            Collapsible {
                c: collapsed,
                {el}   
            } 
        }
    }
}
