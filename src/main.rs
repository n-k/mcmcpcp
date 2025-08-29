use dioxus::{logger::tracing::Level, prelude::*};
use std::{sync::Arc, time::Duration};

use mcmcpcp::{mcp::Host, App};

fn main() {
    dioxus::logger::init(Level::WARN).unwrap();
    let host = Arc::new(Host::new(
        Duration::from_millis(10_000),
        Duration::from_millis(30_000),
    ));

    // dioxus_native::launch(App);
    // dioxus::launch(App);
    LaunchBuilder::new().with_context(host).launch(App)
}
