use std::sync::Arc;

use async_openai::types::{
    ChatCompletionMessageToolCallChunk, ChatCompletionRequestMessage,
    ChatCompletionRequestMessageContentPartText, ChatCompletionRequestToolMessageArgs,
    ChatCompletionRequestToolMessageContent, ChatCompletionRequestToolMessageContentPart,
    ChatCompletionTool,
};
use serde_json::Value;

use crate::mcp::Host;
use crate::mcp::ToolDescriptor;

pub fn tools_to_openai_objects(tools: Vec<ToolDescriptor>) -> Vec<ChatCompletionTool> {
    tools
        .iter()
        .map(move |t| {
            let t = t.clone();
            // eprintln!("==={t:?}===");
            ChatCompletionTool {
                r#type: async_openai::types::ChatCompletionToolType::Function,
                function: async_openai::types::FunctionObject {
                    name: format!("{}/{}", t.server_id, t.tool.name),
                    description: t.tool.description,
                    parameters: Some(t.tool.input_schema),
                    strict: Some(true),
                },
            }
        })
        .collect()
}

pub async fn call_tools(
    tool_calls: Vec<ChatCompletionMessageToolCallChunk>,
    host: Arc<Host>,
) -> anyhow::Result<Vec<ChatCompletionRequestMessage>> {
    let mut new_chat: Vec<ChatCompletionRequestMessage> = vec![];
    for tc in tool_calls.into_iter() {
        let Some(f) = tc.function.as_ref() else {
            continue;
        };
        let parts: Vec<_> = f
            .name
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or_else(|| "")
            .split("/")
            .collect();
        if parts.len() == 2 {
            let server_id = parts[0];
            let tool_name = parts[1];
            let params_str = f
                .arguments
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or_else(|| "{}");
            let arguments: Value = serde_json::from_str(params_str)?;

            eprintln!("Calling {server_id}/{tool_name}({arguments:?})");
            let result = host.tool_call(server_id, tool_name, arguments).await?;
            // convert to request objects
            let messages: Vec<String> = result
                .content
                .into_iter()
                .filter(|c| c.r#type == "text")
                .map(|c| c.text.unwrap_or_else(|| "".to_string()))
                .collect();
            if messages.len() == 0 {
                let tcm = ChatCompletionRequestToolMessageArgs::default()
                    .content(ChatCompletionRequestToolMessageContent::Text(
                        messages[0].clone(),
                    ))
                    .build()?
                    .into();
                new_chat.push(tcm);
            } else {
                let parts = messages
                    .into_iter()
                    .map(|m| {
                        ChatCompletionRequestToolMessageContentPart::Text(
                            ChatCompletionRequestMessageContentPartText { text: m },
                        )
                    })
                    .collect();
                let tcm = ChatCompletionRequestToolMessageArgs::default()
                    .content(ChatCompletionRequestToolMessageContent::Array(parts))
                    .build()?
                    .into();
                new_chat.push(tcm);
            }
        }
    }
    Ok(new_chat)
}
