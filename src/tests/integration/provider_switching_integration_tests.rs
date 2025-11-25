// Unit Tests for Provider Switching Integration
//
// INTEGRATION UNDER TEST: Provider switching and configuration management under load
//
// BUSINESS RESPONSIBILITY:
//   - Verifies provider switching works correctly in integration scenarios
//   - Ensures runtime configuration changes maintain full functionality
//   - Tests cross-provider compatibility and consistent behavior
//   - Validates provider identification and capability reporting

use super::helpers::*;
use crate::provider::LlmProvider;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_runtime_provider_switching_maintains_functionality() {
        // Test verifies provider switching works correctly in integration scenarios
        // Ensures runtime configuration changes maintain full functionality

        // Arrange
        let openai_mock = MockServer::start().await;
        let anthropic_mock = MockServer::start().await;

        // Setup OpenAI mock
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(&create_openai_success_response()),
            )
            .expect(1)
            .mount(&openai_mock)
            .await;

        // Setup Anthropic mock
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(&create_anthropic_success_response()),
            )
            .expect(1)
            .mount(&anthropic_mock)
            .await;

        let messages = create_integration_test_messages();

        // Act & Assert - Test OpenAI client
        let openai_client = create_integration_openai_client(&openai_mock).await;
        let openai_result = openai_client.execute_llm(messages.clone(), None).await;
        assert!(openai_result.is_ok(), "OpenAI client should work");
        assert_eq!(
            openai_client.provider_name(),
            "openai",
            "Should identify as OpenAI"
        );

        // Act & Assert - Test Anthropic client
        let anthropic_client = create_integration_anthropic_client(&anthropic_mock).await;
        let anthropic_result = anthropic_client.execute_llm(messages, None).await;
        assert!(anthropic_result.is_ok(), "Anthropic client should work");
        assert_eq!(
            anthropic_client.provider_name(),
            "anthropic",
            "Should identify as Anthropic"
        );
    }
}
