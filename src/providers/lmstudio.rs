//! LM Studio provider implementation
//!
//! LM Studio uses OpenAI-compatible API, so this implementation leverages
//! the shared OpenAI structures and utilities.

use super::openai_shared::{
    http::OpenAICompatibleClient, utils::apply_config_to_request, OpenAIRequest, OpenAIResponse,
};
// Removed LLMClientCore import - providers now implement their own methods directly
use crate::config::{DefaultLLMParams, LMStudioConfig};
use crate::log_debug;
use crate::error::{LlmError, LlmResult};
use crate::response_parser::ResponseParser;
use crate::core_types::executor::{
    ExecutorLLMConfig, ExecutorLLMProvider, ExecutorLLMResponse, ExecutorTokenUsage,
    LLMBusinessEvent, ToolCallingRound,
};
use crate::core_types::events::{BusinessEvent, EventScope};
use crate::core_types::messages::{MessageContent, MessageRole, UnifiedLLMRequest, UnifiedMessage};
use crate::core_types::event_types;
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
    fn create_base_request(
        &self,
        messages: Vec<super::openai_shared::OpenAIMessage>,
    ) -> OpenAIRequest {
        OpenAIRequest {
            model: self.config.default_model.clone(),
            messages,
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
    fn create_response_event(
        &self,
        response: &OpenAIResponse,
        duration_ms: u64,
        config: Option<&ExecutorLLMConfig>,
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

    /// Internal method for executor pattern - restore default retry policy
    pub(crate) async fn restore_default_retry_policy(&self) {
        // LMStudio provider doesn't need explicit retry policy restoration
        // The client manages retry state internally
    }
}

#[async_trait::async_trait]
impl ExecutorLLMProvider for LMStudioProvider {
    async fn execute_llm(
        &self,
        request: UnifiedLLMRequest,
        _current_tool_round: Option<ToolCallingRound>,
        config: Option<ExecutorLLMConfig>,
    ) -> crate::core_types::Result<(ExecutorLLMResponse, Vec<LLMBusinessEvent>)> {
        let mut events = Vec::new();

        // Create base request
        let openai_messages = self.transform_unified_messages(&request.get_sorted_messages());
        let mut openai_request = self.create_base_request(openai_messages);

        // Apply config and schema
        if let Some(cfg) = config.as_ref() {
            apply_config_to_request(&mut openai_request, Some(cfg.clone()));
        }
        self.apply_response_schema(&mut openai_request, request.response_schema);

        log_debug!(
            provider = "lmstudio",
            request_json = %serde_json::to_string(&openai_request).unwrap_or_default(),
            "Executing LLM request"
        );

        // Log request event
        if let Some(uid) = config.as_ref().and_then(|c| c.user_id.as_ref()) {
            events.push(self.create_request_event(&openai_request.model, uid));
        }

        // Execute API request
        let start_time = Instant::now();
        let url = format!("{}/v1/chat/completions", self.config.base_url);
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        let api_response = match self
            .http_client
            .execute_chat_request(&url, &headers, &openai_request)
            .await
        {
            Ok(response) => response,
            Err(e) => {
                if let Some(uid) = config.as_ref().and_then(|c| c.user_id.as_ref()) {
                    events.push(self.create_error_event(&e, uid));
                }
                return Err(anyhow::anyhow!("LMStudio API error: {}", e));
            }
        };
        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Log response event
        if let Some(event) = self.create_response_event(&api_response, duration_ms, config.as_ref())
        {
            events.push(event);
        }

        // Parse response
        let response = self
            .parse_lmstudio_response(api_response)
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

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

    /// Parse LMStudio response to ExecutorLLMResponse
    /// LMStudio uses OpenAI-compatible response format
    fn parse_lmstudio_response(&self, response: OpenAIResponse) -> LlmResult<ExecutorLLMResponse> {
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
            .map(|tc| crate::core_types::executor::ExecutorToolCall {
                id: tc.id,
                name: tc.function.name,
                arguments: serde_json::from_str(&tc.function.arguments)
                    .unwrap_or(serde_json::Value::Null),
            })
            .collect();

        // LMStudio may not provide usage stats
        let usage = response.usage.map(|u| ExecutorTokenUsage {
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

        Ok(ExecutorLLMResponse {
            content,
            structured_response,
            tool_calls,
            usage,
            model: Some(self.config.default_model.clone()),
            raw_body: None,
        })
    }
}

// TEMP: Commented out trait implementation during refactor
/*
#[async_trait]
impl LLMClientCore for LMStudioProvider {
    async fn send_message(&self, message: &str) -> LlmResult<String> {
        let start_time = Instant::now();
        let estimated_tokens = utils::estimate_tokens(message);

        log_llm!(
            request,
            "lmstudio",
            self.config.default_model,
            estimated_tokens
        );

        // Create OpenAI-compatible request
        let request = OpenAIRequest {
            model: self.config.default_model.clone(),
            messages: vec![super::openai_shared::OpenAIMessage {
                role: "user".to_string(),
                content: message.to_string(),
            }],
            temperature: Some(0.7),
            max_tokens: Some(1000),
            top_p: None,
            presence_penalty: None,
            stream: Some(false),
            tools: None,
            response_format: None,
        };

        let api_response = self.make_chat_request(request).await?;

        let content = api_response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .unwrap_or_else(|| "No response content".to_string());

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Use real token usage from API response if available
        let response_tokens = api_response
            .usage
            .as_ref()
            .map(|u| u.completion_tokens)
            .unwrap_or_else(|| {
                log_debug!(
                    provider = "lmstudio",
                    "No usage data in response, estimating tokens"
                );
                utils::estimate_tokens(&content)
            });

        log_llm!(response, "lmstudio", response_tokens, duration_ms);

        // Log actual usage stats if available
        if let Some(usage) = &api_response.usage {
            log_debug!(
                provider = "lmstudio",
                prompt_tokens = usage.prompt_tokens,
                completion_tokens = usage.completion_tokens,
                total_tokens = usage.total_tokens,
                "Real token usage from LM Studio API"
            );
        }

        Ok(content)
    }

    async fn execute_chat_with_model(
        &self,
        model: &str,
        messages: Vec<Message>,
        config: Option<LLMRequestConfig>,
    ) -> LlmResult<LLMResponse> {
        let start_time = Instant::now();

        // Convert neutral messages to OpenAI format
        let openai_messages = utils::convert_neutral_messages_to_openai(&messages);
        let estimated_prompt_tokens = openai_messages
            .iter()
            .map(|m| utils::estimate_tokens(&m.content))
            .sum::<u32>();

        log_llm!(request, "lmstudio", model, estimated_prompt_tokens);

        log_debug!(
            provider = "lmstudio",
            model = model,
            message_count = openai_messages.len(),
            has_config = config.is_some(),
            estimated_prompt_tokens = estimated_prompt_tokens,
            base_url = %self.config.base_url,
            "Executing chat completion"
        );

        // Build OpenAI-compatible request
        let mut request = OpenAIRequest {
            model: model.to_string(),
            messages: openai_messages,
            temperature: None,
            max_tokens: None,
            top_p: None,
            presence_penalty: None,
            stream: Some(false),
            tools: None,
            response_format: None,
        };

        // Apply config parameters if provided and capture response_format for structured response parsing
        let has_response_format = config.as_ref()
            .and_then(|cfg| cfg.response_format.as_ref())
            .is_some();
        utils::apply_config_to_request(&mut request, config);

        let api_response = self.make_chat_request(request).await?;

        // Process tool calls and get cleaned content
        let (content, tool_calls) = if let Some(choice) = api_response.choices.first() {
            let tool_result = utils::handle_tool_calls_with_content_cleaning(&choice.message)?;
            let has_cleaned = tool_result.cleaned_content.is_some();
            let final_content = tool_result.cleaned_content.unwrap_or(choice.message.content.clone());

            tracing::debug!(
                has_cleaned_content = has_cleaned,
                tool_calls_detected = tool_result.tool_calls.len(),
                original_content_length = choice.message.content.len(),
                final_content_length = final_content.len(),
                final_content_preview = &final_content.chars().take(200).collect::<String>(),
                "DEBUG: LM Studio provider (method 2) - processed tool calls and content cleaning"
            );

            (final_content, tool_result.tool_calls)
        } else {
            ("No response content".to_string(), vec![])
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Use real token usage from API response
        let usage: Option<LLMUsage> = api_response.usage.map(|u| u.into());

        let completion_tokens = usage
            .as_ref()
            .map(|u: &LLMUsage| u.completion_tokens)
            .unwrap_or_else(|| {
                log_debug!(
                    provider = "lmstudio",
                    "No usage data in response, estimating tokens"
                );
                utils::estimate_tokens(&content)
            });

        log_llm!(response, "lmstudio", completion_tokens, duration_ms);

        // Parse structured response if response_format was specified
        let structured_response = if has_response_format {
            // Attempt to parse the content as JSON for structured response
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(json_value) => {
                    log_debug!(
                        provider = "lmstudio",
                        "Successfully parsed structured response from content"
                    );
                    Some(json_value)
                },
                Err(parse_error) => {
                    log_debug!(
                        provider = "lmstudio",
                        error = %parse_error,
                        content_preview = %if content.len() > 100 { &content[..100] } else { &content },
                        "Failed to parse content as JSON for structured response, content may not be valid JSON"
                    );
                    None
                }
            }
        } else {
            None
        };

        log_debug!(
            provider = "lmstudio",
            model = model,
            prompt_tokens = usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(estimated_prompt_tokens),
            completion_tokens = completion_tokens,
            total_tokens = usage.as_ref().map(|u| u.total_tokens).unwrap_or(estimated_prompt_tokens + completion_tokens),
            tool_calls_count = tool_calls.len(),
            duration_ms = duration_ms,
            base_url = %self.config.base_url,
            "Chat completion successful"
        );

        Ok(LLMResponse {
            content,
            structured_response,
            tool_calls,
            usage,
            raw_body: None, // Could store raw body for debugging if needed
        })
    }

    async fn execute_chat_default(
        &self,
        messages: Vec<Message>,
        config: Option<LLMRequestConfig>,
    ) -> LlmResult<LLMResponse> {
        self.execute_chat_with_model(&self.config.default_model, messages, config)
            .await
    }

    fn max_context_tokens(&self) -> usize {
        self.config.max_context_tokens
    }

    fn provider_name(&self) -> &'static str {
        "lmstudio"
    }

    async fn set_retry_policy(&self, policy: RetryPolicy) {
        self.http_client.set_retry_policy(policy).await;
        log_debug!(
            provider = "lmstudio",
            "Set custom retry policy for agent request"
        );
    }

    async fn restore_default_retry_policy(&self) {
        self.http_client
            .restore_default_retry_policy(&self.config.retry_policy)
            .await;
        // log_debug!(provider = "lmstudio", "Restored default retry policy");
    }
}
*/
