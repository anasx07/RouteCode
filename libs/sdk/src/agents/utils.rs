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
                        if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                            chunks.push(StreamChunk::Text {
                                content: content.to_string(),
                            });
                        }
                        if let Some(thought) = delta
                            .get("reasoning_content")
                            .and_then(|v| v.as_str())
                            .or_else(|| delta.get("thought").and_then(|v| v.as_str()))
                        {
                            chunks.push(StreamChunk::Thought {
                                content: thought.to_string(),
                            });
                        }
                        if let Some(tool_calls) = delta.get("tool_calls").and_then(|v| v.as_array())
                        {
                            for tc_delta in tool_calls {
                                if let Some(idx_val) = tc_delta.get("index") {
                                    let index = idx_val.as_u64().unwrap_or(0) as usize;
                                    let entry =
                                        active_tool_calls.entry(index).or_insert_with(|| {
                                            ToolCall {
                                                index: Some(index),
                                                id: String::new(),
                                                r#type: "function".to_string(),
                                                function: FunctionCall {
                                                    name: String::new(),
                                                    arguments: String::new(),
                                                },
                                            }
                                        });

                                    if let Some(id) = tc_delta.get("id").and_then(|v| v.as_str()) {
                                        entry.id = id.to_string();
                                    }
                                    if let Some(f) = tc_delta.get("function") {
                                        if let Some(name) = f.get("name").and_then(|v| v.as_str()) {
                                            entry.function.name = name.to_string();
                                        }
                                        if let Some(args) =
                                            f.get("arguments").and_then(|v| v.as_str())
                                        {
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

pub fn parse_anthropic_sse(
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
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                let event_type = val["type"].as_str().unwrap_or("");
                match event_type {
                    "content_block_delta" => {
                        if let Some(delta) = val.get("delta") {
                            if let Some(text) = delta["text"].as_str() {
                                chunks.push(StreamChunk::Text {
                                    content: text.to_string(),
                                });
                            }
                            if let Some(thought) = delta["thought"].as_str() {
                                chunks.push(StreamChunk::Thought {
                                    content: thought.to_string(),
                                });
                            }
                            if let Some(partial_json) = delta["partial_json"].as_str() {
                                let index = val["index"].as_u64().unwrap_or(0) as usize;
                                if let Some(tool_call) = active_tool_calls.get_mut(&index) {
                                    tool_call.function.arguments.push_str(partial_json);
                                    chunks.push(StreamChunk::ToolCall {
                                        tool_call: tool_call.clone(),
                                    });
                                }
                            }
                        }
                    }
                    "content_block_start" => {
                        let index = val["index"].as_u64().unwrap_or(0) as usize;
                        if let Some(block) = val.get("content_block") {
                            if block["type"] == "tool_use" {
                                let id = block["id"].as_str().unwrap_or("").to_string();
                                let name = block["name"].as_str().unwrap_or("").to_string();
                                let tool_call = ToolCall {
                                    id,
                                    r#type: "function".to_string(),
                                    index: Some(index),
                                    function: FunctionCall {
                                        name,
                                        arguments: String::new(),
                                    },
                                };
                                active_tool_calls.insert(index, tool_call.clone());
                                chunks.push(StreamChunk::ToolCall { tool_call });
                            }
                        }
                    }
                    "content_block_stop" => {
                        let index = val["index"].as_u64().unwrap_or(0) as usize;
                        if let Some(tool_call) = active_tool_calls.remove(&index) {
                            chunks.push(StreamChunk::ToolCall { tool_call });
                        }
                    }
                    "message_delta" => {
                        if let Some(usage) = val.get("usage") {
                            let prompt = usage["input_tokens"].as_u64().unwrap_or(0) as u32;
                            let completion = usage["output_tokens"].as_u64().unwrap_or(0) as u32;
                            chunks.push(StreamChunk::Usage {
                                usage: Usage {
                                    prompt_tokens: prompt,
                                    completion_tokens: completion,
                                    total_tokens: prompt + completion,
                                },
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    chunks
}
