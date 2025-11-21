use crate::error::{LlmError, LlmResult};
use crate::{log_debug, log_error, log_info, log_warn};
use crate::retry::RetryPolicy;
use crate::core_types::log_debug;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;

/// Trait for provider-specific configuration
pub trait ProviderConfig: Send + Sync + Debug + Any {
    /// Get the provider name
    fn provider_name(&self) -> &'static str;

    /// Get maximum context tokens for this provider
    fn max_context_tokens(&self) -> usize;

    /// Validate the configuration is complete
    /// Validate provider configuration
    ///
    /// # Errors
    ///
    /// Returns [`LlmError::ConfigurationError`] if:
    /// - Required provider-specific fields are missing (e.g., API key)
    /// - Configuration values are invalid (e.g., malformed URLs)
    /// - Provider-specific validation rules fail
    fn validate(&self) -> LlmResult<()>;

    /// Get the base URL for API calls
    fn base_url(&self) -> &str;

    /// Get the API key if required
    fn api_key(&self) -> Option<&str>;

    /// Get the default model name
    fn default_model(&self) -> &str;

    /// Helper for downcasting to concrete config types
    fn as_any(&self) -> &dyn Any;

    /// Get the retry policy for this provider
    fn retry_policy(&self) -> &RetryPolicy;
}

/// System-wide LLM configuration
#[derive(Debug)]
pub struct LLMConfig {
    /// The selected provider configuration
    pub provider: Box<dyn ProviderConfig>,

    /// Default model parameters that apply across providers
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultLLMParams {
    pub temperature: f64,
    pub max_tokens: u32,
    pub top_p: f64,
    pub top_k: u32,
    pub min_p: f64,
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

/// Anthropic-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    pub api_key: Option<String>,
    pub base_url: String,
    pub default_model: String,
    pub max_context_tokens: usize,
    pub retry_policy: RetryPolicy,
    /// Enable prompt caching for static system prompts (reduces costs and latency)
    pub enable_prompt_caching: bool,
    /// Cache TTL setting: "5m" for 5-minute cache, "1h" for 1-hour cache
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

/// OpenAI-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    pub api_key: Option<String>,
    pub base_url: String,
    pub default_model: String,
    pub max_context_tokens: usize,
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

/// LM Studio-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LMStudioConfig {
    pub base_url: String,
    pub default_model: String,
    pub max_context_tokens: usize,
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

/// Ollama-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub base_url: String,
    pub default_model: String,
    pub max_context_tokens: usize,
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

/// Path identifier for dual LLM configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LLMPath {
    /// Main conversation path - user-facing responses
    User,
    /// Background NLP analysis path - structured processing
    Nlp,
}

impl std::fmt::Display for LLMPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMPath::User => write!(f, "user"),
            LLMPath::Nlp => write!(f, "nlp"),
        }
    }
}

/// Dual LLM configuration supporting separate user and NLP paths
#[derive(Debug, Clone)]
pub struct DualLLMConfig {
    /// Configuration for user-facing conversation path
    pub user_llm: LLMConfig,
    /// Configuration for background NLP analysis path
    pub nlp_llm: LLMConfig,
}

impl DualLLMConfig {
    /// Create a new dual configuration with separate configs for each path
    pub fn new(user_llm: LLMConfig, nlp_llm: LLMConfig) -> Self {
        log_debug!(
            user_provider = user_llm.provider.provider_name(),
            nlp_provider = nlp_llm.provider.provider_name(),
            "Creating dual LLM configuration"
        );

        Self { user_llm, nlp_llm }
    }

    /// Get configuration for the specified path
    pub fn get_config(&self, path: LLMPath) -> &LLMConfig {
        match path {
            LLMPath::User => &self.user_llm,
            LLMPath::Nlp => &self.nlp_llm,
        }
    }

    /// Validate both configurations
    pub fn validate(&self) -> LlmResult<()> {
        self.user_llm.provider.validate().map_err(|e| {
            LlmError::configuration_error(format!("User LLM validation failed: {}", e))
        })?;

        self.nlp_llm.provider.validate().map_err(|e| {
            LlmError::configuration_error(format!("NLP LLM validation failed: {}", e))
        })?;

        Ok(())
    }

    /// Create dual LLM configuration from parsed section data
    ///
    /// # Arguments
    ///
    /// * `user_section` - Configuration section for user-facing LLM
    /// * `nlp_section` - Configuration section for NLP analysis LLM
    ///
    /// # Errors
    ///
    /// Returns `LlmError::ConfigurationError` if:
    /// - Either section is missing required fields (e.g., provider)
    /// - Provider configuration validation fails
    /// - Parameter parsing fails (e.g., invalid temperature value)
    pub fn from_sections(
        user_section: &std::collections::HashMap<String, String>,
        nlp_section: &std::collections::HashMap<String, String>,
    ) -> LlmResult<Self> {
        log_debug!("Creating dual LLM configuration from parsed sections");

        let user_llm = Self::create_llm_config_from_section(user_section.clone())?;
        let nlp_llm = Self::create_llm_config_from_section(nlp_section.clone())?;

        Ok(Self::new(user_llm, nlp_llm))
    }

    /// Create LLMConfig from parsed section data
    fn create_llm_config_from_section(
        section: std::collections::HashMap<String, String>,
    ) -> LlmResult<LLMConfig> {
        let provider = section.get("provider").ok_or_else(|| {
            LlmError::configuration_error("Missing 'provider' field in LLM config")
        })?;

        let model = section.get("model");
        let api_key = section.get("api_key");
        let base_url = section.get("base_url");

        // Create the base config
        let mut config = LLMConfig::create_provider(
            provider,
            api_key.cloned(),
            base_url.cloned(),
            model.cloned(),
        )?;

        // Apply additional parameters
        Self::apply_optional_params(&mut config, &section);

        Ok(config)
    }

    /// Apply optional parameters from section to config
    fn apply_optional_params(
        config: &mut LLMConfig,
        section: &std::collections::HashMap<String, String>,
    ) {
        if let Some(temp) = Self::parse_param::<f64>(section, "temperature") {
            config.default_params.temperature = temp;
        }
        if let Some(max_tokens) = Self::parse_param::<u32>(section, "max_tokens") {
            config.default_params.max_tokens = max_tokens;
        }
        if let Some(top_p) = Self::parse_param::<f64>(section, "top_p") {
            config.default_params.top_p = top_p;
        }
        if let Some(presence_penalty) = Self::parse_param::<f64>(section, "presence_penalty") {
            config.default_params.presence_penalty = presence_penalty;
        }
    }

    /// Parse a parameter from the section HashMap
    fn parse_param<T: std::str::FromStr>(
        section: &std::collections::HashMap<String, String>,
        key: &str,
    ) -> Option<T> {
        section.get(key).and_then(|s| s.parse::<T>().ok())
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
