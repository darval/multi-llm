//! Multi-instance example demonstrating simultaneous LLM usage patterns.
//!
//! This example shows how to:
//! - Create multiple independent LLM clients
//! - Use DualLLMConfig for user-facing vs background processing
//! - Run parallel requests to different models
//! - Mix providers (e.g., Anthropic for quality, local for speed)
//!
//! # Running
//!
//! ```bash
//! export ANTHROPIC_API_KEY="sk-ant-..."
//! cargo run --example multi_instance
//! ```
//!
//! # Use Cases
//!
//! - **Fast + Smart**: Quick local model for drafts, cloud model for polish
//! - **User + NLP**: High-quality for user responses, cheap for background analysis
//! - **Parallel Processing**: Multiple simultaneous requests to same/different providers

use multi_llm::{
    unwrap_response, AnthropicConfig, DefaultLLMParams, DualLLMConfig, LLMConfig, LLMPath,
    LlmProvider, OllamaConfig, UnifiedLLMClient, UnifiedLLMRequest, UnifiedMessage,
};
use std::time::Instant;

/// Create an Anthropic configuration (high quality)
fn create_anthropic_config() -> LLMConfig {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY environment variable must be set");

    LLMConfig {
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
    }
}

/// Create an Anthropic Haiku configuration (fast + cheap)
fn create_haiku_config() -> LLMConfig {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY environment variable must be set");

    LLMConfig {
        provider: Box::new(AnthropicConfig {
            api_key: Some(api_key),
            base_url: "https://api.anthropic.com".to_string(),
            default_model: "claude-3-5-haiku-20241022".to_string(),
            max_context_tokens: 200_000,
            retry_policy: Default::default(),
            enable_prompt_caching: false,
            cache_ttl: "5m".to_string(),
        }),
        default_params: DefaultLLMParams::default(),
    }
}

/// Create an Ollama configuration (local, free)
fn create_ollama_config() -> Option<LLMConfig> {
    // Check if Ollama is available
    let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".to_string());

    Some(LLMConfig {
        provider: Box::new(OllamaConfig {
            base_url: "http://localhost:11434".to_string(),
            default_model: model,
            max_context_tokens: 8192,
            retry_policy: Default::default(),
        }),
        default_params: DefaultLLMParams::default(),
    })
}

/// Demonstrates using DualLLMConfig for user vs NLP paths
async fn demo_dual_config() -> anyhow::Result<()> {
    println!("=== Demo 1: DualLLMConfig (User vs NLP paths) ===\n");

    // Create dual config: Sonnet for user-facing, Haiku for background NLP
    let dual_config = DualLLMConfig::new(create_anthropic_config(), create_haiku_config());

    // Create clients for each path
    let user_client = UnifiedLLMClient::from_config(dual_config.get_config(LLMPath::User).clone())?;
    let nlp_client = UnifiedLLMClient::from_config(dual_config.get_config(LLMPath::Nlp).clone())?;

    // User-facing request (high quality response)
    let user_request = UnifiedLLMRequest::new(vec![
        UnifiedMessage::system("You are a helpful assistant. Be friendly and thorough."),
        UnifiedMessage::user("Explain what makes a good cup of coffee in 2-3 sentences."),
    ]);

    // NLP request (background processing - structured extraction)
    let nlp_request = UnifiedLLMRequest::new(vec![
        UnifiedMessage::system("Extract key information. Respond with only the extracted data."),
        UnifiedMessage::user(
            "Extract the sentiment (positive/negative/neutral) from: 'I love this product!'",
        ),
    ]);

    println!("User path (Sonnet - high quality):");
    let start = Instant::now();
    let user_response = unwrap_response!(user_client.execute_llm(user_request, None, None).await?);
    println!("  Response: {}", user_response.content);
    println!("  Time: {:?}\n", start.elapsed());

    println!("NLP path (Haiku - fast/cheap):");
    let start = Instant::now();
    let nlp_response = unwrap_response!(nlp_client.execute_llm(nlp_request, None, None).await?);
    println!("  Response: {}", nlp_response.content);
    println!("  Time: {:?}\n", start.elapsed());

    Ok(())
}

/// Demonstrates parallel requests to the same provider
async fn demo_parallel_same_provider() -> anyhow::Result<()> {
    println!("=== Demo 2: Parallel Requests (Same Provider) ===\n");

    let client = UnifiedLLMClient::from_config(create_haiku_config())?;

    let questions = [
        "What is 2 + 2? Answer with just the number.",
        "What color is the sky? Answer in one word.",
        "Is water wet? Answer yes or no.",
    ];

    println!("Sending 3 requests in parallel to Haiku...\n");
    let start = Instant::now();

    // Create all requests
    let requests: Vec<_> = questions
        .iter()
        .map(|q| {
            UnifiedLLMRequest::new(vec![
                UnifiedMessage::system("Be extremely concise."),
                UnifiedMessage::user(*q),
            ])
        })
        .collect();

    // Execute in parallel using tokio::join!
    let (r1, r2, r3) = tokio::join!(
        client.execute_llm(requests[0].clone(), None, None),
        client.execute_llm(requests[1].clone(), None, None),
        client.execute_llm(requests[2].clone(), None, None),
    );

    let total_time = start.elapsed();

    // Print results - unwrap_response! handles both with/without events feature
    for (i, (question, result)) in questions.iter().zip([r1, r2, r3].into_iter()).enumerate() {
        match result {
            Ok(result) => {
                let response = unwrap_response!(result);
                println!("  Q{}: {} -> {}", i + 1, question, response.content.trim());
            }
            Err(e) => {
                println!("  Q{}: {} -> Error: {}", i + 1, question, e);
            }
        }
    }

    println!("\n  Total parallel time: {:?}", total_time);
    println!("  (Sequential would take ~3x longer)\n");

    Ok(())
}

/// Demonstrates using multiple different providers simultaneously
async fn demo_multiple_providers() -> anyhow::Result<()> {
    println!("=== Demo 3: Multiple Providers (Cloud + Local) ===\n");

    // Create Anthropic client
    let cloud_client = UnifiedLLMClient::from_config(create_haiku_config())?;

    // Try to create local client (Ollama)
    let local_config = create_ollama_config();

    let question = "What is the capital of Japan? One word answer.";

    println!("Sending same question to cloud and local providers...\n");

    // Cloud request
    let cloud_request = UnifiedLLMRequest::new(vec![
        UnifiedMessage::system("Be concise."),
        UnifiedMessage::user(question),
    ]);

    let start = Instant::now();
    let cloud_response =
        unwrap_response!(cloud_client.execute_llm(cloud_request, None, None).await?);
    let cloud_time = start.elapsed();

    println!(
        "  Cloud (Haiku): {} ({:?})",
        cloud_response.content.trim(),
        cloud_time
    );

    // Local request (if available)
    if let Some(config) = local_config {
        match UnifiedLLMClient::from_config(config) {
            Ok(local_client) => {
                let local_request = UnifiedLLMRequest::new(vec![
                    UnifiedMessage::system("Be concise."),
                    UnifiedMessage::user(question),
                ]);

                let start = Instant::now();
                match local_client.execute_llm(local_request, None, None).await {
                    Ok(result) => {
                        let response = unwrap_response!(result);
                        let local_time = start.elapsed();
                        println!(
                            "  Local (Ollama): {} ({:?})",
                            response.content.trim(),
                            local_time
                        );
                    }
                    Err(e) => {
                        println!("  Local (Ollama): Not available - {}", e);
                    }
                }
            }
            Err(e) => {
                println!("  Local (Ollama): Not configured - {}", e);
            }
        }
    } else {
        println!("  Local (Ollama): Skipped (set OLLAMA_MODEL to enable)");
    }

    println!();
    Ok(())
}

/// Demonstrates the "fast draft, smart polish" pattern
async fn demo_draft_and_polish() -> anyhow::Result<()> {
    println!("=== Demo 4: Draft & Polish Pattern ===\n");

    let fast_client = UnifiedLLMClient::from_config(create_haiku_config())?;
    let smart_client = UnifiedLLMClient::from_config(create_anthropic_config())?;

    let topic = "the benefits of exercise";

    // Step 1: Fast draft with Haiku
    println!("Step 1: Quick draft (Haiku)...");
    let draft_request = UnifiedLLMRequest::new(vec![
        UnifiedMessage::system("Write a rough first draft. Don't worry about perfection."),
        UnifiedMessage::user(format!("Write 2 sentences about {}", topic)),
    ]);

    let start = Instant::now();
    let draft = unwrap_response!(fast_client.execute_llm(draft_request, None, None).await?);
    println!("  Draft: {}", draft.content);
    println!("  Time: {:?}\n", start.elapsed());

    // Step 2: Polish with Sonnet
    println!("Step 2: Polish (Sonnet)...");
    let polish_request = UnifiedLLMRequest::new(vec![
        UnifiedMessage::system(
            "Improve the following draft. Make it more engaging while keeping it concise.",
        ),
        UnifiedMessage::user(format!("Polish this: {}", draft.content)),
    ]);

    let start = Instant::now();
    let polished = unwrap_response!(smart_client.execute_llm(polish_request, None, None).await?);
    println!("  Polished: {}", polished.content);
    println!("  Time: {:?}\n", start.elapsed());

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Multi-Instance LLM Examples\n");
    println!("This demonstrates using multiple LLM instances simultaneously.\n");

    // Demo 1: DualLLMConfig
    demo_dual_config().await?;

    // Demo 2: Parallel requests
    demo_parallel_same_provider().await?;

    // Demo 3: Multiple providers
    demo_multiple_providers().await?;

    // Demo 4: Draft and polish
    demo_draft_and_polish().await?;

    println!("=== Summary ===\n");
    println!("Key patterns demonstrated:");
    println!("  1. DualLLMConfig - Separate configs for user-facing vs background");
    println!("  2. Parallel requests - tokio::join! for concurrent execution");
    println!("  3. Multiple providers - Mix cloud and local models");
    println!("  4. Draft & polish - Fast model for drafts, smart model for refinement");

    Ok(())
}
