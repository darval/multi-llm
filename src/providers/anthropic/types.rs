//! Anthropic API request and response type definitions

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Cache control for prompt caching
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheControl {
    #[serde(rename = "type")]
    pub cache_type: String, // "ephemeral"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>, // "5m" or "1h"
}

/// System message with optional cache control
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemMessage {
    #[serde(rename = "type")]
    pub message_type: String, // "text"
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

/// System field can be either a string (legacy) or array of system messages (with caching)
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum SystemField {
    String(String),
    Messages(Vec<SystemMessage>),
}

/// Anthropic Messages API request structure
/// Fields are ordered to match Anthropic's caching hierarchy: tools->system->messages->params
#[derive(Debug, Serialize, Clone)]
pub(super) struct AnthropicRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemField>,
    pub messages: Vec<AnthropicMessage>,
    pub model: String,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

/// Anthropic message structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(super) struct AnthropicMessage {
    pub role: String,
    pub content: AnthropicContent,
}

/// Anthropic content structure (can be string or array of content blocks)
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub(super) enum AnthropicContent {
    Text(String),
    Blocks(Vec<AnthropicContentBlock>),
}

/// Anthropic content block structure
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub(super) enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

/// Anthropic API response structure
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(super) struct AnthropicResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<AnthropicContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: AnthropicUsage,
}

/// Anthropic usage information with cache statistics
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(super) struct AnthropicUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u32>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u32>,
    #[serde(default)]
    pub cache_creation: Option<CacheCreationDetails>,
}

/// Detailed cache creation breakdown
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(super) struct CacheCreationDetails {
    #[serde(default)]
    pub ephemeral_5m_input_tokens: Option<u32>,
    #[serde(default)]
    pub ephemeral_1h_input_tokens: Option<u32>,
}
