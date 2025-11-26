//! Provider switching example - same code works across different providers.
//!
//! This example demonstrates:
//! - Creating configurations for multiple providers
//! - Using environment variables to select provider at runtime
//! - The same request code working across all providers
//!
//! # Running
//!
//! ```bash
//! # For OpenAI
//! export AI_PROVIDER=openai
//! export OPENAI_API_KEY="sk-..."
//! cargo run --example provider_switching
//!
//! # For Anthropic
//! export AI_PROVIDER=anthropic
//! export ANTHROPIC_API_KEY="sk-ant-..."
//! cargo run --example provider_switching
//!
//! # For Ollama (no API key needed)
//! export AI_PROVIDER=ollama
//! cargo run --example provider_switching
//! ```

use multi_llm::{
    unwrap_response, AnthropicConfig, DefaultLLMParams, LLMConfig, LMStudioConfig, LlmProvider,
    OllamaConfig, OpenAIConfig, UnifiedLLMClient, UnifiedLLMRequest, UnifiedMessage,
};

fn create_config(provider: &str) -> anyhow::Result<LLMConfig> {
    match provider {
        "openai" => {
            let api_key = std::env::var("OPENAI_API_KEY")
                .expect("OPENAI_API_KEY required for OpenAI provider");
            Ok(LLMConfig {
                provider: Box::new(OpenAIConfig {
                    api_key: Some(api_key),
                    base_url: "https://api.openai.com".to_string(),
                    default_model: "gpt-4o-mini".to_string(),
                    max_context_tokens: 128_000,
                    retry_policy: Default::default(),
                }),
                default_params: DefaultLLMParams::default(),
            })
        }
        "anthropic" => {
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .expect("ANTHROPIC_API_KEY required for Anthropic provider");
            Ok(LLMConfig {
                provider: Box::new(AnthropicConfig {
                    api_key: Some(api_key),
                    base_url: "https://api.anthropic.com".to_string(),
                    default_model: "claude-sonnet-4-20250514".to_string(),
                    max_context_tokens: 200_000,
                    retry_policy: Default::default(),
                    enable_prompt_caching: false,
                    cache_ttl: "5m".to_string(),
                }),
                default_params: DefaultLLMParams::default(),
            })
        }
        "ollama" => Ok(LLMConfig {
            provider: Box::new(OllamaConfig {
                base_url: "http://localhost:11434".to_string(),
                default_model: "llama3.2".to_string(),
                max_context_tokens: 8192,
                retry_policy: Default::default(),
            }),
            default_params: DefaultLLMParams::default(),
        }),
        "lmstudio" => Ok(LLMConfig {
            provider: Box::new(LMStudioConfig {
                base_url: "http://localhost:1234".to_string(),
                default_model: "local-model".to_string(),
                max_context_tokens: 4096,
                retry_policy: Default::default(),
            }),
            default_params: DefaultLLMParams::default(),
        }),
        _ => anyhow::bail!(
            "Unknown provider: {}. Use: openai, anthropic, ollama, lmstudio",
            provider
        ),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Get provider from environment (default to openai)
    let provider = std::env::var("AI_PROVIDER").unwrap_or_else(|_| "openai".to_string());
    println!("Using provider: {}\n", provider);

    // Create config based on provider
    let config = create_config(&provider)?;

    // Create the client
    let client = UnifiedLLMClient::from_config(config)?;

    // Build a request - this code is identical regardless of provider!
    let request = UnifiedLLMRequest::new(vec![
        UnifiedMessage::system("You are a helpful assistant. Be concise."),
        UnifiedMessage::user("What is 2 + 2? Answer with just the number."),
    ]);

    println!("Sending request...");

    // Execute - same API for all providers
    // The unwrap_response! macro handles both with/without "events" feature
    let response = unwrap_response!(client.execute_llm(request, None, None).await?);

    println!("Response: {}", response.content);

    if let Some(usage) = &response.usage {
        println!(
            "Tokens: {} input + {} output = {} total",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    Ok(())
}
