use dioxus::prelude::*;
use pulldown_cmark::{Event, Parser, TagEnd, HeadingLevel};

pub fn markdown_to_rsx<'a>(md: &'a str) -> Element {
    let parser = Parser::new(md);

    let mut stack: Vec<Vec<Element>> = vec![vec![]];

    for ev in parser {
        match ev {
            Event::Start(tag) => {
                stack.push(vec![]);
                match tag {
                    _ => {}
                }
            }
            Event::End(tag) => {
                let children = stack.pop().unwrap().into_iter();
                let node = match tag {
                    TagEnd::Paragraph => rsx! { p { {children} } },
                    TagEnd::Heading(level) => match level {
                        HeadingLevel::H1 => rsx! { h1 { {children} } },
                        HeadingLevel::H2 => rsx! { h2 { {children} } },
                        HeadingLevel::H3 => rsx! { h3 { {children} } },
                        HeadingLevel::H4 => rsx! { h4 { {children} } },
                        HeadingLevel::H5 => rsx! { h5 { {children} } },
                        _ => rsx! { h6 { {children} } },
                    },
                    TagEnd::BlockQuote(_) => rsx! { blockquote { {children} } },
                    TagEnd::CodeBlock => {
                        rsx! { pre { code { {children} } } }
                    },
                    TagEnd::HtmlBlock => rsx! { blockquote { {children} } },
                    TagEnd::List(_) => rsx! { ul { {children} } },
                    TagEnd::Item => rsx! { li { {children} } },
                    TagEnd::Table => rsx! { table { {children} } },
                    TagEnd::TableHead => rsx! { thead { {children} } },
                    TagEnd::TableRow => rsx! { tr { {children} } },
                    TagEnd::TableCell => rsx! { td { {children} } },
                    TagEnd::Emphasis => rsx! { em { {children} } },
                    TagEnd::Strong => rsx! { strong { {children} } },
                    // TagEnd::FootnoteDefinition => todo!(),
                    // TagEnd::Strikethrough => todo!(),
                    // TagEnd::Link => todo!(),
                    // TagEnd::Image => todo!(),
                    // TagEnd::MetadataBlock(metadata_block_kind) => todo!(),

                    _ => rsx! { div { {children} } },
                };
                stack.last_mut().unwrap().push(node);
            }
            Event::Text(text) => {
                stack.last_mut().unwrap().push(rsx! { "{text}" });
            }
            Event::Code(code) => {
                stack.last_mut().unwrap().push(rsx! { code { "{code}" } });
            }
            Event::Rule => {
                stack.last_mut().unwrap().push(rsx! { hr {} });
            }
            Event::SoftBreak | Event::HardBreak => {
                stack.last_mut().unwrap().push(rsx! { br {} });
            }
            _ => {}
        }
    }
    let children = stack.into_iter().flatten();
    rsx! { div { {children} } }
}
