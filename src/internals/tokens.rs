//! Token counting utilities for LLM providers.
//!
//! This module provides token counting implementations for different LLM providers.
//! Accurate token counting is important for:
//! - Staying within context window limits
//! - Estimating API costs
//! - Optimizing prompts
//!
//! # Usage
//!
//! Use [`TokenCounterFactory`] to create counters for specific providers:
//!
//! ```rust,no_run
//! use multi_llm::{TokenCounterFactory, TokenCounter};
//!
//! // Create counter for OpenAI GPT-4
//! let counter = TokenCounterFactory::create_counter("openai", "gpt-4")?;
//!
//! // Count tokens in text
//! let tokens = counter.count_tokens("Hello, world!")?;
//! println!("Token count: {}", tokens);
//!
//! // Check context limit
//! let max = counter.max_context_tokens();
//! println!("Max context: {} tokens", max);
//! # Ok::<(), multi_llm::LlmError>(())
//! ```
//!
//! # Provider-Specific Notes
//!
//! - **OpenAI**: Uses tiktoken with exact tokenization
//! - **Anthropic**: Uses cl100k_base with 1.1x approximation factor
//! - **Ollama/LM Studio**: Uses cl100k_base (may vary by model)
//!
//! # Available Types
//!
//! - [`TokenCounter`]: Trait for all token counters
//! - [`OpenAITokenCounter`]: OpenAI GPT model tokenizer
//! - [`AnthropicTokenCounter`]: Anthropic Claude approximation
//! - [`TokenCounterFactory`]: Factory for creating counters

use crate::error::{LlmError, LlmResult};
use crate::logging::{log_debug, log_warn};

use std::sync::Arc;
use tiktoken_rs::{cl100k_base, o200k_base, CoreBPE};

/// Trait for counting tokens in text and messages.
///
/// Implement this trait to add support for new tokenizers.
/// Use [`TokenCounterFactory`] to create instances for supported providers.
///
/// # Example
///
/// ```rust,no_run
/// use multi_llm::{TokenCounter, TokenCounterFactory};
///
/// # fn example() -> multi_llm::LlmResult<()> {
/// let counter = TokenCounterFactory::create_counter("openai", "gpt-4")?;
///
/// // Count tokens
/// let count = counter.count_tokens("Hello, world!")?;
///
/// // Validate against limit
/// counter.validate_token_limit("Some text...")?;
///
/// // Truncate if needed
/// let truncated = counter.truncate_to_limit("Very long text...", 100)?;
/// # Ok(())
/// # }
/// ```
pub trait TokenCounter: Send + Sync + std::fmt::Debug {
    /// Count tokens in a text string.
    ///
    /// # Errors
    ///
    /// Returns [`LlmError::ConfigurationError`] if the tokenizer
    /// fails to encode the text.
    fn count_tokens(&self, text: &str) -> LlmResult<u32>;

    /// Count tokens in a list of messages (includes formatting overhead).
    ///
    /// The count includes tokens for role markers, message separators,
    /// and other provider-specific formatting.
    fn count_message_tokens(&self, messages: &[serde_json::Value]) -> LlmResult<u32>;

    /// Get the maximum context window size for this tokenizer.
    fn max_context_tokens(&self) -> u32;

    /// Validate that text doesn't exceed the token limit.
    ///
    /// # Errors
    ///
    /// Returns [`LlmError::TokenLimitExceeded`] if the text exceeds
    /// the maximum context window.
    fn validate_token_limit(&self, text: &str) -> LlmResult<()>;

    /// Truncate text to fit within a token limit.
    ///
    /// If the text already fits, it's returned unchanged.
    fn truncate_to_limit(&self, text: &str, max_tokens: u32) -> LlmResult<String>;
}

/// Token counter for OpenAI GPT models using tiktoken.
///
/// Provides exact token counts for OpenAI models. Automatically selects
/// the correct tokenizer based on the model name.
///
/// # Supported Models
///
/// | Model | Tokenizer | Context Window |
/// |-------|-----------|---------------|
/// | gpt-4-turbo | cl100k_base | 128K |
/// | gpt-4 | cl100k_base | 8K |
/// | gpt-3.5-turbo | cl100k_base | 16K |
/// | o1-* | o200k_base | 200K |
///
/// # Example
///
/// ```rust,no_run
/// use multi_llm::{OpenAITokenCounter, TokenCounter};
///
/// let counter = OpenAITokenCounter::new("gpt-4")?;
/// let tokens = counter.count_tokens("Hello, world!")?;
/// # Ok::<(), multi_llm::LlmError>(())
/// ```
pub struct OpenAITokenCounter {
    tokenizer: CoreBPE,
    max_tokens: u32,
    model_name: String,
}

impl std::fmt::Debug for OpenAITokenCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAITokenCounter")
            .field("max_tokens", &self.max_tokens)
            .field("model_name", &self.model_name)
            .finish()
    }
}

impl OpenAITokenCounter {
    /// Determine max tokens for GPT-4 model
    fn gpt4_max_tokens(model: &str) -> u32 {
        if model.contains("turbo") || model.contains("preview") {
            128000
        } else if model.contains("32k") {
            32768
        } else {
            8192
        }
    }

    /// Determine max tokens for GPT-3.5 model
    fn gpt35_max_tokens(model: &str) -> u32 {
        if model.contains("16k") {
            16384
        } else {
            4096
        }
    }

    /// Get tokenizer and max tokens for model
    fn get_model_config(model: &str) -> LlmResult<(CoreBPE, u32)> {
        match model {
            m if m.starts_with("gpt-4") => {
                let tokenizer = cl100k_base().map_err(|e| {
                    LlmError::configuration_error(format!("Failed to initialize tokenizer: {}", e))
                })?;
                Ok((tokenizer, Self::gpt4_max_tokens(m)))
            }
            m if m.starts_with("gpt-3.5") => {
                let tokenizer = cl100k_base().map_err(|e| {
                    LlmError::configuration_error(format!("Failed to initialize tokenizer: {}", e))
                })?;
                Ok((tokenizer, Self::gpt35_max_tokens(m)))
            }
            m if m.starts_with("o1") => {
                let tokenizer = o200k_base().map_err(|e| {
                    LlmError::configuration_error(format!("Failed to initialize tokenizer: {}", e))
                })?;
                Ok((tokenizer, 200000))
            }
            _ => {
                log_warn!(model = %model, "Unknown model, using cl100k_base tokenizer with 4k context");
                let tokenizer = cl100k_base().map_err(|e| {
                    LlmError::configuration_error(format!("Failed to initialize tokenizer: {}", e))
                })?;
                Ok((tokenizer, 4096))
            }
        }
    }

    /// Create token counter for specific OpenAI model
    pub fn new(model: &str) -> LlmResult<Self> {
        let (tokenizer, max_tokens) = Self::get_model_config(model)?;

        Ok(Self {
            tokenizer,
            max_tokens,
            model_name: model.to_string(),
        })
    }

    /// Create token counter for LM Studio (uses cl100k_base as default)
    pub fn for_lm_studio(max_tokens: u32) -> LlmResult<Self> {
        // log_debug!(max_tokens = max_tokens, "Creating LM Studio token counter");

        let tokenizer = cl100k_base().map_err(|e| {
            LlmError::configuration_error(format!(
                "Failed to initialize LM Studio tokenizer: {}",
                e
            ))
        })?;

        Ok(Self {
            tokenizer,
            max_tokens,
            model_name: "lm-studio".to_string(),
        })
    }
}

impl TokenCounter for OpenAITokenCounter {
    fn count_tokens(&self, text: &str) -> LlmResult<u32> {
        let tokens = self.tokenizer.encode_with_special_tokens(text);
        Ok(tokens.len() as u32)
    }

    fn count_message_tokens(&self, messages: &[serde_json::Value]) -> LlmResult<u32> {
        let mut total_tokens = 3u32; // Base conversation formatting

        for message in messages {
            total_tokens += self.count_single_message_tokens(message);
        }

        total_tokens += 3; // Reply end tokens

        log_debug!(
            total_tokens = total_tokens,
            message_count = messages.len(),
            model = %self.model_name,
            "Calculated message token count"
        );

        Ok(total_tokens)
    }

    fn max_context_tokens(&self) -> u32 {
        self.max_tokens
    }

    fn validate_token_limit(&self, text: &str) -> LlmResult<()> {
        let token_count = self.count_tokens(text)?;
        if token_count > self.max_tokens {
            return Err(LlmError::token_limit_exceeded(
                token_count as usize,
                self.max_tokens as usize,
            ));
        }
        Ok(())
    }

    fn truncate_to_limit(&self, text: &str, max_tokens: u32) -> LlmResult<String> {
        let tokens = self.tokenizer.encode_with_special_tokens(text);

        if tokens.len() <= max_tokens as usize {
            return Ok(text.to_string());
        }

        // log_debug!(
        //     original_tokens = tokens.len(),
        //     max_tokens = max_tokens,
        //     "Truncating text to fit token limit"
        // );

        let truncated_tokens = &tokens[..max_tokens as usize];
        let truncated_text = self
            .tokenizer
            .decode(truncated_tokens.to_vec())
            .map_err(|e| {
                LlmError::response_parsing_error(format!(
                    "Failed to decode truncated tokens: {}",
                    e
                ))
            })?;

        Ok(truncated_text)
    }
}

impl OpenAITokenCounter {
    fn count_single_message_tokens(&self, message: &serde_json::Value) -> u32 {
        let role = message
            .get("role")
            .and_then(|r| r.as_str())
            .unwrap_or("user");
        let content = message
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("");

        let mut tokens = 4u32; // Message formatting tokens
        tokens += self.tokenizer.encode_with_special_tokens(role).len() as u32;
        tokens += self.tokenizer.encode_with_special_tokens(content).len() as u32;
        tokens += self.count_tool_call_tokens(message);

        tokens
    }

    fn count_tool_call_tokens(&self, message: &serde_json::Value) -> u32 {
        let Some(tool_calls) = message.get("tool_calls") else {
            return 0;
        };

        let Some(calls_array) = tool_calls.as_array() else {
            return 0;
        };

        calls_array
            .iter()
            .filter_map(|call| {
                call.get("function")
                    .and_then(|f| f.get("arguments"))
                    .and_then(|a| a.as_str())
            })
            .map(|args_str| self.tokenizer.encode_with_special_tokens(args_str).len() as u32)
            .sum()
    }
}

/// Token counter for Anthropic Claude models.
///
/// Uses cl100k_base tokenizer with a 1.1x approximation factor, since
/// Claude's actual tokenizer isn't publicly available. This provides
/// conservative estimates (slightly over-counting).
///
/// # Context Windows
///
/// | Model | Context Window |
/// |-------|---------------|
/// | claude-3-5-sonnet | 200K |
/// | claude-3-opus | 200K |
/// | claude-3-haiku | 200K |
/// | claude-2.x | 100K |
///
/// # Example
///
/// ```rust,no_run
/// use multi_llm::{AnthropicTokenCounter, TokenCounter};
///
/// let counter = AnthropicTokenCounter::new("claude-3-5-sonnet-20241022")?;
/// let tokens = counter.count_tokens("Hello, world!")?;
/// # Ok::<(), multi_llm::LlmError>(())
/// ```
///
/// # Accuracy Note
///
/// Token counts are approximate. The 1.1x factor provides a safety margin
/// to avoid accidentally exceeding context limits.
pub struct AnthropicTokenCounter {
    tokenizer: CoreBPE,
    max_tokens: u32,
}

impl std::fmt::Debug for AnthropicTokenCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnthropicTokenCounter")
            .field("max_tokens", &self.max_tokens)
            .finish()
    }
}

impl AnthropicTokenCounter {
    /// Create token counter for Anthropic Claude models
    pub fn new(model: &str) -> LlmResult<Self> {
        // log_debug!(model = %model, "Creating Anthropic token counter");

        let max_tokens = match model {
            m if m.contains("claude-3-5-sonnet") => 200000,
            m if m.contains("claude-3") => 200000,
            m if m.contains("claude-2") => 100000,
            _ => {
                log_warn!(model = %model, "Unknown Anthropic model, using 100k context");
                100000
            }
        };

        // Use cl100k_base as approximation for Claude tokenization
        let tokenizer = cl100k_base().map_err(|e| {
            LlmError::configuration_error(format!(
                "Failed to initialize Anthropic tokenizer: {}",
                e
            ))
        })?;

        Ok(Self {
            tokenizer,
            max_tokens,
        })
    }
}

impl TokenCounter for AnthropicTokenCounter {
    fn count_tokens(&self, text: &str) -> LlmResult<u32> {
        let tokens = self.tokenizer.encode_with_special_tokens(text);
        // Apply approximation factor for Claude tokenization differences
        Ok((tokens.len() as f32 * 1.1) as u32)
    }

    fn count_message_tokens(&self, messages: &[serde_json::Value]) -> LlmResult<u32> {
        let mut total_tokens = 0u32;

        for message in messages {
            let content = message
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("");

            let content_tokens = self.count_tokens(content)?;
            total_tokens += content_tokens;
            total_tokens += 10; // Overhead for role and formatting
        }

        log_debug!(
            total_tokens = total_tokens,
            message_count = messages.len(),
            "Calculated Anthropic message token count"
        );

        Ok(total_tokens)
    }

    fn max_context_tokens(&self) -> u32 {
        self.max_tokens
    }

    fn validate_token_limit(&self, text: &str) -> LlmResult<()> {
        let token_count = self.count_tokens(text)?;
        if token_count > self.max_tokens {
            return Err(LlmError::token_limit_exceeded(
                token_count as usize,
                self.max_tokens as usize,
            ));
        }
        Ok(())
    }

    fn truncate_to_limit(&self, text: &str, max_tokens: u32) -> LlmResult<String> {
        let tokens = self.tokenizer.encode_with_special_tokens(text);
        let adjusted_limit = (max_tokens as f32 / 1.1) as usize; // Account for approximation factor

        if tokens.len() <= adjusted_limit {
            return Ok(text.to_string());
        }

        log_debug!(
            original_tokens = tokens.len(),
            max_tokens = max_tokens,
            adjusted_limit = adjusted_limit,
            "Truncating Anthropic text to fit token limit"
        );

        let truncated_tokens = &tokens[..adjusted_limit];
        let truncated_text = self
            .tokenizer
            .decode(truncated_tokens.to_vec())
            .map_err(|e| {
                LlmError::response_parsing_error(format!(
                    "Failed to decode truncated tokens: {}",
                    e
                ))
            })?;

        Ok(truncated_text)
    }
}

/// Factory for creating token counters for different providers.
///
/// Use this factory to get the appropriate token counter for your provider
/// and model. The factory handles selecting the correct tokenizer and
/// context window size.
///
/// # Example
///
/// ```rust,no_run
/// use multi_llm::{TokenCounterFactory, TokenCounter};
///
/// // Create counter for OpenAI
/// let openai = TokenCounterFactory::create_counter("openai", "gpt-4")?;
///
/// // Create counter for Anthropic
/// let anthropic = TokenCounterFactory::create_counter("anthropic", "claude-3-5-sonnet")?;
///
/// // Create counter with custom limit
/// let custom = TokenCounterFactory::create_counter_with_limit("openai", "gpt-4", 4096)?;
/// # Ok::<(), multi_llm::LlmError>(())
/// ```
///
/// # Supported Providers
///
/// - `openai`: Uses tiktoken for exact counts
/// - `anthropic`: Uses approximation with safety margin
/// - `ollama`: Uses cl100k_base (approximation)
/// - `lmstudio`: Uses cl100k_base (approximation)
pub struct TokenCounterFactory;

impl TokenCounterFactory {
    /// Create token counter for specific provider and model
    pub fn create_counter(provider: &str, model: &str) -> LlmResult<Arc<dyn TokenCounter>> {
        match provider.to_lowercase().as_str() {
            "openai" => {
                let counter = OpenAITokenCounter::new(model)?;
                Ok(Arc::new(counter))
            }
            "lmstudio" => {
                // Default to 4k context for local models, but this should be configurable
                let counter = OpenAITokenCounter::for_lm_studio(4096)?;
                Ok(Arc::new(counter))
            }
            "ollama" => {
                // Default to 4k context for Ollama models, but this should be configurable
                let counter = OpenAITokenCounter::for_lm_studio(4096)?;
                Ok(Arc::new(counter))
            }
            "anthropic" => {
                let counter = AnthropicTokenCounter::new(model)?;
                Ok(Arc::new(counter))
            }
            _ => Err(LlmError::unsupported_provider(provider)),
        }
    }

    /// Create counter with custom context window size
    pub fn create_counter_with_limit(
        provider: &str,
        model: &str,
        max_tokens: u32,
    ) -> LlmResult<Arc<dyn TokenCounter>> {
        match provider.to_lowercase().as_str() {
            "openai" => {
                let mut counter = OpenAITokenCounter::new(model)?;
                counter.max_tokens = max_tokens;
                Ok(Arc::new(counter))
            }
            "lmstudio" => {
                let counter = OpenAITokenCounter::for_lm_studio(max_tokens)?;
                Ok(Arc::new(counter))
            }
            "ollama" => {
                let counter = OpenAITokenCounter::for_lm_studio(max_tokens)?;
                Ok(Arc::new(counter))
            }
            "anthropic" => {
                let mut counter = AnthropicTokenCounter::new(model)?;
                counter.max_tokens = max_tokens;
                Ok(Arc::new(counter))
            }
            _ => Err(LlmError::unsupported_provider(provider)),
        }
    }
}
