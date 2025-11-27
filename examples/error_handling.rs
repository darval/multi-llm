//! Error handling example demonstrating error types, categories, and retry logic.
//!
//! This example shows how to:
//! - Handle different error types from LLM operations
//! - Use error categories for routing decisions
//! - Implement retry logic for transient errors
//! - Convert errors to user-friendly messages
//!
//! # Running
//!
//! ```bash
//! # This example demonstrates error handling patterns
//! cargo run --example error_handling
//!
//! # To test with a real (invalid) API call:
//! cargo run --example error_handling -- --live
//! ```
//!
//! # Error Categories
//!
//! | Category | Examples | Action |
//! |----------|----------|--------|
//! | Client | Auth failed, config error | Fix configuration |
//! | Transient | Rate limit, timeout | Retry with backoff |
//! | External | Provider errors | Log, maybe retry |
//!
//! # Key Methods
//!
//! - `error.category()` - Get high-level category for routing
//! - `error.severity()` - Get logging severity level
//! - `error.is_retryable()` - Check if retry makes sense
//! - `error.user_message()` - Get safe user-facing message

use multi_llm::{
    error::ErrorCategory, DefaultLLMParams, LLMConfig, LlmError, LlmProvider, OpenAIConfig,
    UnifiedLLMClient, UnifiedLLMRequest, UnifiedMessage,
};

/// Demonstrates handling different error types
fn demonstrate_error_types() {
    println!("=== Error Types and Categories ===\n");

    // Configuration errors (Category: Client)
    let config_error = LlmError::configuration_error("Missing API key");
    print_error_info("ConfigurationError", &config_error);

    // Rate limit errors (Category: Transient)
    let rate_limit = LlmError::rate_limit_exceeded(60);
    print_error_info("RateLimitExceeded", &rate_limit);

    // Timeout errors (Category: Transient)
    let timeout = LlmError::timeout(30);
    print_error_info("Timeout", &timeout);

    // Authentication errors (Category: Client)
    let auth_error = LlmError::authentication_failed("Invalid API key");
    print_error_info("AuthenticationFailed", &auth_error);

    // Token limit errors (Category: Client)
    let token_error = LlmError::token_limit_exceeded(150_000, 128_000);
    print_error_info("TokenLimitExceeded", &token_error);
}

/// Print detailed information about an error
fn print_error_info(name: &str, error: &LlmError) {
    println!("{}:", name);
    println!("  Display: {}", error);
    println!("  Category: {:?}", error.category());
    println!("  Severity: {:?}", error.severity());
    println!("  Retryable: {}", error.is_retryable());
    println!("  User message: {}", error.user_message());
    println!();
}

/// Demonstrates category-based error routing
fn demonstrate_error_routing() {
    println!("=== Category-Based Error Routing ===\n");

    let errors = vec![
        LlmError::configuration_error("Bad config"),
        LlmError::rate_limit_exceeded(30),
        LlmError::timeout(60),
        LlmError::authentication_failed("Bad key"),
    ];

    for error in errors {
        let action = match error.category() {
            ErrorCategory::Client => "Fix configuration and retry",
            ErrorCategory::Transient => "Retry with exponential backoff",
            ErrorCategory::External => "Log and alert ops team",
            ErrorCategory::Internal => "Log, alert, investigate bug",
            ErrorCategory::BusinessLogic => "Handle as expected flow",
            _ => "Unknown category - update error handling",
        };

        println!("Error: {}", error);
        println!("  Category: {:?}", error.category());
        println!("  Action: {}\n", action);
    }
}

/// Demonstrates retry logic pattern for transient errors
fn demonstrate_retry_pattern() {
    println!("=== Retry Pattern for Transient Errors ===\n");

    // Simulate handling different errors with retry logic
    let test_errors = vec![
        LlmError::rate_limit_exceeded(5),
        LlmError::timeout(30),
        LlmError::authentication_failed("Invalid key"),
        LlmError::request_failed("Network error".to_string(), None),
    ];

    println!("Checking which errors should be retried:\n");

    for error in test_errors {
        if error.is_retryable() {
            println!("  {} -> RETRY", error);
        } else {
            println!("  {} -> DO NOT RETRY", error);
        }
    }

    println!("\n--- Example Retry Loop Pattern ---\n");
    println!(
        r#"
async fn execute_with_retry(
    client: &UnifiedLLMClient,
    request: UnifiedLLMRequest,
) -> Result<Response, LlmError> {{
    let max_retries = 3;
    let mut delay = Duration::from_secs(1);

    for attempt in 1..=max_retries {{
        match client.execute_llm(request.clone(), None, None).await {{
            Ok(response) => return Ok(response),
            Err(e) if e.is_retryable() && attempt < max_retries => {{
                println!("Attempt {{}} failed: {{}}", attempt, e);
                tokio::time::sleep(delay).await;
                delay *= 2; // Exponential backoff
            }}
            Err(e) => return Err(e),
        }}
    }}
    unreachable!()
}}
"#
    );
}

/// Demonstrates user-friendly error messages
fn demonstrate_user_messages() {
    println!("=== User-Friendly Error Messages ===\n");

    println!("Technical errors should be translated to safe user messages:\n");

    let errors = vec![
        LlmError::rate_limit_exceeded(60),
        LlmError::timeout(30),
        LlmError::authentication_failed("sk-invalid-key-12345"),
        LlmError::token_limit_exceeded(200_000, 128_000),
        LlmError::configuration_error("Provider config validation failed: missing base_url"),
    ];

    for error in errors {
        println!("Technical: {}", error);
        println!("User-safe: {}\n", error.user_message());
    }
}

/// Demonstrates real error handling with an LLM client
async fn demonstrate_real_error_handling() -> Result<(), LlmError> {
    println!("=== Real Error Handling Example ===\n");

    // Try to create a client with an invalid API key
    let config = LLMConfig {
        provider: Box::new(OpenAIConfig {
            api_key: Some("sk-invalid-key-for-demo".to_string()),
            base_url: "https://api.openai.com".to_string(),
            default_model: "gpt-4o-mini".to_string(),
            max_context_tokens: 128_000,
            retry_policy: Default::default(),
        }),
        default_params: DefaultLLMParams::default(),
    };

    let client = UnifiedLLMClient::from_config(config)?;

    let request = UnifiedLLMRequest::new(vec![UnifiedMessage::user("Hello!")]);

    println!("Attempting request with invalid API key...\n");

    // Note: unwrap_response! handles both with/without "events" feature
    // We use map here because we want to demonstrate error handling on the Result
    match client.execute_llm(request, None, None).await {
        #[cfg(feature = "events")]
        Ok((response, _events)) => {
            println!("Unexpected success: {}", response.content);
        }
        #[cfg(not(feature = "events"))]
        Ok(response) => {
            println!("Unexpected success: {}", response.content);
        }
        Err(error) => {
            // Now we have direct access to LlmError methods!
            println!("Error occurred (expected):");
            println!("  Type: {}", error);
            println!("  Category: {:?}", error.category());
            println!("  Retryable: {}", error.is_retryable());
            println!("  User message: {}", error.user_message());
            println!();

            // Demonstrate how to handle based on category
            match error.category() {
                ErrorCategory::Client => {
                    println!(
                        "Action: This is a client error. Check your API key and configuration."
                    );
                }
                ErrorCategory::Transient => {
                    println!("Action: This is transient. Retry with backoff.");
                }
                ErrorCategory::External => {
                    println!("Action: Provider issue. Check status page or try later.");
                }
                _ => {
                    println!("Action: Investigate the error.");
                }
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let run_live = args.contains(&"--live".to_string());

    // Section 1: Error types overview
    demonstrate_error_types();

    // Section 2: Category-based routing
    demonstrate_error_routing();

    // Section 3: Retry pattern
    demonstrate_retry_pattern();

    // Section 4: User-friendly messages
    demonstrate_user_messages();

    // Section 5: Real error handling (optional - makes actual API call)
    if run_live {
        println!("Running live error handling test...\n");
        if let Err(e) = demonstrate_real_error_handling().await {
            println!("Live test resulted in error: {}", e);
        }
    } else {
        println!("=== Live Test Skipped ===\n");
        println!("Run with --live flag to test with actual API calls:");
        println!("  cargo run --example error_handling -- --live\n");
    }

    println!("=== Error Handling Patterns Summary ===\n");
    println!("1. Use error.category() to route errors to appropriate handlers");
    println!("2. Use error.is_retryable() to decide if retry makes sense");
    println!("3. Use error.user_message() for safe user-facing messages");
    println!("4. Use error.severity() for logging level decisions");
    println!("5. The library's RetryPolicy handles transient errors automatically");

    Ok(())
}
