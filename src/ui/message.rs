use dioxus::prelude::*;

use crate::{ui::collapsible::Collapsible, llm::{ContentPart, Message}};

#[component]
pub fn MessageEl(msg: Message) -> Element {
    let (class, collapsed, content) = match msg {
        Message::System { content } => {
            ("message system-message", true, content)
        }
        Message::Assistant { content, .. } => {
            ("message ai-message", false, content.unwrap_or_else(|| "".to_string()))
        }
        Message::Tool { content , .. } => {
            ("message tool-message", true, content)
        }
        Message::User { content } => {
            let strings: Vec<String> = content.into_iter()
                .map(|p| match p {
                    ContentPart::Text { text } => text,
                    ContentPart::ImageUrl { .. } => "[Image]".to_string(),
                })
                .collect();
            let text = strings.join("\n");
            ("message human-message", false, text)
        }
    };
    let el = crate::md2rsx::markdown_to_rsx(&content)?;
    rsx! {
        div { class,
            Collapsible { c: collapsed, {el} }
        }
    }
}
