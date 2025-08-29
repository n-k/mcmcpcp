use reqwest::Client;
// use tokio_stream::StreamExt;
use futures::StreamExt as _;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

/// --- REQUEST SIDE ---

#[derive(Debug, Serialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    System {
        content: String,
    },
    User {
        content: Vec<ContentPart>,
    },
    Assistant {
        content: Option<String>,
    },
    Tool {
        tool_call_id: String,
        content: String,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Serialize)]
pub struct ImageUrl {
    pub url: String,
}

/// --- STREAMING RESPONSE SIDE ---

#[derive(Debug, Deserialize)]
pub struct StreamEvent {
    pub id: String,
    pub object: String,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Debug, Deserialize)]
pub struct ToolCallDelta {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>, // usually "function"
    pub function: Option<FunctionDelta>,
}

#[derive(Debug, Deserialize)]
pub struct FunctionDelta {
    pub name: Option<String>,
    pub arguments: Option<String>, // streamed in pieces
}

/// --- DEMO APP ---

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = "dummy";
    let client = Client::new();

    // Build request messages
    let messages = vec![
        Message::System {
            content: "You are a helpful assistant.".into(),
        },
        Message::User {
            content: vec![
                ContentPart::Text {
                    text: "call the weather tool for New York.".into(),
                },
                // ContentPart::ImageUrl {
                //     image_url: ImageUrl {
                //         url: "data:image/gif;base64,R0lGODlhEAAQAMQAAORHHOVSKudfOulrSOp3WOyDZu6QdvCchPGolfO0o/XBs/fNwfjZ0frl3/zy7////wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACH5BAkAABAALAAAAAAQABAAAAVVICSOZGlCQAosJ6mu7fiyZeKqNKToQGDsM8hBADgUXoGAiqhSvp5QAnQKGIgUhwFUYLCVDFCrKUE1lBavAViFIDlTImbKC5Gm2hB0SlBCBMQiB0UjIQA7".into(),
                //     },
                // },
            ],
        },
    ];

    // Initial request with streaming enabled
    let res = client
        .post("http://192.168.29.3:11434/v1/chat/completions")
        .bearer_auth(&api_key)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "q3c",
            "stream": true,
            "messages": messages,
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "get_weather",
                        "description": "Get the current weather for a location",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "location": {"type": "string"}
                            },
                            "required": ["location"]
                        }
                    }
                }
            ]
        }))
        .send()
        .await?;

    if !res.status().is_success() {
        let status = res.status().clone();
        let body = res.text().await?;
        eprintln!("Request failed: {} - {}", status, body);
        return Ok(());
    }

    // Stream handling
    let mut stream = res.bytes_stream();
    let mut current_tool_id: Option<String> = None;
    let mut current_tool_name: Option<String> = None;
    let mut current_tool_args = String::new();

    while let Some(item) = stream.next().await {
        let chunk = item?;
        let text = String::from_utf8_lossy(&chunk);

        for line in text.lines() {
            if !line.starts_with("data: ") {
                continue;
            }
            let data = &line[6..];
            if data == "[DONE]" {
                println!("\n-- Stream complete --");
                break;
            }

            if let Ok(event) = serde_json::from_str::<StreamEvent>(data) {
                for choice in event.choices {
                    let delta = choice.delta;

                    if let Some(content) = delta.content {
                        print!("{}", content);
                        io::stdout().flush()?;
                    }

                    if let Some(tool_calls) = delta.tool_calls {
                        for tc in tool_calls {
                            if let Some(id) = tc.id {
                                current_tool_id = Some(id);
                            }
                            if let Some(func) = tc.function {
                                if let Some(name) = func.name {
                                    current_tool_name = Some(name);
                                }
                                if let Some(arg_piece) = func.arguments {
                                    current_tool_args.push_str(&arg_piece);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // If a tool call was made, simulate executing it
    if let (Some(id), Some(name)) = (current_tool_id, current_tool_name) {
        println!(
            "\n\n🔧 Tool requested: {} with args {}",
            name, current_tool_args
        );

        // Simulate tool execution
        let tool_result = format!("The weather in New York is Sunny, 25°C.");

        // Build tool message
        let tool_message = Message::Tool {
            tool_call_id: id.clone(),
            content: tool_result,
        };

        // Send follow-up request with tool result
        let followup_res = client
            .post("http://192.168.29.3:11434/v1/chat/completions")
            .bearer_auth(&api_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": "q3c",
                "stream": true,
                "messages": [
                    // normally you'd include the entire prior conversation here
                    {"role": "system", "content": "You are a helpful assistant."},
                    {"role": "user", "content": "call the weather tool for New York."},
                    {"role": "assistant", "tool_calls": [{"id": id, "type": "function", "function": {"name": name, "arguments": current_tool_args}}]},
                    tool_message
                ]
            }))
            .send()
            .await?;

        println!("\n\n-- Assistant continues after tool result --");

        let mut follow_stream = followup_res.bytes_stream();
        while let Some(item) = follow_stream.next().await {
            let chunk = item?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        println!("\n-- Follow-up complete --");
                        return Ok(());
                    }
                    if let Ok(event) = serde_json::from_str::<StreamEvent>(data) {
                        for choice in event.choices {
                            if let Some(content) = choice.delta.content {
                                print!("{}", content);
                                io::stdout().flush()?;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
