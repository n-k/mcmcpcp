use dioxus::prelude::*;

use crate::{
    llm::{ContentPart, FunctionDelta, Message},
    ui::collapsible::Collapsible,
};

#[component]
pub fn MessageEl(msg: Message) -> Element {
    match msg {
        Message::System { content } => {
            let el = crate::md2rsx::markdown_to_rsx(&content)?;
            rsx! {
                div {
                    class: "message system-message",
                    Collapsible { c: true, {el} }
                }
            }
        }
        Message::Assistant {
            content,
            tool_calls,
        } => {
            let content = content.unwrap_or_else(|| "".to_string());
            let el = crate::md2rsx::markdown_to_rsx(&content)?;
            let fns: Vec<FunctionDelta> = tool_calls
                .unwrap_or_default()
                .into_iter()
                .map(|tc| tc.function)
                .filter(|f| f.is_some())
                .map(|f| f.unwrap())
                .collect();
            rsx! {
                div {
                    class: "message ai-message",
                    Collapsible {
                        c: false,
                        {el}
                        for f in fns {
                            div {
                                "{f:?}"
                            }
                        }
                    }
                }
            }
        }
        Message::Tool { content, .. } => {
            let el = crate::md2rsx::markdown_to_rsx(&content)?;
            rsx! {
                div {
                    class: "message tool-message",
                    Collapsible { c: true, {el} }
                }
            }
        }
        Message::User { content } => {
            let strings: Vec<String> = content
                .into_iter()
                .map(|p| match p {
                    ContentPart::Text { text } => text,
                    ContentPart::ImageUrl { .. } => "[Image]".to_string(),
                })
                .collect();
            let text = strings.join("\n");
            let el = crate::md2rsx::markdown_to_rsx(&text)?;
            rsx! {
                div {
                    class: "message human-message",
                    Collapsible { c: false, {el} }
                }
            }
        }
    }
}
