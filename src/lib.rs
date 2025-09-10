// Copyright © 2025 Nipun Kumar

//! MCMCPCP (My Cool MCP Command Post) - A chat interface for MCP-enabled LLM interactions.
//!
//! This library provides a complete chat application built with Dioxus that allows users to:
//! - Chat with Language Learning Models (LLMs) through various APIs
//! - Execute tools and commands through Model Context Protocol (MCP) servers
//! - Configure different LLM providers and models
//! - Manage MCP server connections and tool availability
//!
//! The application is structured as a single-page application with routing between
//! the main chat interface and settings configuration.

use std::sync::Arc;

use anyhow::bail;
use dioxus::logger::tracing::warn;
use dioxus::prelude::*;

// Public modules - exposed for external use
pub mod app_settings; // Settings for the application
pub mod llm; // LLM client and message handling
pub mod mcp; // Model Context Protocol implementation

// Private modules - internal implementation details
mod md2rsx; // Markdown to RSX conversion utilities
mod storage; // DB for settings, chats etc
mod toolset;
mod ui; // User interface components
mod utils; // Utility functions for tool handling // specialised toolsets like storywriting, RP, coding ...

use app_settings::AppSettings;
use ui::home::ChatEl;
use ui::home::NewChat;
use ui::home::NewStory;
use ui::mcp_tools::McpTools;
use ui::settings::Settings;
use ui::slideout::Slideout;

use crate::mcp::host::MCPHost;
use crate::storage::Storage;
use crate::storage::get_storage;
use crate::ui::chat_log::ChatLog;

/// Application favicon - SVG format for scalability
const FAVICON: Asset = asset!("/assets/favicon.ico");
// Chat log icon
const NEW_CHAT_ICON: Asset = asset!("/assets/new_chat.svg");
// Chat log icon
const NEW_STORY_ICON: Asset = asset!("/assets/new_story.svg");
// Chat log icon
// const NEW_PPT_ICON: Asset = asset!("/assets/new_presentation.svg");
/// Main CSS stylesheet for application styling
const MAIN_CSS: Asset = asset!("/assets/main.css");
// Home icon
// const HOME_ICON: Asset = asset!("/assets/home.svg");
// Chat log icon
const CHATS_ICON: Asset = asset!("/assets/chat_list.svg");
// Settings icon
const SETTINGS_ICON: Asset = asset!("/assets/settings.svg");
// Tools icon
const TOOLS_ICON: Asset = asset!("/assets/tools.svg");

/// Root application component that sets up routing and global resources.
///
/// This component:
/// - Loads the favicon and main CSS stylesheet
/// - Shows a loading state during initialization
/// - Sets up the router for navigation between pages
#[component]
pub fn App() -> Element {
    let mut settings: Signal<Option<AppSettings>> = use_signal(|| None);
    use_context_provider(|| Arc::new(MCPHost::new()));
    use_context_provider(|| settings);

    let init = use_resource(move || async move {
        let storage = match get_storage().await {
            Ok(s) => s,
            Err(e) => {
                warn!("Could not get storage: {e:?}");
                bail!("Could not get storage: {e:?}");
            }
        };
        let s = storage.load_settings().await.unwrap();
        settings.set(s);
        anyhow::Ok(())
    });
    let _ = use_resource(move || async move {
        let st = settings();
        // sync MCP servers with settings
        let host = consume_context::<Arc<MCPHost>>();
        let specs = st.and_then(|st| st.mcp_servers).unwrap_or_default();
        host.sync_servers(specs).await?;

        anyhow::Ok(())
    });

    rsx! {
        // Set up document head with favicon and stylesheet
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        // Show loading state until initialization is complete
        if init.read().is_none() {
            "Loading..."
        } else {
            // Render the main router once initialization is done
            Router::<Route> {}
        }
    }
}

/// Application routes defining the available pages and their URL patterns.
/// 
/// The application has two main routes:
/// - `/` - Home page with the main chat interface
/// - `/chats/:id` - Individual chat pages
/// - `/*` - Catch-all for 404 pages
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Layout)]
    #[route("/")]
    NewChat { },
    #[route("/story")]
    NewStory { },
    #[route("/chats/:id")]
    ChatEl { id: u32 },
    #[route("/:..segments")]
    PageNotFound { segments: Vec<String> },
}

/// Shared layout component that wraps all pages.
///
/// Currently just renders the page content directly, but could be extended
/// to include navigation bars, headers, footers, or other shared UI elements.
#[component]
fn Layout() -> Element {
    let mut slideout = use_signal(|| false);
    let mut slideout_content = use_signal(|| SlideoutContent::ChatLog);
    let nav = navigator();

    rsx! {
        div {
            class: "tool-icons", 
            style: "
                position: fixed;
                top: 6rem;
                left: 1rem;
                z-index: 9;
                display: flex;
                flex-direction: column;
            ",
            button {
                onclick: move |_e: Event<MouseData>| {
                    nav.replace(crate::Route::NewChat {});
                },
                img { src: NEW_CHAT_ICON }
            }
            button {
                onclick: move |_e: Event<MouseData>| {
                    nav.replace(crate::Route::NewStory {});
                },
                img { src: NEW_STORY_ICON }
            }
            button {
                onclick: move |_e: Event<MouseData>| {
                    slideout_content.set(SlideoutContent::Settings);
                    slideout.set(true);
                },
                img { src: SETTINGS_ICON }
            }
            button {
                onclick: move |_e: Event<MouseData>| {
                    slideout_content.set(SlideoutContent::ChatLog);
                    slideout.toggle();
                },
                img { src: CHATS_ICON }
            }
            button {
                onclick: move |_e: Event<MouseData>| {
                    slideout_content.set(SlideoutContent::McpTools);
                    slideout.set(true);
                },
                img { src: TOOLS_ICON }
            }
        }
        Slideout {
            open: slideout,
            children: rsx! {
                match slideout_content() {
                    SlideoutContent::ChatLog => rsx! {
                        ChatLog {
                            on_close: move |_| {
                                slideout.set(false);
                            },
                        }
                    },
                    SlideoutContent::Settings => rsx! {
                        Settings {
                            on_close: move |_| {
                                slideout.set(false);
                            },
                        }
                    },
                    SlideoutContent::McpTools => rsx! {
                        McpTools {
                            on_close: move |_| {
                                slideout.set(false);
                            },
                        }
                    },
                }
            },
        }
        Outlet::<Route> {}
    }
}

#[derive(Clone, Copy, PartialEq)]
enum SlideoutContent {
    ChatLog,
    Settings,
    McpTools,
}

/// 404 page component shown when a user navigates to an invalid route.
///
/// Displays an error message and provides a link back to the home page.
#[component]
fn PageNotFound(segments: Vec<String>) -> Element {
    rsx! {
        "Could not find the page you are looking for."
        Link { to: Route::NewChat {}, "Go To Home" }
    }
}
