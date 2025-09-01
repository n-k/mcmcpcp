use std::rc::Rc;

use dioxus::prelude::*;

const SEND_ICON: Asset = asset!("/assets/send.svg");
const SETTINGS_ICON: Asset = asset!("/assets/settings.svg");

#[component]
pub fn ChatInput(disabled: bool, on_send: Callback<String, ()>) -> Element {
    let mut text =
        use_signal(|| "fetch and summarize front page of hackernews".to_string());
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
                img { src: SETTINGS_ICON }
            }
            textarea {
                style: "flex-grow: 1; max-height: 10em; height: 4em;",
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
            button { onclick: send, disabled,
                img { src: SEND_ICON }
            }
        }
    }
}
