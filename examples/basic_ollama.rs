//! Basic Ollama example demonstrating simple request/response with local models.
//!
//! This example shows how to:
//! - Create an Ollama configuration for local models
//! - Build a simple conversation
//! - Execute a request and get a response
//!
//! # Prerequisites
//!
//! 1. Install Ollama: https://ollama.ai
//! 2. Pull a model: `ollama pull llama3.2`
//! 3. Ollama server runs automatically on first use
//!
//! # Running
//!
//! ```bash
//! cargo run --example basic_ollama
//! ```

use multi_llm::{
    unwrap_response, DefaultLLMParams, LLMConfig, LlmProvider, OllamaConfig, UnifiedLLMClient,
    UnifiedLLMRequest, UnifiedMessage,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create Ollama configuration (no API key needed for local models)
    // Use OLLAMA_MODEL env var or default to llama3.2
    let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".to_string());
    let config = LLMConfig {
        provider: Box::new(OllamaConfig {
            base_url: "http://localhost:11434".to_string(),
            default_model: model.clone(),
            max_context_tokens: 8192,
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

    println!(
        "Sending request to Ollama (local) using model: {}...",
        model
    );
    println!("Note: First request may be slow if the model isn't loaded yet.\n");

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
