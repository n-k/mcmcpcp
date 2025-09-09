use std::collections::HashMap;

use anyhow::{anyhow, bail};
use html2md::{TagHandler, TagHandlerFactory, parse_html_custom};
use serde_json::{Value, json};

use crate::mcp::{McpTool, ToolResult, ToolResultContent, host::MCPServer};

/// Built-in MCP server that provides web fetching functionality.
///
/// This server is always available and provides a "fetch" tool that can
/// retrieve content from URLs. It's implemented as a built-in server to
/// provide basic web access without requiring external MCP server setup.
pub struct FetchMcpServer {}

#[async_trait::async_trait]
impl MCPServer for FetchMcpServer {
    /// Returns the fetch tool definition.
    ///
    /// Provides a single "fetch" tool that can retrieve content from URLs.
    async fn list_tools(&self) -> Vec<McpTool> {
        vec![
            McpTool {
                name: "fetch_raw_html".into(),
                description: Some("Fetch the contents of a URL as raw HTML.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "The URL to fetch"
                        }
                    },
                    "required": ["url"]
                }),
            },
            McpTool {
                name: "fetch".into(),
                description: Some("Fetch the contents of a URL.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "The URL to fetch"
                        }
                    },
                    "required": ["url"]
                }),
            },
        ]
    }

    /// Handles RPC calls for the fetch server.
    ///
    /// Currently only supports the "tools/call" method with the "fetch" tool.
    /// The fetch tool retrieves content from the specified URL and returns it as text.
    async fn rpc(&mut self, method: &str, params: Value) -> anyhow::Result<serde_json::Value> {
        // Only support tool calls for this built-in server
        if method != "tools/call" {
            bail!("Error: unknown RPC method {method}");
        }

        // Extract the tool name from parameters
        let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");

        // Only support the "fetch" tool
        if name != "fetch" && name != "fetch_raw_html" {
            bail!("Unknown tool: {name}")
        };

        // Extract tool arguments
        let params = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        // Execute the fetch if URL is provided
        if let Some(Value::String(url)) = params.get("url") {
            let text = match _fetch(url.to_string()).await {
                Ok(s) => s,
                Err(e) => format!("Fetch error: {e:?}"),
            };

            let text = if name == "fetch" {
                let mut handlers: HashMap<String, Box<dyn TagHandlerFactory>> = HashMap::new();
                handlers.insert("style".to_string(), Box::new(CustomFactory));
                handlers.insert("script".to_string(), Box::new(CustomFactory));
                handlers.insert("link".to_string(), Box::new(CustomFactory));
                handlers.insert("a".to_string(), Box::new(CustomFactory));
                handlers.insert("img".to_string(), Box::new(CustomFactory));
                handlers.insert("noscript".to_string(), Box::new(CustomFactory));

                parse_html_custom(&text, &handlers)
            } else {
                text
            };

            // Return the result in MCP tool result format
            return Ok(serde_json::to_value(ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".into(),
                    text: Some(text),
                    mime_type: None,
                    data: None,
                    resource: None,
                }],
                is_error: None,
            })?);
        }

        Ok(Value::Null)
    }
}

/// Fetches content from a URL (WASM version).
///
/// Uses a CORS proxy service to bypass browser CORS restrictions when running
/// in WASM. The fetch is performed in a spawned local task and the result is
/// communicated back through a oneshot channel.
///
/// # Arguments
/// * `url` - The URL to fetch content from
///
/// # Returns
/// The fetched content as a string, or an error message if the fetch fails
#[cfg(target_arch = "wasm32")]
async fn _fetch(url: String) -> anyhow::Result<String> {
    use dioxus::logger::tracing::warn;
    use gloo_net::http::Request;
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    use tokio::sync::oneshot;

    // Create a channel to receive the result from the spawned task
    let (tx, rx) = oneshot::channel::<String>();

    // Spawn a local task to perform the fetch (required for WASM)
    wasm_bindgen_futures::spawn_local(async move {
        use dioxus::logger::tracing::warn;

        // Use CORS proxy to bypass browser restrictions
        let encoded = utf8_percent_encode(&url, NON_ALPHANUMERIC).to_string();
        let _url = format!("https://api.allorigins.win/raw?url={encoded}");
        let req = Request::get(&_url).send().await;

        let text = match req {
            Ok(req) => {
                let response = req.text().await;
                match response {
                    Ok(s) => s,
                    Err(e) => format!("Error in builtin/fetch: {e:?}"),
                }
            }
            Err(e) => format!("Error in builtin/fetch: {e:?}"),
        };

        // Send the result back through the channel
        if tx.send(text).is_err() {
            warn!("Receiver dropped before message was sent");
        }
    });

    // Wait for the result from the spawned task
    let s = match rx.await {
        Ok(val) => val,
        Err(_e) => "Error fetching data during tool call!".to_string(),
    };
    Ok(s)
}

/// Fetches content from a URL (native version).
///
/// Uses reqwest to directly fetch content from the URL without CORS restrictions.
/// This is simpler than the WASM version since native applications don't have
/// browser security restrictions.
///
/// # Arguments
/// * `url` - The URL to fetch content from
///
/// # Returns
/// The fetched content as a string, or an error if the fetch fails
#[cfg(not(target_arch = "wasm32"))]
async fn _fetch(url: String) -> anyhow::Result<String> {
    reqwest::Client::new()
        .get(&url)
        .send()
        .await?
        .text()
        .await
        .map_err(|e| anyhow!("{e:?}"))
}

struct CustomFactory;
impl TagHandlerFactory for CustomFactory {
    fn instantiate(&self) -> Box<dyn TagHandler> {
        Box::new(Dummy)
    }
}

struct Dummy;
impl TagHandler for Dummy {
    fn handle(&mut self, _tag: &html2md::Handle, _printer: &mut html2md::StructuredPrinter) {}

    fn after_handle(&mut self, _printer: &mut html2md::StructuredPrinter) {}

    fn skip_descendants(&self) -> bool {
        true
    }
}
