use crate::tools::{Tool, ToolSpec};
use serde_json::Value;
use std::fmt::Write;
use zeroclaw_providers::{ChatMessage, ChatResponse, ConversationMessage, ToolResultMessage};

#[derive(Debug, Clone)]
pub struct ParsedToolCall {
    pub name: String,
    pub arguments: Value,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    pub name: String,
    pub output: String,
    pub success: bool,
    pub tool_call_id: Option<String>,
}

pub trait ToolDispatcher: Send + Sync {
    fn parse_response(&self, response: &ChatResponse) -> (String, Vec<ParsedToolCall>);
    fn format_results(&self, results: &[ToolExecutionResult]) -> ConversationMessage;
    fn prompt_instructions(&self, tools: &[Box<dyn Tool>]) -> String;
    fn to_provider_messages(&self, history: &[ConversationMessage]) -> Vec<ChatMessage>;
    fn should_send_tool_specs(&self) -> bool;
}

#[derive(Default)]
pub struct XmlToolDispatcher;

impl XmlToolDispatcher {
    pub(crate) fn parse_xml_tool_calls(response: &str) -> (String, Vec<ParsedToolCall>) {
        // Strip `<think>...</think>` blocks before parsing tool calls.
        // Qwen and other reasoning models may embed chain-of-thought inline.
        let cleaned = Self::strip_think_tags(response);
        let mut text_parts = Vec::new();
        let mut calls = Vec::new();
        let mut remaining = cleaned.as_str();

        loop {
            // Support both <tool_call> and <tool_calls> (some models use plural)
            let (open_tag, open_len, close_tag, close_len) = match (
                remaining.find("<tool_calls>"),
                remaining.find("<tool_call>"),
            ) {
                (Some(p), Some(s)) if p <= s => ("<tool_calls>", 12, "</tool_calls>", 13),
                (_, Some(s)) => ("<tool_call>", 11, "</tool_call>", 12),
                (Some(_p), _) => ("<tool_calls>", 12, "</tool_calls>", 13),
                (None, None) => break,
            };
            let start = remaining.find(open_tag).unwrap();
            let before = &remaining[..start];
            if !before.trim().is_empty() {
                text_parts.push(before.trim().to_string());
            }

            if let Some(end) = remaining[start..].find(close_tag) {
                let inner = &remaining[start + open_len..start + end];
                match serde_json::from_str::<Value>(inner.trim()) {
                    Ok(parsed) => {
                        let name = parsed
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        if name.is_empty() {
                            remaining = &remaining[start + end + close_len..];
                            continue;
                        }
                        let arguments = parsed
                            .get("arguments")
                            .cloned()
                            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
                        calls.push(ParsedToolCall {
                            name,
                            arguments,
                            tool_call_id: None,
                        });
                    }
                    Err(e) => {
                        tracing::warn!("Malformed {open_tag} JSON: {e}");
                    }
                }
                remaining = &remaining[start + end + close_len..];
            } else {
                break;
            }
        }

        if !remaining.trim().is_empty() {
            text_parts.push(remaining.trim().to_string());
        }

        // Fallback: detect bare JSON tool calls without any XML wrapping.
        // Some models output {"name":"tool","arguments":{...}} directly.
        if calls.is_empty() {
            let (bare_text, bare_calls) = Self::parse_bare_json_tool_calls(&text_parts.join("\n"));
            if !bare_calls.is_empty() {
                return (bare_text, bare_calls);
            }
        }

        (text_parts.join("\n"), calls)
    }

    /// Parse bare `{"name":"tool","arguments":{...}}` JSON tool calls from text
    /// when no `<tool_call>` or `<tool_calls>` XML wrapping is present.
    fn parse_bare_json_tool_calls(text: &str) -> (String, Vec<ParsedToolCall>) {
        let mut text_parts = Vec::new();
        let mut calls = Vec::new();
        let mut remaining = text;

        while let Some(start) = remaining.find("{\"name\":") {
            let before = &remaining[..start];
            if !before.trim().is_empty() {
                text_parts.push(before.trim().to_string());
            }

            let slice = &remaining[start..];
            match Self::extract_json_object(slice) {
                Some(json_str) => {
                    let obj_len = json_str.len();
                    if let Ok(parsed) = serde_json::from_str::<Value>(json_str) {
                        let name = parsed
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        if !name.is_empty()
                            && parsed.get("arguments").is_some()
                        {
                            let arguments = parsed
                                .get("arguments")
                                .cloned()
                                .unwrap_or(Value::Object(serde_json::Map::new()));
                            calls.push(ParsedToolCall {
                                name,
                                arguments,
                                tool_call_id: None,
                            });
                        } else {
                            text_parts.push(json_str.to_string());
                        }
                    } else {
                        text_parts.push(json_str.to_string());
                    }
                    remaining = &remaining[start + obj_len..];
                }
                None => {
                    remaining = &remaining[start + 9..];
                }
            }
        }

        if !remaining.trim().is_empty() {
            text_parts.push(remaining.trim().to_string());
        }

        (text_parts.join("\n"), calls)
    }

    /// Extract a balanced JSON object starting at the given string position.
    /// Returns the JSON string including the outermost `{` and `}`.
    fn extract_json_object(s: &str) -> Option<&str> {
        let bytes = s.as_bytes();
        if bytes.is_empty() || bytes[0] != b'{' {
            return None;
        }
        let mut depth = 0u32;
        let mut in_string = false;
        let mut escaped = false;
        for (i, &ch) in bytes.iter().enumerate() {
            if escaped {
                escaped = false;
                continue;
            }
            if in_string {
                match ch {
                    b'"' => in_string = false,
                    b'\\' => escaped = true,
                    _ => {}
                }
                continue;
            }
            match ch {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&s[..=i]);
                    }
                }
                b'"' => in_string = true,
                b'\\' => escaped = true,
                _ => {}
            }
        }
        None
    }

    /// Remove `<think>...</think>` blocks from model output.
    fn strip_think_tags(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut rest = s;
        loop {
            if let Some(start) = rest.find("<think>") {
                result.push_str(&rest[..start]);
                if let Some(end) = rest[start..].find("</think>") {
                    rest = &rest[start + end + "</think>".len()..];
                } else {
                    break;
                }
            } else {
                result.push_str(rest);
                break;
            }
        }
        result
    }

    pub fn tool_specs(tools: &[Box<dyn Tool>]) -> Vec<ToolSpec> {
        tools.iter().map(|tool| tool.spec()).collect()
    }
}

impl ToolDispatcher for XmlToolDispatcher {
    fn parse_response(&self, response: &ChatResponse) -> (String, Vec<ParsedToolCall>) {
        let text = response.text_or_empty();
        Self::parse_xml_tool_calls(text)
    }

    fn format_results(&self, results: &[ToolExecutionResult]) -> ConversationMessage {
        let mut content = String::new();
        for result in results {
            let status = if result.success { "ok" } else { "error" };
            let _ = writeln!(
                content,
                "<tool_result name=\"{}\" status=\"{}\">\n{}\n</tool_result>",
                result.name, status, result.output
            );
        }
        ConversationMessage::Chat(ChatMessage::user(format!("[Tool results]\n{content}")))
    }

    fn prompt_instructions(&self, _tools: &[Box<dyn Tool>]) -> String {
        let mut instructions = String::new();
        instructions.push_str("## Tool Use Protocol\n\n");
        instructions
            .push_str("To use a tool, wrap a JSON object in <tool_call></tool_call> tags:\n\n");
        instructions.push_str(
            "```\n<tool_call>\n{\"name\": \"tool_name\", \"arguments\": {\"param\": \"value\"}}\n</tool_call>\n```\n\n",
        );

        instructions
    }

    fn to_provider_messages(&self, history: &[ConversationMessage]) -> Vec<ChatMessage> {
        history
            .iter()
            .flat_map(|msg| match msg {
                ConversationMessage::Chat(chat) => vec![chat.clone()],
                ConversationMessage::AssistantToolCalls { text, .. } => {
                    vec![ChatMessage::assistant(text.clone().unwrap_or_default())]
                }
                ConversationMessage::ToolResults(results) => {
                    let mut content = String::new();
                    for result in results {
                        let _ = writeln!(
                            content,
                            "<tool_result id=\"{}\">\n{}\n</tool_result>",
                            result.tool_call_id, result.content
                        );
                    }
                    vec![ChatMessage::user(format!("[Tool results]\n{content}"))]
                }
            })
            .collect()
    }

    fn should_send_tool_specs(&self) -> bool {
        false
    }
}

pub struct NativeToolDispatcher;

impl ToolDispatcher for NativeToolDispatcher {
    fn parse_response(&self, response: &ChatResponse) -> (String, Vec<ParsedToolCall>) {
        let text = response.text.clone().unwrap_or_default();
        let calls: Vec<ParsedToolCall> = response
            .tool_calls
            .iter()
            .map(|tc| ParsedToolCall {
                name: tc.name.clone(),
                arguments: serde_json::from_str(&tc.arguments).unwrap_or_else(|e| {
                    tracing::warn!(
                        tool = %tc.name,
                        error = %e,
                        "Failed to parse native tool call arguments as JSON; defaulting to empty object"
                    );
                    Value::Object(serde_json::Map::new())
                }),
                tool_call_id: Some(tc.id.clone()),
            })
            .collect();

        // Fallback: if the provider returned no native tool calls but the
        // text contains <tool_call> XML tags, try XML parsing instead.
        // Some models (especially when streaming) output tool calls in
        // XML format as text rather than as structured tool_calls, even
        // when the provider advertises native tool call support.
        if calls.is_empty() && (text.contains("<tool_call>") || text.contains("<tool_calls>") || text.contains("{\"name\":")) {
            return XmlToolDispatcher::parse_xml_tool_calls(&text);
        }

        (text, calls)
    }

    fn format_results(&self, results: &[ToolExecutionResult]) -> ConversationMessage {
        let messages = results
            .iter()
            .map(|result| ToolResultMessage {
                tool_call_id: result
                    .tool_call_id
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                content: result.output.clone(),
            })
            .collect();
        ConversationMessage::ToolResults(messages)
    }

    fn prompt_instructions(&self, _tools: &[Box<dyn Tool>]) -> String {
        String::new()
    }

    fn to_provider_messages(&self, history: &[ConversationMessage]) -> Vec<ChatMessage> {
        history
            .iter()
            .flat_map(|msg| match msg {
                ConversationMessage::Chat(chat) => vec![chat.clone()],
                ConversationMessage::AssistantToolCalls {
                    text,
                    tool_calls,
                    reasoning_content,
                } => {
                    let mut payload = serde_json::json!({
                        "content": text,
                        "tool_calls": tool_calls,
                    });
                    if let Some(rc) = reasoning_content {
                        payload["reasoning_content"] = serde_json::json!(rc);
                    }
                    vec![ChatMessage::assistant(payload.to_string())]
                }
                ConversationMessage::ToolResults(results) => results
                    .iter()
                    .map(|result| {
                        ChatMessage::tool(
                            serde_json::json!({
                                "tool_call_id": result.tool_call_id,
                                "content": result.content,
                            })
                            .to_string(),
                        )
                    })
                    .collect(),
            })
            .collect()
    }

    fn should_send_tool_specs(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_dispatcher_parses_tool_calls() {
        let response = ChatResponse {
            text: Some(
                "Checking\n<tool_call>{\"name\":\"shell\",\"arguments\":{\"command\":\"ls\"}}</tool_call>"
                    .into(),
            ),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        };
        let dispatcher = XmlToolDispatcher;
        let (_, calls) = dispatcher.parse_response(&response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "shell");
    }

    #[test]
    fn xml_dispatcher_strips_think_before_tool_call() {
        let response = ChatResponse {
            text: Some(
                "<think>I should list files</think>\n<tool_call>{\"name\":\"shell\",\"arguments\":{\"command\":\"ls\"}}</tool_call>"
                    .into(),
            ),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        };
        let dispatcher = XmlToolDispatcher;
        let (text, calls) = dispatcher.parse_response(&response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "shell");
        assert!(
            !text.contains("<think>"),
            "think tags should be stripped from text"
        );
    }

    #[test]
    fn xml_dispatcher_think_only_returns_no_calls() {
        let response = ChatResponse {
            text: Some("<think>Just thinking</think>".into()),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        };
        let dispatcher = XmlToolDispatcher;
        let (_, calls) = dispatcher.parse_response(&response);
        assert!(calls.is_empty());
    }

    #[test]
    fn native_dispatcher_roundtrip() {
        let response = ChatResponse {
            text: Some("ok".into()),
            tool_calls: vec![zeroclaw_providers::ToolCall {
                id: "tc1".into(),
                name: "file_read".into(),
                arguments: "{\"path\":\"a.txt\"}".into(),
            }],
            usage: None,
            reasoning_content: None,
        };
        let dispatcher = NativeToolDispatcher;
        let (_, calls) = dispatcher.parse_response(&response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_call_id.as_deref(), Some("tc1"));

        let msg = dispatcher.format_results(&[ToolExecutionResult {
            name: "file_read".into(),
            output: "hello".into(),
            success: true,
            tool_call_id: Some("tc1".into()),
        }]);
        match msg {
            ConversationMessage::ToolResults(results) => {
                assert_eq!(results.len(), 1);
                assert_eq!(results[0].tool_call_id, "tc1");
            }
            _ => panic!("expected tool results"),
        }
    }

    #[test]
    fn xml_format_results_contains_tool_result_tags() {
        let dispatcher = XmlToolDispatcher;
        let msg = dispatcher.format_results(&[ToolExecutionResult {
            name: "shell".into(),
            output: "ok".into(),
            success: true,
            tool_call_id: None,
        }]);
        let rendered = match msg {
            ConversationMessage::Chat(chat) => chat.content,
            _ => String::new(),
        };
        assert!(rendered.contains("<tool_result"));
        assert!(rendered.contains("shell"));
    }

    #[test]
    fn native_format_results_keeps_tool_call_id() {
        let dispatcher = NativeToolDispatcher;
        let msg = dispatcher.format_results(&[ToolExecutionResult {
            name: "shell".into(),
            output: "ok".into(),
            success: true,
            tool_call_id: Some("tc-1".into()),
        }]);

        match msg {
            ConversationMessage::ToolResults(results) => {
                assert_eq!(results.len(), 1);
                assert_eq!(results[0].tool_call_id, "tc-1");
            }
            _ => panic!("expected ToolResults variant"),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // reasoning_content pass-through tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn native_to_provider_messages_includes_reasoning_content() {
        let dispatcher = NativeToolDispatcher;
        let history = vec![ConversationMessage::AssistantToolCalls {
            text: Some("answer".into()),
            tool_calls: vec![zeroclaw_providers::ToolCall {
                id: "tc_1".into(),
                name: "shell".into(),
                arguments: "{}".into(),
            }],
            reasoning_content: Some("thinking step".into()),
        }];

        let messages = dispatcher.to_provider_messages(&history);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "assistant");

        let payload: serde_json::Value = serde_json::from_str(&messages[0].content).unwrap();
        assert_eq!(payload["reasoning_content"].as_str(), Some("thinking step"));
        assert_eq!(payload["content"].as_str(), Some("answer"));
        assert!(payload["tool_calls"].is_array());
    }

    #[test]
    fn native_to_provider_messages_omits_reasoning_content_when_none() {
        let dispatcher = NativeToolDispatcher;
        let history = vec![ConversationMessage::AssistantToolCalls {
            text: Some("answer".into()),
            tool_calls: vec![zeroclaw_providers::ToolCall {
                id: "tc_1".into(),
                name: "shell".into(),
                arguments: "{}".into(),
            }],
            reasoning_content: None,
        }];

        let messages = dispatcher.to_provider_messages(&history);
        assert_eq!(messages.len(), 1);

        let payload: serde_json::Value = serde_json::from_str(&messages[0].content).unwrap();
        assert!(payload.get("reasoning_content").is_none());
    }

    #[test]
    fn xml_to_provider_messages_ignores_reasoning_content() {
        let dispatcher = XmlToolDispatcher;
        let history = vec![ConversationMessage::AssistantToolCalls {
            text: Some("answer".into()),
            tool_calls: vec![zeroclaw_providers::ToolCall {
                id: "tc_1".into(),
                name: "shell".into(),
                arguments: "{}".into(),
            }],
            reasoning_content: Some("should be ignored".into()),
        }];

        let messages = dispatcher.to_provider_messages(&history);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "assistant");
        // XmlToolDispatcher returns text only, not JSON payload
        assert_eq!(messages[0].content, "answer");
        assert!(!messages[0].content.contains("reasoning_content"));
    }

    #[test]
    fn xml_dispatcher_parses_tool_calls_plural_tag() {
        let response = ChatResponse {
            text: Some("OK\n<tool_calls>{\"name\":\"shell\",\"arguments\":{\"command\":\"ls\"}}</tool_calls>".into()),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        };
        let dispatcher = XmlToolDispatcher;
        let (_, calls) = dispatcher.parse_response(&response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "shell");
    }

    #[test]
    fn xml_dispatcher_parses_bare_json_tool_calls() {
        let response = ChatResponse {
            text: Some("Let me check.\n{\"name\":\"shell\",\"arguments\":{\"command\":\"ls\"}}\n{\"name\":\"weather\",\"arguments\":{\"location\":\"dalian\"}}".into()),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        };
        let dispatcher = XmlToolDispatcher;
        let (text, calls) = dispatcher.parse_response(&response);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "shell");
        assert_eq!(calls[1].name, "weather");
        assert!(!text.contains("{\"name\":"));
    }

    #[test]
    fn xml_dispatcher_parses_nested_json_in_bare_tool_call() {
        let json = r#"<tool_call>{"name": "shell", "arguments": {"command": "curl -d '{\"model\":\"seedream\"}'"}}</tool_call>"#;
        let response = ChatResponse {
            text: Some(json.into()),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        };
        let dispatcher = XmlToolDispatcher;
        let (_, calls) = dispatcher.parse_response(&response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "shell");
        assert!(calls[0].arguments["command"].as_str().unwrap().contains("seedream"));
    }

    #[test]
    fn native_dispatcher_falls_back_to_xml_for_bare_json() {
        let response = ChatResponse {
            text: Some("OK\n{\"name\":\"echo\",\"arguments\":{}}".into()),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        };
        let dispatcher = NativeToolDispatcher;
        let (_, calls) = dispatcher.parse_response(&response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "echo");
    }

    #[test]
    fn native_dispatcher_falls_back_to_xml_for_plural_tag() {
        let response = ChatResponse {
            text: Some("<tool_calls>{\"name\":\"echo\",\"arguments\":{}}</tool_calls>".into()),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        };
        let dispatcher = NativeToolDispatcher;
        let (_, calls) = dispatcher.parse_response(&response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "echo");
    }
}
