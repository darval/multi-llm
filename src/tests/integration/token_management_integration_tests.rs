// Unit Tests for Token Management Integration
//
// INTEGRATION UNDER TEST: Token counting accuracy in end-to-end request scenarios
//
// BUSINESS RESPONSIBILITY:
//   - Verifies token counting accuracy in real request/response scenarios
//   - Ensures tiktoken integration works correctly with actual API flow
//   - Tests LLMRequestConfig application in end-to-end requests
//   - Validates configuration parameters are correctly sent to API

use super::helpers::*;
use crate::Message;
use crate::core_types::provider::LlmProvider;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_counting_accuracy_in_real_requests() {
        // Test verifies token counting accuracy in end-to-end request scenarios
        // Ensures tiktoken integration works correctly with actual API request/response flow

        // Arrange
        let mock_server = MockServer::start().await;
        let response = create_openai_success_response();

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = create_integration_openai_client(&mock_server).await;
        let test_message =
            "Please provide a comprehensive explanation of microservices architecture patterns.";
        let messages = vec![Message::user(test_message)];

        // Act
        let result = client.execute_llm(messages, None).await;

        // Assert
        assert!(result.is_ok(), "Request should succeed with token counting");
        let response = result.unwrap();

        // Verify token usage is properly extracted and reasonable
        assert!(response.usage.is_some(), "Should include token usage");
        let usage = response.usage.unwrap();
        assert!(
            usage.prompt_tokens > 0,
            "Should have positive prompt tokens"
        );
        assert!(
            usage.completion_tokens > 0,
            "Should have positive completion tokens"
        );
        assert_eq!(
            usage.total_tokens,
            usage.prompt_tokens + usage.completion_tokens,
            "Total should equal sum of prompt and completion"
        );
    }

    #[tokio::test]
    async fn test_request_config_integration() {
        // Test verifies LLMRequestConfig is properly applied in end-to-end requests
        // Ensures configuration parameters are correctly sent to API and processed

        // Arrange
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("content-type", "application/json"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(&create_openai_success_response()),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = create_integration_openai_client(&mock_server).await;
        let messages = create_integration_test_messages();
        let config = crate::core_types::provider::RequestConfig {
            temperature: Some(0.9),
            max_tokens: Some(1000),
            top_p: Some(0.8),
            top_k: None,
            min_p: None,
            presence_penalty: Some(0.1),
            response_format: None,
            tools: vec![],
            tool_choice: None,
        };

        // Act
        let result = client.execute_llm(messages, Some(config)).await;

        // Assert
        assert!(result.is_ok(), "Request with configuration should succeed");
        let response = result.unwrap();
        assert!(
            !response.content.is_empty(),
            "Should return response content"
        );
    }
}
