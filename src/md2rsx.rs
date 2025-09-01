//! Markdown to RSX conversion utilities for MCMCPCP.
//! 
//! This module provides functionality to convert Markdown text into Dioxus RSX elements,
//! allowing LLM responses formatted in Markdown to be rendered as proper HTML in the UI.
//! It uses the pulldown-cmark parser to process Markdown and converts it to a tree of
//! Dioxus elements.
//! 
//! The converter supports most common Markdown elements including headings, paragraphs,
//! lists, code blocks, emphasis, tables, and more.

use dioxus::prelude::*;
use pulldown_cmark::{Event, HeadingLevel, Parser, TagEnd};

/// Converts a Markdown string to a Dioxus RSX Element.
/// 
/// This function parses Markdown text using pulldown-cmark and converts it into
/// a tree of Dioxus elements that can be rendered in the UI. It maintains a stack
/// of element vectors to handle nested structures properly.
/// 
/// # Arguments
/// * `md` - The Markdown string to convert
/// 
/// # Returns
/// A Dioxus `Element` containing the rendered Markdown content
/// 
/// # Supported Markdown Features
/// - Headings (H1-H6)
/// - Paragraphs
/// - Emphasis (italic) and strong (bold) text
/// - Inline code and code blocks
/// - Lists (unordered)
/// - Tables
/// - Blockquotes
/// - Horizontal rules
/// - Line breaks
pub fn markdown_to_rsx<'a>(md: &'a str) -> Element {
    // Create a Markdown parser for the input text
    let parser = Parser::new(md);

    // Stack to handle nested elements - each level contains a vector of child elements
    let mut stack: Vec<Vec<Element>> = vec![vec![]];

    // Process each Markdown event from the parser
    for ev in parser {
        match ev {
            // Start of a container element - push a new level onto the stack
            Event::Start(tag) => {
                stack.push(vec![]);
                match tag {
                    _ => {} // Container handling is done in Event::End
                }
            }
            // End of a container element - pop the stack and create the appropriate RSX element
            Event::End(tag) => {
                let children = stack.pop().unwrap().into_iter();
                let node = match tag {
                    // Block-level elements
                    TagEnd::Paragraph => rsx! {
                        p { {children} }
                    },
                    TagEnd::Heading(level) => match level {
                        HeadingLevel::H1 => rsx! { h1 { {children} } },
                        HeadingLevel::H2 => rsx! { h2 { {children} } },
                        HeadingLevel::H3 => rsx! { h3 { {children} } },
                        HeadingLevel::H4 => rsx! { h4 { {children} } },
                        HeadingLevel::H5 => rsx! { h5 { {children} } },
                        _ => rsx! { h6 { {children} } }, // H6 and any other levels
                    },
                    TagEnd::BlockQuote(_) => rsx! {
                        blockquote { {children} }
                    },
                    TagEnd::CodeBlock => rsx! {
                        pre {
                            code { {children} }
                        }
                    },
                    TagEnd::HtmlBlock => rsx! {
                        blockquote { {children} } // Treat HTML blocks as blockquotes for safety
                    },
                    
                    // List elements
                    TagEnd::List(_) => rsx! {
                        ul { {children} }
                    },
                    TagEnd::Item => rsx! {
                        li { {children} }
                    },
                    
                    // Table elements
                    TagEnd::Table => rsx! {
                        table { {children} }
                    },
                    TagEnd::TableHead => rsx! {
                        thead { {children} }
                    },
                    TagEnd::TableRow => rsx! {
                        tr { {children} }
                    },
                    TagEnd::TableCell => rsx! {
                        td { {children} }
                    },
                    
                    // Inline formatting elements
                    TagEnd::Emphasis => rsx! {
                        em { {children} }
                    },
                    TagEnd::Strong => rsx! {
                        strong { {children} }
                    },
                    
                    // Fallback for unsupported elements
                    // TODO: Add support for links, images, strikethrough, footnotes
                    _ => rsx! {
                        div { {children} }
                    },
                };
                // Add the created node to the parent level
                stack.last_mut().unwrap().push(node);
            }
            // Leaf elements that don't contain other elements
            Event::Text(text) => {
                // Plain text content
                stack.last_mut().unwrap().push(rsx! { "{text}" });
            }
            Event::Code(code) => {
                // Inline code
                stack.last_mut().unwrap().push(rsx! {
                    code { "{code}" }
                });
            }
            Event::Rule => {
                // Horizontal rule
                stack.last_mut().unwrap().push(rsx! {
                    hr {}
                });
            }
            Event::SoftBreak | Event::HardBreak => {
                // Line breaks
                stack.last_mut().unwrap().push(rsx! {
                    br {}
                });
            }
            _ => {
                // Ignore other events (like HTML, links, images for now)
                // These could be implemented in future versions
            }
        }
    }
    
    // Flatten all remaining stack levels and wrap in a div
    let children = stack.into_iter().flatten();
    rsx! {
        div { {children} }
    }
}
