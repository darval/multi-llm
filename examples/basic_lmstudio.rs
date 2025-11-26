//! Basic LM Studio example demonstrating simple request/response with local models.
//!
//! This example shows how to:
//! - Create an LM Studio configuration for local models
//! - Build a simple conversation
//! - Execute a request and get a response
//!
//! # Prerequisites
//!
//! 1. Install LM Studio: https://lmstudio.ai
//! 2. Load a model in LM Studio
//! 3. Start the local server (default port: 1234)
//!
//! # Running
//!
//! ```bash
//! cargo run --example basic_lmstudio
//! ```

use multi_llm::{
    unwrap_response, DefaultLLMParams, LLMConfig, LMStudioConfig, LlmProvider, UnifiedLLMClient,
    UnifiedLLMRequest, UnifiedMessage,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create LM Studio configuration (no API key needed for local models)
    // Note: The model name is ignored - LM Studio uses whatever model is loaded
    let config = LLMConfig {
        provider: Box::new(LMStudioConfig {
            base_url: "http://localhost:1234".to_string(),
            default_model: "local-model".to_string(), // Ignored by LM Studio
            max_context_tokens: 4096,
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

    println!("Sending request to LM Studio (local)...");
    println!("Note: Make sure LM Studio is running with a model loaded.\n");

    // Execute the request
    // The unwrap_response! macro handles both with/without "events" feature
    let response = unwrap_response!(client.execute_llm(request, None, None).await?);

    // Print the response
    println!("Response: {}", response.content);

    // Print token usage if available
    if let Some(usage) = &response.usage {
        println!(
            "\nToken usage: {} input + {} output = {} total",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    Ok(())
}
