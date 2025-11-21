//! Unit Tests for OpenAI Provider HTTP Integration
//!
//! UNIT UNDER TEST: OpenAIProvider HTTP request handling
//!
//! BUSINESS RESPONSIBILITY:
//!   - Execute HTTP requests to OpenAI API with authentication
//!   - Handle successful responses and parse into unified format
//!   - Handle API errors (401, 429, 500)
//!   - Apply retry logic for transient failures
//!   - Convert UnifiedMessage to OpenAI format
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
use multi_llm::config::{DefaultLLMParams, OpenAIConfig};
use multi_llm::core_types::executor::ExecutorLLMProvider;
use multi_llm::core_types::messages::{
    MessageAttributes, MessageCategory, MessageContent, MessageRole, UnifiedLLMRequest,
    UnifiedMessage,
};
use multi_llm::providers::openai::OpenAIProvider;
use multi_llm::retry::RetryPolicy;
use std::collections::HashMap;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_config(base_url: String) -> OpenAIConfig {
    OpenAIConfig {
        api_key: Some("test-key".to_string()),
        base_url,
        default_model: "gpt-4".to_string(),
        max_context_tokens: 128_000,
        retry_policy: RetryPolicy {
            max_attempts: 2,
            initial_delay: std::time::Duration::from_millis(10),
            max_delay: std::time::Duration::from_millis(50),
            backoff_multiplier: 2.0,
            total_timeout: std::time::Duration::from_secs(10),
            request_timeout: std::time::Duration::from_secs(5),
        },
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
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "Hello!"
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15
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
    let config = create_test_config("https://api.openai.com".to_string());
    let params = create_default_params();

    let result = OpenAIProvider::new(config, params);

    assert!(result.is_ok(), "Should initialize with valid config");
}

#[test]
fn test_provider_new_without_api_key() {
    let mut config = create_test_config("https://api.openai.com".to_string());
    config.api_key = None;
    let params = create_default_params();

    let result = OpenAIProvider::new(config, params);

    assert!(result.is_err(), "Should fail without API key");
}

// ============================================================================
// HTTP Request Tests
// ============================================================================

#[tokio::test]
async fn test_execute_request_success() {
    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    let response = create_success_response();

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .mount(&mock_server)
        .await;

    let provider = OpenAIProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_ok(), "Request should succeed");
    let (response, _events) = result.unwrap();
    assert!(response.usage.is_some(), "Should have usage data");
}

#[tokio::test]
async fn test_handle_401_authentication_error() {
    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    let error_body = serde_json::json!({
        "error": {
            "message": "Invalid API key",
            "type": "invalid_request_error"
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_json(&error_body))
        .mount(&mock_server)
        .await;

    let provider = OpenAIProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_err(), "Should fail with authentication error");
}

#[tokio::test]
async fn test_handle_429_rate_limit_error() {
    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    let error_body = serde_json::json!({
        "error": {
            "message": "Rate limit exceeded",
            "type": "rate_limit_error"
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "30")
                .set_body_json(&error_body),
        )
        .mount(&mock_server)
        .await;

    let provider = OpenAIProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_err(), "Should fail with rate limit error");
}

#[tokio::test]
async fn test_handle_500_server_error() {
    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    let error_body = serde_json::json!({
        "error": {
            "message": "Internal server error",
            "type": "server_error"
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_json(&error_body))
        .mount(&mock_server)
        .await;

    let provider = OpenAIProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_err(), "Should fail with server error");
}

#[tokio::test]
async fn test_handle_invalid_json_response() {
    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
        .mount(&mock_server)
        .await;

    let provider = OpenAIProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_err(), "Should fail with parsing error");
}

#[tokio::test]
async fn test_handle_network_failure() {
    let config = create_test_config("http://localhost:1".to_string());
    let params = create_default_params();

    let provider = OpenAIProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_err(), "Should fail with network error");
}

#[tokio::test]
async fn test_request_includes_authentication_headers() {
    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    let response = create_success_response();

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = OpenAIProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_ok(), "Request with headers should succeed");
}
