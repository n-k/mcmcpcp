use std::{sync::Arc, time::Duration};

use dioxus::{logger::tracing::Level, prelude::*};

mod box_select;
mod home;
mod settings;
mod mcp;
mod md2rsx;
mod message;

use home::Home;
use settings::Settings;
use mcp::{Host};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::logger::init(Level::WARN).unwrap();
    let host = Arc::new(Host::new(
        Duration::from_millis(10_000),
        Duration::from_millis(30_000),
    ));

    // {
    //     let spec = ServerSpec {
    //         id: "fetch".into(),
    //         cmd: "npx".into(),
    //         args: vec!["@tokenizin/mcp-npx-fetch".into()],
    //     };
    //     let res = tokio::runtime::Runtime::new().unwrap().block_on(async {
    //         host.add_server(spec).await
    //     });
    //     if let Err(e) = res {
    //         eprintln!("failed to start server {e}");
    //     }
    // }

    // dioxus_native::launch(App);
    // dioxus::launch(App);
    LaunchBuilder::new()
        .with_context(host)
        .launch(App)
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
