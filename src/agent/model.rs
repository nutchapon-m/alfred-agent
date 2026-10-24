//! Typed model for `claude -p --output-format stream-json --verbose` (NDJSON).
//!
//! Every stdout line is one JSON object discriminated by `type`.
//! Design rules:
//! - Unknown `type` / `subtype` / content-block types fall into an `Unknown`/`Other`
//!   variant instead of failing, so a newer CLI doesn't break the parser.
//! - Unknown fields are ignored (serde default).
//! - Most fields are `Option` or `#[serde(default)]` because the CLI adds/removes
//!   fields between versions.
//!
//! Cargo.toml:
//!   serde = { version = "1", features = ["derive"] }
//!   serde_json = "1"

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io::BufRead;

// ───────────────────────────── top level ─────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamMsg {
    System(SystemMsg),
    Assistant(AssistantMsg),
    /// Usually carries `tool_result` blocks after Claude calls a tool.
    User(UserMsg),
    /// Only with `--include-partial-messages`.
    StreamEvent(StreamEventMsg),
    RateLimitEvent(RateLimitEventMsg),
    Result(ResultMsg),
    #[serde(other)]
    Unknown,
}

impl StreamMsg {
    /// Parse one NDJSON line. Blank lines return `Ok(None)`.
    pub fn parse_line(line: &str) -> Result<Option<Self>, serde_json::Error> {
        let line = line.trim();
        if line.is_empty() {
            return Ok(None);
        }
        serde_json::from_str(line).map(Some)
    }

    pub fn session_id(&self) -> Option<&str> {
        match self {
            StreamMsg::System(SystemMsg::Init(m)) => Some(&m.session_id),
            StreamMsg::Assistant(m) => Some(&m.session_id),
            StreamMsg::User(m) => Some(&m.session_id),
            StreamMsg::StreamEvent(m) => m.session_id.as_deref(),
            StreamMsg::RateLimitEvent(m) => m.session_id.as_deref(),
            StreamMsg::Result(m) => Some(&m.session_id),
            _ => None,
        }
    }
}

// ───────────────────────────── system ─────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum SystemMsg {
    Init(SystemInit),
    /// e.g. `compact_boundary`, `api_retry`, `status` … (not modelled; ignored)
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SystemInit {
    pub session_id: String,
    pub cwd: Option<String>,
    pub model: Option<String>,
    #[serde(rename = "permissionMode")]
    pub permission_mode: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub mcp_servers: Vec<McpServer>,
    #[serde(default)]
    pub slash_commands: Vec<String>,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub plugins: Vec<Plugin>,
    pub claude_code_version: Option<String>,
    pub output_style: Option<String>,
    #[serde(rename = "apiKeySource")]
    pub api_key_source: Option<String>,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct McpServer {
    pub name: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Plugin {
    pub name: String,
    pub path: Option<String>,
    pub source: Option<String>,
    pub version: Option<String>,
}

// ───────────────────────────── assistant / user ─────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct AssistantMsg {
    pub message: ApiMessage,
    pub parent_tool_use_id: Option<String>,
    pub session_id: String,
    pub uuid: Option<String>,
}

impl AssistantMsg {
    /// Concatenated text of all `text` blocks.
    pub fn text(&self) -> String {
        self.message
            .content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// (id, name, input) of every `tool_use` block.
    pub fn tool_uses(&self) -> impl Iterator<Item = (&str, &str, &Value)> {
        self.message.content.iter().filter_map(|b| match b {
            ContentBlock::ToolUse { id, name, input } => Some((id.as_str(), name.as_str(), input)),
            _ => None,
        })
    }
}

/// The Anthropic Messages API message object, as embedded by the CLI.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiMessage {
    pub id: Option<String>,
    pub model: Option<String>,
    pub role: Option<String>,
    #[serde(default)]
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserMsg {
    pub message: UserMessage,
    pub parent_tool_use_id: Option<String>,
    pub session_id: String,
    pub uuid: Option<String>,
    /// CLI-specific structured copy of the tool result (shape varies per tool).
    pub tool_use_result: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserMessage {
    pub role: Option<String>,
    pub content: MessageContent,
}

/// `content` may be a plain string or an array of blocks.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
        signature: Option<String>,
    },
    RedactedThinking {
        data: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: Option<ToolResultContent>,
        #[serde(default)]
        is_error: bool,
    },
    Image {
        source: Value,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    pub service_tier: Option<String>,
}

// ───────────────────────────── stream_event ─────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct StreamEventMsg {
    /// Raw Messages API SSE event (`message_start`, `content_block_delta`, …).
    pub event: Value,
    pub parent_tool_use_id: Option<String>,
    pub session_id: Option<String>,
    pub uuid: Option<String>,
}

// ───────────────────────────── rate_limit_event ─────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitEventMsg {
    pub rate_limit_info: RateLimitInfo,
    pub session_id: Option<String>,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitInfo {
    /// e.g. "allowed"
    pub status: String,
    /// Unix seconds.
    pub resets_at: Option<i64>,
    /// e.g. "five_hour"
    pub rate_limit_type: Option<String>,
    pub overage_status: Option<String>,
    pub overage_disabled_reason: Option<String>,
    pub is_using_overage: Option<bool>,
    /// Keys like "five_hour", "seven_day".
    #[serde(default)]
    pub unified_windows: HashMap<String, RateLimitWindow>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitWindow {
    /// 0.0 – 1.0
    pub utilization: f64,
    pub resets_at: Option<i64>,
}

// ───────────────────────────── result ─────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ResultMsg {
    pub subtype: ResultSubtype,
    #[serde(default)]
    pub is_error: bool,
    pub session_id: String,
    /// Final text. Absent on most error subtypes.
    pub result: Option<String>,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub duration_api_ms: u64,
    #[serde(default)]
    pub num_turns: u32,
    #[serde(default)]
    pub total_cost_usd: f64,
    pub usage: Option<Usage>,
    #[serde(default, rename = "modelUsage")]
    pub model_usage: HashMap<String, ModelUsage>,
    #[serde(default)]
    pub permission_denials: Vec<Value>,
    pub stop_reason: Option<String>,
    pub terminal_reason: Option<String>,
    pub api_error_status: Option<Value>,
    /// Error messages on error subtypes.
    #[serde(default)]
    pub errors: Vec<String>,
    pub uuid: Option<String>,
}

impl ResultMsg {
    pub fn is_success(&self) -> bool {
        !self.is_error && self.subtype == ResultSubtype::Success
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultSubtype {
    Success,
    ErrorMaxTurns,
    ErrorDuringExecution,
    ErrorMaxBudgetUsd,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub web_search_requests: u64,
    #[serde(default, rename = "costUSD")]
    pub cost_usd: f64,
    pub context_window: Option<u64>,
    pub max_output_tokens: Option<u64>,
}

// ───────────────────────────── reader ─────────────────────────────

/// Read NDJSON from e.g. `child.stdout`, calling `on_msg` per message.
/// Lines that fail to parse are passed to `on_bad_line` instead of aborting.
/// Returns the final `result` message, if one arrived.
pub fn read_stream<R: BufRead>(
    reader: R,
    mut on_msg: impl FnMut(&StreamMsg),
    mut on_bad_line: impl FnMut(&str, serde_json::Error),
) -> std::io::Result<Option<ResultMsg>> {
    let mut last_result = None;
    for line in reader.lines() {
        let line = line?;
        match StreamMsg::parse_line(&line) {
            Ok(Some(msg)) => {
                on_msg(&msg);
                if let StreamMsg::Result(r) = msg {
                    last_result = Some(r);
                }
            }
            Ok(None) => {}
            Err(e) => on_bad_line(&line, e),
        }
    }
    Ok(last_result)
}

// ───────────────────────────── tests ─────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const INIT: &str = r#"{"type":"system","subtype":"init","cwd":"/Users/nutchapon/repo/alfred-agent","session_id":"46205a8e","tools":["Task","Bash"],"mcp_servers":[],"model":"claude-opus-5-5","permissionMode":"acceptEdits","slash_commands":["init"],"apiKeySource":"none","claude_code_version":"2.1.301","output_style":"default","agents":["Explore"],"skills":["debug"],"plugins":[{"name":"gopls-lsp","path":"/x","source":"gopls-lsp@claude-plugins-official","version":"1.0.0"},{"name":"cc-plugin-sec-default","path":"builtin","source":"cc-plugin-sec-default@builtin"}],"capabilities":["x"],"uuid":"a38bb446"}"#;

    const ASSISTANT: &str = r#"{"type":"assistant","message":{"model":"claude-opus-5-5","id":"msg_1","type":"message","role":"assistant","content":[{"type":"text","text":"Hello!"}],"container":null,"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":8620,"cache_read_input_tokens":11170,"cache_creation":{"ephemeral_5m_input_tokens":0},"output_tokens":5,"service_tier":"standard"}},"parent_tool_use_id":null,"session_id":"46205a8e","uuid":"7d3dd8c0"}"#;

    const RATE: &str = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1792873800,"rateLimitType":"five_hour","overageStatus":"rejected","overageDisabledReason":"org_level_disabled","isUsingOverage":false,"unifiedWindows":{"five_hour":{"utilization":0.03,"resetsAt":1792873800},"seven_day":{"utilization":0.03,"resetsAt":1793203200}}},"uuid":"c54823bc","session_id":"46205a8e"}"#;

    // Note: "type" is NOT the first key here — same as the real CLI output.
    const RESULT: &str = r#"{"duration_api_ms":2151,"stop_reason":"end_turn","session_id":"46205a8e","total_cost_usd":0.073722,"usage":{"input_tokens":2,"output_tokens":126,"cache_read_input_tokens":11170,"cache_creation_input_tokens":8620},"modelUsage":{"claude-opus-5-5":{"inputTokens":2,"outputTokens":126,"cacheReadInputTokens":11170,"cacheCreationInputTokens":8620,"webSearchRequests":0,"costUSD":0.073722,"contextWindow":1000000,"maxOutputTokens":128000}},"permission_denials":[],"terminal_reason":"completed","is_error":false,"num_turns":1,"subtype":"success","api_error_status":null,"result":"Hello!","type":"result","duration_ms":3129,"uuid":"32374c0a"}"#;

    const TOOL_USE: &str = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"toolu_1","name":"Bash","input":{"command":"ls"}}]},"parent_tool_use_id":null,"session_id":"s"}"#;

    const TOOL_RESULT: &str = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"Cargo.toml\nsrc","is_error":false}]},"parent_tool_use_id":null,"session_id":"s","tool_use_result":{"stdout":"Cargo.toml\nsrc"}}"#;

    fn parse(s: &str) -> StreamMsg {
        StreamMsg::parse_line(s).unwrap().unwrap()
    }

    #[test]
    fn init() {
        let StreamMsg::System(SystemMsg::Init(m)) = parse(INIT) else {
            panic!()
        };
        assert_eq!(m.permission_mode.as_deref(), Some("acceptEdits"));
        assert_eq!(m.plugins.len(), 2);
        assert_eq!(m.plugins[1].version, None);
    }

    #[test]
    fn assistant_text() {
        let StreamMsg::Assistant(m) = parse(ASSISTANT) else {
            panic!()
        };
        assert_eq!(m.text(), "Hello!");
        assert_eq!(m.message.usage.unwrap().cache_read_input_tokens, 11170);
    }

    #[test]
    fn rate_limit() {
        let StreamMsg::RateLimitEvent(m) = parse(RATE) else {
            panic!()
        };
        assert_eq!(
            m.rate_limit_info.unified_windows["seven_day"].utilization,
            0.03
        );
    }

    #[test]
    fn result() {
        let StreamMsg::Result(m) = parse(RESULT) else {
            panic!()
        };
        assert!(m.is_success());
        assert_eq!(m.model_usage["claude-opus-5-5"].cost_usd, 0.073722);
    }

    #[test]
    fn tool_roundtrip() {
        let StreamMsg::Assistant(a) = parse(TOOL_USE) else {
            panic!()
        };
        let (id, name, input) = a.tool_uses().next().unwrap();
        assert_eq!(
            (id, name, input["command"].as_str()),
            ("toolu_1", "Bash", Some("ls"))
        );

        let StreamMsg::User(u) = parse(TOOL_RESULT) else {
            panic!()
        };
        let MessageContent::Blocks(b) = u.message.content else {
            panic!()
        };
        assert!(matches!(
            &b[0],
            ContentBlock::ToolResult {
                is_error: false,
                ..
            }
        ));
    }

    #[test]
    fn unknowns_dont_fail() {
        assert!(matches!(
            parse(r#"{"type":"brand_new","x":1}"#),
            StreamMsg::Unknown
        ));
        assert!(matches!(
            parse(r#"{"type":"system","subtype":"compact_boundary","session_id":"s"}"#),
            StreamMsg::System(SystemMsg::Other)
        ));
        assert!(matches!(
            parse(
                r#"{"type":"result","subtype":"error_something_new","is_error":true,"session_id":"s"}"#
            ),
            StreamMsg::Result(ResultMsg {
                subtype: ResultSubtype::Other,
                ..
            })
        ));
        assert!(StreamMsg::parse_line("   ").unwrap().is_none());
    }

    #[test]
    fn reader() {
        let input = [INIT, ASSISTANT, RATE, "", "not json", RESULT].join("\n");
        let mut n = 0;
        let mut bad = 0;
        let r = read_stream(input.as_bytes(), |_| n += 1, |_, _| bad += 1).unwrap();
        assert_eq!((n, bad), (4, 1));
        assert_eq!(r.unwrap().result.as_deref(), Some("Hello!"));
    }
}
