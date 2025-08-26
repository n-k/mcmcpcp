use dioxus::prelude::*;

mod box_select;
mod home;
mod settings;
mod mcp;
mod md2rsx;
mod message;

use home::Home;
use settings::Settings;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    // dioxus_native::launch(App);
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        Router::<Route> {}
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
