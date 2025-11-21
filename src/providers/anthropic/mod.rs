//! Anthropic Claude provider implementation
//!
//! This provider uses Anthropic's native API format with the Messages API.
//!
//! ## Module Organization
//!
//! - `types`: Request/response structures for Anthropic API
//! - `conversion`: Message conversion between unified and Anthropic formats
//! - `caching`: Prompt caching control logic
//! - `provider`: Main provider implementation

mod caching;
mod conversion;
mod provider;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types and the provider
pub use provider::AnthropicProvider;
pub use types::{
    CacheControl, SystemField as AnthropicSystemField, SystemMessage as AnthropicSystemMessage,
};
