use async_openai::{config::OpenAIConfig, Client};
use dioxus::prelude::*;
use dioxus_primitives::select::{
    SelectValue, Select, SelectList, SelectOption, SelectTrigger
};

pub static SETTINGS: GlobalSignal<AppSettings> = Signal::global(|| AppSettings {
    api_url: "http://192.168.29.3:11434/v1".to_string(),
    api_key: "dummy".to_string(),
    model: "qwen3-coder:30b".to_string(),
});

#[derive(Clone)]
pub struct AppSettings {
    pub api_url: String,
    pub api_key: String,
    pub model: String,
}

#[allow(non_snake_case)]
#[component]
pub fn Settings() -> Element {
    let settings = use_signal(|| SETTINGS());
    let mut model = use_signal(|| settings().model);
    let mut available_models = use_signal(|| Vec::<String>::new());
    let handle_url_change = move |e: Event<FormData>| {
        let current_settings = SETTINGS();
        let s = AppSettings {
            api_url: e.value(),
            ..current_settings
        };
        *SETTINGS.write() = s;
    };
    let handle_key_change = move |e: Event<FormData>| {
        let current_settings = SETTINGS();
        let s = AppSettings {
            api_key: e.value(),
            ..current_settings
        };
        *SETTINGS.write() = s;
    };
    let mut set_model = move |v: Option<String>| {
        let current_settings = SETTINGS();
        let s = AppSettings {
            model: v.clone().unwrap_or_else(|| "".to_string()),
            ..current_settings
        };
        *SETTINGS.write() = s;
        model.set(v.unwrap_or_else(|| "".to_string()));
    };
    let get_available_models = move || async move {
        let s = settings();
        let client = Client::with_config(
            OpenAIConfig::new()
                .with_api_key(s.api_key)
                .with_api_base(s.api_url),
        );
        let models = client.models().list().await?;
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
    let nav = navigator();

    let model_options = available_models().into_iter().enumerate().map(|(i, m)| {
        rsx! {
            SelectOption::<String> {
                index: i,
                class: "select-option",
                value: m.clone(),
                text_value: "{m}",
                {m}
            }
        }
    });

    rsx! {
        div {
            class: "content",
            div {
                style: "
                flex-grow: 0;
                display: flex;
                flex-direction: row;
                margin-top: 1em;
                ",
                h2 {
                    "Settings"
                }
                button {
                    onclick: move |_e: Event<MouseData>| {
                        nav.replace(crate::Route::Home {});
                    },
                    "Cancel"
                }
                button {
                    onclick: move |_e: Event<MouseData>| {
                        *SETTINGS.write() = settings();
                    },
                    "Save"
                }
            }
            div {
                style: "
                flex-grow: 1;
                overflow: auto;
                display: flex;
                flex-direction: column;
                ",
                label {
                    style: "margin-top: 1em;",
                    "API endpoint"
                }
                input {
                    value: settings().api_url,
                    oninput: handle_url_change,
                }
                label {
                    style: "margin-top: 1em;",
                    "API Key"
                }
                input {
                    value: settings().api_key,
                    oninput: handle_key_change,
                }
                label {
                    style: "margin-top: 1em;",
                    "Select Model"
                }
                div {
                    style: "
                    display: flex;
                    flex-direction: row;
                    ",
                    Select::<String> {
                        class: "select",
                        style: "flex-grow: 1; width: 100%;",
                        value: Some(model()),
                        on_value_change: move |e: Option<String>| {
                            set_model(e);
                        },
                        placeholder: if model().is_empty() {"Select Model"} else {model()},
                        SelectTrigger {
                            width: "100%",
                            height: "2em",
                            SelectValue {}
                        }
                        SelectList {
                            SelectOption::<String> {
                                index: 0usize,
                                class: "select-option",
                                value: model(),
                                text_value: model(),
                                SelectValue {}
                            }
                            {model_options}
                        }
                    }
                    button {
                        onclick: refresh_model_list,
                        "⟳"
                    }
                }
            }
            div {
                {model}
            }
        }
    }
}
