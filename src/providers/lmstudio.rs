//! LM Studio provider implementation
//!
//! LM Studio uses OpenAI-compatible API, so this implementation leverages
//! the shared OpenAI structures and utilities.

use super::openai_shared::{
    http::OpenAICompatibleClient, utils::apply_config_to_request, OpenAIRequest, OpenAIResponse,
};
// Removed LLMClientCore import - providers now implement their own methods directly
use crate::config::{DefaultLLMParams, LMStudioConfig};
use crate::error::{LlmError, LlmResult};
#[cfg(feature = "events")]
use crate::internals::events::{event_types, BusinessEvent, EventScope};
use crate::internals::response_parser::ResponseParser;
use crate::logging::log_debug;
use crate::messages::{MessageContent, MessageRole, UnifiedLLMRequest, UnifiedMessage};
#[cfg(feature = "events")]
use crate::provider::LLMBusinessEvent;
use crate::provider::{LlmProvider, RequestConfig, Response, TokenUsage, ToolCallingRound};
use std::time::Instant;

/// LM Studio local provider implementation
///
/// Uses OpenAI-compatible API endpoints for local model inference
#[derive(Debug)]
pub struct LMStudioProvider {
    http_client: OpenAICompatibleClient,
    config: LMStudioConfig,
    default_params: DefaultLLMParams,
}

impl LMStudioProvider {
    /// Create a new LM Studio provider instance
    ///
    /// # Errors
    ///
    /// Returns [`LlmError::ConfigurationError`] if:
    /// - Base URL is missing or invalid
    /// - Provider configuration validation fails
    /// - HTTP client initialization fails
    pub fn new(config: LMStudioConfig, default_params: DefaultLLMParams) -> LlmResult<Self> {
        // log_debug!(
        //     provider = "lmstudio",
        //     base_url = %config.base_url,
        //     max_context_tokens = config.max_context_tokens,
        //     default_model = %config.default_model,
        //     default_temperature = default_params.temperature,
        //     "Creating LM Studio provider"
        // );

        if config.base_url.is_empty() {
            return Err(LlmError::configuration_error(
                "LM Studio base URL is required",
            ));
        }

        log_debug!(
            provider = "lmstudio",
            base_url = %config.base_url,
            max_context_tokens = config.max_context_tokens,
            default_temperature = default_params.temperature,
            "LM Studio provider initialized"
        );

        Ok(Self {
            http_client: OpenAICompatibleClient::with_retry_policy(config.retry_policy.clone()),
            config,
            default_params,
        })
    }

    /// Create base OpenAI-compatible request
    fn create_base_request(&self, request: &UnifiedLLMRequest) -> OpenAIRequest {
        let openai_messages = self.transform_unified_messages(&request.get_sorted_messages());

        OpenAIRequest {
            model: self.config.default_model.clone(),
            messages: openai_messages,
            temperature: Some(self.default_params.temperature),
            max_tokens: Some(self.default_params.max_tokens),
            top_p: Some(self.default_params.top_p),
            stream: None,
            presence_penalty: None,
            tools: None,
            tool_choice: None,
            response_format: None,
        }
    }

    /// Send request to LM Studio API
    async fn send_lmstudio_request(&self, request: &OpenAIRequest) -> LlmResult<OpenAIResponse> {
        let url = format!("{}/v1/chat/completions", self.config.base_url);
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        self.http_client
            .execute_chat_request(&url, &headers, request)
            .await
            .map_err(|e| {
                LlmError::request_failed(format!("LM Studio API error: {}", e), Some(Box::new(e)))
            })
    }

    /// Apply response schema to request
    fn apply_response_schema(
        &self,
        request: &mut OpenAIRequest,
        schema: Option<serde_json::Value>,
    ) {
        if let Some(schema) = schema {
            request.response_format = Some(super::openai_shared::OpenAIResponseFormat {
                format_type: "json_schema".to_string(),
                json_schema: Some(super::openai_shared::OpenAIJsonSchema {
                    name: "structured_response".to_string(),
                    schema,
                    strict: Some(true),
                }),
            });
        }
    }

    /// Create LLM request business event
    #[cfg(feature = "events")]
    fn create_request_event(&self, model: &str, user_id: &str) -> LLMBusinessEvent {
        let event = BusinessEvent::new(event_types::LLM_REQUEST)
            .with_metadata("provider", "lmstudio")
            .with_metadata("model", model);

        LLMBusinessEvent {
            event,
            scope: EventScope::User(user_id.to_string()),
        }
    }

    /// Create LLM error business event
    #[cfg(feature = "events")]
    fn create_error_event(&self, error: &LlmError, user_id: &str) -> LLMBusinessEvent {
        let event = BusinessEvent::new(event_types::LLM_ERROR)
            .with_metadata("provider", "lmstudio")
            .with_metadata("error", error.to_string());

        LLMBusinessEvent {
            event,
            scope: EventScope::User(user_id.to_string()),
        }
    }

    /// Create LLM response business event
    #[cfg(feature = "events")]
    fn create_response_event(
        &self,
        response: &OpenAIResponse,
        duration_ms: u64,
        config: Option<&RequestConfig>,
    ) -> Option<LLMBusinessEvent> {
        let user_id = config.and_then(|c| c.user_id.as_ref())?;

        let usage_tokens = response
            .usage
            .as_ref()
            .map(|u| (u.prompt_tokens, u.completion_tokens));
        let mut event = BusinessEvent::new(event_types::LLM_RESPONSE)
            .with_metadata("provider", "lmstudio")
            .with_metadata("model", &self.config.default_model)
            .with_metadata("input_tokens", usage_tokens.map(|(i, _)| i).unwrap_or(0))
            .with_metadata("output_tokens", usage_tokens.map(|(_, o)| o).unwrap_or(0))
            .with_metadata("duration_ms", duration_ms);

        if let Some(sess_id) = config.and_then(|c| c.session_id.as_ref()) {
            event = event.with_metadata("session_id", sess_id);
        }

        Some(LLMBusinessEvent {
            event,
            scope: EventScope::User(user_id.clone()),
        })
    }

    /// Core LLM execution logic shared between events and non-events versions
    async fn execute_llm_internal(
        &self,
        request: UnifiedLLMRequest,
        config: Option<RequestConfig>,
    ) -> crate::provider::Result<(Response, OpenAIResponse, u64, OpenAIRequest)> {
        // Create base request and apply config
        let mut openai_request = self.create_base_request(&request);
        if let Some(cfg) = config.as_ref() {
            apply_config_to_request(&mut openai_request, Some(cfg.clone()));
        }
        self.apply_response_schema(&mut openai_request, request.response_schema);

        log_debug!(
            provider = "lmstudio",
            request_json = %serde_json::to_string(&openai_request).unwrap_or_default(),
            "Executing LLM request"
        );

        // Clone request for event creation
        let openai_request_for_events = openai_request.clone();

        // Send to LM Studio API
        let start_time = Instant::now();
        let api_response = self
            .send_lmstudio_request(&openai_request)
            .await
            .map_err(|e| anyhow::anyhow!("LM Studio API error: {}", e))?;
        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Parse response
        let response = self
            .parse_lmstudio_response(api_response.clone())
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

        Ok((
            response,
            api_response,
            duration_ms,
            openai_request_for_events,
        ))
    }

    /// Internal method for executor pattern - restore default retry policy
    pub(crate) async fn restore_default_retry_policy(&self) {
        // LMStudio provider doesn't need explicit retry policy restoration
        // The client manages retry state internally
    }
}

#[async_trait::async_trait]
impl LlmProvider for LMStudioProvider {
    #[cfg(feature = "events")]
    async fn execute_llm(
        &self,
        request: UnifiedLLMRequest,
        _current_tool_round: Option<ToolCallingRound>,
        config: Option<RequestConfig>,
    ) -> crate::provider::Result<(Response, Vec<LLMBusinessEvent>)> {
        let mut events = Vec::new();

        // Execute core logic and collect event data
        let (response, api_response, duration_ms, openai_request) =
            match self.execute_llm_internal(request, config.clone()).await {
                Ok(result) => result,
                Err(e) => {
                    // On error, log error event
                    if let Some(uid) = config.as_ref().and_then(|c| c.user_id.as_ref()) {
                        if let Some(llm_error) = e.downcast_ref::<LlmError>() {
                            events.push(self.create_error_event(llm_error, uid));
                        }
                    }
                    return Err(e);
                }
            };

        // Log request event
        if let Some(uid) = config.as_ref().and_then(|c| c.user_id.as_ref()) {
            events.push(self.create_request_event(&openai_request.model, uid));
        }

        // Log response event
        if let Some(event) = self.create_response_event(&api_response, duration_ms, config.as_ref())
        {
            events.push(event);
        }

        Ok((response, events))
    }

    #[cfg(not(feature = "events"))]
    async fn execute_llm(
        &self,
        request: UnifiedLLMRequest,
        _current_tool_round: Option<ToolCallingRound>,
        config: Option<RequestConfig>,
    ) -> crate::provider::Result<Response> {
        let (response, _api_response, _duration_ms, _openai_request) =
            self.execute_llm_internal(request, config).await?;
        Ok(response)
    }

    #[cfg(feature = "events")]
    async fn execute_structured_llm(
        &self,
        mut request: UnifiedLLMRequest,
        current_tool_round: Option<ToolCallingRound>,
        schema: serde_json::Value,
        config: Option<RequestConfig>,
    ) -> crate::provider::Result<(Response, Vec<LLMBusinessEvent>)> {
        // Set the schema in the request
        request.response_schema = Some(schema);

        // Execute with the schema-enabled request (returns tuple with events)
        self.execute_llm(request, current_tool_round, config).await
    }

    #[cfg(not(feature = "events"))]
    async fn execute_structured_llm(
        &self,
        mut request: UnifiedLLMRequest,
        current_tool_round: Option<ToolCallingRound>,
        schema: serde_json::Value,
        config: Option<RequestConfig>,
    ) -> crate::provider::Result<Response> {
        // Set the schema in the request
        request.response_schema = Some(schema);

        // Execute with the schema-enabled request
        self.execute_llm(request, current_tool_round, config).await
    }

    fn provider_name(&self) -> &'static str {
        "lmstudio"
    }
}

impl LMStudioProvider {
    /// Transform unified messages to OpenAI-compatible format for LMStudio
    /// LMStudio has no caching support, so we ignore caching attributes
    fn transform_unified_messages(
        &self,
        messages: &[&UnifiedMessage],
    ) -> Vec<super::openai_shared::OpenAIMessage> {
        messages
            .iter()
            .map(|msg| self.unified_message_to_openai(msg))
            .collect()
    }

    /// Convert a UnifiedMessage to OpenAI format for LMStudio
    /// Note: LMStudio has no caching support, so cacheable attributes are ignored
    fn unified_message_to_openai(
        &self,
        msg: &UnifiedMessage,
    ) -> super::openai_shared::OpenAIMessage {
        let role = match msg.role {
            MessageRole::System => "system".to_string(),
            MessageRole::User => "user".to_string(),
            MessageRole::Assistant => "assistant".to_string(),
            MessageRole::Tool => "user".to_string(), // LMStudio doesn't have native tool role, use user
        };

        let content = match &msg.content {
            MessageContent::Text(text) => text.clone(),
            MessageContent::Json(value) => serde_json::to_string_pretty(value).unwrap_or_default(),
            MessageContent::ToolCall { .. } => {
                // We should never be sending tool calls TO the LLM
                log_debug!(provider = "lmstudio", "Unexpected ToolCall in outgoing message - tool calls are received from LLM, not sent to it");
                "Error: Invalid message type".to_string()
            }
            MessageContent::ToolResult {
                content, is_error, ..
            } => {
                // Tool results become user messages for LMStudio
                if *is_error {
                    format!("Tool execution error: {}", content)
                } else {
                    format!("Tool execution result: {}", content)
                }
            }
        };

        super::openai_shared::OpenAIMessage { role, content }
    }

    /// Parse LMStudio response to Response
    /// LMStudio uses OpenAI-compatible response format
    fn parse_lmstudio_response(&self, response: OpenAIResponse) -> LlmResult<Response> {
        let choice =
            response.choices.into_iter().next().ok_or_else(|| {
                LlmError::response_parsing_error("No choices in LMStudio response")
            })?;

        let content = choice.message.content;

        // LMStudio may have limited tool support, handle gracefully
        let tool_calls = choice
            .message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|tc| crate::provider::ToolCall {
                id: tc.id,
                name: tc.function.name,
                arguments: serde_json::from_str(&tc.function.arguments)
                    .unwrap_or(serde_json::Value::Null),
            })
            .collect();

        // LMStudio may not provide usage stats
        let usage = response.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        // Handle structured response parsing if needed
        // Local models may be less reliable with JSON formatting
        let structured_response = if content.trim_start().starts_with('{') {
            match ResponseParser::parse_llm_output(&content) {
                Ok(json_value) => {
                    log_debug!(
                        provider = "lmstudio",
                        "Successfully parsed structured JSON response"
                    );
                    Some(json_value)
                }
                Err(_) => {
                    log_debug!(provider = "lmstudio", "Failed to parse structured response from local model - this is common with local LLMs");
                    None
                }
            }
        } else {
            None
        };

        Ok(Response {
            content,
            structured_response,
            tool_calls,
            usage,
            model: Some(self.config.default_model.clone()),
            raw_body: None,
        })
    }
}
