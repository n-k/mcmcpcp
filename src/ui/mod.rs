// Copyright © 2025 Nipun Kumar

//! User interface components for MCMCPCP.
//!
//! This module contains all the UI components that make up the application interface,
//! including the main chat interface, settings page, and various reusable components.

mod box_select; // Multi-select dropdown component
mod chat_input; // Chat message input component
pub mod chat_log;
mod collapsible; // Collapsible/expandable content component
pub mod home; // Main chat interface (public for routing)
pub mod mcp_tools;
mod message; // Message display component
pub mod message_group; // Message group component for grouped assistant/tool messages
pub mod settings; // Settings configuration page (public for routing)
pub mod slideout; // MCP tools display component
