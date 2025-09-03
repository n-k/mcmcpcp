use dioxus::{logger::tracing::warn, prelude::*};

use crate::{app_settings::Chat, storage::{get_storage, AppStorage, Storage}, Route};

#[derive(Props, Clone, PartialEq)]
pub struct ChatLogProps {
    pub on_close: Option<EventHandler<()>>,
}

#[component]
pub fn ChatLog(props: ChatLogProps) -> Element {
    let stg: Resource<Option<AppStorage>> = use_resource(move || async move {
        let storage = match get_storage().await {
            Ok(s) => Some(s),
            Err(e) => {
                warn!("Could not get storage: {e:?}");
                None
            }
        };
        storage
    });
    
    let mut refresh_trigger = use_signal(|| 0);
    
    let chats: Resource<Option<Vec<Chat>>> = use_resource(move || {
        let _ = refresh_trigger(); // Subscribe to refresh trigger
        async move {
            let Some(stg) = &*stg.read() else { return None };
            let Some(stg) = stg else { return None };
            let chats = match stg.list_chats().await {
                Ok(c) => c,
                Err(e) => {
                    warn!("Could not get chats: {e:?}");
                    return None;
                }
            };
            Some(chats)
        }
    });

    let delete_chat = move |chat_id: u32| {
        spawn(async move {
            if let Ok(storage) = get_storage().await {
                if let Err(e) = storage.delete_chat(chat_id).await {
                    warn!("Failed to delete chat {}: {e:?}", chat_id);
                } else {
                    // Trigger refresh of chat list
                    refresh_trigger.set(refresh_trigger() + 1);
                }
            }
        });
    };

    let Some(chats) = chats() else {
        return rsx! {
            div {
                style: "padding: 1rem;",
                "Loading..."
            }
        }
    };
    let Some(chats) = chats else {
        return rsx! {
            div {
                style: "padding: 1rem;",
                "Loading..."
            }
        }
    };

    rsx! {
        div {
            style: "padding: 1rem; height: 100%; overflow-y: auto;",
            onclick: move |e: Event<MouseData>| {
                e.stop_propagation();
            },
            
            div {
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;",
                h3 { style: "margin: 0;", "Chat History" }
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
            
            if chats.is_empty() {
                div {
                    style: "text-align: center; color: #666; padding: 2rem;",
                    "No chats yet"
                }
            } else {
                for c in chats {
                    {
                        let chat_id = c.id;
                        let message_count = c.messages.len();
                        let on_close_handler = props.on_close.clone();
                        
                        rsx! {
                            div {
                                style: "
                                    display: flex;
                                    align-items: center;
                                    justify-content: space-between;
                                    padding: 0.5rem;
                                    margin-bottom: 0.5rem;
                                    border: 1px solid #ddd;
                                    border-radius: 4px;
                                    background: #f9f9f9;
                                ",
                                
                                div {
                                    style: "flex: 1;",
                                    if let Some(id) = chat_id {
                                        Link {
                                            style: "text-decoration: none; color: #333;",
                                            to: Route::ChatEl { id },
                                            onclick: move |_| {
                                                if let Some(on_close) = &on_close_handler {
                                                    on_close.call(());
                                                }
                                            },
                                            div {
                                                style: "font-weight: bold; margin-bottom: 0.25rem;",
                                                "Chat #{id}"
                                            }
                                            div {
                                                style: "font-size: 0.8rem; color: #666;",
                                                "{message_count} messages"
                                            }
                                        }
                                    } else {
                                        Link {
                                            style: "text-decoration: none; color: #333;",
                                            to: Route::NewChat {},
                                            onclick: move |_| {
                                                if let Some(on_close) = &on_close_handler {
                                                    on_close.call(());
                                                }
                                            },
                                            div {
                                                style: "font-weight: bold; margin-bottom: 0.25rem;",
                                                "Unnamed chat"
                                            }
                                            div {
                                                style: "font-size: 0.8rem; color: #666;",
                                                "{message_count} messages"
                                            }
                                        }
                                    }
                                }
                                
                                if let Some(id) = chat_id {
                                    button {
                                        style: "
                                            background: #ff4444;
                                            color: white;
                                            border: none;
                                            border-radius: 3px;
                                            padding: 0.25rem 0.5rem;
                                            cursor: pointer;
                                            font-size: 0.8rem;
                                            margin-left: 0.5rem;
                                        ",
                                        onclick: move |e: Event<MouseData>| {
                                            e.stop_propagation();
                                            delete_chat(id);
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
    }
}
