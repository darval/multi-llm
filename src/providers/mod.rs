//! LLM Provider implementations
//!
//! This module contains implementations for different LLM providers:
//!
//! - **anthropic**: Anthropic Claude provider with native API format
//! - **openai**: OpenAI provider using OpenAI-compatible API
//! - **lmstudio**: LM Studio provider using OpenAI-compatible API
//! - **openai_shared**: Shared structures and utilities for OpenAI-compatible providers
//!
//! ## Architecture
//!
//! The providers are organized to highlight code reuse:
//!
//! ```text
//! openai_shared.rs    <- Shared OpenAI-compatible structures and utilities
//!      |        |        |
//!      |        |        |
//! openai.rs  lmstudio.rs  ollama.rs  <- All use OpenAI-compatible API
//!
//! anthropic.rs        <- Uses Anthropic's native API format
//! ```

pub mod anthropic;
pub mod lmstudio;
pub mod ollama;
pub mod openai;
pub mod openai_shared;

// Tests will be rewritten following the research checklist and unit test template
// to test the current LlmProvider trait API (execute_llm, not execute_chat_with_model)
#[cfg(test)]
mod tests;

// Re-export the provider structs
pub use anthropic::AnthropicProvider;
pub use lmstudio::LMStudioProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;
