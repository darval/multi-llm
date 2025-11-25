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
//! use multi_llm::{UnifiedLLMClient, LLMConfig, OpenAIConfig, DefaultLLMParams, UnifiedMessage};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = LLMConfig {
//!     provider: Box::new(OpenAIConfig {
//!         api_key: Some("your-api-key".to_string()),
//!         base_url: "https://api.openai.com".to_string(),
//!         default_model: "gpt-4".to_string(),
//!         max_context_tokens: 128_000,
//!         retry_policy: Default::default(),
//!     }),
//!     default_params: DefaultLLMParams::default(),
//! };
//!
//! let client = UnifiedLLMClient::from_config(config)?;
//! let messages = vec![UnifiedMessage::user("Hello, how are you?")];
//! // Use client.execute_llm(...) for actual requests
//! # Ok(())
//! # }
//! ```

// Allow missing errors documentation - errors are self-documenting via type signatures
#![allow(clippy::missing_errors_doc)]
// Allow unreachable in provider clone - all types are covered but compiler can't verify
#![allow(clippy::unreachable)]

// Core types for unified LLM abstraction
// Phase 2 will refactor these into proper public API modules
pub mod core_types;

// Logging utilities (re-exports tracing with log_* naming) - internal only
pub(crate) mod logging;

pub mod client;
pub mod config;
pub mod error;
pub mod providers;
pub(crate) mod response_parser;
pub mod retry;
pub mod tokens;

#[cfg(test)]
pub mod tests;

// Re-export main types
pub use client::UnifiedLLMClient;
pub use config::{
    AnthropicConfig, DefaultLLMParams, DualLLMConfig, LLMConfig, LLMPath, LMStudioConfig,
    OllamaConfig, OpenAIConfig, ProviderConfig,
};
pub use error::{LlmError, LlmResult};
pub use providers::{AnthropicProvider, LMStudioProvider, OllamaProvider, OpenAIProvider};
pub use tokens::{AnthropicTokenCounter, OpenAITokenCounter, TokenCounter, TokenCounterFactory};

// Re-export core types (unified messages and provider types)
pub use core_types::{
    // Provider trait
    LlmProvider,
    // Messages - the core unified message architecture
    MessageAttributes,
    MessageCategory,
    MessageContent,
    MessageRole,
    RequestConfig,
    Response,
    ResponseFormat,
    TokenUsage,
    Tool,
    ToolCall,
    ToolChoice,
    ToolResult,
    UnifiedLLMRequest,
    UnifiedMessage,
};

// Event types - only available with "events" feature
#[cfg(feature = "events")]
pub use core_types::{event_types, BusinessEvent, EventScope, LLMBusinessEvent};
