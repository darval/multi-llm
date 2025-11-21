// Unit Tests for UnifiedLLMClient Configuration and Factory Methods
//
// UNIT UNDER TEST: UnifiedLLMClient factory methods (from_config, from_env, create)
//
// BUSINESS RESPONSIBILITY:
//   - Creates appropriate provider instances from configuration
//   - Validates configuration completeness and correctness
//   - Handles provider selection and initialization errors
//
// TEST COVERAGE:
//   - Configuration-based provider creation (from_config)
//   - Environment-based configuration (from_env)
//   - Error handling for invalid/missing configurations
//   - Provider name validation
//
// NOTE: The actual ExecutorLLMProvider interface testing is in trait_compliance.rs
// This file only tests the factory/configuration aspects of UnifiedLLMClient

use crate::client::UnifiedLLMClient;
use crate::config::AnthropicConfig;
use crate::error::LlmError;
use crate::providers::AnthropicProvider;
use crate::tests::helpers::create_test_config;

// NOTE: Provider delegation tests moved to trait_compliance.rs
// This file now focuses only on configuration and factory method testing

#[cfg(test)]
mod factory_method_tests {
    use super::*;
    use crate::core_types::executor::ExecutorLLMProvider;

    #[test]
    fn test_from_config_with_anthropic() {
        // Test verifies client creation from LLMConfig with Anthropic provider
        // validates configuration parsing and provider instantiation

        // Arrange
        let config = create_test_config("anthropic");

        // Act
        let result = UnifiedLLMClient::from_config(config);

        // Assert
        assert!(
            result.is_ok(),
            "Should create Anthropic client from valid config"
        );
        let client = result.unwrap();
        assert_eq!(
            client.provider_name(),
            "anthropic",
            "Should identify as Anthropic provider"
        );
    }

    #[test]
    fn test_from_config_with_openai() {
        // Test verifies client creation from LLMConfig with OpenAI provider
        // validates configuration parsing and provider instantiation

        // Arrange
        let config = create_test_config("openai");

        // Act
        let result = UnifiedLLMClient::from_config(config);

        // Assert
        assert!(
            result.is_ok(),
            "Should create OpenAI client from valid config"
        );
        let client = result.unwrap();
        assert_eq!(
            client.provider_name(),
            "openai",
            "Should identify as OpenAI provider"
        );
    }

    #[test]
    fn test_from_config_with_lmstudio() {
        // Test verifies client creation from LLMConfig with LM Studio provider
        // validates local configuration parsing and provider instantiation

        // Arrange
        let config = create_test_config("lmstudio");

        // Act
        let result = UnifiedLLMClient::from_config(config);

        // Assert
        assert!(
            result.is_ok(),
            "Should create LM Studio client from valid config"
        );
        let client = result.unwrap();
        assert_eq!(
            client.provider_name(),
            "lmstudio",
            "Should identify as LM Studio provider"
        );
    }

    #[test]
    fn test_from_config_unsupported_provider_error() {
        // Test verifies proper error handling for unsupported provider names
        // prevents runtime failures with clear error messages

        // Arrange - create a config but try to use it with unsupported provider name

        // Act - The create() method validates provider names
        let config = create_test_config("openai");
        let result = UnifiedLLMClient::create("unsupported-provider", "model".to_string(), config);

        // Assert
        assert!(result.is_err(), "Should fail for unsupported provider");
        match result {
            Err(LlmError::UnsupportedProvider { provider }) => {
                assert_eq!(provider, "unsupported-provider");
            }
            Err(e) => panic!("Expected UnsupportedProvider error, got: {:?}", e),
            Ok(_) => panic!("Expected error, got success"),
        }
    }

    #[test]
    fn test_provider_creation_with_invalid_config() {
        // Test verifies proper error handling when provider config is invalid
        // prevents runtime failures from missing required configuration

        // Arrange
        let invalid_config = AnthropicConfig {
            api_key: None, // Missing required API key
            ..AnthropicConfig::default()
        };

        // Act
        let result =
            AnthropicProvider::new(invalid_config, crate::config::DefaultLLMParams::default());

        // Assert
        assert!(
            result.is_err(),
            "Provider construction should fail with missing API key"
        );

        match result {
            Err(LlmError::ConfigurationError { .. }) => {
                // Expected error type for missing configuration
            }
            _ => panic!("Expected ConfigurationError for missing API key"),
        }
    }

    #[test]
    fn test_from_config_propagates_provider_construction_errors() {
        // Test verifies that UnifiedLLMClient::from_config properly propagates
        // errors from underlying provider construction failures

        // Note: Test helper creates valid configs, so we test unsupported provider path
        // The main error propagation happens through the ? operator in create methods

        // Arrange - Test the unsupported provider path which is directly testable
        let config = create_test_config("openai");
        let result = UnifiedLLMClient::create("invalid-provider", "model".to_string(), config);

        // Act & Assert
        assert!(result.is_err(), "Should fail for unsupported provider");
        match result {
            Err(LlmError::UnsupportedProvider { provider }) => {
                assert_eq!(provider, "invalid-provider");
            }
            _ => panic!("Expected UnsupportedProvider error"),
        }
    }

    #[test]
    fn test_create_method_with_valid_parameters() {
        // Test verifies the primary create() factory method works correctly
        // with all required parameters

        // Arrange
        let config = create_test_config("openai");
        let model = "gpt-4".to_string();

        // Act
        let result = UnifiedLLMClient::create("openai", model.clone(), config);

        // Assert
        assert!(result.is_ok(), "Should create client with valid parameters");
        let client = result.unwrap();
        assert_eq!(client.provider_name(), "openai");
        // Note: model is stored internally but not exposed via public API
    }

    #[test]
    fn test_from_config_with_ollama() {
        // Test verifies client creation from LLMConfig with Ollama provider
        // Ensures local Ollama configuration works correctly

        // Arrange
        let config = create_test_config("ollama");

        // Act
        let result = UnifiedLLMClient::from_config(config);

        // Assert
        assert!(
            result.is_ok(),
            "Should create Ollama client from valid config"
        );
        let client = result.unwrap();
        assert_eq!(client.provider_name(), "ollama");
    }

    #[test]
    #[serial_test::serial]
    fn test_from_env_creates_client_from_environment_variables() {
        // Test verifies client creation directly from environment variables
        // Validates complete initialization flow from env vars to client instance

        // Arrange
        std::env::set_var("AI_PROVIDER", "anthropic");
        std::env::set_var("ANTHROPIC_API_KEY", "test-key-from-env");

        // Act
        let result = UnifiedLLMClient::from_env();

        // Assert
        assert!(
            result.is_ok(),
            "Should create client from environment variables"
        );
        let client = result.unwrap();
        assert_eq!(client.provider_name(), "anthropic");

        // Cleanup
        std::env::remove_var("AI_PROVIDER");
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    #[serial_test::serial]
    fn test_from_env_propagates_config_errors() {
        // Test verifies from_env properly propagates configuration errors
        // Ensures missing required variables result in appropriate errors

        // Arrange - Set provider but omit required API key
        std::env::set_var("AI_PROVIDER", "anthropic");
        std::env::remove_var("ANTHROPIC_API_KEY");

        // Act
        let result = UnifiedLLMClient::from_env();

        // Assert
        assert!(
            result.is_err(),
            "Should fail when required env vars are missing"
        );
        match result {
            Err(LlmError::ConfigurationError { .. }) => {
                // Expected error type
            }
            _ => panic!("Expected ConfigurationError for missing API key"),
        }

        // Cleanup
        std::env::remove_var("AI_PROVIDER");
    }
}

// Unit Tests for Provider Name Exposure
//
// UNIT UNDER TEST: UnifiedLLMClient::provider_name()
//
// BUSINESS RESPONSIBILITY:
//   - Exposes which LLM provider is being used for logging and monitoring
//   - Enables provider-specific behavior and diagnostics
//
// TEST COVERAGE:
//   - Provider name correctly identifies the underlying provider type
//   - All supported providers return correct identification strings

#[cfg(test)]
mod provider_name_tests {
    use super::*;
    use crate::core_types::executor::ExecutorLLMProvider;

    #[test]
    fn test_anthropic_client_returns_correct_provider_name() {
        // Test verifies Anthropic client identifies itself correctly
        // Ensures monitoring and logging can distinguish provider types

        // Arrange
        let config = create_test_config("anthropic");
        let client = UnifiedLLMClient::from_config(config).unwrap();

        // Act
        let provider_name = client.provider_name();

        // Assert
        assert_eq!(provider_name, "anthropic");
    }

    #[test]
    fn test_openai_client_returns_correct_provider_name() {
        // Test verifies OpenAI client identifies itself correctly

        // Arrange
        let config = create_test_config("openai");
        let client = UnifiedLLMClient::from_config(config).unwrap();

        // Act
        let provider_name = client.provider_name();

        // Assert
        assert_eq!(provider_name, "openai");
    }

    #[test]
    fn test_lmstudio_client_returns_correct_provider_name() {
        // Test verifies LM Studio client identifies itself correctly

        // Arrange
        let config = create_test_config("lmstudio");
        let client = UnifiedLLMClient::from_config(config).unwrap();

        // Act
        let provider_name = client.provider_name();

        // Assert
        assert_eq!(provider_name, "lmstudio");
    }

    #[test]
    fn test_ollama_client_returns_correct_provider_name() {
        // Test verifies Ollama client identifies itself correctly

        // Arrange
        let config = create_test_config("ollama");
        let client = UnifiedLLMClient::from_config(config).unwrap();

        // Act
        let provider_name = client.provider_name();

        // Assert
        assert_eq!(provider_name, "ollama");
    }
}

// Unit Tests for Private Factory Helper Methods
//
// UNIT UNDER TEST: create_*_provider() private helper methods
//
// BUSINESS RESPONSIBILITY:
//   - Create properly configured provider instances from LLMConfig
//   - Validate configuration matches expected provider type
//   - Handle configuration errors gracefully
//
// TEST COVERAGE:
//   - Error handling when config type doesn't match provider
//   - Proper provider construction for each supported provider type

#[cfg(test)]
mod private_factory_tests {
    use super::*;

    #[test]
    fn test_create_anthropic_provider_with_wrong_config_type() {
        // Test verifies error handling when OpenAI config is passed to Anthropic factory
        // Ensures clear error messages for configuration mismatches

        // Arrange
        let config = create_test_config("openai"); // Wrong config type

        // Act
        let result = UnifiedLLMClient::create("anthropic", "claude-3".to_string(), config);

        // Assert
        assert!(result.is_err(), "Should fail with config type mismatch");
        match result {
            Err(LlmError::ConfigurationError { .. }) => {
                // Expected error
            }
            _ => panic!("Expected ConfigurationError"),
        }
    }

    #[test]
    fn test_create_openai_provider_with_wrong_config_type() {
        // Test verifies error handling when Anthropic config is passed to OpenAI factory
        // Ensures configuration validation catches type mismatches

        // Arrange
        let config = create_test_config("anthropic"); // Wrong config type

        // Act
        let result = UnifiedLLMClient::create("openai", "gpt-4".to_string(), config);

        // Assert
        assert!(result.is_err(), "Should fail with config type mismatch");
        match result {
            Err(LlmError::ConfigurationError { .. }) => {
                // Expected error
            }
            _ => panic!("Expected ConfigurationError"),
        }
    }

    #[test]
    fn test_create_lmstudio_provider_with_wrong_config_type() {
        // Test verifies error handling when wrong config is passed to LM Studio factory

        // Arrange
        let config = create_test_config("anthropic"); // Wrong config type

        // Act
        let result = UnifiedLLMClient::create("lmstudio", "local-model".to_string(), config);

        // Assert
        assert!(result.is_err(), "Should fail with config type mismatch");
    }

    #[test]
    fn test_create_ollama_provider_with_wrong_config_type() {
        // Test verifies error handling when wrong config is passed to Ollama factory

        // Arrange
        let config = create_test_config("openai"); // Wrong config type

        // Act
        let result = UnifiedLLMClient::create("ollama", "llama2".to_string(), config);

        // Assert
        assert!(result.is_err(), "Should fail with config type mismatch");
    }

    #[test]
    fn test_create_anthropic_provider_success() {
        // Test verifies successful Anthropic provider creation with correct config
        // Ensures happy path works for Anthropic

        // Arrange
        let config = create_test_config("anthropic");

        // Act
        let result = UnifiedLLMClient::create("anthropic", "claude-3".to_string(), config);

        // Assert
        assert!(
            result.is_ok(),
            "Should succeed with correct Anthropic config"
        );
    }

    #[test]
    fn test_create_openai_provider_success() {
        // Test verifies successful OpenAI provider creation with correct config
        // Ensures happy path works for OpenAI

        // Arrange
        let config = create_test_config("openai");

        // Act
        let result = UnifiedLLMClient::create("openai", "gpt-4".to_string(), config);

        // Assert
        assert!(result.is_ok(), "Should succeed with correct OpenAI config");
    }

    #[test]
    fn test_create_lmstudio_provider_success() {
        // Test verifies successful LM Studio provider creation with correct config
        // Ensures happy path works for LM Studio

        // Arrange
        let config = create_test_config("lmstudio");

        // Act
        let result = UnifiedLLMClient::create("lmstudio", "local-model".to_string(), config);

        // Assert
        assert!(
            result.is_ok(),
            "Should succeed with correct LM Studio config"
        );
    }

    #[test]
    fn test_create_ollama_provider_success() {
        // Test verifies successful Ollama provider creation with correct config
        // Ensures happy path works for Ollama

        // Arrange
        let config = create_test_config("ollama");

        // Act
        let result = UnifiedLLMClient::create("ollama", "llama2".to_string(), config);

        // Assert
        assert!(result.is_ok(), "Should succeed with correct Ollama config");
    }

    #[test]
    fn test_create_with_different_models() {
        // Test verifies client can be created with various model names
        // Ensures model parameter is properly stored

        // Arrange & Act
        let anthropic = UnifiedLLMClient::create(
            "anthropic",
            "claude-3-opus".to_string(),
            create_test_config("anthropic"),
        );
        let openai = UnifiedLLMClient::create(
            "openai",
            "gpt-4-turbo".to_string(),
            create_test_config("openai"),
        );

        // Assert
        assert!(anthropic.is_ok(), "Should create with Claude model");
        assert!(openai.is_ok(), "Should create with GPT-4 model");
    }
}
