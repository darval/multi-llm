//! Utility functions and HTTP client for OpenAI-compatible providers
//!
//! Contains configuration utilities, HTTP client functionality,
//! custom format parsing, and conversion functions.

// Allow unwrap in custom format parser - regex captures are verified before unwrap
#![allow(clippy::unwrap_used)]

use super::types::*;
use crate::error::{LlmError, LlmResult};
use crate::retry::{RetryExecutor, RetryPolicy};
use crate::{MessageContent, MessageRole, UnifiedMessage};
use crate::core_types::executor::{ExecutorLLMConfig, ExecutorTool, ExecutorToolCall, ToolChoice};
use crate::log_error;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::Mutex;

/// Errors related to custom tool format parsing
#[derive(Debug, Error)]
pub enum CustomFormatError {
    #[error("Failed to parse custom format: {0}")]
    ParseError(String),
    #[error("Invalid JSON in custom format: {0}")]
    InvalidJson(#[from] serde_json::Error),
}

/// Represents a detected custom tool call format
#[derive(Debug)]
pub struct CustomToolCallMatch {
    pub function_name: String,
    pub arguments: Value,
    pub cleaned_content: String,
    pub raw_match: String,
}

/// Parser for custom tool call formats
pub struct CustomFormatParser {
    patterns: Vec<(String, Regex)>, // (format_name, pattern)
}

impl Default for CustomFormatParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CustomFormatParser {
    pub fn new() -> Self {
        // Create patterns with safe fallback on compilation failure
        let mut patterns = Vec::new();

        // Pattern 1: GPT-OSS v1 format - only capture function name, let extract_balanced_json handle the JSON
        if let Ok(regex) =
            Regex::new(r"commentary to=functions\.(\w+)\s+<\|constrain\|>json<\|message\|>")
        {
            patterns.push(("gpt_oss_v1".to_string(), regex));
        }

        // Pattern 2: XML tool_call format (Qwen models)
        // Matches: <tool_call>{"name": "func_name", "arguments": {...}}</tool_call>
        // Also matches: <tool_call>{"name": "func_name", "arguments": {...}} (without closing tag)
        // Uses (?s) flag to match across multiple lines and improved capture for JSON objects
        if let Ok(regex) = Regex::new(r#"(?s)<tool_call>\s*(\{.*?\})\s*(?:</tool_call>|$)"#) {
            patterns.push(("xml_tool_call".to_string(), regex));
        }

        // Pattern 3: DeepSeek TOOL_REQUEST format
        // Matches: [TOOL_REQUEST]{"name": "func_name", "arguments": {...}}[END_TOOL_REQUEST]
        if let Ok(regex) = Regex::new(r#"(?s)\[TOOL_REQUEST\](.*?)\[END_TOOL_REQUEST\]"#) {
            patterns.push(("deepseek_tool_request".to_string(), regex));
        }

        // Pattern 4: "Tool call:" format (self-generated format from structured content)
        // Matches: Tool call: function_name with args: {...}
        if let Ok(regex) = Regex::new(r#"(?s)Tool call:\s+(\w+)\s+with args:\s+(\{.*\})"#) {
            patterns.push(("tool_call_with_args".to_string(), regex));
        }

        // Pattern 5: Bracketed JSON format (some models)
        // Matches: {"name": "func_name", "arguments": {...}} when isolated in response
        if let Ok(regex) = Regex::new(r#"(?s)^(\{[^{}]*"name"[^{}]*"arguments"[^{}]*\})$"#) {
            patterns.push(("json_only".to_string(), regex));
        }

        Self { patterns }
    }

    /// Attempts to parse custom tool call formats from content
    pub fn parse(&self, content: &str) -> Result<Option<CustomToolCallMatch>, CustomFormatError> {
        for (format_name, pattern) in &self.patterns {
            if let Some(result) = self.try_parse_pattern(format_name, pattern, content)? {
                return Ok(Some(result));
            }
        }

        Self::log_no_match(content, self.patterns.len());
        Ok(None)
    }

    /// Try to parse a single pattern match
    fn try_parse_pattern(
        &self,
        format_name: &str,
        pattern: &regex::Regex,
        content: &str,
    ) -> Result<Option<CustomToolCallMatch>, CustomFormatError> {
        let Some(captures) = pattern.captures(content) else {
            return Ok(None);
        };

        tracing::debug!(
            format_name = format_name,
            capture_count = captures.len(),
            "FOUND MATCH for custom tool format"
        );

        match format_name {
            "gpt_oss_v1" => self.parse_gpt_oss_v1(&captures, content),
            "xml_tool_call" => self.parse_xml_tool_call(&captures, content, format_name),
            "deepseek_tool_request" => {
                self.parse_deepseek_tool_request(&captures, content, format_name)
            }
            "tool_call_with_args" => {
                self.parse_tool_call_with_args(&captures, content, format_name)
            }
            "json_only" => self.parse_json_only(&captures, format_name),
            _ => Ok(None),
        }
    }

    /// Log when no pattern matches
    fn log_no_match(content: &str, pattern_count: usize) {
        tracing::warn!(
            content_preview = content.chars().take(300).collect::<String>(),
            full_content = content,
            pattern_count = pattern_count,
            content_length = content.len(),
            "No custom tool format patterns matched - content may contain unrecognized tool call format"
        );
    }

    fn parse_gpt_oss_v1(
        &self,
        captures: &regex::Captures,
        content: &str,
    ) -> Result<Option<CustomToolCallMatch>, CustomFormatError> {
        let function_name = captures
            .get(1)
            .ok_or_else(|| CustomFormatError::ParseError("No function name".to_string()))?
            .as_str()
            .to_string();

        if let Some(message_start) = content.find("<|constrain|>json<|message|>") {
            let json_start = message_start + "<|constrain|>json<|message|>".len();
            let remaining_content = &content[json_start..];

            if let Some((json_str, json_end_pos)) = Self::extract_balanced_json(remaining_content) {
                let arguments = serde_json::from_str::<Value>(json_str.trim())?;

                let pattern_start = content.find("commentary to=functions.").ok_or_else(|| {
                    CustomFormatError::ParseError("Pattern start not found".to_string())
                })?;
                let pattern_end = json_start + json_end_pos;
                let full_match = &content[pattern_start..pattern_end];
                let cleaned_content = content.replace(full_match, "").trim().to_string();

                return Ok(Some(CustomToolCallMatch {
                    function_name,
                    arguments,
                    cleaned_content,
                    raw_match: full_match.to_string(),
                }));
            }
        }
        Ok(None)
    }

    fn parse_xml_tool_call(
        &self,
        captures: &regex::Captures,
        content: &str,
        format_name: &str,
    ) -> Result<Option<CustomToolCallMatch>, CustomFormatError> {
        let captured_content = captures
            .get(1)
            .ok_or_else(|| {
                CustomFormatError::ParseError("No content captured from XML tool call".to_string())
            })?
            .as_str()
            .trim();

        let json_content =
            if let Some((extracted_json, _)) = Self::extract_balanced_json(captured_content) {
                extracted_json
            } else {
                Self::attempt_json_repair(captured_content)
            };

        let json_obj = serde_json::from_str::<Value>(&json_content)?;
        let function_name = json_obj
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| {
                CustomFormatError::ParseError("Missing 'name' field in tool call".to_string())
            })?
            .to_string();

        let arguments = json_obj
            .get("arguments")
            .ok_or_else(|| {
                CustomFormatError::ParseError("Missing 'arguments' field in tool call".to_string())
            })?
            .clone();

        let full_match = captures.get(0).unwrap().as_str();
        let cleaned_content = content.replace(full_match, "").trim().to_string();

        tracing::debug!(
            format = format_name,
            function = &function_name,
            json_length = json_content.len(),
            "Successfully parsed XML tool call with balanced JSON extraction"
        );

        Ok(Some(CustomToolCallMatch {
            function_name,
            arguments,
            cleaned_content,
            raw_match: full_match.to_string(),
        }))
    }

    fn parse_deepseek_tool_request(
        &self,
        captures: &regex::Captures,
        content: &str,
        format_name: &str,
    ) -> Result<Option<CustomToolCallMatch>, CustomFormatError> {
        let json_content = captures.get(1).unwrap().as_str().trim();

        if let Some((json_str, _)) = Self::extract_balanced_json(json_content) {
            let json_obj = serde_json::from_str::<Value>(&json_str)?;

            let function_name = json_obj
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| {
                    CustomFormatError::ParseError(
                        "Missing 'name' field in DeepSeek tool call".to_string(),
                    )
                })?
                .to_string();

            let arguments = json_obj
                .get("arguments")
                .ok_or_else(|| {
                    CustomFormatError::ParseError(
                        "Missing 'arguments' field in DeepSeek tool call".to_string(),
                    )
                })?
                .clone();

            let full_match = captures.get(0).unwrap().as_str();
            let cleaned_content = content.replace(full_match, "").trim().to_string();

            tracing::debug!(
                format = format_name,
                function = &function_name,
                json_length = json_str.len(),
                "Successfully parsed DeepSeek TOOL_REQUEST format"
            );

            return Ok(Some(CustomToolCallMatch {
                function_name,
                arguments,
                cleaned_content,
                raw_match: full_match.to_string(),
            }));
        }

        Err(CustomFormatError::ParseError(
            "Failed to extract balanced JSON from DeepSeek TOOL_REQUEST".to_string(),
        ))
    }

    fn parse_tool_call_with_args(
        &self,
        captures: &regex::Captures,
        content: &str,
        format_name: &str,
    ) -> Result<Option<CustomToolCallMatch>, CustomFormatError> {
        let function_name = captures
            .get(1)
            .ok_or_else(|| {
                CustomFormatError::ParseError(
                    "No function name captured from tool call format".to_string(),
                )
            })?
            .as_str()
            .to_string();

        let args_json = captures
            .get(2)
            .ok_or_else(|| {
                CustomFormatError::ParseError(
                    "No arguments captured from tool call format".to_string(),
                )
            })?
            .as_str();

        let arguments = serde_json::from_str::<Value>(args_json)?;
        let full_match = captures.get(0).unwrap().as_str();
        let cleaned_content = content.replace(full_match, "").trim().to_string();

        tracing::debug!(
            format = format_name,
            function = &function_name,
            "Successfully parsed 'Tool call:' format"
        );

        Ok(Some(CustomToolCallMatch {
            function_name,
            arguments,
            cleaned_content,
            raw_match: full_match.to_string(),
        }))
    }

    fn parse_json_only(
        &self,
        captures: &regex::Captures,
        format_name: &str,
    ) -> Result<Option<CustomToolCallMatch>, CustomFormatError> {
        let json_str = captures.get(1).unwrap().as_str();
        let json_obj = serde_json::from_str::<Value>(json_str)?;

        let function_name = json_obj
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| {
                CustomFormatError::ParseError(
                    "Missing 'name' field in JSON-only tool call".to_string(),
                )
            })?
            .to_string();

        let arguments = json_obj
            .get("arguments")
            .ok_or_else(|| {
                CustomFormatError::ParseError(
                    "Missing 'arguments' field in JSON-only tool call".to_string(),
                )
            })?
            .clone();

        tracing::debug!(
            format = format_name,
            function = &function_name,
            "Successfully parsed JSON-only tool call format"
        );

        Ok(Some(CustomToolCallMatch {
            function_name,
            arguments,
            cleaned_content: "".to_string(),
            raw_match: json_str.to_string(),
        }))
    }

    /// Clean tool call patterns from content as a fallback when parsing fails
    /// This removes obvious tool call patterns to prevent showing raw XML/tags to users
    fn clean_tool_call_patterns(content: &str) -> String {
        let mut cleaned = content.to_string();

        // Remove XML tool call patterns (even malformed ones)
        if let Ok(regex) = Regex::new(r#"(?s)<tool_call>.*?(?:</tool_call>|$)"#) {
            cleaned = regex.replace_all(&cleaned, "").to_string();
        }

        // Remove DeepSeek tool request patterns
        if let Ok(regex) = Regex::new(r#"(?s)\[TOOL_REQUEST\].*?(?:\[END_TOOL_REQUEST\]|$)"#) {
            cleaned = regex.replace_all(&cleaned, "").to_string();
        }

        // Remove "Tool call:" format patterns
        if let Ok(regex) = Regex::new(r#"(?s)Tool call:\s+\w+\s+with args:\s+\{.*?\}"#) {
            cleaned = regex.replace_all(&cleaned, "").to_string();
        }

        // Remove standalone JSON objects that look like tool calls
        if let Ok(regex) = Regex::new(r#"(?s)^\s*\{[^{}]*"name"[^{}]*"arguments"[^{}]*\}\s*$"#) {
            cleaned = regex.replace_all(&cleaned, "").to_string();
        }

        cleaned.trim().to_string()
    }

    /// Attempt to repair common JSON formatting issues
    fn attempt_json_repair(text: &str) -> String {
        let trimmed = text.trim();

        // If it doesn't start with {, return as-is
        if !trimmed.starts_with('{') {
            return trimmed.to_string();
        }

        // Count braces to see if we're missing closing braces
        let (open_braces, close_braces) = Self::count_json_braces(trimmed);

        // If we have more open braces than close braces, add missing closing braces
        if open_braces > close_braces {
            Self::add_missing_braces(trimmed, open_braces - close_braces)
        } else {
            // Return original if no obvious repair needed
            trimmed.to_string()
        }
    }

    /// Count open and close braces in JSON, respecting string contexts
    fn count_json_braces(text: &str) -> (usize, usize) {
        let mut open_braces = 0;
        let mut close_braces = 0;
        let mut in_string = false;
        let mut escaped = false;

        for ch in text.chars() {
            match ch {
                '"' if !escaped => in_string = !in_string,
                '\\' if in_string => escaped = !escaped,
                '{' if !in_string => open_braces += 1,
                '}' if !in_string => close_braces += 1,
                _ => escaped = false,
            }

            if ch != '\\' {
                escaped = false;
            }
        }

        (open_braces, close_braces)
    }

    /// Add missing closing braces to JSON text
    fn add_missing_braces(text: &str, missing_count: usize) -> String {
        let mut repaired = text.to_string();
        for _ in 0..missing_count {
            repaired.push('}');
        }

        tracing::debug!(
            original_length = text.len(),
            repaired_length = repaired.len(),
            added_braces = missing_count,
            "Repaired JSON by adding missing closing braces"
        );

        repaired
    }

    /// Extract balanced JSON from text, handling nested braces properly
    fn extract_balanced_json(text: &str) -> Option<(String, usize)> {
        let trimmed = text.trim_start();
        if !trimmed.starts_with('{') {
            return None;
        }

        let chars: Vec<char> = trimmed.chars().collect();
        let json_end = Self::find_balanced_json_end(&chars)?;

        let json_chars: String = chars[0..=json_end].iter().collect();
        let json_byte_len = json_chars.len();
        let offset = text.len() - trimmed.len(); // Account for leading whitespace
        Some((json_chars, offset + json_byte_len))
    }

    /// Find the index where balanced JSON ends
    fn find_balanced_json_end(chars: &[char]) -> Option<usize> {
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escaped = false;

        for (char_idx, ch) in chars.iter().enumerate() {
            match ch {
                '"' if !escaped => in_string = !in_string,
                '\\' if in_string => escaped = !escaped,
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        return Some(char_idx);
                    }
                }
                _ => escaped = false,
            }

            if *ch != '\\' {
                escaped = false;
            }
        }

        None // Unbalanced braces
    }
}

/// HTTP client functionality for OpenAI-compatible providers
pub mod http {
    use super::*;

    /// Shared HTTP client for OpenAI-compatible providers
    #[derive(Debug)]
    pub struct OpenAICompatibleClient {
        client: reqwest::Client,
        retry_executor: Mutex<RetryExecutor>,
    }

    impl Default for OpenAICompatibleClient {
        fn default() -> Self {
            Self::new()
        }
    }

    impl OpenAICompatibleClient {
        /// Create a new OpenAI-compatible HTTP client
        pub fn new() -> Self {
            Self {
                client: reqwest::Client::new(),
                retry_executor: Mutex::new(RetryExecutor::new(RetryPolicy::default())),
            }
        }

        /// Create a new OpenAI-compatible HTTP client with custom retry policy
        pub fn with_retry_policy(retry_policy: RetryPolicy) -> Self {
            Self {
                client: reqwest::Client::new(),
                retry_executor: Mutex::new(RetryExecutor::new(retry_policy)),
            }
        }

        /// Execute a chat completion request with retry logic
        pub async fn execute_chat_request(
            &self,
            url: &str,
            headers: &HeaderMap,
            request: &OpenAIRequest,
        ) -> LlmResult<OpenAIResponse> {
            // log_debug!(
            //     url = %url,
            //     model = %request.model,
            //     message_count = request.messages.len(),
            //     "Sending OpenAI-compatible request with retry logic"
            // );

            let mut retry_executor = self.retry_executor.lock().await;
            retry_executor
                .execute(|| self.execute_single_request(url, headers, request))
                .await
        }

        /// Execute authentication header for OpenAI-compatible APIs
        pub fn build_auth_headers(api_key: &str) -> LlmResult<HeaderMap> {
            let mut headers = HeaderMap::new();

            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {api_key}")).map_err(|e| {
                    LlmError::configuration_error(format!("Invalid API key format: {e}"))
                })?,
            );

            Ok(headers)
        }

        /// Execute a single HTTP request
        async fn execute_single_request(
            &self,
            url: &str,
            headers: &HeaderMap,
            request: &OpenAIRequest,
        ) -> LlmResult<OpenAIResponse> {
            let response = self
                .client
                .post(url)
                .headers(headers.clone())
                .json(request)
                .send()
                .await
                .map_err(|e| {
                    log_error!(
                        url = %url,
                        error = %e,
                        "HTTP request failed"
                    );
                    LlmError::request_failed(format!("Request failed: {e}"), Some(Box::new(e)))
                })?;

            if !response.status().is_success() {
                return Err(handle_error_response(response).await);
            }

            parse_success_response(response).await
        }

        /// Set retry policy for subsequent requests
        pub async fn set_retry_policy(&self, policy: RetryPolicy) {
            let mut retry_executor = self.retry_executor.lock().await;
            *retry_executor = RetryExecutor::new(policy);
        }

        /// Restore default retry policy
        pub async fn restore_default_retry_policy(&self, default_policy: &RetryPolicy) {
            let mut retry_executor = self.retry_executor.lock().await;
            *retry_executor = RetryExecutor::new(default_policy.clone());
        }
    }

    /// Handle non-success HTTP responses
    async fn handle_error_response(response: reqwest::Response) -> LlmError {
        let status = response.status();
        let headers = response.headers().clone();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        log_error!(
            status = %status,
            error_text = %error_text,
            "API error response"
        );

        match status.as_u16() {
            401 => {
                // Parse error details for authentication failures
                if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&error_text) {
                    if let Some(error_obj) = error_json.get("error") {
                        if let Some(code) = error_obj.get("code").and_then(|c| c.as_str()) {
                            if code.contains("api_key") || code.contains("auth") {
                                return LlmError::authentication_failed(
                                    "Invalid API key or authentication failed",
                                );
                            }
                        }
                    }
                }
                LlmError::authentication_failed("Authentication failed")
            }
            429 => {
                let retry_after_seconds = headers
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(60);

                LlmError::rate_limit_exceeded(retry_after_seconds)
            }
            _ => LlmError::request_failed(format!("API error {status}: {error_text}"), None),
        }
    }

    /// Parse successful HTTP response into OpenAIResponse
    async fn parse_success_response(response: reqwest::Response) -> LlmResult<OpenAIResponse> {
        let raw_body = response.text().await.map_err(|e| {
            log_error!(
                error = %e,
                "Failed to read response body"
            );
            LlmError::response_parsing_error(format!("Failed to read response: {e}"))
        })?;

        serde_json::from_str(&raw_body).map_err(|e| {
            log_error!(
                error = %e,
                raw_body = %raw_body,
                "Failed to parse response"
            );
            LlmError::response_parsing_error(format!("Invalid response: {e}"))
        })
    }
}

/// Convert neutral messages to OpenAI format
pub fn convert_neutral_messages_to_openai(messages: &[UnifiedMessage]) -> Vec<OpenAIMessage> {
    messages
        .iter()
        .map(|msg| {
            let role = match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool",
            };

            match &msg.content {
                MessageContent::Text(text) => OpenAIMessage {
                    role: role.to_string(),
                    content: text.clone(),
                },
                MessageContent::Json(json_value) => OpenAIMessage {
                    role: role.to_string(),
                    content: serde_json::to_string_pretty(json_value).unwrap_or_default(),
                },
                MessageContent::ToolCall {
                    id: _,
                    name,
                    arguments,
                } => {
                    // We shouldn't be sending tool calls TO the LLM, tool calls come FROM the LLM
                    // This is likely an error, but convert to text for compatibility
                    OpenAIMessage {
                        role: role.to_string(),
                        content: format!(
                            "Tool call: {} with args: {}",
                            name,
                            serde_json::to_string(arguments).unwrap_or_default()
                        ),
                    }
                }
                MessageContent::ToolResult {
                    tool_call_id: _,
                    content,
                    is_error,
                } => {
                    let prefix = if *is_error {
                        "Tool error"
                    } else {
                        "Tool result"
                    };
                    OpenAIMessage {
                        role: role.to_string(),
                        content: format!("{}: {}", prefix, content),
                    }
                }
            }
        })
        .collect()
}

/// Convert neutral tools to OpenAI format
pub fn convert_neutral_tools_to_openai(tools: &[ExecutorTool]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters
                }
            })
        })
        .collect()
}

/// Apply LLM config to OpenAI request
pub fn apply_config_to_request(request: &mut OpenAIRequest, config: Option<ExecutorLLMConfig>) {
    if let Some(cfg) = config {
        apply_llm_parameters(request, &cfg);
        apply_tools_if_user_llm(request, &cfg);
        apply_tool_choice(request, cfg.tool_choice);
        apply_response_format(request, cfg.response_format);
    }
}

fn apply_llm_parameters(request: &mut OpenAIRequest, cfg: &ExecutorLLMConfig) {
    if let Some(temp) = cfg.temperature {
        request.temperature = Some(temp);
    }
    if let Some(max_tokens) = cfg.max_tokens {
        request.max_tokens = Some(max_tokens);
    }
    if let Some(top_p) = cfg.top_p {
        request.top_p = Some(top_p);
    }
    if let Some(presence_penalty) = cfg.presence_penalty {
        request.presence_penalty = Some(presence_penalty);
    }
}

fn apply_tools_if_user_llm(request: &mut OpenAIRequest, cfg: &ExecutorLLMConfig) {
    if cfg.tools.is_empty() {
        return;
    }

    let is_user_llm = cfg
        .llm_path
        .as_ref()
        .map(|path| path == "user_llm")
        .unwrap_or(true);

    if is_user_llm {
        let openai_tools = convert_neutral_tools_to_openai(&cfg.tools);
        request.tools = Some(openai_tools);
    }
}

fn apply_tool_choice(
    request: &mut OpenAIRequest,
    tool_choice: Option<crate::core_types::executor::ToolChoice>,
) {
    if let Some(choice) = tool_choice {
        request.tool_choice = Some(match choice {
            ToolChoice::Auto => "auto".to_string(),
            ToolChoice::None => "none".to_string(),
            ToolChoice::Required => "required".to_string(),
            ToolChoice::Specific(tool_name) => tool_name,
        });
    }
}

fn apply_response_format(
    request: &mut OpenAIRequest,
    response_format: Option<crate::core_types::executor::ExecutorResponseFormat>,
) {
    if let Some(format) = response_format {
        request.response_format = Some(OpenAIResponseFormat {
            format_type: "json_schema".to_string(),
            json_schema: Some(OpenAIJsonSchema {
                name: format.name,
                schema: format.schema,
                strict: Some(true),
            }),
        });
    }
}

/// Convert OpenAI tool calls to LLM tool calls
pub fn convert_tool_calls(openai_calls: &[OpenAIToolCall]) -> Vec<ExecutorToolCall> {
    openai_calls
        .iter()
        .map(|call| ExecutorToolCall {
            id: call.id.clone(),
            name: call.function.name.clone(),
            arguments: serde_json::from_str(&call.function.arguments)
                .unwrap_or_else(|_| serde_json::json!({})),
        })
        .collect()
}

/// Fast token estimation for logging and diagnostics
/// Uses simple chars/4 approximation - sufficient for monitoring and diagnostics.
/// Actual token usage is tracked from LLM provider responses.
pub fn estimate_tokens(text: &str) -> u32 {
    // Simple approximation: ~4 characters per token on average
    // This is fast and sufficient for logging/monitoring purposes.
    // Actual token counts come from provider responses.
    (text.len() / 4) as u32
}

/// Fast token estimation for message arrays with formatting overhead
/// Uses simple approximation - sufficient for logging and diagnostics.
pub fn estimate_message_tokens(messages: &[OpenAIMessage]) -> u32 {
    // Simple approach: combine all content and add formatting overhead
    let total_text: String = messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    // Add overhead for message formatting (role markers, etc.)
    estimate_tokens(&total_text) + (messages.len() as u32 * 8)
}

/// Result of processing tool calls and content cleaning
#[derive(Debug)]
pub struct ToolCallProcessingResult {
    pub tool_calls: Vec<ExecutorToolCall>,
    pub cleaned_content: Option<String>,
}

/// Handle tool calls from OpenAI response message, including custom formats
/// This function integrates standard tool call parsing with custom format detection
pub fn handle_tool_calls(
    message: &OpenAIResponseMessage,
) -> crate::error::LlmResult<Vec<ExecutorToolCall>> {
    let result = handle_tool_calls_with_content_cleaning(message)?;
    Ok(result.tool_calls)
}

/// Process standard OpenAI tool calls
fn process_standard_tool_calls(tool_calls: &[OpenAIToolCall]) -> Option<ToolCallProcessingResult> {
    if tool_calls.is_empty() {
        return None;
    }

    Some(ToolCallProcessingResult {
        tool_calls: convert_tool_calls(tool_calls),
        cleaned_content: None,
    })
}

/// Create tool call from custom format match result
fn create_custom_tool_call(match_result: CustomToolCallMatch) -> ToolCallProcessingResult {
    let tool_call = ExecutorToolCall {
        id: format!("custom_{}", uuid::Uuid::new_v4()),
        name: match_result.function_name,
        arguments: match_result.arguments,
    };

    ToolCallProcessingResult {
        tool_calls: vec![tool_call],
        cleaned_content: Some(match_result.cleaned_content),
    }
}

/// Handle parsing error by attempting content cleaning
fn handle_parsing_error(
    content: &str,
    error: &CustomFormatError,
) -> Option<ToolCallProcessingResult> {
    tracing::warn!(
        error = ?error,
        content_preview = content.chars().take(100).collect::<String>(),
        "Failed to parse custom tool format - attempting content cleaning"
    );

    let cleaned_content = CustomFormatParser::clean_tool_call_patterns(content);
    if cleaned_content == content {
        return None;
    }

    tracing::debug!(
        original_length = content.len(),
        cleaned_length = cleaned_content.len(),
        "Cleaned tool call patterns from failed parse"
    );

    let final_content = if cleaned_content.trim().is_empty() {
        "I attempted to process your request, but encountered a formatting issue. Please try rephrasing your request.".to_string()
    } else {
        cleaned_content
    };

    Some(ToolCallProcessingResult {
        tool_calls: vec![],
        cleaned_content: Some(final_content),
    })
}

/// Try to parse custom format from content
fn try_parse_custom_format(
    content: &str,
) -> Result<Option<ToolCallProcessingResult>, CustomFormatError> {
    let parser = CustomFormatParser::new();

    match parser.parse(content)? {
        Some(match_result) => Ok(Some(create_custom_tool_call(match_result))),
        None => Ok(None),
    }
}

/// Handle tool calls and return both tool calls and cleaned content
/// This function integrates standard tool call parsing with custom format detection
/// and provides cleaned content when custom formats are detected
pub fn handle_tool_calls_with_content_cleaning(
    message: &OpenAIResponseMessage,
) -> crate::error::LlmResult<ToolCallProcessingResult> {
    // Check for standard tool calls first
    if let Some(result) = check_standard_tool_calls(message) {
        return Ok(result);
    }

    // Try custom format parsing if content is present
    if let Some(result) = try_custom_format_parsing(&message.content)? {
        return Ok(result);
    }

    Ok(ToolCallProcessingResult {
        tool_calls: vec![],
        cleaned_content: None,
    })
}

/// Check for standard OpenAI tool calls
fn check_standard_tool_calls(message: &OpenAIResponseMessage) -> Option<ToolCallProcessingResult> {
    let tool_calls = message.tool_calls.as_ref()?;
    process_standard_tool_calls(tool_calls)
}

/// Try parsing custom tool call formats
fn try_custom_format_parsing(
    content: &str,
) -> crate::error::LlmResult<Option<ToolCallProcessingResult>> {
    if content.is_empty() {
        return Ok(None);
    }

    match try_parse_custom_format(content) {
        Ok(Some(result)) => Ok(Some(result)),
        Ok(None) => Ok(None), // No custom format found - normal case
        Err(e) => Ok(handle_parsing_error(content, &e)),
    }
}
