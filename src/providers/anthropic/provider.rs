//! Anthropic provider implementation

use super::caching;
use super::conversion;
use super::types::{AnthropicContentBlock, AnthropicRequest, AnthropicResponse, SystemField};
use crate::{log_debug, log_error, log_warn};
use crate::config::{AnthropicConfig, DefaultLLMParams};
use crate::error::{LlmError, LlmResult};
use crate::retry::RetryExecutor;
use crate::core_types::executor::{
    ExecutorLLMConfig, ExecutorLLMProvider, ExecutorLLMResponse, ExecutorResponseFormat,
    LLMBusinessEvent, ToolCallingRound,
};
use crate::core_types::events::{BusinessEvent, EventScope};
use crate::core_types::messages::UnifiedLLMRequest;

use crate::core_types::event_types;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use std::time::Instant;
use tokio::sync::Mutex;

/// Anthropic Claude provider implementation
#[derive(Debug)]
pub struct AnthropicProvider {
    client: reqwest::Client,
    retry_executor: Mutex<RetryExecutor>,
    config: AnthropicConfig,
    default_params: DefaultLLMParams,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider instance
    ///
    /// # Errors
    ///
    /// Returns [`LlmError::ConfigurationError`] if:
    /// - API key is missing or invalid
    /// - Provider configuration validation fails
    /// - HTTP client initialization fails
    pub fn new(config: AnthropicConfig, default_params: DefaultLLMParams) -> LlmResult<Self> {
        if config.api_key.is_none() {
            return Err(LlmError::configuration_error(
                "Anthropic API key is required",
            ));
        }

        log_debug!(
            provider = "anthropic",
            max_context_tokens = config.max_context_tokens,
            "Anthropic provider initialized"
        );

        Ok(Self {
            client: reqwest::Client::new(),
            retry_executor: Mutex::new(RetryExecutor::new(config.retry_policy.clone())),
            config,
            default_params,
        })
    }

    /// Sends a request to the Anthropic Messages API with retry logic
    async fn send_anthropic_request(
        &self,
        request: AnthropicRequest,
    ) -> LlmResult<AnthropicResponse> {
        let url = format!("{}/v1/messages", self.config.base_url);
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| LlmError::configuration_error("Anthropic API key is required"))?;

        // Build headers required by Anthropic API
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(api_key).map_err(|e| {
                LlmError::configuration_error(format!("Invalid API key format: {e}"))
            })?,
        );
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

        let mut retry_executor = self.retry_executor.lock().await;
        retry_executor
            .execute(|| self.execute_single_anthropic_request(&url, &headers, &request))
            .await
    }

    /// Execute a single HTTP request to Anthropic API
    async fn execute_single_anthropic_request(
        &self,
        url: &str,
        headers: &HeaderMap,
        request: &AnthropicRequest,
    ) -> LlmResult<AnthropicResponse> {
        let response = self
            .client
            .post(url)
            .headers(headers.clone())
            .json(request)
            .send()
            .await
            .map_err(|e| {
                log_error!(
                    provider = "anthropic",
                    url = %url,
                    error = %e,
                    "HTTP request failed"
                );
                LlmError::request_failed(
                    format!("Anthropic request failed: {e}"),
                    Some(Box::new(e)),
                )
            })?;

        if !response.status().is_success() {
            return Err(self.handle_anthropic_error_response(response).await);
        }

        self.parse_anthropic_success_response(response).await
    }

    /// Check if error JSON indicates auth failure
    fn is_auth_error(error_json: &serde_json::Value) -> bool {
        error_json
            .get("error")
            .and_then(|obj| obj.get("type"))
            .and_then(|t| t.as_str())
            .map(|error_type| {
                error_type.contains("authentication") || error_type.contains("invalid_api_key")
            })
            .unwrap_or(false)
    }

    /// Parse authentication error from response text
    fn parse_auth_error(error_text: &str) -> LlmError {
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(error_text) {
            if Self::is_auth_error(&error_json) {
                return LlmError::authentication_failed(
                    "Invalid Anthropic API key or authentication failed",
                );
            }
        }
        LlmError::authentication_failed("Anthropic authentication failed")
    }

    /// Extract retry-after value from headers
    fn extract_retry_after(headers: &reqwest::header::HeaderMap) -> u64 {
        headers
            .get("retry-after")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60)
    }

    /// Handle non-success HTTP responses from Anthropic API
    async fn handle_anthropic_error_response(&self, response: reqwest::Response) -> LlmError {
        let status = response.status();
        let headers = response.headers().clone();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        log_error!(
            provider = "anthropic",
            status = %status,
            error_text = %error_text,
            "Anthropic API error"
        );

        match status.as_u16() {
            401 => Self::parse_auth_error(&error_text),
            429 => {
                let retry_after_seconds = Self::extract_retry_after(&headers);
                LlmError::rate_limit_exceeded(retry_after_seconds)
            }
            _ => LlmError::request_failed(
                format!("Anthropic API error {status}: {error_text}"),
                None,
            ),
        }
    }

    /// Parse successful HTTP response from Anthropic API
    async fn parse_anthropic_success_response(
        &self,
        response: reqwest::Response,
    ) -> LlmResult<AnthropicResponse> {
        let raw_body = response.text().await.map_err(|e| {
            log_error!(
                provider = "anthropic",
                error = %e,
                "Failed to read Anthropic response body"
            );
            LlmError::response_parsing_error(format!("Failed to read response: {e}"))
        })?;

        let api_response: AnthropicResponse = serde_json::from_str(&raw_body).map_err(|e| {
            log_error!(
                provider = "anthropic",
                error = %e,
                raw_body = %raw_body,
                "Failed to parse Anthropic response"
            );
            LlmError::response_parsing_error(format!("Invalid Anthropic response: {e}"))
        })?;

        // Debug log the full network response JSON
        log_debug!(
            provider = "anthropic",
            response_json = %raw_body,
            "Network response JSON"
        );

        Ok(api_response)
    }

    /// Extract balanced JSON from content, handling trailing text
    /// Returns (json_string, trailing_text_option)
    #[allow(dead_code)]
    fn extract_json_from_content(content: &str) -> (Option<String>, Option<String>) {
        let trimmed = content.trim();
        if !trimmed.starts_with('{') {
            return (None, None);
        }

        let chars: Vec<char> = trimmed.chars().collect();
        Self::find_json_end(&chars)
            .map(|end_idx| Self::split_json_and_trailing(&chars, end_idx))
            .unwrap_or((None, None))
    }

    fn find_json_end(chars: &[char]) -> Option<usize> {
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escaped = false;

        for (idx, ch) in chars.iter().enumerate() {
            match ch {
                '"' if !escaped => in_string = !in_string,
                '\\' if in_string => escaped = !escaped,
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        return Some(idx);
                    }
                }
                _ => escaped = false,
            }

            if *ch != '\\' {
                escaped = false;
            }
        }

        None
    }

    fn split_json_and_trailing(chars: &[char], end_idx: usize) -> (Option<String>, Option<String>) {
        let json_str: String = chars[0..=end_idx].iter().collect();
        let trailing = Self::extract_trailing_text(chars, end_idx);
        (Some(json_str), trailing)
    }

    fn extract_trailing_text(chars: &[char], end_idx: usize) -> Option<String> {
        if end_idx + 1 >= chars.len() {
            return None;
        }

        let remaining: String = chars[end_idx + 1..].iter().collect();
        let trimmed = remaining.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    /// Internal method for executor pattern - restore default retry policy
    pub(crate) async fn restore_default_retry_policy(&self) {
        // Anthropic provider doesn't need explicit retry policy restoration
        // The client manages retry state internally
    }

    /// Convert AnthropicResponse to ExecutorLLMResponse
    fn convert_anthropic_to_executor_response(
        &self,
        api_response: AnthropicResponse,
        response_format: Option<ExecutorResponseFormat>,
        config: Option<ExecutorLLMConfig>,
    ) -> crate::core_types::Result<ExecutorLLMResponse> {
        let (content, tool_calls) = self.extract_content_and_tools(&api_response);
        let usage = self.create_usage_stats(&api_response, config.as_ref());
        let (final_content, structured_response) =
            self.parse_structured_response(content, &tool_calls, response_format);

        Ok(crate::core_types::executor::ExecutorLLMResponse {
            content: final_content,
            structured_response,
            tool_calls,
            usage: Some(usage),
            model: Some(api_response.model),
            raw_body: None,
        })
    }

    fn extract_content_and_tools(
        &self,
        api_response: &AnthropicResponse,
    ) -> (String, Vec<crate::core_types::executor::ExecutorToolCall>) {
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        for content_block in &api_response.content {
            match content_block {
                AnthropicContentBlock::Text { text, .. } => {
                    if !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(text);
                }
                AnthropicContentBlock::ToolUse { id, name, input } => {
                    tool_calls.push(crate::core_types::executor::ExecutorToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        arguments: input.clone(),
                    });
                }
                AnthropicContentBlock::ToolResult { .. } => {}
            }
        }

        (content, tool_calls)
    }

    fn create_usage_stats(
        &self,
        api_response: &AnthropicResponse,
        config: Option<&ExecutorLLMConfig>,
    ) -> crate::core_types::executor::ExecutorTokenUsage {
        let cache_creation_tokens = api_response.usage.cache_creation_input_tokens.unwrap_or(0);
        let cache_read_tokens = api_response.usage.cache_read_input_tokens.unwrap_or(0);
        let total_tokens_with_cache = api_response.usage.input_tokens
            + api_response.usage.output_tokens
            + cache_creation_tokens
            + cache_read_tokens;

        if self.config.enable_prompt_caching {
            self.log_cache_usage(api_response, config);
        }

        crate::core_types::executor::ExecutorTokenUsage {
            prompt_tokens: api_response.usage.input_tokens,
            completion_tokens: api_response.usage.output_tokens,
            total_tokens: total_tokens_with_cache,
        }
    }

    fn log_cache_usage(
        &self,
        api_response: &AnthropicResponse,
        config: Option<&ExecutorLLMConfig>,
    ) {
        let provider_with_path = config
            .and_then(|cfg| cfg.llm_path.as_ref().map(|p| format!("{}:anthropic", p)))
            .unwrap_or_else(|| "anthropic".to_string());

        let cache_read = api_response.usage.cache_read_input_tokens.unwrap_or(0);
        let total_input = api_response.usage.input_tokens + cache_read;
        let cache_hit_rate = if total_input > 0 {
            cache_read as f64 / total_input as f64
        } else {
            0.0
        };

        log_debug!(
            provider = %provider_with_path,
            user_id = config.and_then(|c| c.user_id.as_deref()).unwrap_or("unknown"),
            session_id = config.and_then(|c| c.session_id.as_deref()).unwrap_or("unknown"),
            cache_usage = ?serde_json::json!({
                "input_tokens": api_response.usage.input_tokens,
                "output_tokens": api_response.usage.output_tokens,
                "cache_creation_input_tokens": api_response.usage.cache_creation_input_tokens.unwrap_or(0),
                "cache_read_input_tokens": cache_read,
                "cache_creation_5m": api_response.usage.cache_creation.as_ref()
                    .and_then(|c| c.ephemeral_5m_input_tokens).unwrap_or(0),
                "cache_creation_1h": api_response.usage.cache_creation.as_ref()
                    .and_then(|c| c.ephemeral_1h_input_tokens).unwrap_or(0),
                "cache_hit_rate": cache_hit_rate,
                "ttl_setting": self.config.cache_ttl
            }),
            "Anthropic cache usage statistics"
        );
    }

    fn parse_structured_response(
        &self,
        content: String,
        tool_calls: &[crate::core_types::executor::ExecutorToolCall],
        response_format: Option<ExecutorResponseFormat>,
    ) -> (String, Option<serde_json::Value>) {
        if response_format.is_none() {
            return (content, None);
        }

        // Try to extract from tool calls first
        if let Some(tool_data) = tool_calls
            .iter()
            .find(|tc| tc.name == "structured_response")
        {
            log_debug!(
                provider = "anthropic",
                "Successfully extracted structured response from tool_use"
            );
            return (content.clone(), Some(tool_data.arguments.clone()));
        }

        // Fallback: parse content as JSON
        self.parse_json_content(content)
    }

    fn parse_json_content(&self, content: String) -> (String, Option<serde_json::Value>) {
        let json_content = if content.trim_start().starts_with('{') {
            content.clone()
        } else {
            format!("{{{}", content)
        };

        match serde_json::from_str::<serde_json::Value>(&json_content) {
            Ok(json_value) => {
                log_debug!(
                    provider = "anthropic",
                    "Successfully parsed structured JSON response from content"
                );
                (json_content, Some(json_value))
            }
            Err(e) => {
                log_warn!(
                    provider = "anthropic",
                    content_preview = &json_content[..json_content.len().min(200)],
                    error = %e,
                    "Failed to parse structured JSON response from content"
                );
                (content, None)
            }
        }
    }

    /// Apply executor config to Anthropic request
    fn apply_executor_config(
        &self,
        request: &mut AnthropicRequest,
        config: ExecutorLLMConfig,
        enable_caching: bool,
    ) -> Option<ExecutorResponseFormat> {
        // Apply LLM parameters
        // Anthropic doesn't allow both temperature and top_p - enforce mutual exclusivity
        if let Some(temp) = config.temperature {
            request.temperature = Some(temp as f32);
            request.top_p = None; // Clear top_p if temperature is set
        } else if let Some(top_p) = config.top_p {
            // Only set top_p if temperature wasn't provided
            request.top_p = Some(top_p as f32);
            request.temperature = None; // Clear temperature if top_p is set
        }
        if let Some(max_tokens) = config.max_tokens {
            request.max_tokens = max_tokens;
        }
        if let Some(top_k) = config.top_k {
            request.top_k = Some(top_k);
        }

        // Convert tools - only add user tools for User LLM path
        if !config.tools.is_empty() {
            // Check if this is a user LLM request based on llm_path
            let is_user_llm = config
                .llm_path
                .as_ref()
                .map(|path| path == "user_llm")
                .unwrap_or(true); // Default to user LLM for backwards compatibility

            if is_user_llm {
                request.tools = Some(caching::convert_executor_tools_to_anthropic(
                    &config.tools,
                    enable_caching,
                    &self.config.cache_ttl,
                ));
            }
            // For non-user LLM paths (story_analysis, nlp_llm), skip user tools
            // They will only get structured_response tool if response_schema is present
        }

        // Return response format for structured parsing
        config.response_format
    }

    /// Create base Anthropic request from unified request
    fn create_base_request(
        &self,
        request: &UnifiedLLMRequest,
        enable_caching: bool,
    ) -> AnthropicRequest {
        let sorted_messages = request.get_sorted_messages();
        let (system_messages, conversation_messages) =
            conversion::transform_unified_messages(&sorted_messages, &self.config, enable_caching);

        AnthropicRequest {
            model: self.config.default_model.clone(),
            max_tokens: self.default_params.max_tokens,
            system: if system_messages.is_empty() {
                None
            } else {
                Some(SystemField::Messages(system_messages))
            },
            messages: conversation_messages,
            temperature: Some(self.default_params.temperature as f32),
            top_p: None,
            top_k: Some(self.default_params.top_k),
            stop_sequences: None,
            tools: None,
        }
    }

    /// Determine if caching should be enabled for this request
    fn should_enable_caching(&self, config: Option<&ExecutorLLMConfig>) -> bool {
        self.config.enable_prompt_caching
            && config
                .and_then(|c| c.llm_path.as_ref())
                .map(|path| path != "nlp_llm")
                .unwrap_or(true)
    }

    /// Apply response schema to request if present
    fn apply_response_schema(
        &self,
        request: &mut AnthropicRequest,
        schema: Option<serde_json::Value>,
    ) {
        if let Some(schema) = schema {
            request.tools = Some(vec![serde_json::json!({
                "name": "structured_response",
                "description": "Provide a structured response matching the required schema",
                "input_schema": schema
            })]);
        }
    }

    /// Create LLM request business event
    fn create_request_event(
        &self,
        anthropic_request: &AnthropicRequest,
        config: Option<&ExecutorLLMConfig>,
    ) -> Option<LLMBusinessEvent> {
        let user_id = config.and_then(|c| c.user_id.clone())?;

        let mut event = BusinessEvent::new(event_types::LLM_REQUEST)
            .with_metadata("provider", "anthropic")
            .with_metadata("model", &anthropic_request.model);

        if let Some(cfg) = config {
            if let Some(ref path) = cfg.llm_path {
                event = event.with_metadata("llm_path", path);
            }
        }

        Some(LLMBusinessEvent {
            event,
            scope: EventScope::User(user_id),
        })
    }

    /// Create LLM error business event
    fn create_error_event(
        &self,
        error: &LlmError,
        config: Option<&ExecutorLLMConfig>,
    ) -> Option<LLMBusinessEvent> {
        let user_id = config.and_then(|c| c.user_id.clone())?;

        let event = BusinessEvent::new(event_types::LLM_ERROR)
            .with_metadata("provider", "anthropic")
            .with_metadata("error", error.to_string());

        Some(LLMBusinessEvent {
            event,
            scope: EventScope::User(user_id),
        })
    }

    /// Create LLM response business event
    fn create_response_event(
        &self,
        api_response: &AnthropicResponse,
        duration_ms: u64,
        config: Option<&ExecutorLLMConfig>,
    ) -> Option<LLMBusinessEvent> {
        let user_id = config.and_then(|c| c.user_id.clone())?;

        let mut event = BusinessEvent::new(event_types::LLM_RESPONSE)
            .with_metadata("provider", "anthropic")
            .with_metadata("model", &api_response.model)
            .with_metadata("input_tokens", api_response.usage.input_tokens)
            .with_metadata("output_tokens", api_response.usage.output_tokens)
            .with_metadata("duration_ms", duration_ms);

        if let Some(cache_write) = api_response.usage.cache_creation_input_tokens {
            event = event.with_metadata("cache_creation_tokens", cache_write);
        }
        if let Some(cache_read) = api_response.usage.cache_read_input_tokens {
            event = event.with_metadata("cache_read_tokens", cache_read);
        }

        if let Some(cfg) = config {
            if let Some(ref sess_id) = cfg.session_id {
                event = event.with_metadata("session_id", sess_id);
            }
            if let Some(ref path) = cfg.llm_path {
                event = event.with_metadata("llm_path", path);
            }
        }

        Some(LLMBusinessEvent {
            event,
            scope: EventScope::User(user_id),
        })
    }
}

#[async_trait::async_trait]
impl ExecutorLLMProvider for AnthropicProvider {
    async fn execute_llm(
        &self,
        request: UnifiedLLMRequest,
        _current_tool_round: Option<ToolCallingRound>,
        config: Option<ExecutorLLMConfig>,
    ) -> crate::core_types::Result<(ExecutorLLMResponse, Vec<LLMBusinessEvent>)> {
        let mut events = Vec::new();

        // Determine caching and create base request
        let enable_caching = self.should_enable_caching(config.as_ref());
        let mut anthropic_request = self.create_base_request(&request, enable_caching);

        // Apply executor config if provided
        let response_format = if let Some(ref cfg) = config {
            self.apply_executor_config(&mut anthropic_request, cfg.clone(), enable_caching)
        } else {
            None
        };

        // Apply response schema from request if present
        self.apply_response_schema(&mut anthropic_request, request.response_schema);

        // Debug log the full network request JSON
        log_debug!(
            provider = "anthropic",
            request_json = %serde_json::to_string(&anthropic_request).unwrap_or_default(),
            "Network request JSON"
        );

        // Log LLM request event
        if let Some(event) = self.create_request_event(&anthropic_request, config.as_ref()) {
            events.push(event);
        }

        // Send to Anthropic API
        let start_time = Instant::now();
        let api_response = match self.send_anthropic_request(anthropic_request).await {
            Ok(response) => response,
            Err(e) => {
                if let Some(event) = self.create_error_event(&e, config.as_ref()) {
                    events.push(event);
                }
                return Err(anyhow::anyhow!("Anthropic API error: {}", e));
            }
        };
        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Log LLM response event
        if let Some(event) = self.create_response_event(&api_response, duration_ms, config.as_ref())
        {
            events.push(event);
        }

        // Convert Anthropic response to ExecutorLLMResponse
        let response =
            self.convert_anthropic_to_executor_response(api_response, response_format, config)?;

        Ok((response, events))
    }

    async fn execute_structured_llm(
        &self,
        mut request: UnifiedLLMRequest,
        current_tool_round: Option<ToolCallingRound>,
        schema: serde_json::Value,
        config: Option<ExecutorLLMConfig>,
    ) -> crate::core_types::Result<(ExecutorLLMResponse, Vec<LLMBusinessEvent>)> {
        // Set the schema in the request
        request.response_schema = Some(schema);

        // Execute with the schema-enabled request (returns tuple with events)
        self.execute_llm(request, current_tool_round, config).await
    }

    fn provider_name(&self) -> &'static str {
        "anthropic"
    }
}
