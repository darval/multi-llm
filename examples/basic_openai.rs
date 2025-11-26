//! Basic OpenAI example demonstrating simple request/response.
//!
//! This example shows how to:
//! - Create an OpenAI configuration
//! - Build a simple conversation
//! - Execute a request and get a response
//!
//! # Running
//!
//! ```bash
//! export OPENAI_API_KEY="sk-..."
//! cargo run --example basic_openai
//! ```

use multi_llm::{
    unwrap_response, DefaultLLMParams, LLMConfig, LlmProvider, OpenAIConfig, UnifiedLLMClient,
    UnifiedLLMRequest, UnifiedMessage,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Get API key from environment
    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable must be set");

    // Create OpenAI configuration
    let config = LLMConfig {
        provider: Box::new(OpenAIConfig {
            api_key: Some(api_key),
            base_url: "https://api.openai.com".to_string(),
            default_model: "gpt-4o-mini".to_string(),
            max_context_tokens: 128_000,
            retry_policy: Default::default(),
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

    println!("Sending request to OpenAI...");

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
