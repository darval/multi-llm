use crate::config::{AnthropicConfig, LLMConfig, LMStudioConfig, OllamaConfig, OpenAIConfig};
use crate::{log_debug, log_error};
use crate::error::{LlmError, LlmResult};
use crate::providers::{AnthropicProvider, LMStudioProvider, OllamaProvider, OpenAIProvider};
use async_trait::async_trait;
use crate::core_types::{
    executor::{
        ExecutorLLMConfig, ExecutorLLMProvider, ExecutorLLMResponse, ExecutorResponseFormat,
        LLMBusinessEvent, ToolCallingRound,
    },
    messages::UnifiedLLMRequest,
};
// StructuredSystemPrompt methods are now directly on the type in mystory-core
use std::time::Instant;

// UnifiedLLMClient will directly implement ExecutorLLMProvider from mystory-core

/// Internal provider enum for UnifiedLLMClient
enum LLMProvider {
    Anthropic(AnthropicProvider),
    OpenAI(OpenAIProvider),
    LMStudio(LMStudioProvider),
    Ollama(OllamaProvider),
}

/// Unified LLM client that implements ExecutorLLMProvider
/// This is the primary interface for LLM operations in the myStory system
pub struct UnifiedLLMClient {
    provider: LLMProvider,
    model: String,
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

        Ok(Self { provider, model })
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

/// Implement ExecutorLLMProvider for UnifiedLLMClient
/// This preserves all the existing functionality while adapting to the new trait interface
#[async_trait]
impl ExecutorLLMProvider for UnifiedLLMClient {
    async fn execute_llm(
        &self,
        request: UnifiedLLMRequest,
        current_tool_round: Option<ToolCallingRound>,
        config: Option<ExecutorLLMConfig>,
    ) -> crate::core_types::Result<(ExecutorLLMResponse, Vec<LLMBusinessEvent>)> {
        let start = Instant::now();

        // Convert ExecutorLLMConfig to internal ExecutorLLMConfig (they're the same now)
        let internal_config = config;

        // Use default retry policy for executor operations
        match &self.provider {
            LLMProvider::Anthropic(provider) => {
                provider.restore_default_retry_policy().await;
            }
            LLMProvider::OpenAI(provider) => {
                provider.restore_default_retry_policy().await;
            }
            LLMProvider::LMStudio(provider) => {
                provider.restore_default_retry_policy().await;
            }
            LLMProvider::Ollama(provider) => {
                provider.restore_default_retry_policy().await;
            }
        }

        // Execute request with the configured retry policy
        let result = match &self.provider {
            LLMProvider::Anthropic(provider) => {
                provider
                    .execute_llm(request.clone(), current_tool_round.clone(), internal_config)
                    .await
            }
            LLMProvider::OpenAI(provider) => {
                provider
                    .execute_llm(request.clone(), current_tool_round.clone(), internal_config)
                    .await
            }
            LLMProvider::LMStudio(provider) => {
                provider
                    .execute_llm(request.clone(), current_tool_round.clone(), internal_config)
                    .await
            }
            LLMProvider::Ollama(provider) => {
                provider
                    .execute_llm(request.clone(), current_tool_round.clone(), internal_config)
                    .await
            }
        };

        match result {
            Ok((response, events)) => {
                log_debug!(
                    duration_ms = start.elapsed().as_millis(),
                    input_tokens = response.usage.as_ref().map_or(0, |u| u.prompt_tokens),
                    output_tokens = response.usage.as_ref().map_or(0, |u| u.completion_tokens),
                    total_tokens = response.usage.as_ref().map_or(0, |u| u.total_tokens),
                    provider = self.provider_name(),
                    model = %self.model,
                    "ExecutorLLMProvider request completed successfully"
                );

                // Return response and events (events are already collected by provider)
                Ok((response, events))
            }
            Err(e) => {
                log_error!(
                    duration_ms = start.elapsed().as_millis(),
                    error = %e,
                    provider = self.provider_name(),
                    model = %self.model,
                    "ExecutorLLMProvider request failed"
                );
                Err(anyhow::anyhow!("LLM execution failed: {}", e))
            }
        }
    }

    async fn execute_structured_llm(
        &self,
        request: UnifiedLLMRequest,
        current_tool_round: Option<ToolCallingRound>,
        schema: serde_json::Value,
        config: Option<ExecutorLLMConfig>,
    ) -> crate::core_types::Result<(ExecutorLLMResponse, Vec<LLMBusinessEvent>)> {
        let start = Instant::now();

        // Convert ExecutorLLMConfig and add structured response format
        let mut internal_config = config.unwrap_or_default();

        // Add the JSON schema to the response format (overrides any existing format)
        internal_config.response_format = Some(ExecutorResponseFormat {
            name: "structured_response".to_string(),
            schema: schema.clone(),
        });

        // Use default retry policy for executor operations
        match &self.provider {
            LLMProvider::Anthropic(provider) => {
                provider.restore_default_retry_policy().await;
            }
            LLMProvider::OpenAI(provider) => {
                provider.restore_default_retry_policy().await;
            }
            LLMProvider::LMStudio(provider) => {
                provider.restore_default_retry_policy().await;
            }
            LLMProvider::Ollama(provider) => {
                provider.restore_default_retry_policy().await;
            }
        }

        // Execute request with the configured retry policy
        let result = match &self.provider {
            LLMProvider::Anthropic(provider) => {
                provider
                    .execute_structured_llm(
                        request.clone(),
                        current_tool_round.clone(),
                        schema.clone(),
                        Some(internal_config),
                    )
                    .await
            }
            LLMProvider::OpenAI(provider) => {
                provider
                    .execute_structured_llm(
                        request.clone(),
                        current_tool_round.clone(),
                        schema.clone(),
                        Some(internal_config),
                    )
                    .await
            }
            LLMProvider::LMStudio(provider) => {
                provider
                    .execute_structured_llm(
                        request.clone(),
                        current_tool_round.clone(),
                        schema.clone(),
                        Some(internal_config),
                    )
                    .await
            }
            LLMProvider::Ollama(provider) => {
                provider
                    .execute_structured_llm(
                        request.clone(),
                        current_tool_round.clone(),
                        schema.clone(),
                        Some(internal_config),
                    )
                    .await
            }
        };

        match result {
            Ok((response, events)) => {
                log_debug!(
                    duration_ms = start.elapsed().as_millis(),
                    input_tokens = response.usage.as_ref().map_or(0, |u| u.prompt_tokens),
                    output_tokens = response.usage.as_ref().map_or(0, |u| u.completion_tokens),
                    total_tokens = response.usage.as_ref().map_or(0, |u| u.total_tokens),
                    provider = self.provider_name(),
                    model = %self.model,
                    "ExecutorLLMProvider structured request completed successfully"
                );

                // Return response and events (events are already collected by provider)
                Ok((response, events))
            }
            Err(e) => {
                log_error!(
                    duration_ms = start.elapsed().as_millis(),
                    error = %e,
                    provider = self.provider_name(),
                    model = %self.model,
                    "ExecutorLLMProvider structured request failed"
                );
                Err(anyhow::anyhow!("Structured LLM execution failed: {}", e))
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
