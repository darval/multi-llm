// Unit Tests for LLM Error Handling System
//
// UNIT UNDER TEST: LlmError
//
// BUSINESS RESPONSIBILITY:
//   - Provides comprehensive error categorization for LLM operations
//   - Implements proper error severity mapping for monitoring and alerting
//   - Generates user-friendly error messages without exposing technical details
//   - Determines retry logic for transient vs permanent failures
//   - Automatically logs errors at creation with structured context
//
// TEST COVERAGE:
//   - Error categorization accuracy for different failure types
//   - Severity level assignment for proper alerting behavior
//   - User message generation that hides internal implementation details
//   - Retry logic determination for operational resilience
//   - Automatic logging with structured fields for observability
//   - Error constructor functions with proper context preservation

use crate::core_types::{ErrorCategory, ErrorSeverity, MyStoryError};
use crate::error::LlmError;

#[cfg(test)]
mod llm_error_categorization_tests {
    use super::*;

    #[test]
    fn test_unsupported_provider_error_categorization() {
        // Test verifies unsupported provider errors are categorized as client errors
        // Ensures proper alerting behavior for configuration issues

        // Arrange
        let provider_name = "unsupported-provider";

        // Act
        let error = LlmError::unsupported_provider(provider_name);

        // Assert
        assert_eq!(error.category(), ErrorCategory::Client);
        assert_eq!(error.severity(), ErrorSeverity::Error);
        assert!(
            !error.is_retryable(),
            "Configuration errors should not be retryable"
        );
    }

    #[test]
    fn test_configuration_error_categorization() {
        // Test verifies configuration errors are properly categorized as client errors
        // Ensures configuration issues are treated as non-retryable failures

        // Arrange
        let error_message = "Missing API key";

        // Act
        let error = LlmError::configuration_error(error_message);

        // Assert
        assert_eq!(error.category(), ErrorCategory::Client);
        assert_eq!(error.severity(), ErrorSeverity::Error);
        assert!(
            !error.is_retryable(),
            "Configuration errors should not be retryable"
        );
    }

    #[test]
    fn test_request_failed_error_categorization() {
        // Test verifies network/API failures are categorized as external errors
        // Ensures proper retry behavior for transient network issues

        // Arrange
        let error_message = "HTTP request timeout";

        // Act
        let error = LlmError::request_failed(error_message, None);

        // Assert
        assert_eq!(error.category(), ErrorCategory::External);
        assert_eq!(error.severity(), ErrorSeverity::Error);
        assert!(
            error.is_retryable(),
            "External request failures should be retryable"
        );
    }

    #[test]
    fn test_rate_limit_exceeded_categorization() {
        // Test verifies rate limit errors are categorized as transient
        // Ensures proper backoff and retry behavior for rate limiting

        // Arrange
        let retry_after_seconds = 60;

        // Act
        let error = LlmError::rate_limit_exceeded(retry_after_seconds);

        // Assert
        assert_eq!(error.category(), ErrorCategory::Transient);
        assert_eq!(error.severity(), ErrorSeverity::Warning);
        assert!(
            error.is_retryable(),
            "Rate limit errors should be retryable"
        );
    }

    #[test]
    fn test_timeout_error_categorization() {
        // Test verifies timeout errors are categorized as transient
        // Ensures proper retry behavior for temporary service slowdowns

        // Arrange
        let timeout_seconds = 30;

        // Act
        let error = LlmError::timeout(timeout_seconds);

        // Assert
        assert_eq!(error.category(), ErrorCategory::Transient);
        assert_eq!(error.severity(), ErrorSeverity::Warning);
        assert!(error.is_retryable(), "Timeout errors should be retryable");
    }

    #[test]
    fn test_authentication_failed_categorization() {
        // Test verifies authentication errors are categorized as client errors
        // Ensures credential issues are treated as non-retryable configuration problems

        // Arrange
        let auth_message = "Invalid API key";

        // Act
        let error = LlmError::authentication_failed(auth_message);

        // Assert
        assert_eq!(error.category(), ErrorCategory::Client);
        assert_eq!(error.severity(), ErrorSeverity::Error);
        assert!(
            !error.is_retryable(),
            "Authentication errors should not be retryable"
        );
    }

    #[test]
    fn test_token_limit_exceeded_categorization() {
        // Test verifies token limit errors are categorized as client errors with info severity
        // Ensures token limits are treated as user input issues, not system failures

        // Arrange
        let current_tokens = 5000;
        let max_tokens = 4000;

        // Act
        let error = LlmError::token_limit_exceeded(current_tokens, max_tokens);

        // Assert
        assert_eq!(error.category(), ErrorCategory::Client);
        assert_eq!(error.severity(), ErrorSeverity::Info);
        assert!(
            !error.is_retryable(),
            "Token limit errors require user input changes"
        );
    }
}

#[cfg(test)]
mod llm_error_user_messages_tests {
    use super::*;

    #[test]
    fn test_unsupported_provider_user_message() {
        // Test verifies user messages hide technical provider details
        // Ensures users get actionable guidance without internal information

        // Arrange
        let provider_name = "internal-test-provider";

        // Act
        let error = LlmError::unsupported_provider(provider_name);
        let user_message = error.user_message();

        // Assert
        assert_eq!(user_message, "The requested AI provider is not supported");
        assert!(
            !user_message.contains("internal-test-provider"),
            "User message should not expose internal provider names"
        );
    }

    #[test]
    fn test_configuration_error_user_message() {
        // Test verifies configuration error messages guide users to fix settings
        // Ensures technical configuration details are not exposed to users

        // Arrange
        let technical_message = "ANTHROPIC_API_KEY environment variable not set";

        // Act
        let error = LlmError::configuration_error(technical_message);
        let user_message = error.user_message();

        // Assert
        assert_eq!(
            user_message,
            "AI service configuration issue. Please check your settings"
        );
        assert!(
            !user_message.contains("ANTHROPIC_API_KEY"),
            "User message should not expose technical configuration details"
        );
    }

    #[test]
    fn test_rate_limit_exceeded_user_message_includes_wait_time() {
        // Test verifies rate limit messages include actionable wait time information
        // Ensures users know exactly how long to wait before retrying

        // Arrange
        let retry_after_seconds = 120;

        // Act
        let error = LlmError::rate_limit_exceeded(retry_after_seconds);
        let user_message = error.user_message();

        // Assert
        assert_eq!(
            user_message,
            "Service is busy. Please wait 120 seconds and try again"
        );
        assert!(
            user_message.contains("120"),
            "Should include specific wait time"
        );
    }

    #[test]
    fn test_authentication_failed_user_message() {
        // Test verifies authentication error messages guide users to check credentials
        // Ensures specific API key details are not exposed in user messages

        // Arrange
        let technical_message = "API key 'sk-abc123...' is invalid or expired";

        // Act
        let error = LlmError::authentication_failed(technical_message);
        let user_message = error.user_message();

        // Assert
        assert_eq!(
            user_message,
            "Authentication failed. Please check your credentials"
        );
        assert!(
            !user_message.contains("sk-abc123"),
            "User message should not expose API key details"
        );
    }

    #[test]
    fn test_token_limit_exceeded_user_message() {
        // Test verifies token limit messages provide actionable guidance
        // Ensures users understand they need to shorten their input

        // Arrange
        let current_tokens = 5000;
        let max_tokens = 4000;

        // Act
        let error = LlmError::token_limit_exceeded(current_tokens, max_tokens);
        let user_message = error.user_message();

        // Assert
        assert_eq!(
            user_message,
            "Your request is too long. Please shorten it and try again"
        );
        assert!(
            !user_message.contains("5000"),
            "User message should not expose technical token counts"
        );
    }
}

#[cfg(test)]
mod llm_error_constructor_tests {
    use super::*;

    #[test]
    fn test_response_parsing_error_creation() {
        // Test verifies response parsing errors are created with proper severity
        // Ensures parsing failures are treated as warnings, not critical errors

        // Arrange
        let parsing_message = "Invalid JSON response format";

        // Act
        let error = LlmError::response_parsing_error(parsing_message);

        // Assert
        assert_eq!(error.category(), ErrorCategory::External);
        assert_eq!(error.severity(), ErrorSeverity::Warning);
        assert!(
            !error.is_retryable(),
            "Parsing errors typically require different handling"
        );

        // Verify error message is preserved
        assert!(error.to_string().contains("Invalid JSON response format"));
    }

    #[test]
    fn test_tool_execution_failed_creation() {
        // Test verifies tool execution errors preserve context about which tool failed
        // Ensures proper debugging information for tool-related issues

        // Arrange
        let tool_name = "web_search";
        let error_message = "Network timeout during search";

        // Act
        let error = LlmError::tool_execution_failed(tool_name, error_message);

        // Assert
        assert_eq!(error.category(), ErrorCategory::External);
        assert_eq!(error.severity(), ErrorSeverity::Error);
        assert!(
            !error.is_retryable(),
            "Tool execution failures require specific handling"
        );

        // Verify both tool name and message are preserved
        let error_string = error.to_string();
        assert!(error_string.contains("web_search"));
        assert!(error_string.contains("Network timeout during search"));
    }

    #[test]
    fn test_schema_validation_failed_creation() {
        // Test verifies schema validation errors are properly categorized
        // Ensures structured output validation failures get appropriate treatment

        // Arrange
        let validation_message = "Required field 'title' missing from response";

        // Act
        let error = LlmError::schema_validation_failed(validation_message);

        // Assert
        assert_eq!(error.category(), ErrorCategory::Client);
        assert_eq!(error.severity(), ErrorSeverity::Warning);
        assert!(
            !error.is_retryable(),
            "Schema validation requires schema or prompt fixes"
        );

        // Verify validation details are preserved
        assert!(error.to_string().contains("Required field 'title' missing"));
    }

    #[test]
    fn test_request_failed_with_source_error() {
        // Test verifies request failures properly preserve source error context
        // Ensures error chain is maintained for debugging purposes

        // Arrange
        let request_message = "HTTP request failed";
        let source_error = Box::new(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "Connection timeout",
        ));

        // Act
        let error = LlmError::request_failed(request_message, Some(source_error));

        // Assert
        assert_eq!(error.category(), ErrorCategory::External);
        assert_eq!(error.severity(), ErrorSeverity::Error);
        assert!(
            error.is_retryable(),
            "Network request failures should be retryable"
        );

        // Verify both message and source are available
        assert!(error.to_string().contains("HTTP request failed"));
        assert!(
            std::error::Error::source(&error).is_some(),
            "Source error should be preserved"
        );
    }
}

#[cfg(test)]
mod llm_error_display_tests {
    use super::*;

    #[test]
    fn test_error_display_format_consistency() {
        // Test verifies error display messages follow consistent formatting
        // Ensures error messages are properly formatted for logging and debugging

        // Arrange
        let provider = "test-provider";
        let timeout_seconds = 45;
        let current_tokens = 1500;
        let max_tokens = 1000;

        // Act
        let unsupported_error = LlmError::unsupported_provider(provider);
        let timeout_error = LlmError::timeout(timeout_seconds);
        let token_error = LlmError::token_limit_exceeded(current_tokens, max_tokens);

        // Assert
        assert_eq!(
            unsupported_error.to_string(),
            "Provider not supported: test-provider"
        );
        assert_eq!(timeout_error.to_string(), "Request timed out after 45s");
        assert_eq!(token_error.to_string(), "Token limit exceeded: 1500 > 1000");
    }

    #[test]
    fn test_error_debug_representation() {
        // Test verifies error debug format includes variant information
        // Ensures debugging output provides sufficient context for troubleshooting

        // Arrange
        let error = LlmError::configuration_error("Test configuration issue");

        // Act
        let debug_string = format!("{:?}", error);

        // Assert
        assert!(debug_string.contains("ConfigurationError"));
        assert!(debug_string.contains("Test configuration issue"));
    }
}
