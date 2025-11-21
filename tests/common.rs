//! Test helper utilities for multi-llm tests
//!
//! This module provides reusable test fixtures and helper functions
//! that are shared across multiple test modules.
//!
//! IMPORTANT: These helpers are test-only and should NEVER be used in production code.

// Allow dead code in test utilities - functions are used across different test files
#![allow(dead_code)]

use chrono::Utc;
use multi_llm::config::{
    AnthropicConfig, DefaultLLMParams, LLMConfig, LMStudioConfig, OllamaConfig, OpenAIConfig,
};
use multi_llm::core_types::executor::{ExecutorLLMConfig, ExecutorTool};
use multi_llm::core_types::messages::{
    MessageContent, MessageRole, UnifiedLLMRequest, UnifiedMessage,
};
use multi_llm::providers::{AnthropicProvider, LMStudioProvider, OllamaProvider, OpenAIProvider};
use multi_llm::retry::RetryPolicy;
use multi_llm::tokens::{AnthropicTokenCounter, OpenAITokenCounter, TokenCounter};
use std::sync::Arc;
use std::time::Duration;
use wiremock::ResponseTemplate;

/// Create test configuration for a specific provider
///
/// This replaces the removed `LLMConfig::for_testing()` anti-pattern.
/// Test configurations include valid defaults suitable for testing.
///
/// # Arguments
///
/// * `provider_name` - Provider name: "anthropic", "openai", "lmstudio", or "ollama"
///
/// # Returns
///
/// A valid `LLMConfig` with test-appropriate defaults for the specified provider.
///
/// # Panics
///
/// Panics if the provider name is not recognized (test failure is appropriate).
pub fn create_test_config(provider_name: &str) -> LLMConfig {
    let provider: Box<dyn multi_llm::config::ProviderConfig> = match provider_name {
        "anthropic" => Box::new(AnthropicConfig {
            api_key: Some("test-anthropic-key".to_string()),
            base_url: "https://api.anthropic.com".to_string(),
            default_model: "claude-3-5-sonnet-20241022".to_string(),
            max_context_tokens: 200_000,
            retry_policy: RetryPolicy::default(),
            enable_prompt_caching: true,
            cache_ttl: "1h".to_string(),
        }),
        "openai" => Box::new(OpenAIConfig {
            api_key: Some("test-openai-key".to_string()),
            base_url: "https://api.openai.com".to_string(),
            default_model: "gpt-4".to_string(),
            max_context_tokens: 128_000,
            retry_policy: RetryPolicy::default(),
        }),
        "lmstudio" => Box::new(LMStudioConfig {
            base_url: "http://localhost:1234".to_string(),
            default_model: "local-model".to_string(),
            max_context_tokens: 4_096,
            retry_policy: RetryPolicy::default(),
        }),
        "ollama" => Box::new(OllamaConfig {
            base_url: "http://localhost:11434".to_string(),
            default_model: "llama2".to_string(),
            max_context_tokens: 4_096,
            retry_policy: RetryPolicy::default(),
        }),
        _ => panic!("Unsupported test provider: {}", provider_name),
    };

    LLMConfig {
        provider,
        default_params: DefaultLLMParams::default(),
    }
}

/// Create test retry policy with fast timeouts for testing
///
/// Returns a `RetryPolicy` with shorter delays suitable for unit tests.
pub fn create_test_retry_policy() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 3,
        initial_delay: Duration::from_millis(10),
        max_delay: Duration::from_millis(100),
        backoff_multiplier: 2.0,
        total_timeout: Duration::from_secs(5),
        request_timeout: Duration::from_secs(2),
    }
}

/// Create test retry policy with no retries (for deterministic testing)
pub fn create_no_retry_policy() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 1,
        initial_delay: Duration::from_millis(0),
        max_delay: Duration::from_millis(0),
        backoff_multiplier: 1.0,
        total_timeout: Duration::from_secs(1),
        request_timeout: Duration::from_secs(1),
    }
}

/// Create test token counter for a specific provider
///
/// # Arguments
///
/// * `provider_name` - Provider name: "anthropic", "openai", "lmstudio", or "ollama"
///
/// # Returns
///
/// An `Arc<dyn TokenCounter>` appropriate for the provider.
pub fn create_test_token_counter(provider_name: &str) -> Arc<dyn TokenCounter> {
    match provider_name {
        "anthropic" => Arc::new(AnthropicTokenCounter::new("claude-3-5-sonnet-20241022").unwrap()),
        "openai" => Arc::new(OpenAITokenCounter::new("gpt-4").unwrap()),
        "lmstudio" => Arc::new(OpenAITokenCounter::for_lm_studio(4096).unwrap()),
        "ollama" => Arc::new(OpenAITokenCounter::for_lm_studio(4096).unwrap()),
        _ => panic!("Unsupported test provider: {}", provider_name),
    }
}

/// Create test token counter with custom max tokens
///
/// Note: This uses the TokenCounterFactory::create_counter_with_limit method
/// since max_tokens fields are private.
pub fn create_test_token_counter_with_limit(
    provider_name: &str,
    max_tokens: u32,
) -> Arc<dyn TokenCounter> {
    use multi_llm::tokens::TokenCounterFactory;

    let model = match provider_name {
        "anthropic" => "claude-3-5-sonnet-20241022",
        "openai" => "gpt-4",
        "lmstudio" | "ollama" => "local-model",
        _ => panic!("Unsupported test provider: {}", provider_name),
    };

    TokenCounterFactory::create_counter_with_limit(provider_name, model, max_tokens)
        .expect("Failed to create test token counter with limit")
}

/// Create test message JSON for token counting tests
pub fn create_test_message(role: &str, content: &str) -> serde_json::Value {
    serde_json::json!({
        "role": role,
        "content": content,
    })
}

/// Create test messages array for token counting tests
pub fn create_test_messages() -> Vec<serde_json::Value> {
    vec![
        create_test_message("system", "You are a helpful assistant."),
        create_test_message("user", "Hello, how are you?"),
        create_test_message("assistant", "I'm doing well, thank you!"),
    ]
}

// ============================================================================
// Provider Creation Helpers (for provider tests with wiremock)
// ============================================================================

/// Create fast retry policy for tests (avoids long waits)
///
/// This retry policy has short delays suitable for unit tests with mock servers.
/// Use this instead of `create_test_retry_policy()` when you need minimal retry attempts.
pub fn create_fast_test_retry_policy() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 2,
        initial_delay: Duration::from_millis(10),
        max_delay: Duration::from_millis(50),
        backoff_multiplier: 2.0,
        request_timeout: Duration::from_secs(5),
        total_timeout: Duration::from_secs(10),
    }
}

/// Create concrete Anthropic provider for testing with mock server
///
/// # Arguments
///
/// * `base_url` - Base URL of the mock server (e.g., `mock_server.uri()`)
///
/// # Returns
///
/// A fully configured `AnthropicProvider` pointing to the mock server.
///
/// # Example
///
/// ```ignore
/// let mock_server = MockServer::start().await;
/// let provider = create_concrete_anthropic_provider(&mock_server.uri());
/// ```
pub fn create_concrete_anthropic_provider(base_url: &str) -> AnthropicProvider {
    let config = AnthropicConfig {
        api_key: Some("test-anthropic-key".to_string()),
        base_url: base_url.to_string(),
        default_model: "claude-3-5-sonnet-20241022".to_string(),
        max_context_tokens: 200_000,
        retry_policy: create_fast_test_retry_policy(),
        enable_prompt_caching: true,
        cache_ttl: "1h".to_string(),
    };
    AnthropicProvider::new(config, DefaultLLMParams::default())
        .expect("Failed to create test Anthropic provider")
}

/// Create concrete OpenAI provider for testing with mock server
///
/// # Arguments
///
/// * `base_url` - Base URL of the mock server (e.g., `mock_server.uri()`)
///
/// # Returns
///
/// A fully configured `OpenAIProvider` pointing to the mock server.
pub fn create_concrete_openai_provider(base_url: &str) -> OpenAIProvider {
    let config = OpenAIConfig {
        api_key: Some("test-openai-key".to_string()),
        base_url: base_url.to_string(),
        default_model: "gpt-4".to_string(),
        max_context_tokens: 128_000,
        retry_policy: create_fast_test_retry_policy(),
    };
    OpenAIProvider::new(config, DefaultLLMParams::default())
        .expect("Failed to create test OpenAI provider")
}

/// Create concrete LMStudio provider for testing with mock server
///
/// # Arguments
///
/// * `base_url` - Base URL of the mock server (e.g., `mock_server.uri()`)
///
/// # Returns
///
/// A fully configured `LMStudioProvider` pointing to the mock server.
pub fn create_concrete_lmstudio_provider(base_url: &str) -> LMStudioProvider {
    let config = LMStudioConfig {
        base_url: base_url.to_string(),
        default_model: "local-model".to_string(),
        max_context_tokens: 4_096,
        retry_policy: create_fast_test_retry_policy(),
    };
    LMStudioProvider::new(config, DefaultLLMParams::default())
        .expect("Failed to create test LMStudio provider")
}

/// Create concrete Ollama provider for testing with mock server
///
/// # Arguments
///
/// * `base_url` - Base URL of the mock server (e.g., `mock_server.uri()`)
///
/// # Returns
///
/// A fully configured `OllamaProvider` pointing to the mock server.
pub fn create_concrete_ollama_provider(base_url: &str) -> OllamaProvider {
    let config = OllamaConfig {
        base_url: base_url.to_string(),
        default_model: "llama2".to_string(),
        max_context_tokens: 4_096,
        retry_policy: create_fast_test_retry_policy(),
    };
    OllamaProvider::new(config, DefaultLLMParams::default())
        .expect("Failed to create test Ollama provider")
}

// ============================================================================
// Test Data Creation Helpers (for LLM requests)
// ============================================================================

/// Create test UnifiedLLMRequest with a simple user message
///
/// Returns a request with a single user message suitable for basic testing.
pub fn create_test_unified_request() -> UnifiedLLMRequest {
    let messages = vec![UnifiedMessage {
        role: MessageRole::User,
        content: MessageContent::Text("Test message".to_string()),
        attributes: Default::default(),
        timestamp: Utc::now(),
    }];
    UnifiedLLMRequest::new(messages)
}

/// Create test UnifiedLLMRequest with system and user messages
///
/// Returns a request with both system and user messages for more complete testing.
pub fn create_test_unified_request_with_system() -> UnifiedLLMRequest {
    let now = Utc::now();
    let messages = vec![
        UnifiedMessage {
            role: MessageRole::System,
            content: MessageContent::Text("You are a helpful assistant.".to_string()),
            attributes: Default::default(),
            timestamp: now,
        },
        UnifiedMessage {
            role: MessageRole::User,
            content: MessageContent::Text("Hello, how are you?".to_string()),
            attributes: Default::default(),
            timestamp: now,
        },
    ];
    UnifiedLLMRequest::new(messages)
}

/// Create test ExecutorLLMConfig with all fields populated
///
/// Returns a complete configuration with all optional fields set.
/// Use this to test configuration completeness across providers.
pub fn create_full_executor_config() -> ExecutorLLMConfig {
    ExecutorLLMConfig {
        llm_path: Some("test_llm".to_string()),
        session_id: Some("test_session".to_string()),
        user_id: Some("test_user".to_string()),
        temperature: Some(0.7),
        max_tokens: Some(1000),
        top_p: Some(0.9),
        top_k: Some(40),
        min_p: Some(0.05),
        presence_penalty: Some(0.1),
        response_format: None,
        tools: vec![],
        tool_choice: None,
    }
}

/// Create minimal test ExecutorLLMConfig with only required fields
///
/// Returns a configuration with minimal fields set.
pub fn create_minimal_executor_config() -> ExecutorLLMConfig {
    ExecutorLLMConfig {
        llm_path: Some("test_llm".to_string()),
        session_id: Some("test_session".to_string()),
        user_id: Some("test_user".to_string()),
        temperature: None,
        max_tokens: None,
        top_p: None,
        top_k: None,
        min_p: None,
        presence_penalty: None,
        response_format: None,
        tools: vec![],
        tool_choice: None,
    }
}

/// Create test ExecutorTool
///
/// Returns a simple tool definition suitable for testing tool call functionality.
pub fn create_test_tool() -> ExecutorTool {
    ExecutorTool {
        name: "test_tool".to_string(),
        description: "A test tool for unit tests".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "arg": {
                    "type": "string",
                    "description": "Test argument"
                }
            },
            "required": ["arg"]
        }),
    }
}

/// Create test JSON schema for structured responses
///
/// Returns a simple JSON schema suitable for testing structured response functionality.
pub fn create_test_json_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "answer": {
                "type": "string",
                "description": "The answer to the question"
            },
            "confidence": {
                "type": "number",
                "description": "Confidence score between 0 and 1"
            }
        },
        "required": ["answer"]
    })
}

// ============================================================================
// Mock Response Helpers (for wiremock)
// ============================================================================

/// Create successful Anthropic API response
///
/// Returns a JSON response matching Anthropic's Messages API format.
pub fn create_successful_anthropic_response() -> serde_json::Value {
    serde_json::json!({
        "id": "msg_test123",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "text",
            "text": "This is a test response from the mock server"
        }],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 20
        }
    })
}

/// Create Anthropic API response with tool calls
///
/// Returns a JSON response with a tool_use content block.
pub fn create_anthropic_response_with_tools() -> serde_json::Value {
    serde_json::json!({
        "id": "msg_test124",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "tool_use",
            "id": "toolu_test123",
            "name": "test_tool",
            "input": {"arg": "test_value"}
        }],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "tool_use",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 15,
            "output_tokens": 10
        }
    })
}

/// Create Anthropic API response with cache usage statistics
///
/// Returns a JSON response with prompt caching metrics.
pub fn create_anthropic_response_with_caching() -> serde_json::Value {
    serde_json::json!({
        "id": "msg_test125",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "text",
            "text": "Response with caching"
        }],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 100,
            "output_tokens": 50,
            "cache_creation_input_tokens": 80,
            "cache_read_input_tokens": 0
        }
    })
}

/// Create successful OpenAI API response
///
/// Returns a JSON response matching OpenAI's Chat Completions API format.
pub fn create_successful_openai_response() -> serde_json::Value {
    serde_json::json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "This is a test response from the mock server"
            },
            "finish_reason": "stop",
            "index": 0
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 20,
            "total_tokens": 30
        },
        "model": "gpt-4",
        "id": "chatcmpl-test123",
        "object": "chat.completion",
        "created": 1234567890
    })
}

/// Create OpenAI API response with tool calls
///
/// Returns a JSON response with tool_calls in the message.
pub fn create_openai_response_with_tools() -> serde_json::Value {
    serde_json::json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "",
                "tool_calls": [{
                    "id": "call_test123",
                    "type": "function",
                    "function": {
                        "name": "test_tool",
                        "arguments": "{\"arg\":\"test_value\"}"
                    }
                }]
            },
            "finish_reason": "tool_calls",
            "index": 0
        }],
        "usage": {
            "prompt_tokens": 15,
            "completion_tokens": 10,
            "total_tokens": 25
        },
        "model": "gpt-4",
        "id": "chatcmpl-test124",
        "object": "chat.completion",
        "created": 1234567890
    })
}

/// Create error response template for wiremock
///
/// # Arguments
///
/// * `status` - HTTP status code
/// * `message` - Error message
///
/// # Returns
///
/// A `ResponseTemplate` that can be mounted on a wiremock `Mock`.
pub fn create_error_response(status: u16, message: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_json(serde_json::json!({
        "error": {
            "message": message,
            "type": "api_error"
        }
    }))
}

/// Create 401 authentication error response
///
/// Returns a ResponseTemplate for authentication failures.
pub fn create_auth_error_response() -> ResponseTemplate {
    ResponseTemplate::new(401).set_body_json(serde_json::json!({
        "error": {
            "message": "Invalid API key",
            "type": "authentication_error"
        }
    }))
}

/// Create 429 rate limit error response
///
/// Returns a ResponseTemplate with retry-after header.
pub fn create_rate_limit_response() -> ResponseTemplate {
    ResponseTemplate::new(429)
        .insert_header("retry-after", "60")
        .set_body_json(serde_json::json!({
            "error": {
                "message": "Rate limit exceeded",
                "type": "rate_limit_error"
            }
        }))
}

/// Create 500 server error response
///
/// Returns a ResponseTemplate for internal server errors.
pub fn create_server_error_response() -> ResponseTemplate {
    ResponseTemplate::new(500).set_body_json(serde_json::json!({
        "error": {
            "message": "Internal server error",
            "type": "server_error"
        }
    }))
}
