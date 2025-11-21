//! Error types extracted from mystory-core
//!
//! Phase 2 will review whether MyStoryError trait is needed or if standard Error is sufficient.

/// Base trait for categorized errors (from mystory-core)
pub trait MyStoryError: std::error::Error + Send + Sync + 'static {
    /// Error category for routing and handling decisions
    fn category(&self) -> ErrorCategory;

    /// Severity for logging and alerting
    fn severity(&self) -> ErrorSeverity;

    /// Whether this error should trigger a retry
    fn is_retryable(&self) -> bool {
        false
    }

    /// Convert to a user-friendly message (strips technical details)
    fn user_message(&self) -> String {
        "An error occurred while processing your request".to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Expected business logic outcomes
    BusinessLogic,
    /// External service failures (LLM, storage)
    External,
    /// Internal system errors (bugs, invariant violations)
    Internal,
    /// Client errors (invalid input, authentication)
    Client,
    /// Temporary failures that should be retried
    Transient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// System is unusable
    Critical,
    /// Action failed but system is stable
    Error,
    /// Unexpected but recoverable
    Warning,
    /// Expected failure (e.g., not found)
    Info,
}

/// User-facing error categories for conversation flow control
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UserErrorCategory {
    /// User needs to complete a prerequisite action
    WorkflowDependency,
    /// Request is missing required context/parameters
    MissingContext,
    /// Requested item/resource not found
    NotFound,
    /// Attempting to create something that already exists
    Duplicate,
    /// Input validation failed
    Validation,
    /// Technical/system error - don't expose details to user
    Technical,
}
