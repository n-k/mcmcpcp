use dioxus::{logger::tracing::warn, prelude::*};

use crate::{app_settings::Chat, storage::{get_storage, AppStorage, Storage}, Route};

#[component]
pub fn ChatLog() -> Element {
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
    let chats: Resource<Option<Vec<Chat>>> = use_resource(move || async move {
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
    });

    let Some(chats) = chats() else {
        return rsx! {"Loading..."}
    };
    let Some(chats) = chats else {
        return rsx! {"Loading..."}
    };

    rsx! {
        "Chat log"
        hr {}
        for c in chats {
            if let Some(id) = &c.id {
                div{
                    Link {
                        to: Route::ChatEl { id: *id, },
                        "{id}"
                    }
                }
            } else {
                div {
                    Link {
                        to: Route::NewChat {},
                        "Unnamed chat"
                    }
                }
            }
        }
    }
}
