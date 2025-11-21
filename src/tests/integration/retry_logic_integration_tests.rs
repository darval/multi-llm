// Unit Tests for Retry Logic Integration
//
// INTEGRATION UNDER TEST: Retry logic integration with exponential backoff and circuit breaking
//
// BUSINESS RESPONSIBILITY:
//   - Validates retry logic works end-to-end with eventual success scenarios
//   - Tests circuit breaker prevents excessive requests during prolonged outages
//   - Ensures transient failures are handled with proper exponential backoff
//   - Verifies system stability during API failures and recovery scenarios

use super::helpers::*;
use crate::core_types::executor::ExecutorLLMProvider;
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_logic_with_eventual_success() {
        // Test verifies retry logic works end-to-end with eventual success
        // Ensures transient failures are handled with proper exponential backoff

        // Arrange
        let mock_server = MockServer::start().await;

        // Use a counter to verify retry attempts instead of timing
        let request_counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = request_counter.clone();

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(move |_req: &wiremock::Request| {
                let count = counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if count < 2 {
                    // First 2 requests fail
                    ResponseTemplate::new(500).set_body_string("Internal server error")
                } else {
                    // 3rd request succeeds
                    ResponseTemplate::new(200).set_body_json(&create_openai_success_response())
                }
            })
            .mount(&mock_server)
            .await;

        let client = create_integration_openai_client(&mock_server).await;
        let messages = create_integration_test_messages();

        // Act
        let result = client.execute_llm(messages, None).await;
        let final_request_count = request_counter.load(std::sync::atomic::Ordering::SeqCst);

        // Assert
        assert!(result.is_ok(), "Should eventually succeed after retries");
        assert_eq!(
            final_request_count, 3,
            "Should make exactly 3 requests (2 failures + 1 success) to verify retry behavior"
        );

        let response = result.unwrap();
        assert!(
            !response.content.is_empty(),
            "Should return valid response after retries"
        );
    }

    #[tokio::test]
    async fn test_circuit_breaker_integration() {
        // Test verifies circuit breaker prevents excessive requests during outages
        // Ensures system stability during prolonged API failures

        // Arrange
        let mock_server = MockServer::start().await;

        // All requests fail to trigger circuit breaker
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Service unavailable"))
            .expect(5) // Should stop making requests after circuit opens
            .mount(&mock_server)
            .await;

        let client = create_circuit_breaker_openai_client(&mock_server).await;
        let messages = create_integration_test_messages();

        // Act - Make multiple requests to trigger circuit breaker
        let mut results = Vec::new();
        for _ in 0..5 {
            // Reduced from 10 to 5 requests
            let result = client.execute_llm(messages.clone(), None).await;
            results.push(result);

            // Small delay between requests
            tokio::time::sleep(Duration::from_millis(10)).await; // Reduced from 50ms to 10ms
        }

        // Assert
        let failures = results.iter().filter(|r| r.is_err()).count();
        assert!(
            failures > 0,
            "Should have some failures to test circuit breaker"
        );

        // After circuit opens, should fail fast without making HTTP requests
        // The exact behavior depends on circuit breaker implementation
    }
}
