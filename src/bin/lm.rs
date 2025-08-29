use mcmcpcp::llm::*;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let lc = LlmClient::new("http://192.168.29.3:11434/v1".into(), "dummy".into());

    // Build request messages
    let mut messages = vec![
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
    let tools = vec![Tool {
        r#type: "function".into(),
        function: Function {
            name: "get_weather".into(),
            description: Some("Get the current weather for a location".into()),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                },
                "required": ["location"]
            })),
            strict: Some(true),
        },
    }];
    let mut rx = lc.stream("q3c", &messages, &tools).await?;
    let mut tool_id: Option<String> = None;
    while let Some(e) = rx.recv().await {
        println!("{e:?}");
        if let Some(ch) = e.choices.first() {
            if let Some(tcs) = &ch.delta.tool_calls {
                for tc in tcs {
                    if let Some(id) = &tc.id {
                        tool_id = Some(id.clone());
                    }
                }
            }
        }
    }

    if let Some(tid) = tool_id {
        let tool_message = Message::Tool {
            tool_call_id: tid.clone(),
            content: "The weather in New York is Sunny, 25°C.".into(),
        };
        messages.push(tool_message);
        let mut rx = lc.stream("q3c", &messages, &tools).await?;
        while let Some(e) = rx.recv().await {
            println!("{e:?}");
        }
    }

    Ok(())
}
