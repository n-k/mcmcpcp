use dioxus::prelude::*;

use crate::{box_select::BoxSelect, llm::LlmClient};

pub static SETTINGS: GlobalSignal<AppSettings> = Signal::global(|| AppSettings {
    api_url: "http://192.168.29.3:11434/v1".to_string(),
    api_key: "dummy".to_string(),
    model: None,
});

#[derive(Clone)]
pub struct AppSettings {
    pub api_url: String,
    pub api_key: String,
    pub model: Option<String>,
}

#[allow(non_snake_case)]
#[component]
pub fn Settings() -> Element {
    let settings = use_signal(|| SETTINGS());
    let mut model = use_signal(|| settings().model);
    let mut api_url = use_signal(|| settings().api_url);
    let mut api_key = use_signal(|| settings().api_key);
    let mut available_models = use_signal(|| Vec::<String>::new());
    let handle_url_change = move |e: Event<FormData>| {
        let current_settings = SETTINGS();
        let s = AppSettings {
            api_url: e.value(),
            ..current_settings
        };
        *SETTINGS.write() = s;
        api_url.set(e.value());
    };
    let handle_key_change = move |e: Event<FormData>| {
        let current_settings = SETTINGS();
        let s = AppSettings {
            api_key: e.value(),
            ..current_settings
        };
        *SETTINGS.write() = s;
        api_key.set(e.value());
    };
    let mut set_model = move |v: Option<String>| {
        let current_settings = SETTINGS();
        let s = AppSettings {
            model: v.clone(),
            ..current_settings
        };
        *SETTINGS.write() = s;
        model.set(v);
    };
    let get_available_models = move || async move {
        let s = settings();
        // let client = Client::with_config(
        //     OpenAIConfig::new()
        //         .with_api_key(s.api_key)
        //         .with_api_base(s.api_url),
        // );
        let lmc = LlmClient::new(s.api_url, s.api_key);
        let models = lmc.models().await?;
        let names = models.data.into_iter().map(|m| m.id).collect::<Vec<_>>();
        anyhow::Ok(names)
    };
    let refresh_model_list = move |_e: Event<MouseData>| async move {
        match get_available_models().await {
            Ok(models) => {
                available_models.set(models);
            }
            Err(e) => {
                eprintln!("{e}");
            }
        }
    };

    rsx! {
        div { class: "content", style: "padding: 1em;",
            div { style: "
                flex-grow: 0;
                display: flex;
                flex-direction: row;
                margin-top: 1em;
                justify-content: space-between;
                ",
                h3 { "Settings" }
                div { style: "
                    align-self: center;

                    ",
                    Link { to: crate::Route::Home {}, "Back" }
                }
            }
            div { style: "
                flex-grow: 1;
                overflow: auto;
                display: flex;
                flex-direction: column;
                ",
                label { style: "margin-top: 1em;", "API endpoint" }
                input { value: api_url(), oninput: handle_url_change }
                label { style: "margin-top: 1em;", "API Key" }
                input { value: api_key(), oninput: handle_key_change }
                label { style: "margin-top: 1em;", "Select Model" }
                div { style: "
                    display: flex;
                    flex-direction: row;
                    ",
                    BoxSelect {
                        value: model(),
                        options: available_models(),
                        on_select: move |v| {
                            set_model(v);
                        },
                    }
                    button {
                        style: "max-height: 2em;",
                        onclick: refresh_model_list,
                        "⟳"
                    }
                }
            }
        }
    }
}
