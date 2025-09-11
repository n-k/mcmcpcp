// Copyright © 2025 Nipun Kumar

use dioxus::{logger::tracing::warn, prelude::*};

use crate::{
    AppSettings,
    app_settings::ProviderSettings,
    llm::LlmClient,
    mcp::ServerSpec,
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
    let mut provider = use_signal(move || ProviderSettings::OpenRouter {
        api_key: "".to_string(),
        model: None,
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
            mcp_servers: Some(vec![ServerSpec {
                id: "playwright".into(),
                cmd: "npx".into(),
                args: vec!["@playwright/mcp@latest".into(), "--headless".into()],
                env: Default::default(),
                enabled: false,
            }]),
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
        if let Some(st) = storage
            && let Err(e) = st.save_settings(&s).await
        {
            warn!("Could not save settings: {e:?}");
        }
        let mut settings_ctx = consume_context::<Signal<Option<AppSettings>>>();
        settings_ctx.set(Some(s));
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
        return rsx! { "Loading..." };
    }
    let settings = settings.unwrap();

    rsx! {
        div {
            style: "padding: 1rem; height: 100%; overflow-y: auto;",
            onclick: move |e: Event<MouseData>| {
                e.stop_propagation();
            },

            div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;",
                h3 { style: "margin: 0;", "Settings" }
                if let Some(on_close) = props.on_close {
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

            ElProviderSettings { ps: provider, onchange: handle_provider_change }

            hr { style: "margin: 2rem 0 1rem 0;" }

            McpServerSettings { settings, on_save: save_settings }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
fn McpServerSettings(settings: AppSettings, on_save: Callback<AppSettings, ()>) -> Element {
    rsx! {}
}

#[cfg(not(target_arch = "wasm32"))]
#[component]
fn McpServerSettings(settings: AppSettings, on_save: Callback<AppSettings, ()>) -> Element {
    let servers = use_signal(|| settings.mcp_servers.clone().unwrap_or_default());
    let mut editing_server = use_signal(|| None::<usize>);
    let mut show_add_form = use_signal(|| false);

    // let mut _s = servers.clone();
    // let _st = settings.clone();
    // let mut handle_servers_change = move |new_servers: Vec<ServerSpec>| {
    //     let updated_settings = AppSettings {
    //         mcp_servers: Some(new_servers.clone()),
    //         .._st
    //     };
    //     _s.set(new_servers);
    //     on_save(updated_settings);
    // };

    let mut _s = servers;
    let _st = settings.clone();
    let add_server = move |server: ServerSpec| {
        let mut current_servers = _s();
        current_servers.push(server);

        let updated_settings = AppSettings {
            mcp_servers: Some(current_servers.clone()),
            .._st.clone()
        };

        _s.set(current_servers);
        on_save(updated_settings);

        show_add_form.set(false);
    };

    let mut _s = servers;
    let _st = settings.clone();
    let update_server = move |(index, server): (usize, ServerSpec)| {
        let mut current_servers = _s();
        if index < current_servers.len() {
            current_servers[index] = server;

            let updated_settings = AppSettings {
                mcp_servers: Some(current_servers.clone()),
                .._st.clone()
            };

            _s.set(current_servers);
            on_save(updated_settings);
        }
        editing_server.set(None);
    };

    let mut _s = servers;
    let _st = settings.clone();
    let delete_server = move |index: usize| {
        let mut current_servers = _s();
        if index < current_servers.len() {
            current_servers.remove(index);
            let updated_settings = AppSettings {
                mcp_servers: Some(current_servers.clone()),
                .._st.clone()
            };

            _s.set(current_servers);
            on_save(updated_settings);
        }
    };

    rsx! {
        div {
            h4 { style: "margin: 0 0 1rem 0;", "MCP Servers" }

            // Server list
            div { style: "margin-bottom: 1rem;",
                if servers().is_empty() {
                    p { style: "color: #666; font-style: italic;", "No MCP servers configured" }
                } else {
                    for (index , server) in servers().iter().enumerate() {
                        ServerItem {
                            key: "{index}",
                            server: server.clone(),
                            index,
                            is_editing: editing_server() == Some(index),
                            on_edit: move |idx: usize| {
                                editing_server.set(Some(idx));
                            },
                            on_save: update_server.clone(),
                            on_cancel: move |_| {
                                editing_server.set(None);
                            },
                            on_delete: delete_server.clone(),
                        }
                    }
                }
            }

            // Add server section
            if show_add_form() {
                ServerForm {
                    server: None,
                    on_save: add_server,
                    on_cancel: move |_| {
                        show_add_form.set(false);
                    },
                }
            } else {
                button {
                    style: "
                        background: #007bff;
                        color: white;
                        border: none;
                        padding: 0.5rem 1rem;
                        border-radius: 4px;
                        cursor: pointer;
                    ",
                    onclick: move |_| {
                        show_add_form.set(true);
                    },
                    "+ Add Server"
                }
            }
        }
    }
}

#[component]
fn ServerItem(
    server: ServerSpec,
    index: usize,
    is_editing: bool,
    on_edit: Callback<usize, ()>,
    on_save: Callback<(usize, ServerSpec), ()>,
    on_cancel: Callback<(), ()>,
    on_delete: Callback<usize, ()>,
) -> Element {
    let on_toggle = {
        let server = server.clone();
        move |e: Event<FormData>| {
            let mut updated_server = server.clone();
            updated_server.enabled = e.checked();
            on_save((index, updated_server));
        }
    };
    if is_editing {
        rsx! {
            ServerForm {
                server: Some(server),
                on_save: move |s: ServerSpec| {
                    on_save((index, s));
                },
                on_cancel,
            }
        }
    } else {
        let args_display = server.args.join(" ");
        let env_display = server
            .env
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(", ");

        let status_color = if server.enabled { "#28a745" } else { "#6c757d" };
        let status_text = if server.enabled {
            "Enabled"
        } else {
            "Disabled"
        };

        rsx! {
            div { style: format!("
                    border: 1px solid #ddd;
                    border-radius: 4px;
                    padding: 1rem;
                    margin-bottom: 0.5rem;
                    background: #f9f9f9;
                    opacity: {};
                ", if server.enabled { "1" } else { "0.7" }),
                div { style: "display: flex; justify-content: space-between; align-items: flex-start;",
                    div { style: "flex-grow: 1;",
                        div { style: "display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.25rem;",
                            div { style: "font-weight: bold;", "{server.id}" }
                            div {
                                style: "
                                    font-size: 0.7rem;
                                    padding: 0.125rem 0.375rem;
                                    border-radius: 10px;
                                    background: {status_color};
                                    color: white;
                                    font-weight: bold;
                                ",
                                "{status_text}"
                            }
                        }
                        div { style: "font-family: monospace; font-size: 0.9em; color: #666; margin-bottom: 0.25rem;",
                            "{server.cmd}"
                        }
                        if !server.args.is_empty() {
                            div { style: "font-family: monospace; font-size: 0.8em; color: #888; margin-bottom: 0.25rem;",
                                "Args: {args_display}"
                            }
                        }
                        if !server.env.is_empty() {
                            div { style: "font-family: monospace; font-size: 0.8em; color: #888;",
                                "Env: {env_display}"
                            }
                        }
                    }
                    div { style: "
                        display: flex;
                        flex-direction: column;
                        align-items: flex-end;
                        gap: 0.5rem;
                        ",
                        // Toggle switch
                        div {
                            style: "
                                display: flex;
                                align-items: center;
                                gap: 0.5rem;
                                margin-bottom: 0.5rem;
                                border: 1px solid silver;
                            ",
                            span { style: "font-size: 0.8rem; color: #666;", "Enable:" }
                            input {
                                r#type: "checkbox",
                                checked: server.enabled,
                                oninput: move |_e| {
                                    on_toggle(_e);
                                },
                            }
                        }
                        // Action buttons
                        div { style: "
                            display: flex;
                            gap: 0.5rem;
                            ",
                            button {
                                style: "
                                    background: #28a745;
                                    color: white;
                                    border: none;
                                    padding: 0.25rem 0.5rem;
                                    border-radius: 3px;
                                    cursor: pointer;
                                    font-size: 0.8rem;
                                ",
                                onclick: move |_| {
                                    on_edit(index);
                                },
                                "Edit"
                            }
                            button {
                                style: "
                                    background: #dc3545;
                                    color: white;
                                    border: none;
                                    padding: 0.25rem 0.5rem;
                                    border-radius: 3px;
                                    cursor: pointer;
                                    font-size: 0.8rem;
                                ",
                                onclick: move |_| {
                                    on_delete(index);
                                },
                                "Delete"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ServerForm(
    server: Option<ServerSpec>,
    on_save: Callback<ServerSpec, ()>,
    on_cancel: Callback<(), ()>,
) -> Element {
    let mut id = use_signal(|| server.as_ref().map(|s| s.id.clone()).unwrap_or_default());
    let mut cmd = use_signal(|| server.as_ref().map(|s| s.cmd.clone()).unwrap_or_default());
    let mut args_text = use_signal(|| {
        server
            .as_ref()
            .map(|s| s.args.join(" "))
            .unwrap_or_default()
    });
    let mut env_vars = use_signal(|| server.as_ref().map(|s| s.env.clone()).unwrap_or_default());
    let mut new_env_key = use_signal(String::new);
    let mut new_env_value = use_signal(String::new);

    let add_env_var = move |_| {
        let key = new_env_key().trim().to_string();
        let value = new_env_value().trim().to_string();

        if !key.is_empty() {
            let mut current_env = env_vars();
            current_env.insert(key, value);
            env_vars.set(current_env);
            new_env_key.set(String::new());
            new_env_value.set(String::new());
        }
    };

    let mut remove_env_var = move |key: String| {
        let mut current_env = env_vars();
        current_env.remove(&key);
        env_vars.set(current_env);
    };

    let server_enabled = server.as_ref().map(|s| s.enabled).unwrap_or(true);
    let handle_save = move |_| async move {
        let id_val = id().trim().to_string();
        let cmd_val = cmd().trim().to_string();
        let args_text = args_text.cloned();
        let args_val = args_text.trim();

        if id_val.is_empty() || cmd_val.is_empty() {
            return; // Basic validation
        }

        let args_vec = if args_val.is_empty() {
            Vec::new()
        } else {
            args_val.split_whitespace().map(|s| s.to_string()).collect()
        };

        let server_spec = ServerSpec {
            id: id_val,
            cmd: cmd_val,
            args: args_vec,
            env: env_vars(),
            enabled: server_enabled,
        };

        on_save(server_spec);
    };

    rsx! {
        div { style: "
                border: 1px solid #007bff;
                border-radius: 4px;
                padding: 1rem;
                margin-bottom: 0.5rem;
                background: #f8f9fa;
            ",
            div { style: "margin-bottom: 1rem;",
                label { style: "display: block; margin-bottom: 0.25rem; font-weight: bold;",
                    "Server ID"
                }
                input {
                    style: "
                        width: 100%;
                        padding: 0.5rem;
                        border: 1px solid #ddd;
                        border-radius: 3px;
                        box-sizing: border-box;
                    ",
                    value: id(),
                    placeholder: "e.g., weather-server",
                    oninput: move |e| {
                        id.set(e.value());
                    },
                }
            }

            div { style: "margin-bottom: 1rem;",
                label { style: "display: block; margin-bottom: 0.25rem; font-weight: bold;",
                    "Command"
                }
                input {
                    style: "
                        width: 100%;
                        padding: 0.5rem;
                        border: 1px solid #ddd;
                        border-radius: 3px;
                        box-sizing: border-box;
                    ",
                    value: cmd(),
                    placeholder: "e.g., python -m weather_server",
                    oninput: move |e| {
                        cmd.set(e.value());
                    },
                }
            }

            div { style: "margin-bottom: 1rem;",
                label { style: "display: block; margin-bottom: 0.25rem; font-weight: bold;",
                    "Arguments (space-separated)"
                }
                input {
                    style: "
                        width: 100%;
                        padding: 0.5rem;
                        border: 1px solid #ddd;
                        border-radius: 3px;
                        box-sizing: border-box;
                    ",
                    value: args_text(),
                    placeholder: "e.g., --port 8080 --config config.json",
                    oninput: move |e| {
                        args_text.set(e.value());
                    },
                }
            }

            // Environment Variables Section
            div { style: "margin-bottom: 1rem;",
                label { style: "display: block; margin-bottom: 0.5rem; font-weight: bold;",
                    "Environment Variables"
                }

                // Existing environment variables
                if !env_vars().is_empty() {
                    div { style: "margin-bottom: 0.5rem;",
                        for (key , value) in env_vars().iter() {
                            div {
                                key: "{key}",
                                style: "
                                    display: flex;
                                    align-items: center;
                                    gap: 0.5rem;
                                    margin-bottom: 0.25rem;
                                    padding: 0.25rem;
                                    background: #f0f0f0;
                                    border-radius: 3px;
                                ",
                                span { style: "font-family: monospace; font-size: 0.9em;",
                                    "{key}={value}"
                                }
                                button {
                                    style: "
                                        background: #dc3545;
                                        color: white;
                                        border: none;
                                        padding: 0.125rem 0.25rem;
                                        border-radius: 2px;
                                        cursor: pointer;
                                        font-size: 0.7rem;
                                    ",
                                    onclick: {
                                        let key = key.clone();
                                        move |_| {
                                            remove_env_var(key.clone());
                                        }
                                    },
                                    "×"
                                }
                            }
                        }
                    }
                }

                // Add new environment variable
                div { style: "display: flex; gap: 0.5rem; align-items: flex-end;",
                    div { style: "flex: 1;",
                        label { style: "display: block; margin-bottom: 0.25rem; font-size: 0.9em;",
                            "Key"
                        }
                        input {
                            style: "
                                width: 100%;
                                padding: 0.375rem;
                                border: 1px solid #ddd;
                                border-radius: 3px;
                                box-sizing: border-box;
                                font-size: 0.9em;
                            ",
                            value: new_env_key(),
                            placeholder: "e.g., API_KEY",
                            oninput: move |e| {
                                new_env_key.set(e.value());
                            },
                        }
                    }
                    div { style: "flex: 2;",
                        label { style: "display: block; margin-bottom: 0.25rem; font-size: 0.9em;",
                            "Value"
                        }
                        input {
                            style: "
                                width: 100%;
                                padding: 0.375rem;
                                border: 1px solid #ddd;
                                border-radius: 3px;
                                box-sizing: border-box;
                                font-size: 0.9em;
                            ",
                            value: new_env_value(),
                            placeholder: "e.g., your-api-key-here",
                            oninput: move |e| {
                                new_env_value.set(e.value());
                            },
                        }
                    }
                    button {
                        style: "
                            background: #28a745;
                            color: white;
                            border: none;
                            padding: 0.375rem 0.75rem;
                            border-radius: 3px;
                            cursor: pointer;
                            font-size: 0.9em;
                        ",
                        onclick: add_env_var,
                        "Add"
                    }
                }
            }

            div { style: "display: flex; gap: 0.5rem; justify-content: flex-end;",
                button {
                    style: "
                        background: #6c757d;
                        color: white;
                        border: none;
                        padding: 0.5rem 1rem;
                        border-radius: 3px;
                        cursor: pointer;
                    ",
                    onclick: move |_| {
                        on_cancel(());
                    },
                    "Cancel"
                }
                button {
                    style: "
                        background: #007bff;
                        color: white;
                        border: none;
                        padding: 0.5rem 1rem;
                        border-radius: 3px;
                        cursor: pointer;
                    ",
                    onclick: handle_save,
                    "Save"
                }
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
        h4 { style: "margin: 0 0 1rem 0;", "API provider" }
        BoxSelect {
            value: Some(p_type()),
            options: vec!["openrouter".to_string(), "ollama".to_string()],
            on_select: move |o: Option<String>| {
                if let Some(o) = o && o != p_type() {
                    p_type.set(o);
                }
            },
        }
        if p_type() == "openrouter" {
            OpenRouterSettings { ps, onchange }
        }
        if p_type() == "ollama" {
            OllamaSettings { ps, onchange }
        }
    }
}

#[component]
fn OllamaSettings(
    ps: Signal<ProviderSettings>,
    onchange: Callback<ProviderSettings, ()>,
) -> Element {
    let mut available_models = use_signal(Vec::<String>::new);

    let handle_url_change = move |e: Event<FormData>| async move {
        let model = if let ProviderSettings::Ollama { model, .. } = ps() {
            model
        } else {
            None
        };
        onchange(ProviderSettings::Ollama {
            api_url: e.value(),
            model,
        });
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
            label { style: "margin-top: 1em;",
                "Select Model"
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
    let mut available_models = use_signal(Vec::<String>::new);
    let mut auth_url = use_signal(|| "".to_string());

    let set_key = move |key: String| async move {
        let model = if let ProviderSettings::OpenRouter { model, .. } = ps() {
            model
        } else {
            None
        };
        onchange(ProviderSettings::OpenRouter {
            api_key: key,
            model,
        });
    };
    let handle_key_change = move |e: Event<FormData>| async move {
        set_key(e.value()).await;
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
        let lmc = LlmClient::new("https://openrouter.ai/api/v1".to_string(), api_key);
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

    #[cfg(not(target_arch = "wasm32"))]
    let start_pkce = move || async move {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        use rand::Rng;
        use rand::distr::Alphanumeric;
        use sha2::{Digest, Sha256};
        use tokio::net::TcpListener;
        // use std::net::TcpListener;
        // use std::io::{Read, Write};
        use urlencoding::encode;

        // auth_url.set("1".to_string());
        // ---- Step 1: PKCE values ----
        let code_verifier: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect();

        let code_challenge = {
            let digest = Sha256::digest(code_verifier.as_bytes());
            URL_SAFE_NO_PAD.encode(digest)
        };

        let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap(); // OS picks port
        let port = listener.local_addr().unwrap().port();
        let redirect_uri = format!("http://127.0.0.1:{}/callback", port);

        // ---- Step 2: Redirect user to OpenRouter auth ----
        let or_auth_url = format!(
            "https://openrouter.ai/auth?callback_url={}&code_challenge={}&code_challenge_method=S256",
            encode(&redirect_uri),
            code_challenge
        );
        println!("Open this URL in your browser:\n\n{}\n", or_auth_url);

        auth_url.set(or_auth_url.clone());

        spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};

            println!("Waiting for OAuth callback on {}", redirect_uri);

            // ---- Step 3: Wait for redirect with auth code ----
            let (mut stream, _) = listener.accept().await.unwrap(); // Accept one connection
            let mut buffer = [0; 1024];
            stream.read(&mut buffer).await.unwrap();

            let request = String::from_utf8_lossy(&buffer);
            eprintln!("Request: {request}");
            let code = request
                .split("code=")
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .and_then(|s| s.split('&').next())
                .unwrap()
                .to_string();

            // Send a simple response to the browser
            let response =
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nYou can close this tab now.";
            stream.write_all(response.as_bytes()).await.unwrap();

            println!("Got authorization code: {}", code);

            // ---- Step 4: Exchange code for tokens ----
            let token_url = "https://openrouter.ai/api/v1/auth/keys";

            let client = reqwest::Client::new();
            let res = client
                .post(token_url)
                .json(&serde_json::json!({
                    "code": &code,
                    "code_verifier": &code_verifier,
                    "code_challenge_method": "S256",
                }))
                .send()
                .await
                .unwrap();
            auth_url.set("".to_string());
            if res.status().is_success() {
                let j: serde_json::Value = res.json().await.unwrap();
                println!("{j:?}");
                let key = j
                    .get("key")
                    .map(|v| v.as_str())
                    .flatten()
                    .map(|s| s.to_string());
                if let Some(key) = key {
                    set_key(key).await;
                }
            } else {
                let text = res.text().await.unwrap();
                println!("Token response: {}", text);
            }
        });

        anyhow::Ok(())
    };

    #[cfg(target_arch = "wasm32")]
    let start_pkce = move || async move {};

    let filtered_models: Vec<String> = available_models()
        .into_iter()
        .filter(|s| s.to_lowercase().contains(&*filter.read()))
        .collect();
    let auth_url = auth_url();
    let has_auth_url = !auth_url.is_empty();

    #[cfg(not(target_arch = "wasm32"))]
    let start_pkce_button = rsx! {
        button {
            disabled: has_auth_url,
            onclick: move |_| async move {
                let _ = start_pkce().await;
            },
            "Login using Openrouter"
        }
    };
    #[cfg(target_arch = "wasm32")]
    let start_pkce_button = rsx! {};

    let (api_key, model) = if let ProviderSettings::OpenRouter { api_key, model } = ps() {
        (api_key, model)
    } else {
        ("".to_string(), None)
    };

    rsx! {
        div { style: "
            flex-grow: 1;
            overflow: auto;
            display: flex;
            flex-direction: column;
            ",
            label { style: "margin-top: 1em;", "API Key" }
            input { value: api_key, oninput: handle_key_change }
            p {
                {start_pkce_button}
            }
            if has_auth_url {
                Link {
                    to: auth_url.clone(),
                    "Open Openrouter"
                }
                "Or copy and paste this URL into your browser"
                textarea {
                    disabled: true,
                    value: "{auth_url}",
                }
            }
            label { style: "margin-top: 1em;",
                "Select Model"
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
