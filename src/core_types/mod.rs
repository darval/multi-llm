//! Core types for unified multi-provider LLM abstraction
//!
//! Provides provider-agnostic message formats, executor traits, and error types.
//! Phase 2 will review and refactor these into a more generic, standalone system.
//!
//! ## Organization
//! - `errors` - Error traits and types
//! - `messages` - Unified message architecture (core feature of multi-llm)
//! - `executor` - Executor types and LLM provider trait
//! - `events` - Business event logging types

pub mod errors;
pub mod events;
pub mod executor;
pub mod messages;

// Re-export commonly used types
pub use errors::{ErrorCategory, ErrorSeverity, MyStoryError, UserErrorCategory};
pub use events::{event_types, BusinessEvent, EventScope};
pub use executor::{
    ExecutorLLMConfig, ExecutorLLMProvider, ExecutorLLMResponse, ExecutorResponseFormat,
    ExecutorTokenUsage, ExecutorTool, ExecutorToolCall, ExecutorToolResult, LLMBusinessEvent,
    ToolCallingRound, ToolChoice,
};
pub use messages::{
    MessageAttributes, MessageCategory, MessageContent, MessageRole, UnifiedLLMRequest,
    UnifiedMessage,
};

/// Result type alias for core operations
pub type Result<T> = std::result::Result<T, anyhow::Error>;
