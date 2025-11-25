// Simplified Trait Compliance Tests for LLM Providers
//
// PURPOSE: Ensures architectural consistency across ALL LLM provider implementations
// FOCUS: Tests the 3 core LlmProvider methods following KISS principles
//
// BUSINESS RESPONSIBILITY:
//   - Validates that ALL providers implement LlmProvider trait consistently
//   - Tests the 3 core methods: execute_llm, execute_structured_llm, provider_name
//   - Ensures error handling produces consistent error types for same failure scenarios

use crate::Message;
use crate::provider::LlmProvider;
use serde_json::json;
use std::sync::Arc;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Create all provider implementations for trait compliance testing
async fn create_all_provider_implementations(
) -> (MockServer, Vec<(String, Arc<dyn LlmProvider>)>) {
    let mock_server = MockServer::start().await;

    // Use integration test helpers to create UnifiedLLMClient instances
    let openai_client =
        crate::tests::integration::create_integration_openai_client(&mock_server).await;
    let anthropic_client =
        crate::tests::integration::create_integration_anthropic_client(&mock_server).await;
    let lmstudio_client =
        crate::tests::integration::create_integration_lmstudio_client(&mock_server).await;

    (
        mock_server,
        vec![
            (
                "OpenAI".to_string(),
                Arc::new(openai_client) as Arc<dyn LlmProvider>,
            ),
            (
                "Anthropic".to_string(),
                Arc::new(anthropic_client) as Arc<dyn LlmProvider>,
            ),
            (
                "LM Studio".to_string(),
                Arc::new(lmstudio_client) as Arc<dyn LlmProvider>,
            ),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_all_providers_basic_interface_compliance() {
        // Test verifies ALL provider implementations expose consistent provider identification
        // Simplified to focus on the core LlmProvider interface

        // Arrange: Create all provider implementations
        let (_mock_server, providers) = create_all_provider_implementations().await;

        // Act & Assert: Test provider name consistency
        for (expected_name, provider) in providers {
            // Test provider identification
            let provider_name = provider.provider_name();
            assert!(
                !provider_name.is_empty(),
                "Provider {} must return non-empty provider name",
                expected_name
            );

            // Verify the provider name makes sense
            assert!(
                provider_name.len() > 2,
                "Provider {} name should be meaningful, got: {}",
                expected_name,
                provider_name
            );
        }
    }

    #[tokio::test]
    async fn test_all_providers_execute_llm_compliance() {
        // Test verifies ALL providers implement execute_llm consistently
        // This is one of the 3 core methods in LlmProvider

        // Arrange: Create all provider implementations and set up mocks
        let (mock_server, providers) = create_all_provider_implementations().await;

        // Mock successful responses for OpenAI-compatible endpoints (OpenAI and LM Studio)
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "Test response"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 5,
                    "total_tokens": 15
                }
            })))
            .mount(&mock_server)
            .await;

        // Mock for Anthropic endpoints
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "msg_01ABC123",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "Test response"}],
                "model": "claude-3-5-sonnet-20241022",
                "stop_reason": "end_turn",
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 5
                }
            })))
            .mount(&mock_server)
            .await;

        // Act & Assert: Test each provider's execute_llm method
        for (provider_name, provider) in providers {
            let messages = vec![Message::user("Test message")];
            let result = provider.execute_llm(messages, None).await;

            assert!(
                result.is_ok(),
                "Provider {} failed execute_llm compliance: {}",
                provider_name,
                result.unwrap_err()
            );

            let response = result.unwrap();
            assert!(
                !response.content.is_empty(),
                "Provider {} should return non-empty content",
                provider_name
            );
        }
    }

    #[tokio::test]
    async fn test_all_providers_execute_structured_llm_compliance() {
        // Test verifies ALL providers implement execute_structured_llm consistently
        // This is the second of the 3 core methods in LlmProvider

        // Arrange: Create all provider implementations and set up mocks
        let (mock_server, providers) = create_all_provider_implementations().await;

        let test_schema = json!({
            "type": "object",
            "properties": {
                "answer": {"type": "string"},
                "confidence": {"type": "number"}
            },
            "required": ["answer"]
        });

        // Mock structured responses for OpenAI-compatible providers
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "{\"answer\": \"Test answer\", \"confidence\": 0.9}"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 15,
                    "completion_tokens": 10,
                    "total_tokens": 25
                }
            })))
            .mount(&mock_server)
            .await;

        // Mock structured responses for Anthropic
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "msg_01ABC124",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "{\"answer\": \"Test answer\", \"confidence\": 0.9}"}],
                "model": "claude-3-5-sonnet-20241022",
                "stop_reason": "end_turn",
                "usage": {
                    "input_tokens": 15,
                    "output_tokens": 10
                }
            })))
            .mount(&mock_server)
            .await;

        // Act & Assert: Test each provider's execute_structured_llm method
        for (provider_name, provider) in providers {
            let messages = vec![Message::user("Give me a structured answer")];
            let result = provider
                .execute_structured_llm(messages, test_schema.clone(), None)
                .await;

            assert!(
                result.is_ok(),
                "Provider {} failed execute_structured_llm compliance: {}",
                provider_name,
                result.unwrap_err()
            );

            let response = result.unwrap();
            assert!(
                response.structured_response.is_some(),
                "Provider {} should return structured data",
                provider_name
            );
        }
    }
}
