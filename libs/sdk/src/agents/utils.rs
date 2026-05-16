use crate::agents::types::{StreamChunk, Usage};
use crate::core::{FunctionCall, ToolCall};
use std::collections::HashMap;

pub fn parse_sse_buffer(
    buffer: &mut String,
    active_tool_calls: &mut HashMap<usize, ToolCall>,
    new_data: &str,
) -> Vec<StreamChunk> {
    buffer.push_str(new_data);
    let mut chunks = Vec::new();

    while let Some(line_end) = buffer.find('\n') {
        let line = buffer[..line_end].to_string();
        buffer.drain(..=line_end);
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                continue;
            }

            if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(choice) = val["choices"].get(0) {
                    if let Some(delta) = choice.get("delta") {
                        if let Some(content) = delta["content"].as_str() {
                            chunks.push(StreamChunk::Text {
                                content: content.to_string(),
                            });
                        }
                        if let Some(thought) =
                            delta.get("reasoning_content").and_then(|v| v.as_str())
                        {
                            chunks.push(StreamChunk::Thought {
                                content: thought.to_string(),
                            });
                        }
                        if let Some(tool_calls) = delta.get("tool_calls").and_then(|v| v.as_array())
                        {
                            for tc_delta in tool_calls {
                                let index = tc_delta["index"].as_u64().unwrap_or(0) as usize;
                                let entry =
                                    active_tool_calls.entry(index).or_insert_with(|| ToolCall {
                                        index: Some(index),
                                        id: String::new(),
                                        r#type: "function".to_string(),
                                        function: FunctionCall {
                                            name: String::new(),
                                            arguments: String::new(),
                                        },
                                    });

                                if let Some(id) = tc_delta["id"].as_str() {
                                    entry.id.push_str(id);
                                }
                                if let Some(f) = tc_delta.get("function") {
                                    if let Some(name) = f["name"].as_str() {
                                        entry.function.name.push_str(name);
                                    }
                                    if let Some(args) = f["arguments"].as_str() {
                                        entry.function.arguments.push_str(args);
                                    }
                                }

                                chunks.push(StreamChunk::ToolCall {
                                    tool_call: entry.clone(),
                                });
                            }
                        }
                    }
                }
                if let Some(usage) = val.get("usage") {
                    if let Ok(u) = serde_json::from_value::<Usage>(usage.clone()) {
                        chunks.push(StreamChunk::Usage { usage: u });
                    }
                }
            }
        }
    }
    chunks
}
