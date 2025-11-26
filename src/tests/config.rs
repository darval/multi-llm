// Unit Tests for LLM Configuration System
//
// UNIT UNDER TEST: LLMConfig
//
// BUSINESS RESPONSIBILITY:
//   - Loads and validates LLM provider configurations from environment variables
//   - Ensures required credentials are present before allowing operations
//   - Provides appropriate defaults and handles missing environment variables
//   - Creates proper provider configurations for different LLM services
//   - Validates configuration completeness to prevent runtime failures
//
// TEST COVERAGE:
//   - Environment variable parsing with different provider types
//   - Configuration validation for missing required fields
//   - Default value application when optional fields are missing
//   - Error handling for unsupported providers and invalid configurations
//   - Test configuration creation for development and testing scenarios

use crate::config::{AnthropicConfig, LLMConfig, LMStudioConfig, OpenAIConfig, ProviderConfig};
use crate::error::LlmError;
use crate::tests::helpers::create_test_config;

#[cfg(test)]
mod llm_config_tests {
    use super::*;

    #[test]
    fn test_create_test_config_with_anthropic_provider() {
        // Test verifies test helper creates valid configuration for Anthropic
        // Ensures test configurations have proper defaults and required fields

        // Arrange & Act
        let config = create_test_config("anthropic");

        // Assert
        assert_eq!(config.provider.provider_name(), "anthropic");
        assert_eq!(config.provider.max_context_tokens(), 200_000);
        assert!(
            config.provider.api_key().is_some(),
            "Test config should include API key"
        );
    }

    #[test]
    fn test_create_test_config_with_openai_provider() {
        // Test verifies test helper creates valid configuration for OpenAI
        // Ensures proper OpenAI-specific defaults and capabilities

        // Arrange & Act
        let config = create_test_config("openai");

        // Assert
        assert_eq!(config.provider.provider_name(), "openai");
        assert_eq!(config.provider.max_context_tokens(), 128_000);
        assert!(
            config.provider.api_key().is_some(),
            "Test config should include API key"
        );
    }

    #[test]
    fn test_create_test_config_with_lmstudio_provider() {
        // Test verifies test helper creates valid configuration for LM Studio
        // Ensures local development defaults without requiring API keys

        // Arrange & Act
        let config = create_test_config("lmstudio");

        // Assert
        assert_eq!(config.provider.provider_name(), "lmstudio");
        assert_eq!(config.provider.max_context_tokens(), 4_096);
        assert!(
            config.provider.api_key().is_none(),
            "LM Studio should not require API key"
        );
    }

    #[test]
    #[should_panic(expected = "Unsupported test provider")]
    fn test_create_test_config_with_unsupported_provider() {
        // Test verifies proper error handling for invalid provider names
        // Prevents runtime failures from configuration typos

        // Arrange & Act (should panic)
        create_test_config("invalid-provider");
    }

    #[test]
    fn test_default_params_applied_correctly() {
        // Test verifies default LLM parameters are properly set
        // Ensures consistent behavior across different providers

        // Arrange & Act
        let config = create_test_config("anthropic");

        // Assert
        assert_eq!(config.default_params.temperature, 0.7);
        assert_eq!(config.default_params.max_tokens, 1000);
        assert_eq!(config.default_params.top_p, 0.9);
        assert_eq!(config.default_params.top_k, 40);
        assert_eq!(config.default_params.min_p, 0.05);
        assert_eq!(config.default_params.presence_penalty, 0.0);
    }
}

// UNIT UNDER TEST: LLMConfig::from_env
//
// BUSINESS RESPONSIBILITY:
//   - Loads LLM configuration from environment variables for deployment flexibility
//   - Validates provider-specific requirements before runtime execution
//   - Supports multiple providers with consistent environment variable patterns
//   - Provides sensible defaults for missing optional configuration
//
// TEST COVERAGE:
//   - Default provider selection when AI_PROVIDER is not set
//   - Provider-specific environment variable loading (ANTHROPIC_API_KEY, OPENAI_API_KEY, etc.)
//   - Base URL customization for OpenAI and LM Studio
//   - Fallback behavior for LM Studio OPENAI_BASE_URL compatibility
//   - Validation error handling for missing required credentials
//   - Unsupported provider error handling

#[cfg(test)]
mod llm_config_from_env_tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_from_env_defaults_to_anthropic_when_no_provider_set() {
        // Test verifies system defaults to Anthropic provider when AI_PROVIDER is not set
        // Ensures graceful handling of missing environment configuration

        // Arrange
        std::env::remove_var("AI_PROVIDER");
        std::env::set_var("ANTHROPIC_API_KEY", "test-key-anthropic");

        // Act
        let config = LLMConfig::from_env().expect("Should create config with defaults");

        // Assert
        assert_eq!(config.provider.provider_name(), "anthropic");
        assert_eq!(config.provider.api_key(), Some("test-key-anthropic"));

        // Cleanup
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    #[serial]
    fn test_from_env_anthropic_with_api_key() {
        // Test verifies Anthropic configuration loads from environment variables
        // Ensures proper API key extraction for cloud provider

        // Arrange
        std::env::set_var("AI_PROVIDER", "anthropic");
        std::env::set_var("ANTHROPIC_API_KEY", "test-anthropic-key");

        // Act
        let config = LLMConfig::from_env().expect("Should create Anthropic config");

        // Assert
        assert_eq!(config.provider.provider_name(), "anthropic");
        assert_eq!(config.provider.api_key(), Some("test-anthropic-key"));
        assert_eq!(config.provider.max_context_tokens(), 200_000);

        // Cleanup
        std::env::remove_var("AI_PROVIDER");
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    #[serial]
    fn test_from_env_openai_with_api_key_and_custom_base_url() {
        // Test verifies OpenAI configuration loads from environment variables
        // Ensures proper API key and optional base URL handling

        // Arrange
        std::env::set_var("AI_PROVIDER", "openai");
        std::env::set_var("OPENAI_API_KEY", "test-openai-key");
        std::env::set_var("OPENAI_BASE_URL", "https://custom.openai.com");

        // Act
        let config = LLMConfig::from_env().expect("Should create OpenAI config");

        // Assert
        assert_eq!(config.provider.provider_name(), "openai");
        assert_eq!(config.provider.api_key(), Some("test-openai-key"));
        assert_eq!(config.provider.base_url(), "https://custom.openai.com");

        // Cleanup
        std::env::remove_var("AI_PROVIDER");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("OPENAI_BASE_URL");
    }

    #[test]
    #[serial]
    fn test_from_env_openai_defaults_base_url_when_missing() {
        // Test verifies OpenAI uses default base URL when not specified
        // Ensures graceful handling of optional environment variables

        // Arrange
        std::env::set_var("AI_PROVIDER", "openai");
        std::env::set_var("OPENAI_API_KEY", "test-key");

        // Act
        let config = LLMConfig::from_env().expect("Should create config with defaults");

        // Assert
        assert_eq!(config.provider.base_url(), "https://api.openai.com");

        // Cleanup
        std::env::remove_var("AI_PROVIDER");
        std::env::remove_var("OPENAI_API_KEY");
    }

    #[test]
    #[serial]
    fn test_from_env_lmstudio_with_custom_base_url() {
        // Test verifies LM Studio configuration with custom base URL
        // Ensures local development setup can override defaults

        // Arrange
        std::env::set_var("AI_PROVIDER", "lmstudio");
        std::env::set_var("LM_STUDIO_BASE_URL", "http://localhost:8080");

        // Act
        let config = LLMConfig::from_env().expect("Should create LM Studio config");

        // Assert
        assert_eq!(config.provider.provider_name(), "lmstudio");
        assert_eq!(config.provider.base_url(), "http://localhost:8080");
        assert!(config.provider.api_key().is_none());

        // Cleanup
        std::env::remove_var("AI_PROVIDER");
        std::env::remove_var("LM_STUDIO_BASE_URL");
    }

    #[test]
    #[serial]
    fn test_from_env_lmstudio_falls_back_to_openai_base_url() {
        // Test verifies LM Studio can use OPENAI_BASE_URL as fallback
        // Ensures compatibility with OpenAI-compatible local servers

        // Arrange
        std::env::set_var("AI_PROVIDER", "lmstudio");
        std::env::set_var("OPENAI_BASE_URL", "http://localhost:9999");
        std::env::remove_var("LM_STUDIO_BASE_URL");

        // Act
        let config = LLMConfig::from_env().expect("Should create config with fallback");

        // Assert
        assert_eq!(config.provider.base_url(), "http://localhost:9999");

        // Cleanup
        std::env::remove_var("AI_PROVIDER");
        std::env::remove_var("OPENAI_BASE_URL");
    }

    #[test]
    #[serial]
    fn test_from_env_lmstudio_uses_default_when_no_url_set() {
        // Test verifies LM Studio uses default localhost URL when not configured
        // Ensures out-of-box functionality for local development

        // Arrange
        std::env::set_var("AI_PROVIDER", "lmstudio");
        std::env::remove_var("LM_STUDIO_BASE_URL");
        std::env::remove_var("OPENAI_BASE_URL");

        // Act
        let config = LLMConfig::from_env().expect("Should create config with defaults");

        // Assert
        assert_eq!(config.provider.base_url(), "http://localhost:1234");

        // Cleanup
        std::env::remove_var("AI_PROVIDER");
    }

    #[test]
    #[serial]
    fn test_from_env_unsupported_provider_returns_error() {
        // Test verifies proper error handling for unknown provider names
        // Prevents runtime failures from configuration typos

        // Arrange
        std::env::set_var("AI_PROVIDER", "unknown-provider");

        // Act
        let result = LLMConfig::from_env();

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            LlmError::UnsupportedProvider { provider } => {
                assert_eq!(provider, "unknown-provider");
            }
            _ => panic!("Expected UnsupportedProvider error"),
        }

        // Cleanup
        std::env::remove_var("AI_PROVIDER");
    }

    #[test]
    #[serial]
    fn test_from_env_validates_anthropic_missing_api_key() {
        // Test verifies validation catches missing required API keys
        // Ensures configuration errors are caught early before API calls

        // Arrange
        std::env::set_var("AI_PROVIDER", "anthropic");
        std::env::remove_var("ANTHROPIC_API_KEY");

        // Act
        let result = LLMConfig::from_env();

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LlmError::ConfigurationError { .. }
        ));

        // Cleanup
        std::env::remove_var("AI_PROVIDER");
    }

    #[test]
    #[serial]
    fn test_from_env_validates_openai_missing_api_key() {
        // Test verifies OpenAI validation catches missing API keys
        // Ensures proper error messages for configuration issues

        // Arrange
        std::env::set_var("AI_PROVIDER", "openai");
        std::env::remove_var("OPENAI_API_KEY");

        // Act
        let result = LLMConfig::from_env();

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LlmError::ConfigurationError { .. }
        ));

        // Cleanup
        std::env::remove_var("AI_PROVIDER");
    }
}

// UNIT UNDER TEST: AnthropicConfig
//
// BUSINESS RESPONSIBILITY:
//   - Validates Anthropic-specific configuration requirements
//   - Enforces API key presence for authentication
//   - Provides Anthropic service defaults and capabilities
//   - Implements ProviderConfig trait for unified interface
//
// TEST COVERAGE:
//   - Configuration validation with and without API keys
//   - Default value verification for Anthropic-specific settings
//   - Provider capability reporting (context limits, tool support)
//   - API endpoint and model configuration validation

#[cfg(test)]
mod anthropic_config_tests {
    use super::*;

    #[test]
    fn test_anthropic_config_validation_with_api_key() {
        // Test verifies AnthropicConfig validates successfully with API key
        // Ensures authentication requirements are properly enforced

        // Arrange
        let config = AnthropicConfig {
            api_key: Some("test-api-key".to_string()),
            ..AnthropicConfig::default()
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(result.is_ok(), "Should validate successfully with API key");
    }

    #[test]
    fn test_anthropic_config_validation_without_api_key() {
        // Test verifies AnthropicConfig rejects configuration without API key
        // Prevents runtime authentication failures

        // Arrange
        let config = AnthropicConfig {
            api_key: None,
            ..AnthropicConfig::default()
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(result.is_err(), "Should fail validation without API key");
        let error = result.unwrap_err();
        assert!(matches!(error, LlmError::ConfigurationError { .. }));
    }

    #[test]
    fn test_anthropic_config_provider_capabilities() {
        // Test verifies AnthropicConfig reports correct provider capabilities
        // Ensures proper context limits and feature support

        // Arrange
        let config = AnthropicConfig::default();

        // Act & Assert
        assert_eq!(config.provider_name(), "anthropic");
        assert_eq!(config.max_context_tokens(), 200_000);
        // Tools are always supported now
        assert_eq!(config.base_url(), "https://api.anthropic.com");
        assert_eq!(config.default_model(), "claude-3-5-sonnet-20241022");
    }
}

// UNIT UNDER TEST: OpenAIConfig
//
// BUSINESS RESPONSIBILITY:
//   - Validates OpenAI-specific configuration requirements
//   - Enforces API key presence for OpenAI authentication
//   - Provides OpenAI service defaults and capabilities
//   - Supports custom base URLs for OpenAI-compatible services
//
// TEST COVERAGE:
//   - Configuration validation with and without API keys
//   - Default value verification for OpenAI-specific settings
//   - Provider capability reporting (context limits, tool support)
//   - Custom base URL handling for API compatibility

#[cfg(test)]
mod openai_config_tests {
    use super::*;

    #[test]
    fn test_openai_config_validation_with_api_key() {
        // Test verifies OpenAIConfig validates successfully with API key
        // Ensures proper OpenAI authentication setup

        // Arrange
        let config = OpenAIConfig {
            api_key: Some("test-openai-key".to_string()),
            ..OpenAIConfig::default()
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(result.is_ok(), "Should validate successfully with API key");
    }

    #[test]
    fn test_openai_config_validation_without_api_key() {
        // Test verifies OpenAIConfig rejects configuration without API key
        // Prevents runtime authentication failures with OpenAI API

        // Arrange
        let config = OpenAIConfig {
            api_key: None,
            ..OpenAIConfig::default()
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(result.is_err(), "Should fail validation without API key");
        let error = result.unwrap_err();
        assert!(matches!(error, LlmError::ConfigurationError { .. }));
    }

    #[test]
    fn test_openai_config_provider_capabilities() {
        // Test verifies OpenAIConfig reports correct provider capabilities
        // Ensures proper OpenAI context limits and feature support

        // Arrange
        let config = OpenAIConfig::default();

        // Act & Assert
        assert_eq!(config.provider_name(), "openai");
        assert_eq!(config.max_context_tokens(), 128_000);
        // Tools are always supported now
        assert_eq!(config.base_url(), "https://api.openai.com");
        assert_eq!(config.default_model(), "gpt-4");
    }
}

// UNIT UNDER TEST: LMStudioConfig
//
// BUSINESS RESPONSIBILITY:
//   - Validates local LM Studio connection requirements
//   - Handles local development without API key requirements
//   - Provides conservative defaults for local model capabilities
//   - Supports configurable tool support based on model capabilities
//
// TEST COVERAGE:
//   - Configuration validation with base URL requirements
//   - Local development defaults without authentication
//   - Configurable tool support based on model features
//   - Error handling for missing base URL configuration

#[cfg(test)]
mod lmstudio_config_tests {
    use super::*;

    #[test]
    fn test_lmstudio_config_validation_with_base_url() {
        // Test verifies LMStudioConfig validates successfully with base URL
        // Ensures local connection requirements are properly enforced

        // Arrange
        let config = LMStudioConfig {
            base_url: "http://localhost:1234".to_string(),
            ..LMStudioConfig::default()
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(result.is_ok(), "Should validate successfully with base URL");
    }

    #[test]
    fn test_lmstudio_config_validation_without_base_url() {
        // Test verifies LMStudioConfig rejects empty base URL
        // Prevents connection failures to local LM Studio instance

        // Arrange
        let config = LMStudioConfig {
            base_url: "".to_string(),
            ..LMStudioConfig::default()
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(result.is_err(), "Should fail validation without base URL");
        let error = result.unwrap_err();
        assert!(matches!(error, LlmError::ConfigurationError { .. }));
    }

    #[test]
    fn test_lmstudio_config_provider_capabilities() {
        // Test verifies LMStudioConfig reports conservative local capabilities
        // Ensures realistic expectations for local model performance

        // Arrange
        let config = LMStudioConfig::default();

        // Act & Assert
        assert_eq!(config.provider_name(), "lmstudio");
        assert_eq!(config.max_context_tokens(), 4_096);
        // Tools are always supported now
        assert_eq!(config.base_url(), "http://localhost:1234");
        assert_eq!(config.default_model(), "local-model");
        assert!(
            config.api_key().is_none(),
            "LM Studio should not require API key"
        );
    }
}

// UNIT UNDER TEST: LLMConfig::create_provider
//
// BUSINESS RESPONSIBILITY:
//   - Factory method for creating provider configurations
//   - Validates provider names and configuration parameters
//   - Applies provider-specific defaults and settings
//   - Ensures all created configurations are valid before returning
//
// TEST COVERAGE:
//   - Provider creation for all supported providers
//   - Error handling for unsupported providers
//   - API key and base URL parameter handling
//   - Validation of created configurations

#[cfg(test)]
mod llm_config_create_provider_tests {
    use super::*;

    #[test]
    fn test_create_provider_anthropic_with_api_key() {
        // Test verifies Anthropic provider creation with API key
        // Ensures proper configuration and validation

        // Arrange & Act
        let result =
            LLMConfig::create_provider("anthropic", Some("test-key".to_string()), None, None);

        // Assert
        assert!(result.is_ok(), "Should create Anthropic provider");
        let config = result.unwrap();
        assert_eq!(config.provider.provider_name(), "anthropic");
        assert!(config.provider.api_key().is_some());
    }

    #[test]
    fn test_create_provider_openai_with_api_key() {
        // Test verifies OpenAI provider creation with API key
        // Ensures proper configuration and validation

        // Arrange & Act
        let result = LLMConfig::create_provider("openai", Some("test-key".to_string()), None, None);

        // Assert
        assert!(result.is_ok(), "Should create OpenAI provider");
        let config = result.unwrap();
        assert_eq!(config.provider.provider_name(), "openai");
        assert!(config.provider.api_key().is_some());
    }

    #[test]
    fn test_create_provider_lmstudio() {
        // Test verifies LM Studio provider creation
        // Ensures local provider works without API key

        // Arrange & Act
        let result = LLMConfig::create_provider(
            "lmstudio",
            None,
            Some("http://localhost:1234".to_string()),
            None,
        );

        // Assert
        assert!(result.is_ok(), "Should create LM Studio provider");
        let config = result.unwrap();
        assert_eq!(config.provider.provider_name(), "lmstudio");
        assert!(config.provider.api_key().is_none());
    }

    #[test]
    fn test_create_provider_ollama() {
        // Test verifies Ollama provider creation
        // Ensures Ollama provider works without API key

        // Arrange & Act
        let result = LLMConfig::create_provider(
            "ollama",
            None,
            Some("http://localhost:11434".to_string()),
            None,
        );

        // Assert
        assert!(result.is_ok(), "Should create Ollama provider");
        let config = result.unwrap();
        assert_eq!(config.provider.provider_name(), "ollama");
    }

    #[test]
    fn test_create_provider_unsupported() {
        // Test verifies error handling for unsupported providers
        // Prevents runtime failures from invalid provider names

        // Arrange & Act
        let result = LLMConfig::create_provider("unsupported", None, None, None);

        // Assert
        assert!(result.is_err(), "Should reject unsupported provider");
        match result.unwrap_err() {
            LlmError::ConfigurationError { .. } => {} // Expected
            e => panic!("Expected ConfigurationError, got: {:?}", e),
        }
    }

    #[test]
    fn test_create_provider_validates_configuration() {
        // Test verifies created configurations are validated
        // Prevents invalid configurations from being returned

        // Arrange & Act - OpenAI without API key should fail validation
        let result = LLMConfig::create_provider("openai", None, None, None);

        // Assert
        assert!(
            result.is_err(),
            "Should fail validation when API key missing"
        );
    }

    #[test]
    fn test_create_provider_with_custom_model() {
        // Test verifies custom model parameter is applied
        // Ensures model selection works correctly

        // Arrange & Act
        let result = LLMConfig::create_provider(
            "anthropic",
            Some("test-key".to_string()),
            None,
            Some("claude-3-opus".to_string()),
        );

        // Assert
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.provider.default_model(), "claude-3-opus");
    }

    #[test]
    fn test_create_provider_case_insensitive() {
        // Test verifies provider names are case-insensitive
        // Allows flexible configuration input

        // Arrange & Act
        let result =
            LLMConfig::create_provider("ANTHROPIC", Some("test-key".to_string()), None, None);

        // Assert
        assert!(result.is_ok(), "Should handle uppercase provider name");
        let config = result.unwrap();
        assert_eq!(config.provider.provider_name(), "anthropic");
    }
}

// Simple coverage tests for uncovered paths
#[cfg(test)]
mod coverage_tests {
    use super::*;

    #[test]
    fn test_llm_config_clone() {
        // Test verifies LLMConfig can be cloned
        // Ensures configuration can be safely duplicated

        // Arrange
        let config = create_test_config("anthropic");

        // Act
        let cloned = config.clone();

        // Assert
        assert_eq!(
            cloned.provider.provider_name(),
            config.provider.provider_name()
        );
        assert_eq!(
            cloned.default_params.temperature,
            config.default_params.temperature
        );
    }

    #[test]
    fn test_provider_config_downcasting() {
        // Test verifies provider config downcasting works
        // Ensures as_any() method functions correctly

        // Arrange
        let config = create_test_config("anthropic");

        // Act
        let any_ref = config.provider.as_any();
        let downcast = any_ref.downcast_ref::<AnthropicConfig>();

        // Assert
        assert!(downcast.is_some(), "Should downcast to AnthropicConfig");
    }
}
