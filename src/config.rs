//! Configuration types for LLM providers.
//!
//! This module provides configuration structures for all supported LLM providers.
//! Each provider has its own config type implementing [`ProviderConfig`], plus
//! shared types for default parameters and dual-path setups.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use multi_llm::{LLMConfig, OpenAIConfig, DefaultLLMParams, UnifiedLLMClient};
//!
//! // Create config programmatically
//! let config = LLMConfig {
//!     provider: Box::new(OpenAIConfig {
//!         api_key: Some("sk-...".to_string()),
//!         ..Default::default()
//!     }),
//!     default_params: DefaultLLMParams::default(),
//! };
//!
//! let client = UnifiedLLMClient::from_config(config)?;
//! # Ok::<(), multi_llm::LlmError>(())
//! ```
//!
//! # From Environment Variables
//!
//! ```rust,no_run
//! use multi_llm::{LLMConfig, UnifiedLLMClient};
//!
//! // Uses AI_PROVIDER and provider-specific env vars
//! let config = LLMConfig::from_env()?;
//! let client = UnifiedLLMClient::from_config(config)?;
//! # Ok::<(), multi_llm::LlmError>(())
//! ```
//!
//! # Provider-Specific Configs
//!
//! | Provider | Config Type | Required Env Vars |
//! |----------|------------|-------------------|
//! | OpenAI | [`OpenAIConfig`] | `OPENAI_API_KEY` |
//! | Anthropic | [`AnthropicConfig`] | `ANTHROPIC_API_KEY` |
//! | Ollama | [`OllamaConfig`] | (none, local) |
//! | LM Studio | [`LMStudioConfig`] | (none, local) |

use crate::error::{LlmError, LlmResult};
use crate::internals::retry::RetryPolicy;
use crate::logging::log_debug;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;

/// Trait for provider-specific configuration.
///
/// All provider configs (OpenAI, Anthropic, etc.) implement this trait.
/// You typically don't need to implement this yourself unless adding
/// a custom provider.
///
/// # Provided Implementations
///
/// - [`OpenAIConfig`]
/// - [`AnthropicConfig`]
/// - [`OllamaConfig`]
/// - [`LMStudioConfig`]
pub trait ProviderConfig: Send + Sync + Debug + Any {
    /// Get the provider identifier (e.g., "openai", "anthropic").
    fn provider_name(&self) -> &'static str;

    /// Get the maximum context window size in tokens.
    fn max_context_tokens(&self) -> usize;

    /// Validate that the configuration is complete and valid.
    ///
    /// # Errors
    ///
    /// Returns [`LlmError::ConfigurationError`] if:
    /// - Required fields are missing (e.g., API key for cloud providers)
    /// - Field values are invalid (e.g., malformed URLs)
    /// - Provider-specific validation fails
    fn validate(&self) -> LlmResult<()>;

    /// Get the base URL for API requests.
    fn base_url(&self) -> &str;

    /// Get the API key, if one is configured.
    fn api_key(&self) -> Option<&str>;

    /// Get the default model name for this provider.
    fn default_model(&self) -> &str;

    /// Downcast helper for accessing concrete config types.
    fn as_any(&self) -> &dyn Any;

    /// Get the retry policy for transient failures.
    fn retry_policy(&self) -> &RetryPolicy;
}

/// System-wide LLM configuration.
///
/// Combines a provider-specific configuration with default model parameters.
/// This is the primary config type used to create a [`UnifiedLLMClient`](crate::UnifiedLLMClient).
///
/// # Example
///
/// ```rust,no_run
/// use multi_llm::{LLMConfig, AnthropicConfig, DefaultLLMParams};
///
/// let config = LLMConfig {
///     provider: Box::new(AnthropicConfig {
///         api_key: Some("sk-ant-...".to_string()),
///         default_model: "claude-3-5-sonnet-20241022".to_string(),
///         ..Default::default()
///     }),
///     default_params: DefaultLLMParams {
///         temperature: 0.7,
///         max_tokens: 4096,
///         ..Default::default()
///     },
/// };
/// ```
///
/// # From Environment
///
/// Use [`from_env()`](Self::from_env) to load from environment variables:
/// - `AI_PROVIDER`: Provider name ("anthropic", "openai", "ollama", "lmstudio")
/// - Provider-specific vars (e.g., `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`)
#[derive(Debug)]
pub struct LLMConfig {
    /// The provider-specific configuration.
    ///
    /// Contains API keys, endpoints, model selection, and provider features.
    pub provider: Box<dyn ProviderConfig>,

    /// Default parameters for LLM requests.
    ///
    /// Applied to all requests unless overridden by [`RequestConfig`](crate::RequestConfig).
    pub default_params: DefaultLLMParams,
}

impl LLMConfig {
    /// Clone provider config by downcasting to concrete type
    fn clone_provider(&self) -> Box<dyn ProviderConfig> {
        let any_ref = self.provider.as_any();

        if let Some(config) = any_ref.downcast_ref::<AnthropicConfig>() {
            return Box::new(config.clone());
        }
        if let Some(config) = any_ref.downcast_ref::<OpenAIConfig>() {
            return Box::new(config.clone());
        }
        if let Some(config) = any_ref.downcast_ref::<LMStudioConfig>() {
            return Box::new(config.clone());
        }
        if let Some(config) = any_ref.downcast_ref::<OllamaConfig>() {
            return Box::new(config.clone());
        }

        // This should never happen as all provider types are covered above
        unreachable!("Unknown provider type - all provider types should be handled")
    }
}

impl Clone for LLMConfig {
    fn clone(&self) -> Self {
        Self {
            provider: self.clone_provider(),
            default_params: self.default_params.clone(),
        }
    }
}

/// Default parameters for LLM generation.
///
/// These values are used when a request doesn't specify its own values.
/// All parameters have sensible defaults that work well for most use cases.
///
/// # Defaults
///
/// | Parameter | Default | Description |
/// |-----------|---------|-------------|
/// | `temperature` | 0.7 | Balanced creativity/consistency |
/// | `max_tokens` | 1000 | Reasonable response length |
/// | `top_p` | 0.9 | Standard nucleus sampling |
/// | `top_k` | 40 | Vocabulary restriction |
/// | `min_p` | 0.05 | Minimum probability filter |
/// | `presence_penalty` | 0.0 | No repetition penalty |
///
/// # Example
///
/// ```rust
/// use multi_llm::DefaultLLMParams;
///
/// // Use defaults
/// let params = DefaultLLMParams::default();
///
/// // Or customize
/// let params = DefaultLLMParams {
///     temperature: 0.2,  // More deterministic
///     max_tokens: 4096,  // Longer responses
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultLLMParams {
    /// Temperature for response randomness (0.0 = deterministic, 2.0 = very random).
    pub temperature: f64,

    /// Maximum tokens to generate per response.
    pub max_tokens: u32,

    /// Top-p (nucleus) sampling threshold.
    pub top_p: f64,

    /// Top-k sampling limit.
    pub top_k: u32,

    /// Minimum probability filter.
    pub min_p: f64,

    /// Presence penalty to reduce repetition.
    pub presence_penalty: f64,
}

impl Default for DefaultLLMParams {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: 1000,
            top_p: 0.9,
            top_k: 40,
            min_p: 0.05,
            presence_penalty: 0.0,
        }
    }
}

/// Configuration for Anthropic Claude models.
///
/// Claude models support prompt caching for significant cost savings (90% on cache reads).
/// Enable caching for static system prompts and context that doesn't change often.
///
/// # Example
///
/// ```rust,no_run
/// use multi_llm::AnthropicConfig;
///
/// let config = AnthropicConfig {
///     api_key: Some("sk-ant-api03-...".to_string()),
///     default_model: "claude-3-5-sonnet-20241022".to_string(),
///     enable_prompt_caching: true,
///     cache_ttl: "1h".to_string(),  // 1-hour cache
///     ..Default::default()
/// };
/// ```
///
/// # Environment Variables
///
/// - `ANTHROPIC_API_KEY`: API key (required)
///
/// # Models
///
/// - `claude-3-5-sonnet-20241022`: Latest Sonnet (recommended)
/// - `claude-3-opus-20240229`: Most capable
/// - `claude-3-haiku-20240307`: Fastest, cheapest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    /// Anthropic API key (starts with "sk-ant-").
    pub api_key: Option<String>,

    /// Base URL for API requests (default: `https://api.anthropic.com`).
    pub base_url: String,

    /// Default model to use for requests.
    pub default_model: String,

    /// Maximum context window size in tokens (200K for Claude 3).
    pub max_context_tokens: usize,

    /// Retry policy for transient failures.
    pub retry_policy: RetryPolicy,

    /// Enable prompt caching for cost savings.
    ///
    /// When enabled, static system prompts and context are cached,
    /// reducing costs by 90% on cache reads.
    pub enable_prompt_caching: bool,

    /// Cache TTL setting: "5m" for 5-minute cache, "1h" for 1-hour cache.
    ///
    /// - "5m": Ephemeral cache, 1.25x write cost, good for development
    /// - "1h": Extended cache, 2x write cost, good for production
    pub cache_ttl: String,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: "https://api.anthropic.com".to_string(),
            default_model: "claude-3-5-sonnet-20241022".to_string(),
            max_context_tokens: 200_000,
            retry_policy: RetryPolicy::default(),
            enable_prompt_caching: true, // Enable by default for cost savings
            cache_ttl: "1h".to_string(), // Use 1-hour cache for story writing sessions with infrequent personality changes
        }
    }
}

impl ProviderConfig for AnthropicConfig {
    fn provider_name(&self) -> &'static str {
        "anthropic"
    }

    fn max_context_tokens(&self) -> usize {
        self.max_context_tokens
    }

    fn validate(&self) -> LlmResult<()> {
        if self.api_key.is_none() {
            return Err(LlmError::configuration_error(
                "Anthropic API key is required",
            ));
        }
        Ok(())
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn retry_policy(&self) -> &RetryPolicy {
        &self.retry_policy
    }
}

/// Configuration for OpenAI GPT models.
///
/// Supports GPT-4, GPT-3.5, and other OpenAI models. Also works with
/// OpenAI-compatible APIs by changing the base URL.
///
/// # Example
///
/// ```rust,no_run
/// use multi_llm::OpenAIConfig;
///
/// let config = OpenAIConfig {
///     api_key: Some("sk-...".to_string()),
///     default_model: "gpt-4-turbo-preview".to_string(),
///     ..Default::default()
/// };
/// ```
///
/// # Environment Variables
///
/// - `OPENAI_API_KEY`: API key (required)
/// - `OPENAI_BASE_URL`: Custom base URL (optional)
///
/// # Models
///
/// - `gpt-4-turbo-preview`: Latest GPT-4 Turbo (128K context)
/// - `gpt-4`: Standard GPT-4 (8K context)
/// - `gpt-3.5-turbo`: Fast and affordable (16K context)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    /// OpenAI API key (starts with "sk-").
    pub api_key: Option<String>,

    /// Base URL for API requests (default: `https://api.openai.com`).
    pub base_url: String,

    /// Default model to use for requests.
    pub default_model: String,

    /// Maximum context window size in tokens.
    pub max_context_tokens: usize,

    /// Retry policy for transient failures.
    pub retry_policy: RetryPolicy,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: "https://api.openai.com".to_string(),
            default_model: "gpt-4".to_string(),
            max_context_tokens: 128_000,
            retry_policy: RetryPolicy::default(),
        }
    }
}

impl ProviderConfig for OpenAIConfig {
    fn provider_name(&self) -> &'static str {
        "openai"
    }

    fn max_context_tokens(&self) -> usize {
        self.max_context_tokens
    }

    fn validate(&self) -> LlmResult<()> {
        if self.api_key.is_none() {
            return Err(LlmError::configuration_error("OpenAI API key is required"));
        }
        Ok(())
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn retry_policy(&self) -> &RetryPolicy {
        &self.retry_policy
    }
}

/// Configuration for LM Studio local models.
///
/// LM Studio provides an OpenAI-compatible API for running local models.
/// No API key is required since it runs locally.
///
/// # Example
///
/// ```rust
/// use multi_llm::LMStudioConfig;
///
/// let config = LMStudioConfig {
///     base_url: "http://localhost:1234".to_string(),
///     default_model: "local-model".to_string(),
///     max_context_tokens: 4096,
///     ..Default::default()
/// };
/// ```
///
/// # Environment Variables
///
/// - `LM_STUDIO_BASE_URL` or `OPENAI_BASE_URL`: Server URL (default: `http://localhost:1234`)
///
/// # Notes
///
/// - Start LM Studio server before making requests
/// - Context window depends on the loaded model
/// - Model name in config is ignored; uses whatever model is loaded in LM Studio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LMStudioConfig {
    /// Base URL for the LM Studio server (default: `http://localhost:1234`).
    pub base_url: String,

    /// Default model name (LM Studio uses the loaded model regardless).
    pub default_model: String,

    /// Maximum context window size (depends on loaded model).
    pub max_context_tokens: usize,

    /// Retry policy for transient failures.
    pub retry_policy: RetryPolicy,
}

impl Default for LMStudioConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:1234".to_string(),
            default_model: "local-model".to_string(),
            max_context_tokens: 4_096,
            retry_policy: RetryPolicy::default(),
        }
    }
}

impl ProviderConfig for LMStudioConfig {
    fn provider_name(&self) -> &'static str {
        "lmstudio"
    }

    fn max_context_tokens(&self) -> usize {
        self.max_context_tokens
    }

    fn validate(&self) -> LlmResult<()> {
        if self.base_url.is_empty() {
            return Err(LlmError::configuration_error(
                "LM Studio base URL is required",
            ));
        }
        Ok(())
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn api_key(&self) -> Option<&str> {
        None // LM Studio doesn't require API key
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn retry_policy(&self) -> &RetryPolicy {
        &self.retry_policy
    }
}

/// Configuration for Ollama local models.
///
/// Ollama is a tool for running open-source LLMs locally. It provides
/// an OpenAI-compatible API and doesn't require an API key.
///
/// # Example
///
/// ```rust
/// use multi_llm::OllamaConfig;
///
/// let config = OllamaConfig {
///     base_url: "http://localhost:11434".to_string(),
///     default_model: "llama2".to_string(),
///     max_context_tokens: 4096,
///     ..Default::default()
/// };
/// ```
///
/// # Environment Variables
///
/// None required (local service).
///
/// # Popular Models
///
/// - `llama2`: Meta's Llama 2
/// - `mistral`: Mistral AI's model
/// - `codellama`: Code-specialized Llama
/// - `phi`: Microsoft's Phi model
///
/// Install models with: `ollama pull <model-name>`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Base URL for the Ollama server (default: `http://localhost:11434`).
    pub base_url: String,

    /// Default model to use (must be pulled with `ollama pull`).
    pub default_model: String,

    /// Maximum context window size (depends on model).
    pub max_context_tokens: usize,

    /// Retry policy for transient failures.
    pub retry_policy: RetryPolicy,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            default_model: "llama2".to_string(),
            max_context_tokens: 4_096,
            retry_policy: RetryPolicy::default(),
        }
    }
}

impl ProviderConfig for OllamaConfig {
    fn provider_name(&self) -> &'static str {
        "ollama"
    }

    fn max_context_tokens(&self) -> usize {
        self.max_context_tokens
    }

    fn validate(&self) -> LlmResult<()> {
        if self.base_url.is_empty() {
            return Err(LlmError::configuration_error("Ollama base URL is required"));
        }
        Ok(())
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn api_key(&self) -> Option<&str> {
        None // Ollama doesn't require API key
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn retry_policy(&self) -> &RetryPolicy {
        &self.retry_policy
    }
}

impl LLMConfig {
    /// Create configuration for a specific provider with generic parameters
    /// This is the main factory method for creating provider configurations
    ///
    /// # Errors
    ///
    /// Returns [`LlmError::UnsupportedProvider`] if the provider name is not recognized.
    /// Supported providers are: "anthropic", "openai", "lmstudio".
    ///
    /// Returns [`LlmError::ConfigurationError`] if:
    /// - API key format validation fails
    /// - Provider-specific configuration validation fails
    /// - Required fields for the provider are missing
    pub fn create_provider(
        provider_name: &str,
        api_key: Option<String>,
        base_url: Option<String>,
        model: Option<String>,
    ) -> LlmResult<Self> {
        log_debug!(
            provider = %provider_name,
            has_api_key = api_key.is_some(),
            has_base_url = base_url.is_some(),
            has_model = model.is_some(),
            "Creating provider configuration"
        );

        let provider: Box<dyn ProviderConfig> = match provider_name.to_lowercase().as_str() {
            "anthropic" => Self::create_anthropic_provider(api_key, base_url, model),
            "openai" => Self::create_openai_provider(api_key, base_url, model),
            "lmstudio" => Self::create_lmstudio_provider(base_url, model),
            "ollama" => Self::create_ollama_provider(base_url, model),
            _ => {
                return Err(LlmError::configuration_error(format!(
                    "Unsupported provider: {}. Supported providers: anthropic, openai, lmstudio, ollama",
                    provider_name
                )));
            }
        };

        provider.validate()?;

        Ok(Self {
            provider,
            default_params: DefaultLLMParams::default(),
        })
    }

    fn create_anthropic_provider(
        api_key: Option<String>,
        base_url: Option<String>,
        model: Option<String>,
    ) -> Box<dyn ProviderConfig> {
        let mut config = AnthropicConfig::default();
        if let Some(key) = api_key {
            config.api_key = Some(key);
        } else if let Ok(env_key) = std::env::var("ANTHROPIC_API_KEY") {
            config.api_key = Some(env_key);
        }
        if let Some(url) = base_url {
            config.base_url = url;
        }
        if let Some(m) = model {
            config.default_model = m;
        }
        Box::new(config)
    }

    fn create_openai_provider(
        api_key: Option<String>,
        base_url: Option<String>,
        model: Option<String>,
    ) -> Box<dyn ProviderConfig> {
        let mut config = OpenAIConfig::default();
        if let Some(key) = api_key {
            config.api_key = Some(key);
        }
        if let Some(url) = base_url {
            config.base_url = url;
        }
        if let Some(m) = model {
            config.default_model = m;
        }
        Box::new(config)
    }

    fn create_lmstudio_provider(
        base_url: Option<String>,
        model: Option<String>,
    ) -> Box<dyn ProviderConfig> {
        let mut config = LMStudioConfig::default();
        if let Some(url) = base_url {
            config.base_url = url;
        }
        if let Some(m) = model {
            config.default_model = m;
        }
        Box::new(config)
    }

    fn create_ollama_provider(
        base_url: Option<String>,
        model: Option<String>,
    ) -> Box<dyn ProviderConfig> {
        let mut config = OllamaConfig::default();
        if let Some(url) = base_url {
            config.base_url = url;
        }
        if let Some(m) = model {
            config.default_model = m;
        }
        Box::new(config)
    }

    /// Load configuration from environment variables for the specified provider
    /// This is the ONLY method that should access environment variables
    ///
    /// # Errors
    ///
    /// Returns [`LlmError::ConfigurationError`] if:
    /// - Required environment variables are missing
    /// - Environment variable values are invalid or malformed
    /// - Provider configuration validation fails
    ///
    /// Returns [`LlmError::UnsupportedProvider`] if the AI_PROVIDER environment variable
    /// contains an unrecognized provider name.
    pub fn from_env() -> LlmResult<Self> {
        let provider_name =
            std::env::var("AI_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());

        log_debug!(
            target_provider = %provider_name,
            "Loading LLM configuration from environment"
        );

        let provider: Box<dyn ProviderConfig> = match provider_name.as_str() {
            "anthropic" => Self::anthropic_from_env(),
            "openai" => Self::openai_from_env(),
            "lmstudio" => Self::lmstudio_from_env(),
            _ => {
                return Err(LlmError::unsupported_provider(provider_name));
            }
        };

        provider.validate()?;

        log_debug!(
            provider = provider.provider_name(),
            max_context_tokens = provider.max_context_tokens(),
            base_url = provider.base_url(),
            has_api_key = provider.api_key().is_some(),
            "LLM configuration loaded and validated"
        );

        Ok(Self {
            provider,
            default_params: DefaultLLMParams::default(),
        })
    }

    fn anthropic_from_env() -> Box<dyn ProviderConfig> {
        let mut config = AnthropicConfig::default();
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            config.api_key = Some(api_key);
        }
        Box::new(config)
    }

    fn openai_from_env() -> Box<dyn ProviderConfig> {
        let mut config = OpenAIConfig::default();
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            config.api_key = Some(api_key);
        }
        if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
            config.base_url = base_url;
        }
        Box::new(config)
    }

    fn lmstudio_from_env() -> Box<dyn ProviderConfig> {
        let mut config = LMStudioConfig::default();
        if let Ok(base_url) = std::env::var("LM_STUDIO_BASE_URL") {
            config.base_url = base_url;
        } else if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
            config.base_url = base_url;
        }
        Box::new(config)
    }
}
