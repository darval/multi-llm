// Unit Tests for End-to-End Success Flow Integration
//
// INTEGRATION UNDER TEST: Complete request flow from UnifiedLLMClient to HTTP mocks
//
// BUSINESS RESPONSIBILITY:
//   - Validates end-to-end LLM request processing from client API to HTTP responses
//   - Ensures proper token counting accuracy in real request/response scenarios
//   - Tests provider-specific API formatting and response parsing
//   - Verifies successful completion scenarios across all supported providers

use super::helpers::*;
use crate::core_types::executor::ExecutorLLMProvider;
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_openai_complete_request_response_cycle() {
        // Test verifies complete OpenAI integration from UnifiedLLMClient to HTTP mock
        // Ensures end-to-end request processing with realistic API responses and token counting

        // Arrange
        let mock_server = MockServer::start().await;
        let success_response = create_openai_success_response();

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&success_response))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = create_integration_openai_client(&mock_server).await;
        let messages = create_integration_test_messages();

        // Act
        let result = client.execute_llm(messages, None).await;

        // Assert
        assert!(
            result.is_ok(),
            "OpenAI integration should succeed with valid response"
        );
        let response = result.unwrap();
        assert!(
            !response.content.is_empty(),
            "Should return response content"
        );
        assert!(
            response.content.contains("integration test mock server"),
            "Should contain expected content"
        );
        assert!(
            response.usage.is_some(),
            "Should include token usage information"
        );

        let usage = response.usage.unwrap();
        assert_eq!(
            usage.prompt_tokens, 45,
            "Should parse prompt tokens from response"
        );
        assert_eq!(
            usage.completion_tokens, 23,
            "Should parse completion tokens from response"
        );
        assert_eq!(
            usage.total_tokens, 68,
            "Should parse total tokens from response"
        );
    }

    #[tokio::test]
    async fn test_anthropic_complete_request_response_cycle() {
        // Test verifies complete Anthropic integration from UnifiedLLMClient to HTTP mock
        // Ensures Claude API request processing with proper message formatting and token extraction

        // Arrange
        let mock_server = MockServer::start().await;
        let success_response = create_anthropic_success_response();

        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("x-api-key", "test-key"))
            .and(header("anthropic-version", "2023-06-01"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&success_response))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = create_integration_anthropic_client(&mock_server).await;
        let messages = create_integration_test_messages();

        // Act
        let result = client.execute_llm(messages, None).await;

        // Assert
        assert!(
            result.is_ok(),
            "Anthropic integration should succeed with valid response"
        );
        let response = result.unwrap();
        assert!(
            !response.content.is_empty(),
            "Should return response content"
        );
        assert!(
            response.content.contains("Anthropic integration test"),
            "Should contain expected content"
        );
        assert!(
            response.usage.is_some(),
            "Should include token usage information"
        );

        let usage = response.usage.unwrap();
        assert_eq!(
            usage.prompt_tokens, 42,
            "Should parse input tokens from Anthropic response"
        );
        assert_eq!(
            usage.completion_tokens, 28,
            "Should parse output tokens from Anthropic response"
        );
        assert_eq!(
            usage.total_tokens, 70,
            "Should calculate total tokens correctly"
        );
    }

    #[tokio::test]
    async fn test_lmstudio_complete_request_response_cycle() {
        // Test verifies complete LM Studio integration with local model simulation
        // Ensures OpenAI-compatible local API request processing and response handling

        // Arrange
        let mock_server = MockServer::start().await;
        let success_response = json!({
            "id": "chatcmpl-local123",
            "object": "chat.completion",
            "created": 1699000000,
            "model": "local-model",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "This is a response from the local LM Studio model integration test. Local processing completed successfully."
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 38,
                "completion_tokens": 22,
                "total_tokens": 60
            }
        });

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&success_response))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = create_integration_lmstudio_client(&mock_server).await;
        let messages = create_integration_test_messages();

        // Act
        let result = client.execute_llm(messages, None).await;

        // Assert
        assert!(
            result.is_ok(),
            "LM Studio integration should succeed with valid response"
        );
        let response = result.unwrap();
        assert!(
            !response.content.is_empty(),
            "Should return response content"
        );
        assert!(
            response.content.contains("local LM Studio model"),
            "Should contain expected content"
        );
        assert!(
            response.usage.is_some(),
            "Should include token usage information"
        );

        let usage = response.usage.unwrap();
        assert_eq!(
            usage.total_tokens, 60,
            "Should parse token usage from local model response"
        );
    }
}
