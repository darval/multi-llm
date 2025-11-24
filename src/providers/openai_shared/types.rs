//! OpenAI-compatible data structures and types
//!
//! Contains the request/response structures used by OpenAI-compatible providers.

use crate::types::LLMUsage;
use serde::{Deserialize, Serialize};

/// OpenAI-compatible message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: String,
}

/// OpenAI-compatible response format for structured output
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAIResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String, // "json_schema"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<OpenAIJsonSchema>,
}

/// OpenAI JSON schema structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAIJsonSchema {
    pub name: String,
    pub schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// OpenAI-compatible chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<OpenAIResponseFormat>,
}

/// OpenAI-compatible chat completion response
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAIResponse {
    pub choices: Vec<OpenAIChoice>,
    #[serde(default)]
    pub usage: Option<OpenAIUsage>,
}

/// Choice in OpenAI response
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAIChoice {
    pub message: OpenAIResponseMessage,
    #[allow(dead_code)]
    pub finish_reason: Option<String>,
}

/// Message in OpenAI response choice
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAIResponseMessage {
    #[allow(dead_code)]
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
}

/// Tool call in OpenAI response
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAIToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: OpenAIToolFunction,
}

/// Function details in OpenAI tool call
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAIToolFunction {
    pub name: String,
    pub arguments: String, // JSON string
}

/// Usage information in OpenAI response
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl From<OpenAIUsage> for LLMUsage {
    fn from(usage: OpenAIUsage) -> Self {
        Self {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        }
    }
}
