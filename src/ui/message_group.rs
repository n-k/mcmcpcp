//! Message group component for displaying assistant and tool messages as unified entities.
//!
//! This module provides functionality to group assistant messages with their corresponding
//! tool messages and display them as a single deletable entity in the chat interface.

use dioxus::prelude::*;

use crate::{
    llm::{Message, FunctionDelta},
    ui::collapsible::Collapsible,
};

/// Represents a group of related messages that should be displayed as one entity
#[derive(Clone, Debug, PartialEq)]
pub struct MessageGroup {
    /// The assistant message that initiated this group
    pub assistant_message: Message,
    /// Tool messages that are part of this group
    pub tool_messages: Vec<Message>,
    /// Unique identifier for this group (for deletion purposes)
    pub group_id: String,
}

impl MessageGroup {
    /// Creates a new message group from an assistant message
    pub fn new(assistant_message: Message) -> Self {
        // Generate a stable ID based on message content hash
        let group_id = match &assistant_message {
            Message::Assistant { content, .. } => {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                
                let mut hasher = DefaultHasher::new();
                content.hash(&mut hasher);
                // Add a simple representation of tool calls to make the hash more unique
                if let Message::Assistant { tool_calls, .. } = &assistant_message {
                    if let Some(calls) = tool_calls {
                        calls.len().hash(&mut hasher);
                        for call in calls {
                            if let Some(function) = &call.function {
                                if let Some(name) = &function.name {
                                    name.hash(&mut hasher);
                                }
                            }
                        }
                    }
                }
                format!("group_{}", hasher.finish())
            }
            _ => format!("group_{}", uuid::Uuid::new_v4().to_string()),
        };
        
        Self {
            assistant_message,
            tool_messages: Vec::new(),
            group_id,
        }
    }
    
    /// Adds a tool message to this group
    pub fn add_tool_message(&mut self, tool_message: Message) {
        self.tool_messages.push(tool_message);
    }
    
    // Checks if this group contains any tool messages
    // pub fn has_tool_messages(&self) -> bool {
    //     !self.tool_messages.is_empty()
    // }
}

#[derive(Props, Clone, PartialEq)]
pub struct MessageGroupProps {
    /// The message group to display
    pub group: MessageGroup,
    /// Callback for when the group should be deleted
    pub on_delete: Option<EventHandler<String>>,
    /// Whether to show delete button
    pub show_delete: bool,
}

/// Component for rendering a message group as a unified entity
#[component]
pub fn MessageGroupEl(props: MessageGroupProps) -> Element {
    let group = props.group.clone();
    
    // Render the assistant message content
    let assistant_content = match &group.assistant_message {
        Message::Assistant { content, tool_calls } => {
            let empty_string = String::new();
            let content = content.as_ref().unwrap_or(&empty_string);
            let el = crate::md2rsx::markdown_to_rsx(content)?;
            let fns: Vec<FunctionDelta> = tool_calls
                .as_ref()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|tc| &tc.function)
                .filter(|f| f.is_some())
                .map(|f| f.as_ref().unwrap().clone())
                .collect();
            
            rsx! {
                div { class: "assistant-content",
                    {el}
                    if !fns.is_empty() {
                        div { class: "tool-calls",
                            style: "margin-top: 1em; padding-top: 1em; border-top: 1px solid rgba(255, 255, 255, 0.2);",
                            for f in fns {
                                div { class: "tool-call",
                                    style: "margin-bottom: 0.75em;",
                                    if let Some(name) = &f.name {
                                        div { class: "tool-name", 
                                            style: "font-weight: 600; margin-bottom: 0.25em; opacity: 0.9;",
                                            "🔧 {name}" 
                                        }
                                    }
                                    if let Some(args) = &f.arguments {
                                        div { class: "tool-args", 
                                            style: "
                                                font-family: 'Fira Code', 'JetBrains Mono', 'Courier New', monospace;
                                                font-size: 0.85em;
                                                background: rgba(255, 255, 255, 0.1);
                                                color: rgba(255, 255, 255, 0.8);
                                                padding: 0.5em;
                                                border-radius: 6px;
                                                margin-top: 0.25em;
                                                border: 1px solid rgba(255, 255, 255, 0.1);
                                                overflow-x: auto;
                                            ",
                                            "{args}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => rsx! { div { "Invalid assistant message" } }
    };
    
    // Render tool messages if any
    let tool_content = if !group.tool_messages.is_empty() {
        rsx! {
            div { class: "tool-results",
                style: "margin-top: 1em; padding-top: 1em; border-top: 1px solid rgba(255, 255, 255, 0.2);",
                for tool_msg in &group.tool_messages {
                    match tool_msg {
                        Message::Tool { content, .. } => {
                            let el = crate::md2rsx::markdown_to_rsx(content)?;
                            rsx! {
                                div { class: "tool-result",
                                    style: "
                                        background: rgba(255, 255, 255, 0.1);
                                        border: 1px solid rgba(255, 255, 255, 0.1);
                                        border-radius: 8px;
                                        margin-bottom: 0.75em;
                                        padding: 0.75em;
                                    ",
                                    div { 
                                        style: "
                                            font-size: 0.9em;
                                            font-weight: 600;
                                            margin-bottom: 0.5em;
                                            opacity: 0.9;
                                            display: flex;
                                            align-items: center;
                                            gap: 0.5em;
                                        ",
                                        "🔧 Tool Result"
                                    }
                                    Collapsible { c: true, {el} }
                                }
                            }
                        }
                        _ => rsx! { div { "Invalid tool message" } }
                    }
                }
            }
        }
    } else {
        rsx! { div {} }
    };
    
    rsx! {
        div { 
            class: "message ai-message",
            
            // Delete button (top-right corner)
            if props.show_delete {
                if let Some(on_delete) = props.on_delete {
                    button {
                        class: "delete-group-btn",
                        style: "
                            position: absolute;
                            top: 8px;
                            right: 50px;
                            background: rgba(255, 255, 255, 0.2);
                            color: white;
                            border: none;
                            border-radius: 50%;
                            width: 20px;
                            height: 20px;
                            cursor: pointer;
                            font-size: 14px;
                            display: flex;
                            align-items: center;
                            justify-content: center;
                            opacity: 0.7;
                            transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
                            z-index: 10;
                        ",
                        onclick: move |e: Event<MouseData>| {
                            e.stop_propagation();
                            on_delete.call(group.group_id.clone());
                        },
                        title: "Delete this conversation turn",
                        "×"
                    }
                }
            }
            
            Collapsible {
                c: false,
                div { class: "message-group-content",
                    
                    // Assistant message content
                    {assistant_content}
                    
                    // Tool results (if any)
                    {tool_content}
                }
            }
        }
    }
}

/// Groups a list of messages into message groups
/// 
/// This function takes a flat list of messages and groups assistant messages
/// with their corresponding tool messages.
pub fn group_messages(messages: &[Message]) -> Vec<MessageGroup> {
    let mut groups = Vec::new();
    let mut current_group: Option<MessageGroup> = None;
    
    for message in messages {
        match message {
            Message::Assistant { .. } => {
                // If we have a current group, save it
                if let Some(group) = current_group.take() {
                    groups.push(group);
                }
                // Start a new group
                current_group = Some(MessageGroup::new(message.clone()));
            }
            Message::Tool { .. } => {
                // Add to current group if it exists
                if let Some(ref mut group) = current_group {
                    group.add_tool_message(message.clone());
                }
                // If no current group, this is an orphaned tool message
                // We could handle this case differently if needed
            }
            Message::System { .. } | Message::User { .. } => {
                // These messages don't belong to groups
                // If we have a current group, save it first
                if let Some(group) = current_group.take() {
                    groups.push(group);
                }
                // These will be handled separately in the UI
            }
        }
    }
    
    // Don't forget the last group
    if let Some(group) = current_group {
        groups.push(group);
    }
    
    groups
}
