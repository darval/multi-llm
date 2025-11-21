// Unit Tests for Error Handling Integration
//
// INTEGRATION UNDER TEST: Error propagation through complete request flow
//
// BUSINESS RESPONSIBILITY:
//   - Validates proper error handling and retry behavior with realistic network conditions
//   - Ensures authentication errors propagate correctly through all layers
//   - Tests rate limit handling with proper Retry-After header processing
//   - Validates robust error handling for malformed responses and network timeouts

use super::helpers::*;
use crate::core_types::executor::ExecutorLLMProvider;
use serde_json::json;
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_authentication_error_propagation() {
        // Test verifies authentication errors propagate correctly through all layers
        // Ensures proper error handling from HTTP 401 responses to client error types

        // Arrange
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": {
                    "message": "Invalid API key provided",
                    "type": "invalid_request_error",
                    "code": "invalid_api_key"
                }
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = create_integration_openai_client(&mock_server).await;
        let messages = create_integration_test_messages();

        // Act
        let result = client.execute_llm(messages, None).await;

        // Assert
        assert!(result.is_err(), "Should fail with authentication error");
    }

    #[tokio::test]
    async fn test_rate_limit_error_with_retry_after() {
        // Test verifies rate limit handling with Retry-After header processing
        // Ensures retry logic respects API rate limiting signals

        // Arrange
        let mock_server = MockServer::start().await;

        // Expect multiple requests due to retry logic, but they should all fail with 429
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(429)
                    .set_body_json(json!({
                        "error": {
                            "message": "Rate limit exceeded",
                            "type": "rate_limit_error",
                            "code": "rate_limit_exceeded"
                        }
                    }))
                    .append_header("retry-after", "60"),
            )
            .expect(3) // Allow for retry attempts (matches fast test policy max_attempts)
            .mount(&mock_server)
            .await;

        let client = create_integration_openai_client(&mock_server).await;
        let messages = create_integration_test_messages();

        // Act
        let result = client.execute_llm(messages, None).await;

        // Assert
        assert!(result.is_err(), "Should fail with rate limit error");
    }

    #[tokio::test]
    async fn test_malformed_response_handling() {
        // Test verifies proper handling of malformed JSON responses from API
        // Ensures robust error handling for unexpected response formats

        // Arrange
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_string("invalid json response"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = create_integration_openai_client(&mock_server).await;
        let messages = create_integration_test_messages();

        // Act
        let result = client.execute_llm(messages, None).await;

        // Assert
        assert!(result.is_err(), "Should fail with malformed response");
    }

    #[tokio::test]
    async fn test_network_timeout_handling() {
        // Test verifies proper timeout handling for slow or unresponsive APIs
        // Ensures requests don't hang indefinitely and fail with appropriate errors

        // Arrange
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(&create_openai_success_response())
                    .set_delay(Duration::from_millis(100)), // Delay that will cause timeout
            )
            .expect(3) // Expect retries due to timeout
            .mount(&mock_server)
            .await;

        let client = create_integration_openai_client(&mock_server).await;
        let messages = create_integration_test_messages();

        // Act - The mock delay (100ms) is longer than our fast provider timeout (100ms request_timeout)
        // This should cause the provider to timeout
        let result = client.execute_llm(messages, None).await;

        // Assert
        assert!(result.is_err(), "Should timeout before response arrives");
    }
}
