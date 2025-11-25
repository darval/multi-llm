//! OpenAI provider implementation
//!
//! This provider uses the OpenAI-compatible shared structures and utilities.

use super::openai_shared::{
    http::OpenAICompatibleClient, utils::apply_config_to_request, OpenAIRequest, OpenAIResponse,
};
// Removed LLMClientCore import - providers now implement their own methods directly
use crate::config::{DefaultLLMParams, OpenAIConfig};
#[cfg(feature = "events")]
use crate::core_types::events::{BusinessEvent, EventScope};
use crate::core_types::messages::{MessageContent, MessageRole, UnifiedLLMRequest, UnifiedMessage};
#[cfg(feature = "events")]
use crate::core_types::provider::LLMBusinessEvent;
use crate::core_types::provider::{
    LlmProvider, RequestConfig, Response, TokenUsage, ToolCallingRound,
};
use crate::error::{LlmError, LlmResult};
use crate::logging::{log_debug, log_error};
use crate::response_parser::ResponseParser;

#[cfg(feature = "events")]
use crate::core_types::event_types;
use std::time::Instant;

/// OpenAI provider implementation
#[derive(Debug)]
pub struct OpenAIProvider {
    http_client: OpenAICompatibleClient,
    config: OpenAIConfig,
    default_params: DefaultLLMParams,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider instance
    ///
    /// # Errors
    ///
    /// Returns [`LlmError::ConfigurationError`] if:
    /// - API key is missing or invalid
    /// - Provider configuration validation fails
    /// - HTTP client initialization fails
    pub fn new(config: OpenAIConfig, default_params: DefaultLLMParams) -> LlmResult<Self> {
        log_debug!(
            provider = "openai",
            has_api_key = config.api_key.is_some(),
            max_context_tokens = config.max_context_tokens,
            base_url = %config.base_url,
            default_model = %config.default_model,
            default_temperature = default_params.temperature,
            "Creating OpenAI provider"
        );

        if config.api_key.is_none() {
            return Err(LlmError::configuration_error("OpenAI API key is required"));
        }

        log_debug!(
            provider = "openai",
            max_context_tokens = config.max_context_tokens,
            default_model = %config.default_model,
            default_temperature = default_params.temperature,
            "OpenAI provider initialized"
        );

        Ok(Self {
            http_client: OpenAICompatibleClient::with_retry_policy(config.retry_policy.clone()),
            config,
            default_params,
        })
    }

    /// Internal method for executor pattern - restore default retry policy
    pub(crate) async fn restore_default_retry_policy(&self) {
        // OpenAI provider doesn't need explicit retry policy restoration
        // The client manages retry state internally
    }

    /// Create base OpenAI request from unified request
    fn create_base_request(&self, request: &UnifiedLLMRequest) -> OpenAIRequest {
        let openai_messages = self.transform_unified_messages(&request.get_sorted_messages());

        OpenAIRequest {
            model: self.config.default_model.clone(),
            messages: openai_messages,
            temperature: Some(self.default_params.temperature),
            max_tokens: Some(self.default_params.max_tokens),
            top_p: Some(self.default_params.top_p),
            presence_penalty: None,
            stream: None,
            tools: None,
            tool_choice: None,
            response_format: None,
        }
    }

    /// Apply response schema to request if present
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

    /// Send request to OpenAI API
    async fn send_openai_request(
        &self,
        request: &OpenAIRequest,
    ) -> crate::core_types::Result<OpenAIResponse> {
        // Construct full URL with path (consistent with LMStudio/Ollama)
        let url = format!("{}/v1/chat/completions", self.config.base_url);

        let headers = OpenAICompatibleClient::build_auth_headers(
            self.config.api_key.as_ref().unwrap_or(&String::new()),
        )
        .map_err(|e| anyhow::anyhow!("Failed to build headers: {}", e))?;
        self.http_client
            .execute_chat_request(&url, &headers, request)
            .await
            .map_err(|e| anyhow::anyhow!("OpenAI API error: {}", e))
    }

    /// Create LLM request business event
    #[cfg(feature = "events")]
    fn create_request_event(
        &self,
        request: &OpenAIRequest,
        config: Option<&RequestConfig>,
    ) -> Option<LLMBusinessEvent> {
        let user_id = config.and_then(|c| c.user_id.clone())?;

        let event = BusinessEvent::new(event_types::LLM_REQUEST)
            .with_metadata("provider", "openai")
            .with_metadata("model", &request.model);

        Some(LLMBusinessEvent {
            event,
            scope: EventScope::User(user_id),
        })
    }

    /// Create LLM error business event
    #[cfg(feature = "events")]
    fn create_error_event(
        &self,
        error: &str,
        config: Option<&RequestConfig>,
    ) -> Option<LLMBusinessEvent> {
        let user_id = config.and_then(|c| c.user_id.clone())?;

        let event = BusinessEvent::new(event_types::LLM_ERROR)
            .with_metadata("provider", "openai")
            .with_metadata("error", error);

        Some(LLMBusinessEvent {
            event,
            scope: EventScope::User(user_id),
        })
    }

    /// Create LLM response business event
    #[cfg(feature = "events")]
    fn create_response_event(
        &self,
        api_response: &OpenAIResponse,
        duration_ms: u64,
        config: Option<&RequestConfig>,
    ) -> Option<LLMBusinessEvent> {
        let user_id = config.and_then(|c| c.user_id.clone())?;

        let usage_tokens = api_response
            .usage
            .as_ref()
            .map(|u| (u.prompt_tokens, u.completion_tokens));
        let mut event = BusinessEvent::new(event_types::LLM_RESPONSE)
            .with_metadata("provider", "openai")
            .with_metadata("model", &self.config.default_model)
            .with_metadata("input_tokens", usage_tokens.map(|(i, _)| i).unwrap_or(0))
            .with_metadata("output_tokens", usage_tokens.map(|(_, o)| o).unwrap_or(0))
            .with_metadata("duration_ms", duration_ms);

        if let Some(ref sess_id) = config.and_then(|c| c.session_id.as_ref()) {
            event = event.with_metadata("session_id", sess_id);
        }

        Some(LLMBusinessEvent {
            event,
            scope: EventScope::User(user_id),
        })
    }

    /// Core LLM execution logic shared between events and non-events versions
    ///
    /// Returns (Response, OpenAIResponse, duration_ms, OpenAIRequest) to allow event creation
    async fn execute_llm_internal(
        &self,
        request: UnifiedLLMRequest,
        config: Option<RequestConfig>,
    ) -> crate::core_types::Result<(Response, OpenAIResponse, u64, OpenAIRequest)> {
        // Create base request and apply config
        let mut openai_request = self.create_base_request(&request);
        if let Some(cfg) = config.as_ref() {
            apply_config_to_request(&mut openai_request, Some(cfg.clone()));
        }
        self.apply_response_schema(&mut openai_request, request.response_schema);

        log_debug!(
            provider = "openai",
            request_json = %serde_json::to_string(&openai_request).unwrap_or_default(),
            "Executing LLM request"
        );

        // Clone request for event creation
        let openai_request_for_events = openai_request.clone();

        // Send to OpenAI API
        let start_time = Instant::now();
        let api_response = self.send_openai_request(&openai_request).await?;
        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Parse response
        let response = self
            .parse_openai_response(api_response.clone())
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

        Ok((
            response,
            api_response,
            duration_ms,
            openai_request_for_events,
        ))
    }

    /// Transform unified messages to OpenAI format
    /// OpenAI includes system messages in the messages array and has automatic caching
    fn transform_unified_messages(
        &self,
        messages: &[&UnifiedMessage],
    ) -> Vec<super::openai_shared::OpenAIMessage> {
        messages
            .iter()
            .map(|msg| self.unified_message_to_openai(msg))
            .collect()
    }

    /// Convert a UnifiedMessage to OpenAI format
    /// Note: OpenAI has automatic caching, so we don't need to handle cache_control
    fn unified_message_to_openai(
        &self,
        msg: &UnifiedMessage,
    ) -> super::openai_shared::OpenAIMessage {
        let role = match msg.role {
            MessageRole::System => "system".to_string(),
            MessageRole::User => "user".to_string(),
            MessageRole::Assistant => "assistant".to_string(),
            MessageRole::Tool => "tool".to_string(),
        };

        let content = match &msg.content {
            MessageContent::Text(text) => text.clone(),
            MessageContent::Json(value) => serde_json::to_string_pretty(value).unwrap_or_default(),
            MessageContent::ToolCall { .. } => {
                // We should never be sending tool calls TO the LLM
                log_error!(provider = "openai", "Unexpected ToolCall in outgoing message - tool calls are received from LLM, not sent to it");
                "Error: Invalid message type".to_string()
            }
            MessageContent::ToolResult {
                tool_call_id: _,
                content,
                is_error,
            } => {
                if *is_error {
                    format!("Error: {}", content)
                } else {
                    content.clone()
                }
            }
        };

        super::openai_shared::OpenAIMessage { role, content }
    }

    /// Parse OpenAI response to Response
    fn parse_openai_response(&self, response: OpenAIResponse) -> LlmResult<Response> {
        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| LlmError::response_parsing_error("No choices in OpenAI response"))?;

        let content = choice.message.content;

        let tool_calls = choice
            .message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|tc| crate::core_types::provider::ToolCall {
                id: tc.id,
                name: tc.function.name,
                arguments: serde_json::from_str(&tc.function.arguments).unwrap_or_default(),
            })
            .collect();

        let usage = response.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        // Handle structured response parsing if needed
        let structured_response = if content.trim_start().starts_with('{') {
            match ResponseParser::parse_llm_output(&content) {
                Ok(json_value) => {
                    log_debug!(
                        provider = "openai",
                        "Successfully parsed structured JSON response"
                    );
                    Some(json_value)
                }
                Err(_) => None,
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

#[async_trait::async_trait]
impl LlmProvider for OpenAIProvider {
    #[cfg(feature = "events")]
    async fn execute_llm(
        &self,
        request: UnifiedLLMRequest,
        _current_tool_round: Option<ToolCallingRound>,
        config: Option<RequestConfig>,
    ) -> crate::core_types::Result<(Response, Vec<LLMBusinessEvent>)> {
        let mut events = Vec::new();

        // Execute core logic and collect event data
        let (response, api_response, duration_ms, openai_request) =
            match self.execute_llm_internal(request, config.clone()).await {
                Ok(result) => result,
                Err(e) => {
                    // On error, log error event
                    if let Some(event) = self.create_error_event(&e.to_string(), config.as_ref()) {
                        events.push(event);
                    }
                    return Err(e);
                }
            };

        // Log request event
        if let Some(event) = self.create_request_event(&openai_request, config.as_ref()) {
            events.push(event);
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
    ) -> crate::core_types::Result<Response> {
        // Simple wrapper - just return the response
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
    ) -> crate::core_types::Result<(Response, Vec<LLMBusinessEvent>)> {
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
    ) -> crate::core_types::Result<Response> {
        // Set the schema in the request
        request.response_schema = Some(schema);

        // Execute with the schema-enabled request
        self.execute_llm(request, current_tool_round, config).await
    }

    fn provider_name(&self) -> &'static str {
        "openai"
    }
}
