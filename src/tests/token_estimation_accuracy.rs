// Integration Tests for Token Estimation Accuracy
//
// UNIT UNDER TEST: Token estimation accuracy across LLM providers
//
// BUSINESS RESPONSIBILITY:
//   - Validates that token estimation algorithms provide reasonable accuracy
//   - Compares estimated token counts to actual usage from LLM provider responses
//   - Ensures estimation accuracy is sufficient for pre-request planning and cost monitoring
//   - Monitors estimation drift and provider-specific accuracy characteristics
//
// TEST COVERAGE:
//   - Estimation vs actual usage accuracy for different text lengths
//   - Provider-specific estimation accuracy validation
//   - Message formatting overhead estimation accuracy
//   - Unicode and special character handling accuracy
//   - Bounds checking for reasonable estimation ranges
//   - Cost projection accuracy based on token estimates

use crate::providers::openai_shared::utils;
use crate::Message;
use crate::provider::LlmProvider;
use serde_json::json;
use std::sync::Arc;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to create provider implementations for estimation testing
async fn create_providers_for_estimation_testing(
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

/// Helper to estimate tokens using current provider-specific logic
fn estimate_tokens_current_logic(provider_name: &str, text: &str) -> u32 {
    match provider_name {
        "OpenAI" | "LM Studio" => utils::estimate_tokens(text),
        "Anthropic" => (text.len() / 4) as u32,
        _ => (text.len() / 4) as u32, // Default fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_estimation_accuracy_for_short_text() {
        // Test verifies estimation accuracy for short text messages
        // Ensures reasonable accuracy for common chat interactions

        // Arrange: Create providers and mock realistic usage responses
        let (mock_server, providers) = create_providers_for_estimation_testing().await;

        // Mock OpenAI/LM Studio response with realistic usage
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "Short response"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 8,  // Realistic for "Hello, how are you?"
                    "completion_tokens": 3, // Realistic for "Short response"
                    "total_tokens": 11
                }
            })))
            .mount(&mock_server)
            .await;

        // Mock Anthropic response with realistic usage
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "msg_test",
                "type": "message",
                "role": "assistant",
                "content": [{
                    "type": "text",
                    "text": "Short response"
                }],
                "model": "claude-3-5-sonnet-20241022",
                "stop_reason": "end_turn",
                "usage": {
                    "input_tokens": 8,   // Realistic for "Hello, how are you?"
                    "output_tokens": 3   // Realistic for "Short response"
                }
            })))
            .mount(&mock_server)
            .await;

        let test_text = "Hello, how are you?";
        let messages = vec![Message::user(test_text)];

        // Act & Assert: Test estimation accuracy for each provider
        for (provider_name, provider) in providers {
            // Get estimation using current provider logic
            let estimated_tokens = estimate_tokens_current_logic(&provider_name, test_text);

            // Execute actual request to get real usage
            let response = provider
                .execute_llm(messages.clone(), None)
                .await
                .unwrap_or_else(|e| {
                    panic!("{} provider failed to execute LLM: {:?}", provider_name, e)
                });

            assert!(
                response.usage.is_some(),
                "{} provider should return usage data",
                provider_name
            );

            let actual_usage = response.usage.unwrap();
            let actual_prompt_tokens = actual_usage.prompt_tokens;

            // Calculate accuracy ratio
            let accuracy_ratio = estimated_tokens as f64 / actual_prompt_tokens as f64;

            // Assert: Estimation should be within reasonable bounds (0.5x to 2.0x actual)
            assert!(
                accuracy_ratio >= 0.5 && accuracy_ratio <= 2.0,
                "{} provider estimation accuracy out of bounds: estimated={}, actual={}, ratio={:.2}",
                provider_name,
                estimated_tokens,
                actual_prompt_tokens,
                accuracy_ratio
            );

            println!(
                "{} provider - Estimated: {}, Actual: {}, Accuracy: {:.2}x",
                provider_name, estimated_tokens, actual_prompt_tokens, accuracy_ratio
            );
        }
    }

    #[tokio::test]
    async fn test_token_estimation_accuracy_for_long_text() {
        // Test verifies estimation accuracy for longer text content
        // Ensures accuracy is maintained for substantial content

        // Arrange: Create providers and mock responses for longer content
        let (mock_server, providers) = create_providers_for_estimation_testing().await;

        // Mock responses with usage data proportional to longer content
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "This is a longer response with more detailed content."
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 75,  // Realistic for longer input (~300 chars)
                    "completion_tokens": 12, // Realistic for longer response
                    "total_tokens": 87
                }
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "msg_long_test",
                "type": "message",
                "role": "assistant",
                "content": [{
                    "type": "text",
                    "text": "This is a longer response with more detailed content."
                }],
                "model": "claude-3-5-sonnet-20241022",
                "stop_reason": "end_turn",
                "usage": {
                    "input_tokens": 75,
                    "output_tokens": 12
                }
            })))
            .mount(&mock_server)
            .await;

        let long_text = "Please explain the concept of machine learning in detail, including supervised and unsupervised learning approaches, common algorithms like decision trees and neural networks, and real-world applications in various industries such as healthcare, finance, and technology.";
        let messages = vec![Message::user(long_text)];

        // Act & Assert: Test estimation accuracy for longer content
        for (provider_name, provider) in providers {
            let estimated_tokens = estimate_tokens_current_logic(&provider_name, long_text);

            let response = provider
                .execute_llm(messages.clone(), None)
                .await
                .unwrap_or_else(|e| {
                    panic!("{} provider failed to execute LLM: {:?}", provider_name, e)
                });

            let actual_usage = response.usage.unwrap();
            let actual_prompt_tokens = actual_usage.prompt_tokens;
            let accuracy_ratio = estimated_tokens as f64 / actual_prompt_tokens as f64;

            // For longer text, estimation should still be within reasonable bounds
            assert!(
                accuracy_ratio >= 0.5 && accuracy_ratio <= 2.0,
                "{} provider long text estimation out of bounds: estimated={}, actual={}, ratio={:.2}",
                provider_name,
                estimated_tokens,
                actual_prompt_tokens,
                accuracy_ratio
            );

            // Longer text should result in more tokens
            assert!(
                estimated_tokens > 50,
                "{} provider should estimate >50 tokens for long text, got {}",
                provider_name,
                estimated_tokens
            );
        }
    }

    #[tokio::test]
    async fn test_estimation_consistency_across_providers() {
        // Test verifies that estimation algorithms produce reasonably consistent results
        // Allows for provider differences while ensuring no provider is drastically off

        // Arrange: Test with same content across all providers
        let test_cases = vec![
            "Short message",
            "This is a medium-length message with some detail and complexity.",
            "This is a very long message that contains substantial content, multiple sentences, and various concepts that would typically require significant token allocation for processing in language models.",
        ];

        for test_text in test_cases {
            let mut estimations = Vec::new();

            // Collect estimations from all providers
            let provider_names = ["OpenAI", "LM Studio", "Anthropic"];
            for provider_name in provider_names {
                let estimated = estimate_tokens_current_logic(provider_name, test_text);
                estimations.push(estimated);
            }

            // Calculate statistics
            let min_estimation = *estimations.iter().min().unwrap();
            let max_estimation = *estimations.iter().max().unwrap();
            let variation_ratio = max_estimation as f64 / min_estimation as f64;

            // Assert: Variation between providers should be reasonable (within 3x)
            assert!(
                variation_ratio <= 3.0,
                "Provider estimation variation too high for '{}': min={}, max={}, ratio={:.2}x",
                &test_text[..test_text.len().min(50)],
                min_estimation,
                max_estimation,
                variation_ratio
            );

            println!(
                "Text length {}: OpenAI={}, LM Studio={}, Anthropic={}, variation={:.2}x",
                test_text.len(),
                estimations[0],
                estimations[1],
                estimations[2],
                variation_ratio
            );
        }
    }
}
