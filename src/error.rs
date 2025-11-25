use crate::core_types::{ErrorCategory, ErrorSeverity};
use crate::logging::{log_error, log_warn};
use thiserror::Error;

/// Result type for LLM operations  
pub type LlmResult<T> = std::result::Result<T, LlmError>;

/// Errors that can occur during LLM operations
#[derive(Error, Debug)]
pub enum LlmError {
    #[error("Provider not supported: {provider}")]
    UnsupportedProvider { provider: String },

    #[error("Provider configuration error: {message}")]
    ConfigurationError { message: String },

    #[error("Request failed: {message}")]
    RequestFailed {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Response parsing failed: {message}")]
    ResponseParsingError { message: String },

    #[error("Rate limit exceeded, retry after {retry_after_seconds}s")]
    RateLimitExceeded { retry_after_seconds: u64 },

    #[error("Request timed out after {timeout_seconds}s")]
    Timeout { timeout_seconds: u64 },

    #[error("Authentication failed: {message}")]
    AuthenticationFailed { message: String },

    #[error("Token limit exceeded: {current} > {max}")]
    TokenLimitExceeded { current: usize, max: usize },

    #[error("Tool execution failed: {tool_name} - {message}")]
    ToolExecutionFailed { tool_name: String, message: String },

    #[error("JSON schema validation failed: {message}")]
    SchemaValidationFailed { message: String },
}

impl LlmError {
    /// Get the error category for routing and handling decisions
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::UnsupportedProvider { .. } => ErrorCategory::Client,
            Self::ConfigurationError { .. } => ErrorCategory::Client,
            Self::RequestFailed { .. } => ErrorCategory::External,
            Self::ResponseParsingError { .. } => ErrorCategory::External,
            Self::RateLimitExceeded { .. } => ErrorCategory::Transient,
            Self::Timeout { .. } => ErrorCategory::Transient,
            Self::AuthenticationFailed { .. } => ErrorCategory::Client,
            Self::TokenLimitExceeded { .. } => ErrorCategory::Client,
            Self::ToolExecutionFailed { .. } => ErrorCategory::External,
            Self::SchemaValidationFailed { .. } => ErrorCategory::Client,
        }
    }

    /// Get the error severity for logging and alerting
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::UnsupportedProvider { .. } => ErrorSeverity::Error,
            Self::ConfigurationError { .. } => ErrorSeverity::Error,
            Self::RequestFailed { .. } => ErrorSeverity::Error,
            Self::ResponseParsingError { .. } => ErrorSeverity::Warning,
            Self::RateLimitExceeded { .. } => ErrorSeverity::Warning,
            Self::Timeout { .. } => ErrorSeverity::Warning,
            Self::AuthenticationFailed { .. } => ErrorSeverity::Error,
            Self::TokenLimitExceeded { .. } => ErrorSeverity::Info,
            Self::ToolExecutionFailed { .. } => ErrorSeverity::Error,
            Self::SchemaValidationFailed { .. } => ErrorSeverity::Warning,
        }
    }

    /// Whether this error should trigger a retry
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimitExceeded { .. } | Self::Timeout { .. } | Self::RequestFailed { .. }
        )
    }

    /// Convert to a user-friendly message (strips technical details)
    pub fn user_message(&self) -> String {
        match self {
            Self::UnsupportedProvider { .. } => {
                "The requested AI provider is not supported".to_string()
            }
            Self::ConfigurationError { .. } => {
                "AI service configuration issue. Please check your settings".to_string()
            }
            Self::RequestFailed { .. } => {
                "Unable to communicate with AI service. Please try again".to_string()
            }
            Self::ResponseParsingError { .. } => {
                "Received an invalid response from AI service".to_string()
            }
            Self::RateLimitExceeded {
                retry_after_seconds,
            } => {
                format!("Service is busy. Please wait {retry_after_seconds} seconds and try again")
            }
            Self::Timeout { .. } => "Request timed out. Please try again".to_string(),
            Self::AuthenticationFailed { .. } => {
                "Authentication failed. Please check your credentials".to_string()
            }
            Self::TokenLimitExceeded { .. } => {
                "Your request is too long. Please shorten it and try again".to_string()
            }
            Self::ToolExecutionFailed { .. } => {
                "Unable to execute the requested action".to_string()
            }
            Self::SchemaValidationFailed { .. } => "Response format validation failed".to_string(),
        }
    }

    // Constructor methods with automatic logging
    pub fn unsupported_provider(provider: impl Into<String>) -> Self {
        let provider = provider.into();
        log_error!(
            provider = %provider,
            error_type = "unsupported_provider",
            "Unsupported LLM provider requested"
        );
        Self::UnsupportedProvider { provider }
    }

    pub fn configuration_error(message: impl Into<String>) -> Self {
        let message = message.into();
        log_error!(
            error_type = "configuration_error",
            message = %message,
            "LLM configuration validation failed"
        );
        Self::ConfigurationError { message }
    }

    pub fn request_failed(
        message: impl Into<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        let message = message.into();
        log_error!(
            error_type = "request_failed",
            message = %message,
            has_source = source.is_some(),
            "LLM request execution failed"
        );
        Self::RequestFailed { message, source }
    }

    pub fn response_parsing_error(message: impl Into<String>) -> Self {
        let message = message.into();
        log_warn!(
            error_type = "response_parsing_error",
            message = %message,
            "LLM response format invalid"
        );
        Self::ResponseParsingError { message }
    }

    pub fn rate_limit_exceeded(retry_after_seconds: u64) -> Self {
        log_warn!(
            error_type = "rate_limit_exceeded",
            retry_after_seconds = retry_after_seconds,
            "LLM provider rate limit exceeded"
        );
        Self::RateLimitExceeded {
            retry_after_seconds,
        }
    }

    pub fn timeout(timeout_seconds: u64) -> Self {
        log_warn!(
            error_type = "timeout",
            timeout_seconds = timeout_seconds,
            "LLM request timed out"
        );
        Self::Timeout { timeout_seconds }
    }

    pub fn authentication_failed(message: impl Into<String>) -> Self {
        let message = message.into();
        log_error!(
            error_type = "authentication_failed",
            message = %message,
            "LLM provider authentication failed"
        );
        Self::AuthenticationFailed { message }
    }

    pub fn token_limit_exceeded(current: usize, max: usize) -> Self {
        log_warn!(
            error_type = "token_limit_exceeded",
            current_tokens = current,
            max_tokens = max,
            "Request exceeds LLM token limit"
        );
        Self::TokenLimitExceeded { current, max }
    }

    pub fn tool_execution_failed(tool_name: impl Into<String>, message: impl Into<String>) -> Self {
        let tool_name = tool_name.into();
        let message = message.into();
        log_error!(
            error_type = "tool_execution_failed",
            tool_name = %tool_name,
            message = %message,
            "LLM tool execution failed"
        );
        Self::ToolExecutionFailed { tool_name, message }
    }

    pub fn schema_validation_failed(message: impl Into<String>) -> Self {
        let message = message.into();
        log_warn!(
            error_type = "schema_validation_failed",
            message = %message,
            "LLM response schema validation failed"
        );
        Self::SchemaValidationFailed { message }
    }
}
