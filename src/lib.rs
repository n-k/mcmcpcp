use dioxus::prelude::*;

pub mod llm;
pub mod mcp;
mod md2rsx;
mod ui;
mod utils;

use ui::home::Home;
use ui::settings::Settings;

const FAVICON: Asset = asset!("/assets/favicon.svg");
const MAIN_CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn App() -> Element {
    let init = use_resource(|| async {
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
