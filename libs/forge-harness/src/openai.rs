//! OpenAI-compat backend (lisa-inferenced or any compatible endpoint),
//! speaking function/tool calling. The model's file operations are
//! constrained by the tools' JSON schemas — grammar-valid by
//! construction, never free-form text the harness has to parse.

use crate::agent::{AgentAction, Message, Role};
use crate::tools::{ToolCall, ToolSpec};
use crate::{Backend, ForgeError};
use serde_json::{Value, json};

/// OpenAI-compat backend (lisa-inferenced or any compatible endpoint).
pub struct OpenAiBackend {
    pub url: String,
    pub model: Option<String>,
}

impl Backend for OpenAiBackend {
    fn next_action(&mut self, messages: &[Message], tools: &[ToolSpec]) -> Result<AgentAction, ForgeError> {
        let body = request_body(self.model.as_deref(), messages, tools);
        let endpoint = format!("{}/v1/chat/completions", self.url.trim_end_matches('/'));
        let mut response = ureq::post(&endpoint)
            .send_json(&body)
            .map_err(|e| ForgeError::Backend(e.to_string()))?;
        let json: Value = response
            .body_mut()
            .read_json()
            .map_err(|e| ForgeError::Backend(e.to_string()))?;
        parse_response(&json)
    }
}

/// The chat-completions request: history plus tool declarations, one
/// tool call at a time.
fn request_body(model: Option<&str>, messages: &[Message], tools: &[ToolSpec]) -> Value {
    json!({
        "model": model,
        "messages": wire_messages(messages),
        "tools": tools.iter().map(ToolSpec::wire).collect::<Vec<_>>(),
        "tool_choice": "auto",
        "parallel_tool_calls": false,
    })
}

/// Map the internal history onto the OpenAI message shapes.
fn wire_messages(messages: &[Message]) -> Value {
    messages
        .iter()
        .map(|m| match m.role {
            Role::System => json!({"role": "system", "content": m.content}),
            Role::User => json!({"role": "user", "content": m.content}),
            Role::Assistant => match &m.tool_call {
                Some(call) => json!({
                    "role": "assistant",
                    "content": if m.content.is_empty() { Value::Null } else { json!(m.content) },
                    "tool_calls": [{
                        "id": call.id,
                        "type": "function",
                        "function": {
                            "name": call.name,
                            "arguments": call.args.to_string(),
                        }
                    }],
                }),
                None => json!({"role": "assistant", "content": m.content}),
            },
            Role::Tool => json!({
                "role": "tool",
                "tool_call_id": m.tool_call_id,
                "content": m.content,
            }),
        })
        .collect()
}

/// A response is either a tool call (the first one — parallel calls are
/// disabled) or, with no tool calls, the model's done signal.
fn parse_response(json: &Value) -> Result<AgentAction, ForgeError> {
    let message = &json["choices"][0]["message"];
    if message.is_null() {
        return Err(ForgeError::Backend(format!("no message in {json}")));
    }
    if let Some(calls) = message["tool_calls"].as_array()
        && let Some(first) = calls.first()
    {
        let name = first["function"]["name"]
            .as_str()
            .ok_or_else(|| ForgeError::Backend(format!("tool call without a name in {json}")))?;
        // `arguments` is a JSON *string* on the wire; some endpoints
        // hand back an object instead — accept both.
        let args = match &first["function"]["arguments"] {
            Value::String(s) => serde_json::from_str(s)
                .map_err(|e| ForgeError::Backend(format!("bad tool arguments: {e}")))?,
            other if other.is_object() => other.clone(),
            _ => json!({}),
        };
        return Ok(AgentAction::Call(ToolCall {
            id: first["id"].as_str().unwrap_or("call_0").to_string(),
            name: name.to_string(),
            args,
        }));
    }
    let content = message["content"]
        .as_str()
        .ok_or_else(|| ForgeError::Backend(format!("no content in {json}")))?;
    Ok(AgentAction::Done(content.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn specs() -> Vec<ToolSpec> {
        crate::tools::tool_specs()
    }

    #[test]
    fn request_carries_tools_and_wire_history() {
        let call = ToolCall {
            id: "c1".into(),
            name: "write_file".into(),
            args: json!({"path": "a.txt", "content": "hi"}),
        };
        let history = vec![
            Message::system("sys"),
            Message::user("task"),
            Message::assistant_call(call),
            Message::tool_result("c1", "wrote a.txt"),
            Message::user("findings"),
        ];
        let body = request_body(Some("coder"), &history, &specs());
        assert_eq!(body["model"], "coder");
        assert_eq!(body["parallel_tool_calls"], false);
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 7);
        assert_eq!(tools[0]["type"], "function");
        assert!(tools.iter().any(|t| t["function"]["name"] == "edit_file"));

        let msgs = body["messages"].as_array().unwrap();
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[2]["tool_calls"][0]["function"]["name"], "write_file");
        // Arguments cross the wire as a JSON string.
        let arguments: Value = serde_json::from_str(
            msgs[2]["tool_calls"][0]["function"]["arguments"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(arguments, json!({"path": "a.txt", "content": "hi"}));
        assert_eq!(msgs[3]["role"], "tool");
        assert_eq!(msgs[3]["tool_call_id"], "c1");
    }

    #[test]
    fn parses_a_tool_call_response() {
        let response = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_9",
                        "type": "function",
                        "function": {
                            "name": "read_file",
                            "arguments": "{\"path\": \"lib/main.dart\"}"
                        }
                    }]
                }
            }]
        });
        let action = parse_response(&response).unwrap();
        assert_eq!(
            action,
            AgentAction::Call(ToolCall {
                id: "call_9".into(),
                name: "read_file".into(),
                args: json!({"path": "lib/main.dart"}),
            })
        );
    }

    #[test]
    fn parses_object_shaped_arguments_too() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "x",
                        "function": {"name": "run_tests", "arguments": {}}
                    }]
                }
            }]
        });
        match parse_response(&response).unwrap() {
            AgentAction::Call(call) => assert_eq!(call.name, "run_tests"),
            other => panic!("expected a call, got {other:?}"),
        }
    }

    #[test]
    fn a_plain_reply_is_the_done_signal() {
        let response = json!({
            "choices": [{"message": {"role": "assistant", "content": "all done, analyzer clean"}}]
        });
        assert_eq!(
            parse_response(&response).unwrap(),
            AgentAction::Done("all done, analyzer clean".into())
        );
    }
}
