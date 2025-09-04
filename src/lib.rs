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

use dioxus::prelude::*;

// Public modules - exposed for external use
pub mod app_settings;   // Settings for the application
pub mod llm;            // LLM client and message handling
pub mod mcp;            // Model Context Protocol implementation

// Private modules - internal implementation details
mod md2rsx;     // Markdown to RSX conversion utilities
mod ui;         // User interface components
mod utils;      // Utility functions for tool handling
mod storage;    // DB for settings, chats etc
mod toolset;    // specialised toolsets like storywriting, RP, coding ...

use app_settings::AppSettings;
use ui::home::NewChat;
use ui::home::ChatEl;
use ui::settings::Settings;
use ui::slideout::Slideout;

use crate::ui::chat_log::ChatLog;

/// Application favicon - SVG format for scalability
const FAVICON: Asset = asset!("/assets/favicon.ico");
/// Main CSS stylesheet for application styling
const MAIN_CSS: Asset = asset!("/assets/main.css");
// Home icon
const HOME_ICON: Asset = asset!("/assets/home.svg");
// Chat log icon
const CHATS_ICON: Asset = asset!("/assets/chat_list.svg");
// Settings icon
const SETTINGS_ICON: Asset = asset!("/assets/settings.svg");


/// Root application component that sets up routing and global resources.
/// 
/// This component:
/// - Loads the favicon and main CSS stylesheet
/// - Shows a loading state during initialization
/// - Sets up the router for navigation between pages
#[component]
pub fn App() -> Element {
    // Initialize application resources (currently just a placeholder)
    let init = use_resource(|| async {
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
    let mut slideout = use_signal(|| { false });
    let mut slideout_content = use_signal(|| SlideoutContent::ChatLog);
    let nav = navigator();
    
    rsx! {
        div {
            style: "
            position: fixed;
            top: 1rem;
            right: 1rem;
            z-index: 9;
            display: flex;
            flex-direction: column;
            ",
            button {
                onclick: move |_e: Event<MouseData>| {
                    nav.replace(crate::Route::NewChat {});
                },
                img { src: HOME_ICON }
            },
            button {
                onclick: move |_e: Event<MouseData>| {
                    slideout_content.set(SlideoutContent::Settings);
                    slideout.set(true);
                },
                img { src: SETTINGS_ICON }
            },
            button {
                onclick: move |_e: Event<MouseData>| { 
                    slideout_content.set(SlideoutContent::ChatLog);
                    slideout.toggle();
                },
                img { src: CHATS_ICON }
            },
        }
        Slideout {
            open: slideout,
            children: rsx!{
                match slideout_content() {
                    SlideoutContent::ChatLog => rsx! {
                        ChatLog {
                            on_close: move |_| {
                                slideout.set(false);
                            }
                        }
                    },
                    SlideoutContent::Settings => rsx! {
                        Settings {
                            on_close: move |_| {
                                slideout.set(false);
                            }
                        }
                    }
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
