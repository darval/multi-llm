//! Error categorization and severity types

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
