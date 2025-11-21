// Integration Tests for Token Management System
//
// SYSTEMS UNDER TEST: TokenCounter implementations (OpenAITokenCounter, AnthropicTokenCounter)
//
// BUSINESS RESPONSIBILITY:
//   - Provides accurate token counting using tiktoken for different LLM providers
//   - Validates token limits before sending requests to prevent API errors
//   - Truncates text to fit within context windows while preserving meaning
//   - Manages provider-specific tokenization differences and context limits
//   - Enables pre-request validation and context management for optimal LLM performance
//
// TEST COVERAGE:
//   - Token counting accuracy for different text types (ASCII, Unicode, repeated)
//   - Message token counting with proper formatting overhead calculation
//   - Token limit validation with different context window sizes
//   - Text truncation with preservation of token boundaries
//   - Provider-specific tokenizer selection and model context limits
//   - Factory pattern for creating appropriate counters per provider
//   - Error handling for tokenizer initialization failures

// Integration tests need to import from the crate
use mystory_llm::error::LlmError;
use mystory_llm::tokens::{AnthropicTokenCounter, OpenAITokenCounter};

/// Helper function to create concrete OpenAI token counter for testing
fn create_concrete_openai_counter() -> OpenAITokenCounter {
    OpenAITokenCounter::new("gpt-4").unwrap()
}

/// Helper function to create concrete Anthropic token counter for testing
fn create_concrete_anthropic_counter() -> AnthropicTokenCounter {
    AnthropicTokenCounter::new("claude-3-5-sonnet").unwrap()
}

/// Helper function to create test messages for validation
fn create_test_messages() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "role": "user",
            "content": "What is the capital of France?"
        }),
        serde_json::json!({
            "role": "assistant",
            "content": "The capital of France is Paris."
        }),
    ]
}

#[cfg(test)]
mod openai_token_counter_tests {
    use super::*;
    use mystory_llm::tokens::TokenCounter;

    #[test]
    fn test_token_counting_accuracy_for_simple_text() {
        // Test verifies OpenAI token counter provides accurate counts for basic text
        // Ensures tiktoken integration works correctly for common use cases

        // Arrange
        let counter = create_concrete_openai_counter();
        let simple_text = "The quick brown fox jumps over the lazy dog";

        // Act
        let token_count = counter.count_tokens(simple_text).unwrap();

        // Assert
        assert!(
            token_count > 0,
            "Simple text should have positive token count"
        );
        assert!(
            token_count < 20,
            "Simple sentence should have reasonable token count"
        );
        assert_eq!(
            token_count, 9,
            "Expected tiktoken count for this specific text"
        );
    }

    #[test]
    fn test_token_counting_with_unicode_characters() {
        // Test verifies proper handling of international characters and emojis
        // Ensures tokenizer correctly handles multi-byte Unicode sequences

        // Arrange
        let counter = create_concrete_openai_counter();
        let unicode_text = "Hello 世界 🌍 नमस्ते مرحبا";

        // Act
        let token_count = counter.count_tokens(unicode_text).unwrap();

        // Assert
        assert!(
            token_count > 5,
            "Unicode text should require multiple tokens"
        );
        assert!(token_count < 50, "Should not over-tokenize Unicode text");
    }

    #[test]
    fn test_message_token_counting_includes_formatting_overhead() {
        // Test verifies message token counting includes OpenAI chat formatting
        // Ensures proper accounting for role markers and conversation structure

        // Arrange
        let counter = create_concrete_openai_counter();
        let messages = create_test_messages();

        // Act
        let message_tokens = counter.count_message_tokens(&messages).unwrap();
        let content_only_tokens = counter
            .count_tokens("What is the capital of France?The capital of France is Paris.")
            .unwrap();

        // Assert
        assert!(
            message_tokens > content_only_tokens,
            "Message format should add overhead tokens"
        );
        assert!(
            message_tokens > 15,
            "Should include formatting tokens for roles and structure"
        );
    }

    #[test]
    fn test_context_window_limits_for_different_models() {
        // Test verifies different GPT models have correct context window sizes
        // Ensures model-specific limits are properly configured

        // Arrange & Act
        let gpt4_counter = OpenAITokenCounter::new("gpt-4").unwrap();
        let gpt4_turbo_counter = OpenAITokenCounter::new("gpt-4-turbo").unwrap();
        let gpt35_counter = OpenAITokenCounter::new("gpt-3.5-turbo").unwrap();
        let o1_counter = OpenAITokenCounter::new("o1-preview").unwrap();

        // Assert
        assert_eq!(
            gpt4_counter.max_context_tokens(),
            8192,
            "GPT-4 should have 8k context"
        );
        assert_eq!(
            gpt4_turbo_counter.max_context_tokens(),
            128000,
            "GPT-4 Turbo should have 128k context"
        );
        assert_eq!(
            gpt35_counter.max_context_tokens(),
            4096,
            "GPT-3.5 should have 4k context"
        );
        assert_eq!(
            o1_counter.max_context_tokens(),
            200000,
            "o1 models should have 200k context"
        );
    }

    #[test]
    fn test_token_limit_validation_prevents_oversized_requests() {
        // Test verifies token limit validation catches requests that would exceed context
        // Ensures pre-request validation prevents API errors and wasted requests

        // Arrange
        let counter = OpenAITokenCounter::for_lm_studio(50).unwrap(); // Very small limit for testing
        let short_text = "Hello world";
        let long_text = "This is a very long text that should definitely exceed our small token limit for testing purposes. ".repeat(10);

        // Act & Assert
        assert!(
            counter.validate_token_limit(short_text).is_ok(),
            "Short text should pass validation"
        );
        assert!(
            counter.validate_token_limit(&long_text).is_err(),
            "Long text should fail validation"
        );

        // Verify error type
        let error = counter.validate_token_limit(&long_text).unwrap_err();
        match error {
            LlmError::TokenLimitExceeded { .. } => {} // Expected
            _ => panic!("Should return TokenLimitExceeded error"),
        }
    }

    #[test]
    fn test_text_truncation_preserves_token_boundaries() {
        // Test verifies text truncation maintains valid token boundaries
        // Ensures truncated text remains coherent and doesn't break mid-token

        // Arrange
        let counter = create_concrete_openai_counter();
        let long_text = "The quick brown fox jumps over the lazy dog. ".repeat(20);
        let max_tokens = 50u32;

        // Act
        let truncated = counter.truncate_to_limit(&long_text, max_tokens).unwrap();
        let truncated_tokens = counter.count_tokens(&truncated).unwrap();

        // Assert
        assert!(
            truncated_tokens <= max_tokens,
            "Truncated text should fit within limit"
        );
        assert!(
            truncated.len() < long_text.len(),
            "Truncated text should be shorter"
        );
        assert!(!truncated.is_empty(), "Should not truncate to empty string");
        assert!(
            !truncated.ends_with("�"),
            "Should not break Unicode sequences"
        );
    }

    #[test]
    fn test_lm_studio_counter_creation_with_custom_limits() {
        // Test verifies LM Studio counter supports custom context window sizes
        // Ensures flexibility for local models with different capabilities

        // Arrange & Act
        let counter = OpenAITokenCounter::for_lm_studio(16384).unwrap();

        // Assert
        assert_eq!(
            counter.max_context_tokens(),
            16384,
            "Should use custom context limit"
        );
        assert!(
            counter.count_tokens("test").unwrap() > 0,
            "Should count tokens normally"
        );
    }

    #[test]
    fn test_unknown_model_defaults_to_cl100k_base_with_4k_context() {
        // Test verifies unknown model names fall back to safe defaults
        // Ensures graceful handling of new/unknown models without crashing

        // Arrange & Act
        let counter = OpenAITokenCounter::new("unknown-model-xyz").unwrap();

        // Assert
        assert_eq!(
            counter.max_context_tokens(),
            4096,
            "Unknown models should use conservative 4k limit"
        );
        assert!(
            counter.count_tokens("test text").unwrap() > 0,
            "Should still count tokens with default tokenizer"
        );
    }

    #[test]
    fn test_gpt4_turbo_uses_128k_context() {
        // Test verifies GPT-4 Turbo models get correct 128k context limit
        // Ensures model variant detection works for turbo versions

        // Arrange & Act
        let counter = OpenAITokenCounter::new("gpt-4-turbo").unwrap();

        // Assert
        assert_eq!(
            counter.max_context_tokens(),
            128000,
            "GPT-4 Turbo should have 128k context"
        );
    }

    #[test]
    fn test_gpt4_32k_uses_correct_context() {
        // Test verifies GPT-4 32k variant gets correct context limit
        // Ensures model variant detection works for 32k versions

        // Arrange & Act
        let counter = OpenAITokenCounter::new("gpt-4-32k").unwrap();

        // Assert
        assert_eq!(
            counter.max_context_tokens(),
            32768,
            "GPT-4 32k should have 32k context"
        );
    }

    #[test]
    fn test_gpt35_16k_uses_correct_context() {
        // Test verifies GPT-3.5 16k variant gets correct context limit
        // Ensures model variant detection works for 16k versions

        // Arrange & Act
        let counter = OpenAITokenCounter::new("gpt-3.5-turbo-16k").unwrap();

        // Assert
        assert_eq!(
            counter.max_context_tokens(),
            16384,
            "GPT-3.5 16k should have 16k context"
        );
    }

    #[test]
    fn test_o1_model_uses_o200k_tokenizer() {
        // Test verifies o1 models use the correct o200k tokenizer
        // Ensures newer OpenAI models get appropriate tokenizer

        // Arrange & Act
        let counter = OpenAITokenCounter::new("o1-preview").unwrap();

        // Assert
        assert_eq!(
            counter.max_context_tokens(),
            200000,
            "o1 models should have 200k context"
        );
        assert!(
            counter.count_tokens("test").unwrap() > 0,
            "Should count tokens with o200k tokenizer"
        );
    }

    #[test]
    fn test_message_token_counting_with_tool_calls() {
        // Test verifies tool call arguments are properly counted in message tokens
        // Ensures function calling overhead is accurately tracked

        // Arrange
        let counter = create_concrete_openai_counter();
        let messages_with_tools = vec![serde_json::json!({
            "role": "assistant",
            "content": "",
            "tool_calls": [{
                "function": {
                    "name": "get_weather",
                    "arguments": "{\"location\": \"San Francisco\", \"unit\": \"celsius\"}"
                }
            }]
        })];

        // Act
        let token_count = counter.count_message_tokens(&messages_with_tools).unwrap();

        // Assert
        assert!(
            token_count > 15,
            "Tool calls should add significant token overhead"
        );
    }

    #[test]
    fn test_message_token_counting_with_empty_tool_calls() {
        // Test verifies empty tool_calls array is handled correctly
        // Ensures defensive handling of malformed messages

        // Arrange
        let counter = create_concrete_openai_counter();
        let messages = vec![serde_json::json!({
            "role": "assistant",
            "content": "Response",
            "tool_calls": []
        })];

        // Act
        let token_count = counter.count_message_tokens(&messages).unwrap();

        // Assert
        assert!(
            token_count > 0,
            "Should count message tokens even with empty tool_calls"
        );
    }

    #[test]
    fn test_message_token_counting_with_missing_role() {
        // Test verifies messages without role field default to 'user'
        // Ensures defensive handling of incomplete message structures

        // Arrange
        let counter = create_concrete_openai_counter();
        let messages = vec![serde_json::json!({
            "content": "Message without role"
        })];

        // Act
        let token_count = counter.count_message_tokens(&messages).unwrap();

        // Assert
        assert!(
            token_count > 0,
            "Should handle messages with missing role field"
        );
    }

    #[test]
    fn test_message_token_counting_with_missing_content() {
        // Test verifies messages without content field are handled correctly
        // Ensures defensive handling of tool-only messages

        // Arrange
        let counter = create_concrete_openai_counter();
        let messages = vec![serde_json::json!({
            "role": "assistant"
        })];

        // Act
        let token_count = counter.count_message_tokens(&messages).unwrap();

        // Assert
        assert!(
            token_count > 0,
            "Should count formatting tokens even without content"
        );
    }

    #[test]
    fn test_empty_message_array_token_counting() {
        // Test verifies empty message arrays are handled gracefully
        // Ensures defensive handling of edge cases

        // Arrange
        let counter = create_concrete_openai_counter();
        let empty_messages: Vec<serde_json::Value> = vec![];

        // Act
        let token_count = counter.count_message_tokens(&empty_messages).unwrap();

        // Assert
        assert!(
            token_count > 0,
            "Should include base conversation formatting even for empty messages"
        );
    }

    #[test]
    fn test_truncate_already_short_text_returns_unchanged() {
        // Test verifies truncation is no-op for text already within limits
        // Ensures unnecessary processing is avoided

        // Arrange
        let counter = create_concrete_openai_counter();
        let short_text = "Short message";
        let max_tokens = 1000;

        // Act
        let truncated = counter.truncate_to_limit(short_text, max_tokens).unwrap();

        // Assert
        assert_eq!(truncated, short_text, "Short text should remain unchanged");
    }

    #[test]
    fn test_repeated_text_token_counting() {
        // Test verifies repeated patterns are tokenized consistently
        // Ensures tokenizer handles repetitive content correctly

        // Arrange
        let counter = create_concrete_openai_counter();
        let repeated_text = "repeat ".repeat(10);

        // Act
        let token_count = counter.count_tokens(&repeated_text).unwrap();

        // Assert
        assert!(
            token_count > 9,
            "Repeated words should have at least one token each"
        );
        assert!(
            token_count < 50,
            "Should not over-tokenize repeated patterns"
        );
    }

    #[test]
    fn test_message_with_multiple_tool_calls() {
        // Test verifies multiple tool calls are properly counted
        // Ensures complex function calling scenarios are handled

        // Arrange
        let counter = create_concrete_openai_counter();
        let messages = vec![serde_json::json!({
            "role": "assistant",
            "content": "",
            "tool_calls": [
                {
                    "function": {
                        "name": "get_weather",
                        "arguments": "{\"location\": \"San Francisco\"}"
                    }
                },
                {
                    "function": {
                        "name": "get_time",
                        "arguments": "{\"timezone\": \"PST\"}"
                    }
                }
            ]
        })];

        // Act
        let token_count = counter.count_message_tokens(&messages).unwrap();

        // Assert
        assert!(
            token_count > 20,
            "Multiple tool calls should add significant overhead"
        );
    }

    #[test]
    fn test_message_with_non_array_tool_calls() {
        // Test verifies defensive handling when tool_calls is not an array
        // Ensures robustness against malformed message structures

        // Arrange
        let counter = create_concrete_openai_counter();
        let messages = vec![serde_json::json!({
            "role": "assistant",
            "content": "Response",
            "tool_calls": "not_an_array"
        })];

        // Act
        let token_count = counter.count_message_tokens(&messages).unwrap();

        // Assert
        assert!(
            token_count > 0,
            "Should handle non-array tool_calls gracefully"
        );
    }

    #[test]
    fn test_very_long_message_array() {
        // Test verifies handling of conversations with many messages
        // Ensures performance with large message histories

        // Arrange
        let counter = create_concrete_openai_counter();
        let mut messages = Vec::new();
        for i in 0..50 {
            messages.push(serde_json::json!({
                "role": if i % 2 == 0 { "user" } else { "assistant" },
                "content": format!("Message {}", i)
            }));
        }

        // Act
        let token_count = counter.count_message_tokens(&messages).unwrap();

        // Assert
        assert!(
            token_count > 100,
            "Long conversation should have significant token count"
        );
    }
}

#[cfg(test)]
mod anthropic_token_counter_tests {
    use super::*;
    use mystory_llm::tokens::TokenCounter;

    #[test]
    fn test_anthropic_token_counting_with_approximation_factor() {
        // Test verifies Anthropic counter applies appropriate approximation for Claude tokenization
        // Ensures reasonable estimation when Claude-specific tokenizer is not available

        // Arrange
        let counter = create_concrete_anthropic_counter();
        let test_text = "Explain quantum computing in simple terms";

        // Act
        let token_count = counter.count_tokens(test_text).unwrap();

        // Assert
        assert!(token_count > 0, "Should return positive token count");
        // Anthropic counter applies 1.1x multiplier, so should be slightly higher than raw tiktoken
        assert!(token_count >= 7, "Should account for approximation factor");
    }

    #[test]
    fn test_claude_model_context_limits() {
        // Test verifies Claude models have correct context window configurations
        // Ensures model-specific limits match Anthropic's specifications

        // Arrange & Act
        let claude35_counter = AnthropicTokenCounter::new("claude-3-5-sonnet").unwrap();
        let claude3_counter = AnthropicTokenCounter::new("claude-3-opus").unwrap();
        let claude2_counter = AnthropicTokenCounter::new("claude-2").unwrap();

        // Assert
        assert_eq!(
            claude35_counter.max_context_tokens(),
            200000,
            "Claude-3.5 should have 200k context"
        );
        assert_eq!(
            claude3_counter.max_context_tokens(),
            200000,
            "Claude-3 should have 200k context"
        );
        assert_eq!(
            claude2_counter.max_context_tokens(),
            100000,
            "Claude-2 should have 100k context"
        );
    }

    #[test]
    fn test_anthropic_message_token_counting() {
        // Test verifies Anthropic message formatting overhead is correctly calculated
        // Ensures proper token accounting for Claude's conversation format

        // Arrange
        let counter = create_concrete_anthropic_counter();
        let messages = create_test_messages();

        // Act
        let message_tokens = counter.count_message_tokens(&messages).unwrap();

        // Assert
        assert!(
            message_tokens > 15,
            "Should include message formatting overhead"
        );
        assert!(
            message_tokens < 100,
            "Should not over-estimate simple messages"
        );
    }

    #[test]
    fn test_unknown_anthropic_model_defaults_to_100k_context() {
        // Test verifies unknown Anthropic model names fall back to safe defaults
        // Ensures graceful handling of new Claude models without crashing

        // Arrange & Act
        let counter = AnthropicTokenCounter::new("claude-unknown-version").unwrap();

        // Assert
        assert_eq!(
            counter.max_context_tokens(),
            100000,
            "Unknown Claude models should use conservative 100k limit"
        );
        assert!(
            counter.count_tokens("test text").unwrap() > 0,
            "Should still count tokens with default configuration"
        );
    }

    #[test]
    fn test_anthropic_text_truncation_with_approximation() {
        // Test verifies text truncation accounts for Anthropic's approximation factor
        // Ensures truncated text fits within actual Claude token limits

        // Arrange
        let counter = create_concrete_anthropic_counter();
        let long_text = "Claude is an AI assistant created by Anthropic. ".repeat(30);
        let max_tokens = 100u32;

        // Act
        let truncated = counter.truncate_to_limit(&long_text, max_tokens).unwrap();
        let truncated_tokens = counter.count_tokens(&truncated).unwrap();

        // Assert
        assert!(
            truncated_tokens <= max_tokens,
            "Should respect token limit after approximation"
        );
        assert!(
            truncated.len() < long_text.len(),
            "Should actually truncate text"
        );
    }

    #[test]
    fn test_anthropic_validation_accepts_text_within_limit() {
        // Test verifies Anthropic validation passes for reasonable text
        // Ensures normal requests aren't incorrectly rejected

        // Arrange
        let counter = create_concrete_anthropic_counter();
        let reasonable_text = "This is a normal conversation message.";

        // Act
        let result = counter.validate_token_limit(reasonable_text);

        // Assert
        assert!(result.is_ok(), "Normal text should pass validation");
    }

    #[test]
    fn test_anthropic_validation_rejects_oversized_text() {
        // Test verifies Anthropic validation catches text exceeding limits
        // Ensures API errors are prevented before requests

        // Arrange
        let counter = create_concrete_anthropic_counter();
        let massive_text = "word ".repeat(500000); // Way over 200k tokens

        // Act
        let result = counter.validate_token_limit(&massive_text);

        // Assert
        assert!(result.is_err(), "Massive text should fail validation");
    }

    #[test]
    fn test_anthropic_truncate_already_short_text() {
        // Test verifies truncation is no-op for short text
        // Ensures efficiency for common cases

        // Arrange
        let counter = create_concrete_anthropic_counter();
        let short_text = "Brief message";

        // Act
        let truncated = counter.truncate_to_limit(short_text, 1000).unwrap();

        // Assert
        assert_eq!(truncated, short_text, "Short text should remain unchanged");
    }

    #[test]
    fn test_anthropic_message_with_empty_content() {
        // Test verifies empty message content is handled
        // Ensures defensive handling of edge cases

        // Arrange
        let counter = create_concrete_anthropic_counter();
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": ""
        })];

        // Act
        let token_count = counter.count_message_tokens(&messages).unwrap();

        // Assert
        assert!(
            token_count > 0,
            "Should count formatting tokens for empty content"
        );
    }

    #[test]
    fn test_anthropic_message_with_missing_content_field() {
        // Test verifies missing content field is handled gracefully
        // Ensures defensive handling of incomplete messages

        // Arrange
        let counter = create_concrete_anthropic_counter();
        let messages = vec![serde_json::json!({
            "role": "assistant"
        })];

        // Act
        let token_count = counter.count_message_tokens(&messages).unwrap();

        // Assert
        assert!(token_count > 0, "Should handle missing content field");
    }

    #[test]
    fn test_anthropic_empty_message_array() {
        // Test verifies empty message array is handled
        // Ensures defensive programming for edge cases

        // Arrange
        let counter = create_concrete_anthropic_counter();
        let empty_messages: Vec<serde_json::Value> = vec![];

        // Act
        let token_count = counter.count_message_tokens(&empty_messages).unwrap();

        // Assert
        assert_eq!(
            token_count, 0,
            "Empty message array should have zero tokens"
        );
    }

    #[test]
    fn test_anthropic_multiple_messages_conversation() {
        // Test verifies multi-turn conversation token counting
        // Ensures proper handling of back-and-forth dialogue

        // Arrange
        let counter = create_concrete_anthropic_counter();
        let messages = vec![
            serde_json::json!({"role": "user", "content": "Hello"}),
            serde_json::json!({"role": "assistant", "content": "Hi there!"}),
            serde_json::json!({"role": "user", "content": "How are you?"}),
            serde_json::json!({"role": "assistant", "content": "I'm doing well, thanks!"}),
        ];

        // Act
        let token_count = counter.count_message_tokens(&messages).unwrap();

        // Assert
        assert!(
            token_count > 10,
            "Multi-turn conversation should have significant tokens"
        );
    }

    #[test]
    fn test_anthropic_very_long_content() {
        // Test verifies handling of messages with very long content
        // Ensures no issues with large text blocks

        // Arrange
        let counter = create_concrete_anthropic_counter();
        let long_content = "word ".repeat(1000);
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": long_content
        })];

        // Act
        let token_count = counter.count_message_tokens(&messages).unwrap();

        // Assert
        assert!(
            token_count > 1000,
            "Very long content should have high token count"
        );
    }
}

#[cfg(test)]
mod token_counter_factory_tests {
    use super::*;
    use mystory_llm::tokens::TokenCounterFactory;

    #[test]
    fn test_factory_creates_correct_counter_types() {
        // Test verifies factory creates appropriate counter types for each provider
        // Ensures proper provider-specific tokenizer selection

        // Arrange & Act
        let openai_counter = TokenCounterFactory::create_counter("openai", "gpt-4").unwrap();
        let anthropic_counter =
            TokenCounterFactory::create_counter("anthropic", "claude-3-5-sonnet").unwrap();
        let lmstudio_counter =
            TokenCounterFactory::create_counter("lmstudio", "local-model").unwrap();

        // Assert
        assert_eq!(
            openai_counter.max_context_tokens(),
            8192,
            "OpenAI counter should have correct limits"
        );
        assert_eq!(
            anthropic_counter.max_context_tokens(),
            200000,
            "Anthropic counter should have correct limits"
        );
        assert_eq!(
            lmstudio_counter.max_context_tokens(),
            4096,
            "LM Studio should use default limit"
        );
    }

    #[test]
    fn test_factory_handles_unsupported_providers() {
        // Test verifies factory returns appropriate errors for unknown providers
        // Ensures clear error messaging for configuration issues

        // Arrange & Act
        let result = TokenCounterFactory::create_counter("unsupported-provider", "some-model");

        // Assert
        assert!(result.is_err(), "Should fail for unsupported provider");
        match result.unwrap_err() {
            LlmError::UnsupportedProvider { provider } => {
                assert_eq!(provider, "unsupported-provider");
            }
            _ => panic!("Should return UnsupportedProvider error"),
        }
    }

    #[test]
    fn test_factory_creates_counters_with_custom_limits() {
        // Test verifies factory supports custom context window sizes
        // Ensures flexibility for models with non-standard context limits

        // Arrange & Act
        let custom_counter =
            TokenCounterFactory::create_counter_with_limit("openai", "gpt-4", 32000).unwrap();

        // Assert
        assert_eq!(
            custom_counter.max_context_tokens(),
            32000,
            "Should use custom limit"
        );
        assert!(
            custom_counter.count_tokens("test").unwrap() > 0,
            "Should function normally"
        );
    }

    #[test]
    fn test_factory_creates_ollama_counter() {
        // Test verifies factory creates counters for Ollama provider
        // Ensures Ollama is properly supported in factory

        // Arrange & Act
        let counter = TokenCounterFactory::create_counter("ollama", "llama2").unwrap();

        // Assert
        assert_eq!(
            counter.max_context_tokens(),
            4096,
            "Ollama should default to 4k context"
        );
    }

    #[test]
    fn test_factory_with_custom_limits_for_anthropic() {
        // Test verifies custom limits work for Anthropic models
        // Ensures flexibility for non-standard model configurations

        // Arrange & Act
        let counter =
            TokenCounterFactory::create_counter_with_limit("anthropic", "claude-3-5-sonnet", 50000)
                .unwrap();

        // Assert
        assert_eq!(
            counter.max_context_tokens(),
            50000,
            "Should use custom limit"
        );
    }

    #[test]
    fn test_factory_with_custom_limits_for_ollama() {
        // Test verifies custom limits work for Ollama models
        // Ensures local model flexibility

        // Arrange & Act
        let counter =
            TokenCounterFactory::create_counter_with_limit("ollama", "mixtral", 32000).unwrap();

        // Assert
        assert_eq!(
            counter.max_context_tokens(),
            32000,
            "Ollama should accept custom limits"
        );
    }

    #[test]
    fn test_factory_case_insensitive_provider_names() {
        // Test verifies factory handles case variations in provider names
        // Ensures user-friendly configuration

        // Arrange & Act
        let uppercase = TokenCounterFactory::create_counter("OPENAI", "gpt-4");
        let lowercase = TokenCounterFactory::create_counter("openai", "gpt-4");
        let mixed = TokenCounterFactory::create_counter("OpenAI", "gpt-4");

        // Assert
        assert!(uppercase.is_ok(), "Should handle uppercase provider names");
        assert!(lowercase.is_ok(), "Should handle lowercase provider names");
        assert!(mixed.is_ok(), "Should handle mixed case provider names");
    }

    #[test]
    fn test_factory_preserves_model_specific_behavior() {
        // Test verifies factory-created counters retain model-specific characteristics
        // Ensures different models within same provider have correct configurations

        // Arrange & Act
        let gpt4_counter = TokenCounterFactory::create_counter("openai", "gpt-4").unwrap();
        let gpt4_turbo_counter =
            TokenCounterFactory::create_counter("openai", "gpt-4-turbo").unwrap();

        // Assert
        assert_eq!(
            gpt4_counter.max_context_tokens(),
            8192,
            "GPT-4 should have 8k limit"
        );
        assert_eq!(
            gpt4_turbo_counter.max_context_tokens(),
            128000,
            "GPT-4 Turbo should have 128k limit"
        );
    }
}

#[cfg(test)]
mod token_management_integration_tests {
    use super::*;
    use mystory_llm::tokens::TokenCounter;

    #[test]
    fn test_comprehensive_workflow_from_text_to_validation() {
        // Test verifies complete token management workflow for production usage
        // Ensures all components work together correctly for real-world scenarios

        // Arrange
        let counter = create_concrete_openai_counter();
        let user_input = "Please write a detailed explanation of machine learning algorithms, including supervised and unsupervised learning approaches.";

        // Act - Complete token management workflow
        let token_count = counter.count_tokens(user_input).unwrap();
        let validation_result = counter.validate_token_limit(user_input);
        let truncated = counter.truncate_to_limit(user_input, 10).unwrap(); // Use smaller limit to ensure truncation
        let truncated_count = counter.count_tokens(&truncated).unwrap();

        // Assert
        assert!(
            token_count > 15,
            "Complex request should have substantial token count"
        );
        assert!(
            validation_result.is_ok(),
            "Request should pass validation for GPT-4"
        );
        assert!(truncated_count <= 10, "Truncated text should respect limit");
        assert!(!truncated.is_empty(), "Should not truncate to empty");
        assert!(
            truncated.len() < user_input.len(),
            "Should actually truncate"
        );
    }

    #[test]
    fn test_cross_provider_token_counting_consistency() {
        // Test verifies token counting is reasonably consistent across providers
        // Ensures similar inputs produce comparable token estimates

        // Arrange
        let openai_counter = create_concrete_openai_counter();
        let anthropic_counter = create_concrete_anthropic_counter();
        let test_text = "Compare and contrast renewable energy sources";

        // Act
        let openai_tokens = openai_counter.count_tokens(test_text).unwrap();
        let anthropic_tokens = anthropic_counter.count_tokens(test_text).unwrap();

        // Assert
        assert!(
            openai_tokens > 0 && anthropic_tokens > 0,
            "Both should return positive counts"
        );
        // Anthropic includes approximation factor, so should be slightly higher
        assert!(
            anthropic_tokens >= openai_tokens,
            "Anthropic should account for approximation"
        );
        let difference_ratio = anthropic_tokens as f32 / openai_tokens as f32;
        assert!(
            difference_ratio <= 1.5,
            "Approximation should not be excessive"
        );
    }

    #[test]
    fn test_debug_implementation_for_counters() {
        // Test verifies Debug trait is implemented for token counters
        // Ensures counters can be logged and debugged

        // Arrange
        let openai = create_concrete_openai_counter();
        let anthropic = create_concrete_anthropic_counter();

        // Act
        let openai_debug = format!("{:?}", openai);
        let anthropic_debug = format!("{:?}", anthropic);

        // Assert
        assert!(
            openai_debug.contains("OpenAITokenCounter"),
            "Debug should show type"
        );
        assert!(
            anthropic_debug.contains("AnthropicTokenCounter"),
            "Debug should show type"
        );
    }

    #[test]
    fn test_mixed_content_and_empty_messages() {
        // Test verifies handling of conversation with some empty messages
        // Ensures robustness with varied message content

        // Arrange
        let openai = create_concrete_openai_counter();
        let messages = vec![
            serde_json::json!({"role": "user", "content": "Hello"}),
            serde_json::json!({"role": "assistant", "content": ""}),
            serde_json::json!({"role": "user", "content": "Are you there?"}),
        ];

        // Act
        let token_count = openai.count_message_tokens(&messages).unwrap();

        // Assert
        assert!(token_count > 5, "Should count non-empty messages");
    }
}
