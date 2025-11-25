//! Error types for LLM operations.
//!
//! This module provides structured error handling for multi-llm operations,
//! including categorization, severity levels, and retry guidance.
//!
//! # Error Types
//!
//! The main error type is [`LlmError`], which covers all failure modes:
//! - Configuration errors (missing API keys, invalid settings)
//! - Request failures (network issues, provider errors)
//! - Rate limiting and timeouts
//! - Authentication failures
//! - Token limit exceeded
//! - Tool execution failures
//!
//! # Error Handling Example
//!
//! ```rust,no_run
//! use multi_llm::{LlmError, LlmResult};
//!
//! fn handle_error(err: LlmError) {
//!     // Check if we should retry
//!     if err.is_retryable() {
//!         println!("Retryable error: {}", err);
//!         // Implement retry logic...
//!     }
//!
//!     // Get user-friendly message
//!     let user_msg = err.user_message();
//!     println!("Tell user: {}", user_msg);
//!
//!     // Check error category for routing
//!     match err.category() {
//!         multi_llm::error::ErrorCategory::Transient => {
//!             println!("Temporary issue, try again later");
//!         }
//!         multi_llm::error::ErrorCategory::Client => {
//!             println!("Fix the request and try again");
//!         }
//!         _ => {
//!             println!("System issue, contact support");
//!         }
//!     }
//! }
//! ```
//!
//! # Result Type
//!
//! Use [`LlmResult<T>`] as a convenient alias for `Result<T, LlmError>`:
//!
//! ```rust
//! use multi_llm::LlmResult;
//!
//! fn my_function() -> LlmResult<String> {
//!     Ok("Success".to_string())
//! }
//! ```

use crate::logging::{log_error, log_warn};
use thiserror::Error;

// ============================================================================
// Error categorization types
// ============================================================================

/// High-level categorization of errors for routing and handling decisions.
///
/// Use [`LlmError::category()`] to get the category for any error.
///
/// # Example
///
/// ```rust,no_run
/// use multi_llm::{LlmError, error::ErrorCategory};
///
/// fn should_alert_ops(err: &LlmError) -> bool {
///     matches!(err.category(), ErrorCategory::Internal | ErrorCategory::External)
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Expected business logic outcomes (not typically errors).
    ///
    /// These are "errors" that represent normal application flow,
    /// like "user not found" when checking if a user exists.
    BusinessLogic,

    /// External service failures (LLM providers, network issues).
    ///
    /// The LLM provider or network had an issue. May be transient
    /// or indicate a provider outage.
    External,

    /// Internal system errors (bugs, invariant violations).
    ///
    /// Something went wrong in the code itself. These should be
    /// logged and investigated.
    Internal,

    /// Client errors (invalid input, authentication, configuration).
    ///
    /// The caller made a mistake that they can fix (wrong API key,
    /// invalid parameters, etc.).
    Client,

    /// Temporary failures that should be retried.
    ///
    /// Rate limits, timeouts, and other transient issues. Retry
    /// with exponential backoff.
    Transient,
}

/// Severity level for logging and alerting decisions.
///
/// Use [`LlmError::severity()`] to get the severity for any error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// System is unusable or data integrity is at risk.
    ///
    /// Requires immediate attention. Page on-call if configured.
    Critical,

    /// Action failed but system is stable.
    ///
    /// Should be logged and investigated but not urgent.
    Error,

    /// Unexpected but recoverable situation.
    ///
    /// Worth logging for monitoring but may not require action.
    Warning,

    /// Expected failure (e.g., not found, validation error).
    ///
    /// Normal operation, log at info/debug level.
    Info,
}

/// User-facing error categories for conversation flow control.
///
/// When a tool execution fails, this category helps the LLM understand
/// how to respond to the user and what actions might help.
///
/// # Example
///
/// ```rust
/// use multi_llm::{ToolResult, error::UserErrorCategory};
///
/// // User needs to complete a prerequisite first
/// let result = ToolResult {
///     tool_call_id: "call_123".to_string(),
///     content: "Please log in first".to_string(),
///     is_error: true,
///     error_category: Some(UserErrorCategory::WorkflowDependency),
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UserErrorCategory {
    /// User needs to complete a prerequisite action.
    ///
    /// Example: "You need to log in before accessing your profile."
    WorkflowDependency,

    /// Request is missing required context/parameters.
    ///
    /// Example: "Please specify which city you want weather for."
    MissingContext,

    /// Requested item/resource not found.
    ///
    /// Example: "I couldn't find a user with that email address."
    NotFound,

    /// Attempting to create something that already exists.
    ///
    /// Example: "An account with that email already exists."
    Duplicate,

    /// Input validation failed.
    ///
    /// Example: "That doesn't look like a valid email address."
    Validation,

    /// Technical/system error - don't expose details to user.
    ///
    /// Example: "Something went wrong. Please try again later."
    Technical,
}

// ============================================================================
// LLM Error types
// ============================================================================

/// Convenient result type for LLM operations.
///
/// Alias for `Result<T, LlmError>`. Use this throughout your application
/// for consistent error handling.
///
/// # Example
///
/// ```rust
/// use multi_llm::LlmResult;
///
/// fn process_response(text: &str) -> LlmResult<String> {
///     if text.is_empty() {
///         return Err(multi_llm::LlmError::response_parsing_error("Empty response"));
///     }
///     Ok(text.to_uppercase())
/// }
/// ```
pub type LlmResult<T> = std::result::Result<T, LlmError>;

/// Errors that can occur during LLM operations.
///
/// This enum covers all error conditions you might encounter when using multi-llm.
/// Each variant includes relevant context and can be:
/// - Categorized via [`category()`](Self::category)
/// - Assessed for severity via [`severity()`](Self::severity)
/// - Checked for retryability via [`is_retryable()`](Self::is_retryable)
/// - Converted to user-friendly messages via [`user_message()`](Self::user_message)
///
/// # Creating Errors
///
/// Use the constructor methods which automatically log the error:
///
/// ```rust
/// use multi_llm::LlmError;
///
/// // These methods log automatically
/// let err = LlmError::configuration_error("Missing API key");
/// let err = LlmError::rate_limit_exceeded(60);
/// let err = LlmError::timeout(30);
/// ```
///
/// # Error Categories
///
/// | Variant | Category | Retryable |
/// |---------|----------|-----------|
/// | `UnsupportedProvider` | Client | No |
/// | `ConfigurationError` | Client | No |
/// | `RequestFailed` | External | Yes |
/// | `ResponseParsingError` | External | No |
/// | `RateLimitExceeded` | Transient | Yes |
/// | `Timeout` | Transient | Yes |
/// | `AuthenticationFailed` | Client | No |
/// | `TokenLimitExceeded` | Client | No |
/// | `ToolExecutionFailed` | External | No |
/// | `SchemaValidationFailed` | Client | No |
#[derive(Error, Debug)]
pub enum LlmError {
    /// The specified provider is not supported.
    ///
    /// Supported providers: "anthropic", "openai", "ollama", "lmstudio"
    #[error("Provider not supported: {provider}")]
    UnsupportedProvider {
        /// The provider name that was requested.
        provider: String,
    },

    /// Provider configuration is invalid or incomplete.
    ///
    /// Common causes:
    /// - Missing API key for providers that require one
    /// - Invalid base URL format
    /// - Incompatible configuration values
    #[error("Provider configuration error: {message}")]
    ConfigurationError {
        /// Description of the configuration problem.
        message: String,
    },

    /// The HTTP request to the provider failed.
    ///
    /// This is a general failure that may be retryable. Check the source
    /// error for more details about the underlying cause.
    #[error("Request failed: {message}")]
    RequestFailed {
        /// Description of the failure.
        message: String,
        /// The underlying error, if available.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Failed to parse the provider's response.
    ///
    /// The provider returned a response, but it couldn't be parsed.
    /// This might indicate a provider API change or malformed response.
    #[error("Response parsing failed: {message}")]
    ResponseParsingError {
        /// Details about the parsing failure.
        message: String,
    },

    /// Provider rate limit exceeded.
    ///
    /// The provider is throttling requests. Wait the indicated time
    /// before retrying. Consider implementing exponential backoff.
    #[error("Rate limit exceeded, retry after {retry_after_seconds}s")]
    RateLimitExceeded {
        /// Recommended wait time before retrying.
        retry_after_seconds: u64,
    },

    /// Request timed out.
    ///
    /// The provider didn't respond within the configured timeout.
    /// This is usually retryable but may indicate an overloaded provider.
    #[error("Request timed out after {timeout_seconds}s")]
    Timeout {
        /// The timeout duration that was exceeded.
        timeout_seconds: u64,
    },

    /// Authentication with the provider failed.
    ///
    /// Check your API key or credentials. This is not retryable without
    /// fixing the authentication.
    #[error("Authentication failed: {message}")]
    AuthenticationFailed {
        /// Details about the authentication failure.
        message: String,
    },

    /// Request exceeds the model's token limit.
    ///
    /// The combined input (messages + tools) is too large for the model's
    /// context window. Reduce the input size or use a model with larger context.
    #[error("Token limit exceeded: {current} > {max}")]
    TokenLimitExceeded {
        /// The actual token count of the request.
        current: usize,
        /// The maximum allowed tokens for the model.
        max: usize,
    },

    /// A tool execution failed.
    ///
    /// The tool was called but couldn't complete successfully.
    /// Check the message for details about why the tool failed.
    #[error("Tool execution failed: {tool_name} - {message}")]
    ToolExecutionFailed {
        /// The name of the tool that failed.
        tool_name: String,
        /// Details about the failure.
        message: String,
    },

    /// Response doesn't match the requested JSON schema.
    ///
    /// When using structured output, the model's response didn't conform
    /// to the provided JSON schema. May require a clearer prompt or
    /// different schema design.
    #[error("JSON schema validation failed: {message}")]
    SchemaValidationFailed {
        /// Details about the validation failure.
        message: String,
    },
}

impl LlmError {
    /// Get the error category for routing and handling decisions.
    ///
    /// Use this to determine how to handle different types of errors:
    /// - `Client`: Fix the request (invalid input, auth, config)
    /// - `External`: Provider issue, may need ops attention
    /// - `Transient`: Retry with backoff
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multi_llm::{LlmError, error::ErrorCategory};
    ///
    /// fn handle(err: LlmError) {
    ///     match err.category() {
    ///         ErrorCategory::Transient => {
    ///             // Implement retry logic
    ///         }
    ///         ErrorCategory::Client => {
    ///             // User can fix this, show helpful message
    ///         }
    ///         _ => {
    ///             // Log for investigation
    ///         }
    ///     }
    /// }
    /// ```
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

    /// Get the error severity for logging and alerting.
    ///
    /// Use this to determine logging level and whether to alert on-call.
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

    /// Whether this error is transient and should trigger a retry.
    ///
    /// Returns `true` for:
    /// - Rate limit exceeded
    /// - Timeouts
    /// - General request failures (may be network issues)
    ///
    /// Implement exponential backoff when retrying these errors.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimitExceeded { .. } | Self::Timeout { .. } | Self::RequestFailed { .. }
        )
    }

    /// Convert to a user-friendly message suitable for display.
    ///
    /// Returns a message that's safe to show to end users - technical
    /// details and internal information are stripped or generalized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use multi_llm::LlmError;
    ///
    /// let err = LlmError::rate_limit_exceeded(60);
    /// let msg = err.user_message();
    /// // "Service is busy. Please wait 60 seconds and try again"
    /// ```
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

    // =========================================================================
    // Constructor methods with automatic logging
    // =========================================================================
    //
    // These methods automatically log the error at the appropriate level.
    // Use them instead of constructing variants directly.

    /// Create an unsupported provider error (logs at ERROR level).
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
