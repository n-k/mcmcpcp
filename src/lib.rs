use std::sync::Arc;

use dioxus::prelude::*;

mod box_select;
mod chat_input;
mod collapsible;
mod home;
mod settings;
mod llm;
pub mod mcp;
mod md2rsx;
mod message;
mod utils;

use home::Home;
use settings::Settings;
use mcp::Host;

use crate::mcp::ServerSpec;

const FAVICON: Asset = asset!("/assets/favicon.svg");
const MAIN_CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn App() -> Element {
    let init = use_resource(|| async {
        let host = consume_context::<Arc<Host>>();
        let spec = ServerSpec {
            id: "fetch".into(),
            cmd: "npx".into(),
            args: vec!["@tokenizin/mcp-npx-fetch".into()],
        };
        let res = host.add_server(spec).await;
        if let Err(e) = res {
            eprintln!("failed to start server {e}");
        }
        anyhow::Ok(())
    });
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        if init.read().is_none() {
            "Loading..."
        } else {
            Router::<Route> {}
        }
    }
}

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Layout)]
    #[route("/")]
    Home {},
    #[route("/settings")]
    Settings { },
    #[route("/:..segments")]
    PageNotFound { segments: Vec<String> },
}

/// Shared layout component.
#[component]
fn Layout() -> Element {
    rsx! {
        Outlet::<Route> {}
    }
}

#[component]
fn PageNotFound(segments: Vec<String>) -> Element {
    rsx! {
        "Could not find the page you are looking for."
        Link { to: Route::Home {}, "Go To Home" }
    }
}
