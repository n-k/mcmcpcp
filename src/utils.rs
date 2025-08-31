use std::sync::Arc;

use serde_json::Value;

use crate::llm::Function;
use crate::llm::Message;
use crate::llm::Tool;
use crate::llm::ToolCallDelta;
use crate::mcp::host::Host;
use crate::mcp::ToolDescriptor;

pub fn tools_to_message_objects(tools: Vec<ToolDescriptor>) -> Vec<Tool> {
    tools
        .iter()
        .map(move |t| {
            let t = t.clone();
            // eprintln!("==={t:?}===");
            Tool {
                r#type: "function".into(),
                function: Function {
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
    tool_calls: Vec<ToolCallDelta>,
    host: Arc<Host>,
) -> anyhow::Result<Vec<Message>> {
    let mut new_chat: Vec<Message> = vec![];
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
            let text = messages.join("\n");
            let tcm = Message::Tool { tool_call_id: tc.id.unwrap_or_else(|| "".into()), content: text };
            new_chat.push(tcm);
        }
    }
    Ok(new_chat)
}
