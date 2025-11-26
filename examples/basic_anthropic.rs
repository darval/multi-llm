//! Basic Anthropic example demonstrating simple request/response.
//!
//! This example shows how to:
//! - Create an Anthropic configuration
//! - Build a simple conversation
//! - Execute a request and get a response
//!
//! # Running
//!
//! ```bash
//! export ANTHROPIC_API_KEY="sk-ant-..."
//! cargo run --example basic_anthropic
//! ```

use multi_llm::{
    unwrap_response, AnthropicConfig, DefaultLLMParams, LLMConfig, LlmProvider, UnifiedLLMClient,
    UnifiedLLMRequest, UnifiedMessage,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Get API key from environment
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY environment variable must be set");

    // Create Anthropic configuration
    let config = LLMConfig {
        provider: Box::new(AnthropicConfig {
            api_key: Some(api_key),
            base_url: "https://api.anthropic.com".to_string(),
            default_model: "claude-sonnet-4-20250514".to_string(), // Or use claude-3-5-sonnet-latest
            max_context_tokens: 200_000,
            retry_policy: Default::default(),
            enable_prompt_caching: false, // See prompt_caching.rs for caching example
            cache_ttl: "5m".to_string(),
        }),
        default_params: DefaultLLMParams::default(),
    };

    // Create the client
    let client = UnifiedLLMClient::from_config(config)?;

    // Build a simple conversation
    let request = UnifiedLLMRequest::new(vec![
        UnifiedMessage::system("You are a helpful assistant. Be concise."),
        UnifiedMessage::user("What is the capital of France? Answer in one sentence."),
    ]);

    println!("Sending request to Anthropic...");

    // Execute the request
    // The unwrap_response! macro handles both with/without "events" feature
    let response = unwrap_response!(client.execute_llm(request, None, None).await?);

    // Print the response
    println!("\nResponse: {}", response.content);

    // Print token usage if available
    if let Some(usage) = &response.usage {
        println!(
            "\nToken usage: {} input + {} output = {} total",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    Ok(())
}
