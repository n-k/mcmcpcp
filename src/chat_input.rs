use std::rc::Rc;

use dioxus::prelude::*;

#[component]
pub fn ChatInput(disabled: bool, on_send: Callback<String, ()>) -> Element {
    let mut text = use_signal(|| "fetch markdown and summarize front page of hackernews".to_string());
    let set_text = move |e: Event<FormData>| {
        if disabled {
            return;
        }
        text.set(e.value());
    };
    let mut _send = move || {
        if disabled {
            return;
        }
        on_send(text.cloned());
        text.set("".to_string());
    };
    let send = move |_e: Event<MouseData>| {
        _send();
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
                onkeypress: move |e: Event<KeyboardData>| {
                    let k: Rc<KeyboardData> = e.data;
                    let code = k.code();
                    let modifiers = k.modifiers();
                    if code == Code::Enter && modifiers.ctrl() {
                        _send();
                    }
                },
                value: text,
            }
            button { onclick: send, disabled, "🖅" }
        }
    }
}
