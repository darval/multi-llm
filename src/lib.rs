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

// =============================================================================
// Module declarations
// =============================================================================

// Public modules - flattened structure matching DESIGN.md
pub mod client;
pub mod config;
pub mod error;
pub mod messages;
pub mod provider;
pub mod providers;

// Internal modules
pub(crate) mod internals;
pub(crate) mod logging;

#[cfg(test)]
pub mod tests;

// =============================================================================
// Public API re-exports (~28 types as per issue #4)
// =============================================================================

// Client
pub use client::UnifiedLLMClient;

// Configuration
pub use config::{
    AnthropicConfig, DefaultLLMParams, LLMConfig, LMStudioConfig, OllamaConfig, OpenAIConfig,
    ProviderConfig,
};

// Errors
pub use error::{LlmError, LlmResult};

// Messages - the core unified message architecture
pub use messages::{
    CacheType, MessageAttributes, MessageCategory, MessageContent, MessageRole, UnifiedLLMRequest,
    UnifiedMessage,
};

// Provider trait and types
pub use provider::{
    LlmProvider, RequestConfig, Response, ResponseFormat, TokenUsage, Tool, ToolCall,
    ToolCallingRound, ToolChoice, ToolResult,
};

// Providers
pub use providers::{AnthropicProvider, LMStudioProvider, OllamaProvider, OpenAIProvider};

// Token counting (from internals, re-exported for public use)
pub use internals::tokens::{
    AnthropicTokenCounter, OpenAITokenCounter, TokenCounter, TokenCounterFactory,
};

// Retry policy (from internals, re-exported for public use)
pub use internals::retry::RetryPolicy;

// Event types - only available with "events" feature
#[cfg(feature = "events")]
pub use internals::events::{event_types, BusinessEvent, EventScope};
#[cfg(feature = "events")]
pub use provider::LLMBusinessEvent;

// =============================================================================
// Helper macro for handling response types with/without events feature
// =============================================================================

/// Extract the Response from execute_llm results, regardless of events feature.
///
/// When the `events` feature is enabled, `execute_llm` returns
/// `Result<(Response, Vec<LLMBusinessEvent>)>`. Without the feature, it returns
/// `Result<Response>`. This macro handles both cases uniformly.
///
/// # Example
///
/// ```rust,ignore
/// use multi_llm::{unwrap_response, UnifiedLLMClient, LlmProvider};
///
/// let response = unwrap_response!(client.execute_llm(request, None, None).await?);
/// println!("Content: {}", response.content);
/// ```
///
/// # With events feature
///
/// If you need access to the events, don't use this macro - instead pattern match directly:
///
/// ```rust,ignore
/// #[cfg(feature = "events")]
/// let (response, events) = client.execute_llm(request, None, None).await?;
/// ```
#[cfg(feature = "events")]
#[macro_export]
macro_rules! unwrap_response {
    ($result:expr) => {{
        let (resp, _events) = $result;
        resp
    }};
}

/// Extract the Response from execute_llm results (non-events version).
///
/// See the `events` feature version for full documentation.
#[cfg(not(feature = "events"))]
#[macro_export]
macro_rules! unwrap_response {
    ($result:expr) => {
        $result
    };
}
