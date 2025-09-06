use dioxus::{logger::tracing::warn, prelude::*};

use crate::{
    AppSettings,
    app_settings::ProviderSettings,
    llm::LlmClient,
    storage::{Storage, get_storage},
    ui::box_select::BoxSelect,
};

#[derive(Props, Clone, PartialEq)]
pub struct SettingsProps {
    pub on_close: Option<EventHandler<()>>,
}

#[allow(non_snake_case)]
#[component]
pub fn Settings(props: SettingsProps) -> Element {
    let mut provider = use_signal(move || {
        ProviderSettings::OpenRouter { api_key: "".to_string(), model: None }
    });
    let mut settings = use_resource(move || async move {
        let storage = match get_storage().await {
            Ok(s) => Some(s),
            Err(e) => {
                warn!("Could not get storage: {e:?}");
                None
            }
        };
        let settings = if let Some(st) = storage {
            st.load_settings().await.unwrap()
        } else {
            None
        };
        let s = settings.unwrap_or_else(|| AppSettings {
            id: Some(1),
            provider: ProviderSettings::OpenRouter {
                api_key: "".to_string(),
                model: None,
            },
            last_chat_id: None,
        });
        provider.set(s.provider.clone());
        s
    });
    let save_settings = move |s: AppSettings| async move {
        let storage = match get_storage().await {
            Ok(s) => Some(s),
            Err(e) => {
                warn!("Could not get storage: {e:?}");
                None
            }
        };
        if let Some(st) = storage {
            if let Err(e) = st.save_settings(&s).await {
                warn!("Could not save settings: {e:?}");
            }
        }
        settings.restart();
    };
    let handle_provider_change = move |ps: ProviderSettings| async move {
        let Some(current_settings) = settings() else {
            return;
        };
        let s = AppSettings {
            provider: ps,
            ..current_settings
        };
        save_settings(s).await;
    };

    let settings = settings();
    if settings.is_none() {
        return rsx! {
            "Loading..."
        };
    }

    rsx! {
        div { 
            style: "padding: 1rem; height: 100%; overflow-y: auto;",
            onclick: move |e: Event<MouseData>| {
                e.stop_propagation();
            },
            
            div {
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;",
                h3 { style: "margin: 0;", "Settings" }
                if let Some(on_close) = props.on_close.clone() {
                    button {
                        style: "
                            background: none;
                            border: none;
                            font-size: 1.2rem;
                            cursor: pointer;
                            padding: 0.25rem;
                            color: #666;
                        ",
                        onclick: move |_| {
                            on_close.call(());
                        },
                        "×"
                    }
                }
            }
            
            hr { style: "margin-bottom: 1rem;" }
            
            ElProviderSettings {
                ps: provider,
                onchange: handle_provider_change,
            }
        }
    }
}

#[component]
fn ElProviderSettings(
    ps: Signal<ProviderSettings>,
    onchange: Callback<ProviderSettings, ()>,
) -> Element {
    let mut p_type = use_signal(|| match ps() {
        ProviderSettings::OpenRouter { .. } => "openrouter".to_string(),
        ProviderSettings::Ollama { .. } => "ollama".to_string(),
    });
    rsx! {
        label { style: "margin-top: 1em;", "API provider" }
        BoxSelect {
            value: Some(p_type()),
            options: vec!["openrouter".to_string(), "ollama".to_string()],
            on_select: move |o: Option<String>| {
                if let Some(o) = o {
                    if &o != &p_type() {
                        p_type.set(o);
                    }
                }
            },
        }
        if p_type() == "openrouter" {
            OpenRouterSettings {
                ps: ps,
                onchange,
            }
        }
        if p_type() == "ollama" {
            OllamaSettings {
                ps: ps,
                onchange,
            }
        }
    }
}

#[component]
fn OllamaSettings(
    ps: Signal<ProviderSettings>,
    onchange: Callback<ProviderSettings, ()>,
) -> Element {
    let mut available_models = use_signal(|| Vec::<String>::new());

    let handle_url_change = move |e: Event<FormData>| async move {
        let model = if let ProviderSettings::Ollama { model, .. } = ps() {
            model
        } else {
            None
        };
        onchange(ProviderSettings::Ollama { api_url: e.value(), model });
    };
    let set_model = move |model: Option<String>| async move {
        let api_url = if let ProviderSettings::Ollama { api_url, .. } = ps() {
            api_url
        } else {
            "http://192.168.29.3:11434/v1".to_string()
        };
        onchange(ProviderSettings::Ollama { api_url, model });
    };
    let get_available_models = move || async move {
        let api_url = if let ProviderSettings::Ollama { api_url, .. } = ps() {
            api_url
        } else {
            "http://192.168.29.3:11434/v1".to_string()
        };
        let lmc = LlmClient::new(api_url, "".to_string());
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

    let (api_url, model) = if let ProviderSettings::Ollama { api_url, model } = ps() {
        (api_url, model)
    } else {
        ("http://192.168.29.3:11434/v1".to_string(), None)
    };
    
    rsx! {
        div { style: "
            flex-grow: 1;
            overflow: auto;
            display: flex;
            flex-direction: column;
            ",
            label { style: "margin-top: 1em;", "API endpoint" }
            input { value: api_url, oninput: handle_url_change }
            // label { style: "margin-top: 1em;", "API Key" }
            // input { value: settings.api_key, oninput: handle_key_change }
            label { 
                style: "margin-top: 1em;", 
                "Select Model",
                button {
                    style: "max-height: 2em; margin-left: 1em;",
                    onclick: refresh_model_list,
                    "⟳ refresh list"
                }
            }
            div { style: "
                display: flex;
                flex-direction: row;
                ",
                BoxSelect {
                    value: model,
                    options: available_models(),
                    on_select: set_model,
                }
            }
        }
    }
}

#[component]
fn OpenRouterSettings(
    ps: Signal<ProviderSettings>,
    onchange: Callback<ProviderSettings, ()>,
) -> Element {
    let mut filter = use_signal(|| "".to_string());
    let mut available_models = use_signal(|| Vec::<String>::new());

    let handle_key_change = move |e: Event<FormData>| async move {
        let model = if let ProviderSettings::OpenRouter { model, .. } = ps() {
            model
        } else {
            None
        };
        onchange(ProviderSettings::OpenRouter { api_key: e.value(), model });
    };
    let set_model = move |model: Option<String>| async move {
        let api_key = if let ProviderSettings::OpenRouter { api_key, .. } = ps() {
            api_key
        } else {
            "".to_string()
        };
        onchange(ProviderSettings::OpenRouter { api_key, model });
    };
    let get_available_models = move || async move {
        let api_key = if let ProviderSettings::OpenRouter { api_key, .. } = ps() {
            api_key
        } else {
            "".to_string()
        };
        let lmc = LlmClient::new(
            "https://openrouter.ai/api/v1".to_string(), 
            api_key,
        );
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

    let (api_key, model) = if let ProviderSettings::OpenRouter { api_key, model } = ps() {
        (api_key, model)
    } else {
        ("".to_string(), None)
    };

    let filtered_models: Vec<String> = available_models()
        .into_iter()
        .filter(|s| s.to_lowercase().contains(&*filter.read()))
        .collect();
    
    rsx! {
        div { style: "
            flex-grow: 1;
            overflow: auto;
            display: flex;
            flex-direction: column;
            ",
            // label { style: "margin-top: 1em;", "API endpoint" }
            // input { value: api_url, oninput: handle_url_change }
            label { style: "margin-top: 1em;", "API Key" }
            input { value: api_key, oninput: handle_key_change }
            label { 
                style: "margin-top: 1em;", 
                "Select Model",
                button {
                    style: "max-height: 2em; margin-left: 1em;",
                    onclick: refresh_model_list,
                    "⟳ refresh list"
                }
                input {
                    value: filter,
                    oninput: move |e| {
                        filter.set(e.value());
                    },
                }
            }
            div { style: "
                display: flex;
                flex-direction: row;
                ",
                BoxSelect {
                    value: model,
                    options: filtered_models,
                    on_select: set_model,
                }
            }
        }
    }
}
