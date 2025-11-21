//! Unit Tests for Anthropic Provider HTTP Integration
//!
//! UNIT UNDER TEST: AnthropicProvider HTTP request handling
//!
//! BUSINESS RESPONSIBILITY:
//!   - Execute HTTP requests to Anthropic API with authentication
//!   - Handle successful responses and parse into unified format
//!   - Handle API errors (401, 429, 500)
//!   - Apply retry logic for transient failures
//!   - Convert UnifiedMessage to Anthropic format
//!   - Emit business events for LLM interactions
//!
//! TEST COVERAGE:
//!   - Provider initialization with valid/invalid config
//!   - Successful API requests and response parsing
//!   - Authentication errors (401)
//!   - Rate limiting errors (429)
//!   - Server errors (500)
//!   - Network failures
//!   - Message conversion and tool handling

use chrono::Utc;
use multi_llm::config::{AnthropicConfig, DefaultLLMParams};
use multi_llm::core_types::executor::ExecutorLLMProvider;
use multi_llm::core_types::messages::{
    MessageAttributes, MessageCategory, MessageContent, MessageRole, UnifiedLLMRequest,
    UnifiedMessage,
};
use multi_llm::error::LlmError;
use multi_llm::providers::anthropic::AnthropicProvider;
use multi_llm::retry::RetryPolicy;
use std::collections::HashMap;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// Import shared test helpers
mod common;

fn create_test_config(base_url: String) -> AnthropicConfig {
    AnthropicConfig {
        api_key: Some("test-key".to_string()),
        base_url,
        default_model: "claude-3-5-sonnet-20241022".to_string(),
        max_context_tokens: 200_000,
        retry_policy: RetryPolicy {
            max_attempts: 2, // Reduced for faster tests
            initial_delay: std::time::Duration::from_millis(10),
            max_delay: std::time::Duration::from_millis(50),
            backoff_multiplier: 2.0,
            total_timeout: std::time::Duration::from_secs(10),
            request_timeout: std::time::Duration::from_secs(5),
        },
        enable_prompt_caching: false,
        cache_ttl: "5m".to_string(),
    }
}

fn create_default_params() -> DefaultLLMParams {
    DefaultLLMParams {
        temperature: 0.7,
        max_tokens: 1000,
        top_p: 1.0,
        top_k: 40,
        min_p: 0.0,
        presence_penalty: 0.0,
    }
}

fn create_test_message(content: &str) -> UnifiedMessage {
    UnifiedMessage {
        role: MessageRole::User,
        content: MessageContent::Text(content.to_string()),
        attributes: MessageAttributes {
            priority: 0,
            cacheable: false,
            cache_key: None,
            category: MessageCategory::Current,
            metadata: HashMap::new(),
        },
        timestamp: Utc::now(),
    }
}

fn create_success_response() -> serde_json::Value {
    serde_json::json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "text",
            "text": "Hello!"
        }],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 10,
            "output_tokens": 5
        }
    })
}

fn create_llm_request() -> UnifiedLLMRequest {
    UnifiedLLMRequest {
        messages: vec![create_test_message("Hello")],
        response_schema: None,
        config: None,
    }
}

// ============================================================================
// Provider Initialization Tests
// ============================================================================

#[test]
fn test_provider_new_with_valid_config() {
    // Test provider initialization with valid configuration
    // Verifies that provider can be created with proper config

    let config = create_test_config("https://api.anthropic.com".to_string());
    let params = create_default_params();

    let result = AnthropicProvider::new(config, params);

    assert!(result.is_ok(), "Should initialize with valid config");
}

#[test]
fn test_provider_new_without_api_key() {
    // Test provider initialization fails without API key
    // Verifies that missing API key is caught during initialization

    let mut config = create_test_config("https://api.anthropic.com".to_string());
    config.api_key = None;
    let params = create_default_params();

    let result = AnthropicProvider::new(config, params);

    assert!(result.is_err(), "Should fail without API key");
    match result.unwrap_err() {
        LlmError::ConfigurationError { message } => {
            assert!(message.contains("API key"), "Error should mention API key");
        }
        other => panic!("Expected ConfigurationError, got: {:?}", other),
    }
}

// ============================================================================
// HTTP Request Tests
// ============================================================================

#[tokio::test]
async fn test_execute_request_success() {
    // Test successful HTTP request to Anthropic API
    // Verifies end-to-end request execution and response parsing

    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    let response = create_success_response();

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .mount(&mock_server)
        .await;

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_ok(), "Request should succeed");
    let (response, _events) = result.unwrap();
    assert!(response.usage.is_some(), "Should have usage data");
    assert!(
        response.usage.unwrap().total_tokens > 0,
        "Should have non-zero tokens"
    );
}

#[tokio::test]
async fn test_handle_401_authentication_error() {
    // Test handling of authentication failures (401)
    // Verifies that invalid API keys result in authentication errors

    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    let error_body = serde_json::json!({
        "type": "error",
        "error": {
            "type": "authentication_error",
            "message": "Invalid API key"
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_json(&error_body))
        .mount(&mock_server)
        .await;

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_err(), "Should fail with authentication error");
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Authentication failed") || error.to_string().contains("401"),
        "Error should indicate authentication failure: {}",
        error
    );
}

#[tokio::test]
async fn test_handle_429_rate_limit_error() {
    // Test handling of rate limit errors (429)
    // Verifies that rate limits are properly detected and reported

    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    let error_body = serde_json::json!({
        "type": "error",
        "error": {
            "type": "rate_limit_error",
            "message": "Rate limit exceeded"
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "30")
                .set_body_json(&error_body),
        )
        .mount(&mock_server)
        .await;

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_err(), "Should fail with rate limit error");
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Rate limit") || error.to_string().contains("429"),
        "Error should indicate rate limiting: {}",
        error
    );
}

#[tokio::test]
async fn test_handle_500_server_error() {
    // Test handling of server errors (500)
    // Verifies that server failures are properly reported

    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    let error_body = serde_json::json!({
        "type": "error",
        "error": {
            "type": "api_error",
            "message": "Internal server error"
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_json(&error_body))
        .mount(&mock_server)
        .await;

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_err(), "Should fail with server error");
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("500") || error.to_string().contains("server"),
        "Error should indicate server failure: {}",
        error
    );
}

#[tokio::test]
async fn test_handle_invalid_json_response() {
    // Test handling of malformed JSON responses
    // Verifies that parsing errors are properly detected

    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
        .mount(&mock_server)
        .await;

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_err(), "Should fail with parsing error");
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("parsing") || error.to_string().contains("invalid"),
        "Error should indicate parsing failure: {}",
        error
    );
}

#[tokio::test]
async fn test_handle_network_failure() {
    // Test handling of network connection failures
    // Verifies that connection errors are properly reported

    let config = create_test_config("http://localhost:1".to_string()); // Invalid URL
    let params = create_default_params();

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_err(), "Should fail with network error");
    // Network error occurred - test passes
}

#[tokio::test]
async fn test_request_includes_authentication_headers() {
    // Test that requests include proper authentication headers
    // Verifies that x-api-key and anthropic-version headers are set

    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    let response = create_success_response();

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .and(header("anthropic-version", "2023-06-01"))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .expect(1) // Verify headers were present
        .mount(&mock_server)
        .await;

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_ok(), "Request with headers should succeed");
}
