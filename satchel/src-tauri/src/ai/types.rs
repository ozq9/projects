//! Wire types for the Claude Messages API (`POST /v1/messages`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

impl Message {
    pub fn user(content: Vec<ContentBlock>) -> Self {
        Self { role: Role::User, content }
    }
    pub fn user_text(text: impl Into<String>) -> Self {
        Self::user(vec![ContentBlock::text(text)])
    }
    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self { role: Role::Assistant, content: vec![ContentBlock::text(text)] }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    /// Images (PNG/JPEG/GIF/WebP) for vision.
    Image {
        source: MediaSource,
    },
    /// PDFs.
    Document {
        source: MediaSource,
    },
}

impl ContentBlock {
    pub fn text(text: impl Into<String>) -> Self {
        ContentBlock::Text { text: text.into() }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MediaSource {
    Base64 { media_type: String, data: String },
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct OutputConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<OutputFormat>,
}

impl OutputConfig {
    fn is_empty(&self) -> bool {
        self.effort.is_none() && self.format.is_none()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OutputFormat {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub schema: Value,
}

impl OutputFormat {
    pub fn json_schema(schema: Value) -> Self {
        Self { kind: "json_schema", schema }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MessagesRequest {
    pub model: String,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "OutputConfig::is_empty")]
    pub output_config: OutputConfig,
    /// Server-side refusal fallback; filled in by the client for models that support it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallbacks: Option<Value>,
    /// Sent as the `anthropic-beta` header, not in the body.
    #[serde(skip)]
    pub betas: Vec<&'static str>,
}

impl MessagesRequest {
    pub fn new(model: impl Into<String>, max_tokens: u32, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            max_tokens,
            system: None,
            messages,
            output_config: OutputConfig::default(),
            fallbacks: None,
            betas: Vec::new(),
        }
    }
}

// ---------- responses ----------

#[derive(Debug, Clone, Deserialize)]
pub struct MessagesResponse {
    pub id: String,
    /// The model that produced the final message (may differ from the request after a fallback).
    pub model: String,
    pub content: Vec<ResponseBlock>,
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub stop_details: Option<StopDetails>,
    pub usage: Usage,
}

impl MessagesResponse {
    /// All text blocks joined. Thinking and fallback marker blocks are skipped.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for block in &self.content {
            if let ResponseBlock::Text { text } = block {
                out.push_str(text);
            }
        }
        out
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseBlock {
    #[serde(rename = "text")]
    Text { text: String },
    /// thinking, fallback, and any block type added later.
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StopDetails {
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
}

impl Usage {
    pub fn add(&mut self, other: &Usage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_creation_input_tokens =
            Some(self.cache_creation_input_tokens.unwrap_or(0) + other.cache_creation_input_tokens.unwrap_or(0));
        self.cache_read_input_tokens =
            Some(self.cache_read_input_tokens.unwrap_or(0) + other.cache_read_input_tokens.unwrap_or(0));
    }
}

/// Error body: `{"type":"error","error":{"type":"...","message":"..."}}`.
#[derive(Debug, Deserialize)]
pub struct ApiErrorBody {
    pub error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
pub struct ApiErrorDetail {
    #[serde(rename = "type")]
    pub kind: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_serializes_without_empty_fields() {
        let req = MessagesRequest::new("claude-opus-5-5", 1024, vec![Message::user_text("hi")]);
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(
            v,
            json!({"model":"claude-opus-5-5","max_tokens":1024,"messages":[{"role":"user","content":[{"type":"text","text":"hi"}]}]})
        );

        let mut req = req;
        req.output_config = OutputConfig {
            effort: Some(Effort::Low),
            format: Some(OutputFormat::json_schema(json!({"type":"object"}))),
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["output_config"], json!({"effort":"low","format":{"type":"json_schema","schema":{"type":"object"}}}));
    }

    #[test]
    fn response_skips_non_text_blocks() {
        let body = json!({
            "id": "msg_1", "type": "message", "role": "assistant", "model": "claude-opus-5-5",
            "content": [
                {"type": "thinking", "thinking": "", "signature": "x"},
                {"type": "text", "text": "{\"ok\":"},
                {"type": "text", "text": "true}"}
            ],
            "stop_reason": "end_turn", "stop_details": null,
            "usage": {"input_tokens": 12, "output_tokens": 5, "cache_read_input_tokens": 0}
        });
        let r: MessagesResponse = serde_json::from_value(body).unwrap();
        assert_eq!(r.text(), "{\"ok\":true}");
        assert_eq!(r.usage.input_tokens, 12);
    }
}
