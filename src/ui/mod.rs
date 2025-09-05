//! User interface components for MCMCPCP.
//! 
//! This module contains all the UI components that make up the application interface,
//! including the main chat interface, settings page, and various reusable components.

mod box_select;    // Multi-select dropdown component
mod chat_input;    // Chat message input component
mod collapsible;   // Collapsible/expandable content component
pub mod home;      // Main chat interface (public for routing)
mod message;       // Message display component
pub mod settings;  // Settings configuration page (public for routing)
pub mod slideout;
pub mod chat_log;
mod story;
