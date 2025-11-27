//! Unified LLM client for multi-provider operations.
//!
//! This module provides [`UnifiedLLMClient`], the main entry point for
//! interacting with LLM providers through multi-llm.

use crate::config::{AnthropicConfig, LLMConfig, LMStudioConfig, OllamaConfig, OpenAIConfig};
use crate::error::{LlmError, LlmResult};
use crate::logging::log_debug;
use crate::messages::UnifiedLLMRequest;
#[cfg(feature = "events")]
use crate::provider::LLMBusinessEvent;
use crate::provider::{LlmProvider, RequestConfig, Response, ToolCallingRound};
use crate::providers::{AnthropicProvider, LMStudioProvider, OllamaProvider, OpenAIProvider};
use async_trait::async_trait;

/// Internal provider enum (not exposed publicly).
enum LLMProvider {
    Anthropic(AnthropicProvider),
    OpenAI(OpenAIProvider),
    LMStudio(LMStudioProvider),
    Ollama(OllamaProvider),
}

/// Unified client for multi-provider LLM operations.
///
/// `UnifiedLLMClient` is the primary interface for using multi-llm. It wraps
/// all supported providers behind a single [`LlmProvider`] interface, allowing
/// you to switch providers without changing your application code.
///
/// # Quick Start
///
/// ```rust,no_run
/// use multi_llm::{unwrap_response, UnifiedLLMClient, LLMConfig, UnifiedMessage, UnifiedLLMRequest, LlmProvider};
///
/// # async fn example() -> anyhow::Result<()> {
/// // Create client from environment variables
/// let client = UnifiedLLMClient::from_env()?;
///
/// // Build a request
/// let request = UnifiedLLMRequest::new(vec![
///     UnifiedMessage::system("You are a helpful assistant."),
///     UnifiedMessage::user("What's the capital of France?"),
/// ]);
///
/// // Execute the request
/// let response = unwrap_response!(client.execute_llm(request, None, None).await?);
/// println!("Response: {}", response.content);
/// # Ok(())
/// # }
/// ```
///
/// # From Configuration
///
/// ```rust,no_run
/// use multi_llm::{UnifiedLLMClient, LLMConfig, OpenAIConfig, DefaultLLMParams};
///
/// let config = LLMConfig {
///     provider: Box::new(OpenAIConfig {
///         api_key: Some("sk-...".to_string()),
///         default_model: "gpt-4-turbo-preview".to_string(),
///         ..Default::default()
///     }),
///     default_params: DefaultLLMParams::default(),
/// };
///
/// let client = UnifiedLLMClient::from_config(config)?;
/// # Ok::<(), multi_llm::LlmError>(())
/// ```
///
/// # Tool Calling
///
/// ```rust,no_run
/// use multi_llm::{unwrap_response, UnifiedLLMClient, UnifiedMessage, UnifiedLLMRequest, RequestConfig, Tool, ToolChoice, LlmProvider};
///
/// # async fn example(client: UnifiedLLMClient) -> anyhow::Result<()> {
/// // Define a tool
/// let weather_tool = Tool {
///     name: "get_weather".to_string(),
///     description: "Get current weather".to_string(),
///     parameters: serde_json::json!({
///         "type": "object",
///         "properties": {
///             "city": {"type": "string"}
///         },
///         "required": ["city"]
///     }),
/// };
///
/// let request = UnifiedLLMRequest::new(vec![
///     UnifiedMessage::user("What's the weather in Paris?"),
/// ]);
///
/// let config = RequestConfig {
///     tools: vec![weather_tool],
///     tool_choice: Some(ToolChoice::Auto),
///     ..Default::default()
/// };
///
/// let response = unwrap_response!(client.execute_llm(request, None, Some(config)).await?);
///
/// // Check for tool calls
/// if !response.tool_calls.is_empty() {
///     for call in &response.tool_calls {
///         println!("Tool call: {} with {}", call.name, call.arguments);
///         // Execute tool and continue conversation...
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Supported Providers
///
/// | Provider | Config Type | API Key Required |
/// |----------|------------|------------------|
/// | Anthropic | [`AnthropicConfig`] | Yes |
/// | OpenAI | [`OpenAIConfig`] | Yes |
/// | Ollama | [`OllamaConfig`] | No (local) |
/// | LM Studio | [`LMStudioConfig`] | No (local) |
pub struct UnifiedLLMClient {
    provider: LLMProvider,
}

impl UnifiedLLMClient {
    /// Create Anthropic provider from config
    fn create_anthropic_provider(config: &LLMConfig, model: &str) -> LlmResult<LLMProvider> {
        let anthropic_config = config
            .provider
            .as_any()
            .downcast_ref::<AnthropicConfig>()
            .ok_or_else(|| LlmError::configuration_error("Invalid Anthropic configuration"))?;

        let provider =
            AnthropicProvider::new(anthropic_config.clone(), config.default_params.clone())
                .map_err(|e| {
                    LlmError::configuration_error(format!(
                        "Failed to create Anthropic provider for model {}: {}",
                        model, e
                    ))
                })?;

        Ok(LLMProvider::Anthropic(provider))
    }

    /// Create OpenAI provider from config
    fn create_openai_provider(config: &LLMConfig, model: &str) -> LlmResult<LLMProvider> {
        let openai_config = config
            .provider
            .as_any()
            .downcast_ref::<OpenAIConfig>()
            .ok_or_else(|| LlmError::configuration_error("Invalid OpenAI configuration"))?;

        let provider = OpenAIProvider::new(openai_config.clone(), config.default_params.clone())
            .map_err(|e| {
                LlmError::configuration_error(format!(
                    "Failed to create OpenAI provider for model {}: {}",
                    model, e
                ))
            })?;

        Ok(LLMProvider::OpenAI(provider))
    }

    /// Create LMStudio provider from config
    fn create_lmstudio_provider(config: &LLMConfig, model: &str) -> LlmResult<LLMProvider> {
        let lmstudio_config = config
            .provider
            .as_any()
            .downcast_ref::<LMStudioConfig>()
            .ok_or_else(|| LlmError::configuration_error("Invalid LM Studio configuration"))?;

        let provider =
            LMStudioProvider::new(lmstudio_config.clone(), config.default_params.clone()).map_err(
                |e| {
                    LlmError::configuration_error(format!(
                        "Failed to create LM Studio provider for model {}: {}",
                        model, e
                    ))
                },
            )?;

        Ok(LLMProvider::LMStudio(provider))
    }

    /// Create Ollama provider from config
    fn create_ollama_provider(config: &LLMConfig, model: &str) -> LlmResult<LLMProvider> {
        let ollama_config = config
            .provider
            .as_any()
            .downcast_ref::<OllamaConfig>()
            .ok_or_else(|| LlmError::configuration_error("Invalid Ollama configuration"))?;

        let provider = OllamaProvider::new(ollama_config.clone(), config.default_params.clone())
            .map_err(|e| {
                LlmError::configuration_error(format!(
                    "Failed to create Ollama provider for model {}: {}",
                    model, e
                ))
            })?;

        Ok(LLMProvider::Ollama(provider))
    }

    /// Factory method to create UnifiedLLMClient with all parameters
    /// This is the primary constructor for production use
    ///
    /// # Errors
    ///
    /// Returns [`LlmError::UnsupportedProvider`] if the provider name is not recognized.
    /// Supported providers are: "anthropic", "openai", "lmstudio", "ollama".
    ///
    /// Returns [`LlmError::ConfigurationError`] if:
    /// - The provider configuration type doesn't match the provider name
    /// - Required configuration fields are missing (e.g., API key for OpenAI/Anthropic)
    /// - Configuration validation fails (e.g., invalid base URL format)
    pub fn create(provider_name: &str, model: String, config: LLMConfig) -> LlmResult<Self> {
        let provider = match provider_name {
            "anthropic" => Self::create_anthropic_provider(&config, &model)?,
            "openai" => Self::create_openai_provider(&config, &model)?,
            "lmstudio" => Self::create_lmstudio_provider(&config, &model)?,
            "ollama" => Self::create_ollama_provider(&config, &model)?,
            _ => return Err(LlmError::unsupported_provider(provider_name)),
        };

        log_debug!(
            provider = provider_name,
            model = %model,
            "UnifiedLLMClient created"
        );

        Ok(Self { provider })
    }

    /// Create a client using environment variables for configuration
    ///
    /// # Errors
    ///
    /// Returns [`LlmError::ConfigurationError`] if:
    /// - Required environment variables are missing
    /// - Environment variable values are invalid or malformed
    /// - Provider configuration validation fails
    pub fn from_env() -> LlmResult<Self> {
        let config = LLMConfig::from_env()?;
        Self::from_config(config)
    }

    /// Create a client from an LLMConfig (backward compatibility)
    ///
    /// # Errors
    ///
    /// Returns [`LlmError::UnsupportedProvider`] if the provider name in the config is not recognized.
    ///
    /// Returns [`LlmError::ConfigurationError`] if:
    /// - Provider configuration validation fails
    /// - Required provider-specific settings are missing
    pub fn from_config(config: LLMConfig) -> LlmResult<Self> {
        let provider_name = config.provider.provider_name();
        let model = config.provider.default_model().to_string();

        log_debug!(
            target_provider = provider_name,
            model = %model,
            "Creating UnifiedLLMClient from config"
        );

        Self::create(provider_name, model, config)
    }
}

/// Implement LlmProvider for UnifiedLLMClient
/// Just delegates to the underlying provider - providers already handle events feature correctly
#[async_trait]
impl LlmProvider for UnifiedLLMClient {
    #[cfg(feature = "events")]
    async fn execute_llm(
        &self,
        request: UnifiedLLMRequest,
        current_tool_round: Option<ToolCallingRound>,
        config: Option<RequestConfig>,
    ) -> crate::provider::Result<(Response, Vec<LLMBusinessEvent>)> {
        // Restore default retry policy
        match &self.provider {
            LLMProvider::Anthropic(p) => p.restore_default_retry_policy().await,
            LLMProvider::OpenAI(p) => p.restore_default_retry_policy().await,
            LLMProvider::LMStudio(p) => p.restore_default_retry_policy().await,
            LLMProvider::Ollama(p) => p.restore_default_retry_policy().await,
        }

        // Delegate to provider
        match &self.provider {
            LLMProvider::Anthropic(p) => p.execute_llm(request, current_tool_round, config).await,
            LLMProvider::OpenAI(p) => p.execute_llm(request, current_tool_round, config).await,
            LLMProvider::LMStudio(p) => p.execute_llm(request, current_tool_round, config).await,
            LLMProvider::Ollama(p) => p.execute_llm(request, current_tool_round, config).await,
        }
    }

    #[cfg(not(feature = "events"))]
    async fn execute_llm(
        &self,
        request: UnifiedLLMRequest,
        current_tool_round: Option<ToolCallingRound>,
        config: Option<RequestConfig>,
    ) -> crate::provider::Result<Response> {
        // Restore default retry policy
        match &self.provider {
            LLMProvider::Anthropic(p) => p.restore_default_retry_policy().await,
            LLMProvider::OpenAI(p) => p.restore_default_retry_policy().await,
            LLMProvider::LMStudio(p) => p.restore_default_retry_policy().await,
            LLMProvider::Ollama(p) => p.restore_default_retry_policy().await,
        }

        // Delegate to provider
        match &self.provider {
            LLMProvider::Anthropic(p) => p.execute_llm(request, current_tool_round, config).await,
            LLMProvider::OpenAI(p) => p.execute_llm(request, current_tool_round, config).await,
            LLMProvider::LMStudio(p) => p.execute_llm(request, current_tool_round, config).await,
            LLMProvider::Ollama(p) => p.execute_llm(request, current_tool_round, config).await,
        }
    }

    #[cfg(feature = "events")]
    async fn execute_structured_llm(
        &self,
        request: UnifiedLLMRequest,
        current_tool_round: Option<ToolCallingRound>,
        schema: serde_json::Value,
        config: Option<RequestConfig>,
    ) -> crate::provider::Result<(Response, Vec<LLMBusinessEvent>)> {
        // Restore default retry policy
        match &self.provider {
            LLMProvider::Anthropic(p) => p.restore_default_retry_policy().await,
            LLMProvider::OpenAI(p) => p.restore_default_retry_policy().await,
            LLMProvider::LMStudio(p) => p.restore_default_retry_policy().await,
            LLMProvider::Ollama(p) => p.restore_default_retry_policy().await,
        }

        // Delegate to provider
        match &self.provider {
            LLMProvider::Anthropic(p) => {
                p.execute_structured_llm(request, current_tool_round, schema, config)
                    .await
            }
            LLMProvider::OpenAI(p) => {
                p.execute_structured_llm(request, current_tool_round, schema, config)
                    .await
            }
            LLMProvider::LMStudio(p) => {
                p.execute_structured_llm(request, current_tool_round, schema, config)
                    .await
            }
            LLMProvider::Ollama(p) => {
                p.execute_structured_llm(request, current_tool_round, schema, config)
                    .await
            }
        }
    }

    #[cfg(not(feature = "events"))]
    async fn execute_structured_llm(
        &self,
        request: UnifiedLLMRequest,
        current_tool_round: Option<ToolCallingRound>,
        schema: serde_json::Value,
        config: Option<RequestConfig>,
    ) -> crate::provider::Result<Response> {
        // Restore default retry policy
        match &self.provider {
            LLMProvider::Anthropic(p) => p.restore_default_retry_policy().await,
            LLMProvider::OpenAI(p) => p.restore_default_retry_policy().await,
            LLMProvider::LMStudio(p) => p.restore_default_retry_policy().await,
            LLMProvider::Ollama(p) => p.restore_default_retry_policy().await,
        }

        // Delegate to provider
        match &self.provider {
            LLMProvider::Anthropic(p) => {
                p.execute_structured_llm(request, current_tool_round, schema, config)
                    .await
            }
            LLMProvider::OpenAI(p) => {
                p.execute_structured_llm(request, current_tool_round, schema, config)
                    .await
            }
            LLMProvider::LMStudio(p) => {
                p.execute_structured_llm(request, current_tool_round, schema, config)
                    .await
            }
            LLMProvider::Ollama(p) => {
                p.execute_structured_llm(request, current_tool_round, schema, config)
                    .await
            }
        }
    }

    fn provider_name(&self) -> &'static str {
        match &self.provider {
            LLMProvider::Anthropic(_) => "anthropic",
            LLMProvider::OpenAI(_) => "openai",
            LLMProvider::LMStudio(_) => "lmstudio",
            LLMProvider::Ollama(_) => "ollama",
        }
    }
}
