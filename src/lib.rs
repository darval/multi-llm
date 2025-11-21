//! # multi-llm
//!
//! Unified multi-provider LLM client with support for OpenAI, Anthropic, Ollama, and LMStudio.
//!
//! ## Key Features
//!
//! - **Multiple Providers**: Seamless switching between LLM providers
//! - **Unified Messages**: Provider-agnostic message architecture with caching hints
//! - **Prompt Caching**: Native support for Anthropic prompt caching
//! - **Tool Calling**: First-class function/tool calling support
//! - **Resilience**: Built-in retry logic, rate limiting, and error handling
//!
//! ## Example
//!
//! ```rust,no_run
//! use multi_llm::{UnifiedLLMClient, LLMConfig, OpenAIConfig, UnifiedMessage};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = LLMConfig::openai(OpenAIConfig {
//!     api_key: "your-api-key".to_string(),
//!     model: "gpt-4".to_string(),
//!     ..Default::default()
//! });
//!
//! let client = UnifiedLLMClient::new(config)?;
//! let messages = vec![UnifiedMessage::user("Hello, how are you?")];
//! // Use client.execute_llm(...) for actual requests
//! # Ok(())
//! # }
//! ```

// Allow missing errors documentation - errors are self-documenting via type signatures
#![allow(clippy::missing_errors_doc)]
// Allow unreachable in provider clone - all types are covered but compiler can't verify
#![allow(clippy::unreachable)]

use serde::{Deserialize, Serialize};

// Core types (extracted from mystory-core for Phase 1)
// Phase 2 will refactor these into proper public API modules
pub mod core_types;

// Logging utilities (re-exports tracing with log_* naming)
pub mod logging;

pub mod agents;
pub mod client;
pub mod config;
pub mod error;
pub mod providers;
pub mod response_parser;
pub mod retry;
pub mod tokens;

#[cfg(test)]
pub mod tests;

// Re-export main types
pub use agents::AgentContext;
pub use client::UnifiedLLMClient;
pub use config::{
    AnthropicConfig, DefaultLLMParams, DualLLMConfig, LLMConfig, LLMPath, LMStudioConfig,
    OllamaConfig, OpenAIConfig, ProviderConfig,
};
pub use error::{LlmError, LlmResult};
pub use providers::{AnthropicProvider, LMStudioProvider, OllamaProvider, OpenAIProvider};
pub use tokens::{AnthropicTokenCounter, OpenAITokenCounter, TokenCounter, TokenCounterFactory};

// Re-export core types (unified messages and executor types)
pub use core_types::{
    // Messages - the core unified message architecture
    MessageAttributes, MessageCategory, MessageContent, MessageRole, UnifiedLLMRequest,
    UnifiedMessage,
    // Executor types
    ExecutorLLMConfig, ExecutorLLMProvider, ExecutorLLMResponse, ExecutorResponseFormat,
    ExecutorTool, ExecutorToolCall, ExecutorToolResult, ExecutorTokenUsage, LLMBusinessEvent,
    ToolCallingRound, ToolChoice,
    // Events
    BusinessEvent, EventScope, event_types,
    // Errors
    ErrorCategory, ErrorSeverity, MyStoryError, UserErrorCategory,
};

// Re-export LLM-specific types
pub use types::{LLMMetadata, LLMRequest, LLMToolCall, LLMUsage};

// Convenience type alias
pub use ExecutorTool as Tool;

// Re-export logging macros for convenience
pub use logging::{log_debug, log_error, log_info, log_trace, log_warn};

/// Common types used across the LLM abstraction
pub mod types {
    use super::*;

    // Message types now come from mystory-core - removed duplicates

    // REMOVED: Tool type - now using crate::core_types::executor::ExecutorTool
    // This consolidates tool types following Rusty's three-layer architecture
    //
    // REMOVED: LLMResponseFormat - use crate::core_types::executor::ExecutorResponseFormat instead
    // This type is no longer used following Rusty's consolidation plan.

    /// Tool call from LLM response
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct LLMToolCall {
        /// Unique identifier for this tool call
        pub id: String,
        /// Name of the function/tool to call
        pub name: String,
        /// Arguments to pass to the function (as JSON)
        pub arguments: serde_json::Value,
    }

    /// Token usage statistics
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct LLMUsage {
        /// Number of tokens in the prompt
        pub prompt_tokens: u32,
        /// Number of tokens in the completion
        pub completion_tokens: u32,
        /// Total tokens used (prompt + completion)
        pub total_tokens: u32,
    }

    /// Simple request structure for backward compatibility
    #[derive(Debug, Clone)]
    pub struct LLMRequest {
        pub user_input: String,
        pub add_to_history: bool,
    }

    /// Metadata from LLM response that should be preserved during conversion
    #[derive(Debug, Clone)]
    pub struct LLMMetadata {
        pub usage: Option<LLMUsage>,
        pub raw_body: Option<String>,
    }
}
