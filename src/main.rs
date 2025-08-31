use std::{sync::Arc, time::Duration};

use dioxus::LaunchBuilder;

fn main() {
    dioxus::logger::init(dioxus::logger::tracing::Level::WARN).unwrap();
    
    let host = Arc::new(mcmcpcp::mcp::host::Host::new(
        Duration::from_millis(10_000),
        Duration::from_millis(30_000),
    ));

    // dioxus_native::launch(mcmcpcp::App);
    LaunchBuilder::new().with_context(host).launch(mcmcpcp::App)
}
