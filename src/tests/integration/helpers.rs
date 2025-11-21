//! Helper functions for integration tests
//!
//! Common test utilities and data builders for integration testing.
//! These helpers support end-to-end testing across all LLM providers.

use crate::client::UnifiedLLMClient;
use crate::retry::RetryPolicy;
use crate::Message;
use serde_json::json;
use std::time::Duration;
use wiremock::MockServer;

/// Helper to create fast test retry policy to prevent slow integration tests
pub fn create_fast_test_retry_policy() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 3,
        initial_delay: Duration::from_millis(10),
        max_delay: Duration::from_millis(50),
        backoff_multiplier: 2.0,
        total_timeout: Duration::from_millis(500),
        request_timeout: Duration::from_millis(100),
    }
}

/// Helper to create circuit breaker test retry policy with low failure threshold
pub fn create_circuit_breaker_test_retry_policy() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 1, // Only 1 attempt per call to make circuit breaker behavior predictable
        initial_delay: Duration::from_millis(10),
        max_delay: Duration::from_millis(50),
        backoff_multiplier: 2.0,
        total_timeout: Duration::from_millis(500),
        request_timeout: Duration::from_millis(100),
    }
}

/// Helper to create OpenAI client for integration testing
pub async fn create_integration_openai_client(mock_server: &MockServer) -> UnifiedLLMClient {
    use crate::config::{DefaultLLMParams, LLMConfig, OpenAIConfig};

    let openai_config = OpenAIConfig {
        api_key: Some("test-key".to_string()),
        base_url: mock_server.uri(),
        retry_policy: create_fast_test_retry_policy(),
        ..OpenAIConfig::default()
    };

    let config = LLMConfig {
        provider: Box::new(openai_config),
        default_params: DefaultLLMParams::default(),
    };

    UnifiedLLMClient::from_config(config).unwrap()
}

/// Helper to create OpenAI client for circuit breaker testing
pub async fn create_circuit_breaker_openai_client(mock_server: &MockServer) -> UnifiedLLMClient {
    use crate::config::{DefaultLLMParams, LLMConfig, OpenAIConfig};

    let openai_config = OpenAIConfig {
        api_key: Some("test-key".to_string()),
        base_url: mock_server.uri(),
        retry_policy: create_circuit_breaker_test_retry_policy(),
        ..OpenAIConfig::default()
    };

    let config = LLMConfig {
        provider: Box::new(openai_config),
        default_params: DefaultLLMParams::default(),
    };

    UnifiedLLMClient::from_config(config).unwrap()
}

/// Helper to create Anthropic client for integration testing
pub async fn create_integration_anthropic_client(mock_server: &MockServer) -> UnifiedLLMClient {
    use crate::config::{AnthropicConfig, DefaultLLMParams, LLMConfig};

    let anthropic_config = AnthropicConfig {
        api_key: Some("test-key".to_string()),
        base_url: mock_server.uri(),
        retry_policy: create_fast_test_retry_policy(),
        ..AnthropicConfig::default()
    };

    let config = LLMConfig {
        provider: Box::new(anthropic_config),
        default_params: DefaultLLMParams::default(),
    };

    UnifiedLLMClient::from_config(config).unwrap()
}

/// Helper to create LM Studio client for integration testing
pub async fn create_integration_lmstudio_client(mock_server: &MockServer) -> UnifiedLLMClient {
    use crate::config::{DefaultLLMParams, LLMConfig, LMStudioConfig};

    let lmstudio_config = LMStudioConfig {
        base_url: mock_server.uri(),
        retry_policy: create_fast_test_retry_policy(),
        ..LMStudioConfig::default()
    };

    let config = LLMConfig {
        provider: Box::new(lmstudio_config),
        default_params: DefaultLLMParams::default(),
    };

    UnifiedLLMClient::from_config(config).unwrap()
}

/// Helper to create realistic OpenAI success response
pub fn create_openai_success_response() -> serde_json::Value {
    json!({
        "id": "chatcmpl-test123",
        "object": "chat.completion",
        "created": 1699000000,
        "model": "gpt-4",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "This is a test response from the integration test mock server. The request was processed successfully."
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 45,
            "completion_tokens": 23,
            "total_tokens": 68
        }
    })
}

/// Helper to create realistic Anthropic success response
pub fn create_anthropic_success_response() -> serde_json::Value {
    json!({
        "id": "msg_test123",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "text",
            "text": "This is a test response from the Anthropic integration test. The Claude API processed your request successfully."
        }],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 42,
            "output_tokens": 28,
            "total_tokens": 70
        }
    })
}

/// Helper to create test messages for integration scenarios
pub fn create_integration_test_messages() -> Vec<Message> {
    vec![
        Message::user("Please explain the concept of integration testing in software development, focusing on API integration patterns."),
    ]
}
