use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    rsx! {
        div {
            class: "content",
            Message {msg_type: MsgType::System, content: "This is the system message"},
            Message {msg_type: MsgType::Human, content: "Hi"},
            Message {msg_type: MsgType::Tool, content: "User is sending a greeting"},
            Message {msg_type: MsgType::Ai, content: "hello"},
            InputBox {},
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MsgType {
    System,
    Ai,
    Tool,
    Human,
}

#[component]
fn Message(msg_type: MsgType, content: String) -> Element {
    let class = match &msg_type {
        MsgType::System => "message system-message",
        MsgType::Ai => "message ai-message",
        MsgType::Tool => "message tool-message",
        MsgType::Human => "message human-message",
    };
    rsx! {
        div {
            class: class,
            "{content}"
        }
    }
}

#[component]
fn InputBox() -> Element {
    let mut text = use_signal(|| "".to_string());
    let set_text = move |e: Event<FormData>| {
        *text.write() = e.value();
    };
    let send = move |_e: Event<MouseData>| {
        *text.write() = "".to_string();
    };
    rsx! {
        form {
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
