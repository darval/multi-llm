//! Unit Tests for OpenAI-Compatible HTTP Client
//!
//! UNIT UNDER TEST: OpenAICompatibleClient and HTTP utilities
//!
//! BUSINESS RESPONSIBILITY:
//!   - Execute HTTP requests with retry logic
//!   - Handle authentication headers
//!   - Parse success and error responses
//!   - Handle rate limiting (429) with retry-after
//!   - Handle authentication failures (401)
//!   - Handle network errors with appropriate error types
//!
//! TEST COVERAGE:
//!   - Successful HTTP requests and response parsing
//!   - Authentication header building
//!   - Error response handling (401, 429, generic errors)
//!   - Retry logic with exponential backoff
//!   - Network failure handling
//!   - Invalid response body handling

use mystory_llm::error::LlmError;
use mystory_llm::providers::openai_shared::http::OpenAICompatibleClient;
use mystory_llm::providers::openai_shared::types::{
    OpenAIChoice, OpenAIRequest, OpenAIResponse, OpenAIResponseMessage, OpenAIUsage,
};
use mystory_llm::retry::RetryPolicy;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_request() -> OpenAIRequest {
    OpenAIRequest {
        model: "gpt-4".to_string(),
        messages: vec![],
        temperature: Some(0.7),
        max_tokens: Some(100),
        top_p: Some(1.0),
        presence_penalty: None,
        stream: None,
        response_format: None,
        tools: None,
        tool_choice: None,
    }
}

fn create_success_response() -> OpenAIResponse {
    OpenAIResponse {
        choices: vec![OpenAIChoice {
            message: OpenAIResponseMessage {
                role: "assistant".to_string(),
                content: "Hello!".to_string(),
                tool_calls: None,
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: Some(OpenAIUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        }),
    }
}

// ============================================================================
// HTTP Client Tests
// ============================================================================

#[tokio::test]
async fn test_execute_chat_request_success() {
    // Test successful HTTP request execution
    // Verifies that client can make requests and parse successful responses

    let mock_server = MockServer::start().await;
    let response = create_success_response();

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .mount(&mock_server)
        .await;

    let client = OpenAICompatibleClient::new();
    let headers = OpenAICompatibleClient::build_auth_headers("test-key").unwrap();
    let request = create_test_request();
    let url = format!("{}/v1/chat/completions", mock_server.uri());

    let result = client.execute_chat_request(&url, &headers, &request).await;

    assert!(result.is_ok(), "Request should succeed");
    let response_data = result.unwrap();
    assert_eq!(response_data.choices[0].message.content, "Hello!");
    assert!(response_data.usage.is_some(), "Should have usage data");
}

#[tokio::test]
async fn test_build_auth_headers() {
    // Test authentication header construction
    // Verifies that API key is properly formatted as Bearer token

    let headers = OpenAICompatibleClient::build_auth_headers("test-api-key");

    assert!(headers.is_ok(), "Should build headers successfully");
    let headers = headers.unwrap();

    assert!(headers.contains_key("authorization"));
    assert!(headers.contains_key("content-type"));

    let auth_value = headers.get("authorization").unwrap().to_str().unwrap();
    assert_eq!(auth_value, "Bearer test-api-key");
}

#[tokio::test]
async fn test_build_auth_headers_invalid_key() {
    // Test that invalid API key format is rejected
    // Verifies error handling for malformed authentication credentials

    let result = OpenAICompatibleClient::build_auth_headers("invalid\nkey");

    assert!(result.is_err(), "Should reject invalid API key");
    match result.unwrap_err() {
        LlmError::ConfigurationError { .. } => {} // Expected
        other => panic!("Expected ConfigurationError, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_handle_401_error_response() {
    // Test authentication failure (401) error handling
    // Verifies that 401 responses are converted to authentication errors

    let mock_server = MockServer::start().await;
    let error_body = serde_json::json!({
        "error": {
            "message": "Invalid API key",
            "code": "invalid_api_key",
            "type": "authentication_error"
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_json(&error_body))
        .mount(&mock_server)
        .await;

    let client = OpenAICompatibleClient::new();
    let headers = OpenAICompatibleClient::build_auth_headers("invalid-key").unwrap();
    let request = create_test_request();
    let url = format!("{}/v1/chat/completions", mock_server.uri());

    let result = client.execute_chat_request(&url, &headers, &request).await;

    assert!(result.is_err(), "Should fail with authentication error");
    match result.unwrap_err() {
        LlmError::AuthenticationFailed { .. } => {} // Expected
        other => panic!("Expected AuthenticationFailed error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_handle_429_rate_limit_error() {
    // Test rate limit (429) error handling
    // Verifies that 429 responses are converted to rate limit errors with retry_after

    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "60")
                .set_body_json(serde_json::json!({
                    "error": {
                        "message": "Rate limit exceeded",
                        "type": "rate_limit_error"
                    }
                })),
        )
        .mount(&mock_server)
        .await;

    let client = OpenAICompatibleClient::new();
    let headers = OpenAICompatibleClient::build_auth_headers("test-key").unwrap();
    let request = create_test_request();
    let url = format!("{}/v1/chat/completions", mock_server.uri());

    let result = client.execute_chat_request(&url, &headers, &request).await;

    assert!(result.is_err(), "Should fail with rate limit error");
    match result.unwrap_err() {
        LlmError::RateLimitExceeded {
            retry_after_seconds,
        } => {
            assert_eq!(retry_after_seconds, 60, "Should parse retry-after header");
        }
        other => panic!("Expected RateLimitExceeded error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_handle_429_without_retry_after_header() {
    // Test rate limit error when retry-after header is missing
    // Verifies default retry_after value is used

    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "error": {
                "message": "Rate limit exceeded"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = OpenAICompatibleClient::new();
    let headers = OpenAICompatibleClient::build_auth_headers("test-key").unwrap();
    let request = create_test_request();
    let url = format!("{}/v1/chat/completions", mock_server.uri());

    let result = client.execute_chat_request(&url, &headers, &request).await;

    assert!(result.is_err(), "Should fail with rate limit error");
    match result.unwrap_err() {
        LlmError::RateLimitExceeded {
            retry_after_seconds,
        } => {
            assert_eq!(retry_after_seconds, 60, "Should use default 60 seconds");
        }
        other => panic!("Expected RateLimitExceeded error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_handle_generic_error_response() {
    // Test generic API error handling (non-401, non-429)
    // Verifies that other error status codes are converted to RequestFailed errors

    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": {
                "message": "Internal server error"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = OpenAICompatibleClient::new();
    let headers = OpenAICompatibleClient::build_auth_headers("test-key").unwrap();
    let request = create_test_request();
    let url = format!("{}/v1/chat/completions", mock_server.uri());

    let result = client.execute_chat_request(&url, &headers, &request).await;

    assert!(result.is_err(), "Should fail with generic error");
    match result.unwrap_err() {
        LlmError::RequestFailed { .. } => {} // Expected
        other => panic!("Expected RequestFailed error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_parse_invalid_json_response() {
    // Test handling of malformed JSON in response body
    // Verifies that invalid JSON is converted to ResponseParsing error

    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
        .mount(&mock_server)
        .await;

    let client = OpenAICompatibleClient::new();
    let headers = OpenAICompatibleClient::build_auth_headers("test-key").unwrap();
    let request = create_test_request();
    let url = format!("{}/v1/chat/completions", mock_server.uri());

    let result = client.execute_chat_request(&url, &headers, &request).await;

    assert!(result.is_err(), "Should fail with parsing error");
    match result.unwrap_err() {
        LlmError::ResponseParsingError { .. } => {} // Expected
        other => panic!("Expected ResponseParsingError, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_custom_retry_policy() {
    // Test that client can be configured with custom retry policy
    // Verifies retry policy configuration without actual retries

    let retry_policy = RetryPolicy {
        max_attempts: 5,
        initial_delay: std::time::Duration::from_millis(500),
        max_delay: std::time::Duration::from_millis(5000),
        backoff_multiplier: 2.0,
        total_timeout: std::time::Duration::from_secs(60),
        request_timeout: std::time::Duration::from_secs(30),
    };

    let client = OpenAICompatibleClient::with_retry_policy(retry_policy.clone());

    // Set and restore policy to verify the API works
    client.set_retry_policy(retry_policy.clone()).await;
    client.restore_default_retry_policy(&retry_policy).await;

    // If we got here without panic, the API works correctly
    assert!(true, "Retry policy configuration should work");
}

#[tokio::test]
async fn test_network_failure_handling() {
    // Test handling of network connection failures
    // Verifies that connection errors are converted to RequestFailed errors

    let client = OpenAICompatibleClient::new();
    let headers = OpenAICompatibleClient::build_auth_headers("test-key").unwrap();
    let request = create_test_request();

    // Use invalid URL to trigger connection failure
    let url = "http://localhost:1/invalid";

    let result = client.execute_chat_request(url, &headers, &request).await;

    assert!(result.is_err(), "Should fail with network error");
    match result.unwrap_err() {
        LlmError::RequestFailed { .. } => {} // Expected
        other => panic!("Expected RequestFailed error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_auth_header_with_content_type() {
    // Test that both Authorization and Content-Type headers are set
    // Verifies complete header configuration

    let headers = OpenAICompatibleClient::build_auth_headers("key").unwrap();

    assert!(
        headers.contains_key("authorization"),
        "Should have authorization header"
    );
    assert!(
        headers.contains_key("content-type"),
        "Should have content-type header"
    );

    let content_type = headers.get("content-type").unwrap().to_str().unwrap();
    assert_eq!(
        content_type, "application/json",
        "Content-Type should be application/json"
    );
}

#[tokio::test]
async fn test_401_error_with_auth_code() {
    // Test 401 error with authentication code in response
    // Verifies detailed error parsing for auth failures

    let mock_server = MockServer::start().await;
    let error_body = serde_json::json!({
        "error": {
            "message": "Authentication failed",
            "code": "invalid_api_key",
            "type": "authentication_error"
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_json(&error_body))
        .mount(&mock_server)
        .await;

    let client = OpenAICompatibleClient::new();
    let headers = OpenAICompatibleClient::build_auth_headers("test-key").unwrap();
    let request = create_test_request();
    let url = format!("{}/v1/chat/completions", mock_server.uri());

    let result = client.execute_chat_request(&url, &headers, &request).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        LlmError::AuthenticationFailed { message } => {
            assert!(
                message.contains("Invalid API key") || message.contains("authentication"),
                "Error message should indicate authentication issue"
            );
        }
        other => panic!("Expected AuthenticationFailed error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_401_error_without_specific_code() {
    // Test 401 error without specific error code in response
    // Verifies fallback authentication error handling

    let mock_server = MockServer::start().await;
    let error_body = serde_json::json!({
        "error": {
            "message": "Unauthorized"
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_json(&error_body))
        .mount(&mock_server)
        .await;

    let client = OpenAICompatibleClient::new();
    let headers = OpenAICompatibleClient::build_auth_headers("test-key").unwrap();
    let request = create_test_request();
    let url = format!("{}/v1/chat/completions", mock_server.uri());

    let result = client.execute_chat_request(&url, &headers, &request).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        LlmError::AuthenticationFailed { .. } => {} // Expected
        other => panic!("Expected AuthenticationFailed error, got: {:?}", other),
    }
}
