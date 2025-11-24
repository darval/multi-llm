//! Core types for unified multi-provider LLM abstraction
//!
//! Provides provider-agnostic message formats, provider traits, and error types.
//!
//! ## Organization
//! - `errors` - Error traits and types
//! - `messages` - Unified message architecture (core feature of multi-llm)
//! - `provider` - Provider trait and types (requests, responses, tools)
//! - `events` - Business event logging types

pub mod errors;
#[cfg(feature = "events")]
pub mod events;
pub mod messages;
pub mod provider;

// Test modules
#[cfg(test)]
mod tests;

// Re-export commonly used types
pub use errors::{ErrorCategory, ErrorSeverity, UserErrorCategory};
#[cfg(feature = "events")]
pub use events::{event_types, BusinessEvent, EventScope};
pub use messages::{
    MessageAttributes, MessageCategory, MessageContent, MessageRole, UnifiedLLMRequest,
    UnifiedMessage,
};
#[cfg(feature = "events")]
pub use provider::LLMBusinessEvent;
pub use provider::{
    LlmProvider, RequestConfig, Response, ResponseFormat, TokenUsage, Tool, ToolCall,
    ToolCallingRound, ToolChoice, ToolResult,
};

/// Result type alias for core operations
pub type Result<T> = std::result::Result<T, anyhow::Error>;
