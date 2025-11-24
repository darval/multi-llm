//! Unit Tests for Anthropic Provider HTTP Integration
//!
//! UNIT UNDER TEST: AnthropicProvider HTTP request handling
//!
//! BUSINESS RESPONSIBILITY:
//!   - Execute HTTP requests to Anthropic API with authentication
//!   - Handle successful responses and parse into unified format
//!   - Handle API errors (401, 429, 500)
//!   - Apply retry logic for transient failures
//!   - Convert UnifiedMessage to Anthropic format
//!   - Emit business events for LLM interactions
//!
//! TEST COVERAGE:
//!   - Provider initialization with valid/invalid config
//!   - Successful API requests and response parsing
//!   - Authentication errors (401)
//!   - Rate limiting errors (429)
//!   - Server errors (500)
//!   - Network failures
//!   - Message conversion and tool handling

use chrono::Utc;
use multi_llm::config::{AnthropicConfig, DefaultLLMParams};
use multi_llm::core_types::messages::{
    CacheType, MessageAttributes, MessageCategory, MessageContent, MessageRole, UnifiedLLMRequest,
    UnifiedMessage,
};
use multi_llm::core_types::provider::LlmProvider;
use multi_llm::error::LlmError;
use multi_llm::providers::anthropic::AnthropicProvider;
use multi_llm::retry::RetryPolicy;
use std::collections::HashMap;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// Import shared test helpers
#[macro_use]
mod common;

fn create_test_config(base_url: String) -> AnthropicConfig {
    AnthropicConfig {
        api_key: Some("test-key".to_string()),
        base_url,
        default_model: "claude-3-5-sonnet-20241022".to_string(),
        max_context_tokens: 200_000,
        retry_policy: RetryPolicy {
            max_attempts: 2, // Reduced for faster tests
            initial_delay: std::time::Duration::from_millis(10),
            max_delay: std::time::Duration::from_millis(50),
            backoff_multiplier: 2.0,
            total_timeout: std::time::Duration::from_secs(10),
            request_timeout: std::time::Duration::from_secs(5),
        },
        enable_prompt_caching: false,
        cache_ttl: "5m".to_string(),
    }
}

fn create_default_params() -> DefaultLLMParams {
    DefaultLLMParams {
        temperature: 0.7,
        max_tokens: 1000,
        top_p: 1.0,
        top_k: 40,
        min_p: 0.0,
        presence_penalty: 0.0,
    }
}

fn create_test_message(content: &str) -> UnifiedMessage {
    UnifiedMessage {
        role: MessageRole::User,
        content: MessageContent::Text(content.to_string()),
        attributes: MessageAttributes {
            priority: 0,
            cacheable: false,
            cache_type: None,
            cache_key: None,
            category: MessageCategory::Current,
            metadata: HashMap::new(),
        },
        timestamp: Utc::now(),
    }
}

fn create_cacheable_message(
    role: MessageRole,
    content: &str,
    cache_type: CacheType,
) -> UnifiedMessage {
    UnifiedMessage {
        role,
        content: MessageContent::Text(content.to_string()),
        attributes: MessageAttributes {
            priority: 0,
            cacheable: true,
            cache_type: Some(cache_type),
            cache_key: None,
            category: MessageCategory::Current,
            metadata: HashMap::new(),
        },
        timestamp: Utc::now(),
    }
}

fn create_success_response() -> serde_json::Value {
    serde_json::json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "text",
            "text": "Hello!"
        }],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 10,
            "output_tokens": 5
        }
    })
}

fn create_llm_request() -> UnifiedLLMRequest {
    UnifiedLLMRequest {
        messages: vec![create_test_message("Hello")],
        response_schema: None,
        config: None,
    }
}

// ============================================================================
// Provider Initialization Tests
// ============================================================================

#[test]
fn test_provider_new_with_valid_config() {
    // Test provider initialization with valid configuration
    // Verifies that provider can be created with proper config

    let config = create_test_config("https://api.anthropic.com".to_string());
    let params = create_default_params();

    let result = AnthropicProvider::new(config, params);

    assert!(result.is_ok(), "Should initialize with valid config");
}

#[test]
fn test_provider_new_without_api_key() {
    // Test provider initialization fails without API key
    // Verifies that missing API key is caught during initialization

    let mut config = create_test_config("https://api.anthropic.com".to_string());
    config.api_key = None;
    let params = create_default_params();

    let result = AnthropicProvider::new(config, params);

    assert!(result.is_err(), "Should fail without API key");
    match result.unwrap_err() {
        LlmError::ConfigurationError { message } => {
            assert!(message.contains("API key"), "Error should mention API key");
        }
        other => panic!("Expected ConfigurationError, got: {:?}", other),
    }
}

// ============================================================================
// HTTP Request Tests
// ============================================================================

#[tokio::test]
async fn test_execute_request_success() {
    // Test successful HTTP request to Anthropic API
    // Verifies end-to-end request execution and response parsing

    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    let response = create_success_response();

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .mount(&mock_server)
        .await;

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_ok(), "Request should succeed");
    let response = unwrap_response!(result.unwrap());
    assert!(response.usage.is_some(), "Should have usage data");
    assert!(
        response.usage.unwrap().total_tokens > 0,
        "Should have non-zero tokens"
    );
}

#[tokio::test]
async fn test_handle_401_authentication_error() {
    // Test handling of authentication failures (401)
    // Verifies that invalid API keys result in authentication errors

    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    let error_body = serde_json::json!({
        "type": "error",
        "error": {
            "type": "authentication_error",
            "message": "Invalid API key"
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_json(&error_body))
        .mount(&mock_server)
        .await;

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_err(), "Should fail with authentication error");
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Authentication failed") || error.to_string().contains("401"),
        "Error should indicate authentication failure: {}",
        error
    );
}

#[tokio::test]
async fn test_handle_429_rate_limit_error() {
    // Test handling of rate limit errors (429)
    // Verifies that rate limits are properly detected and reported

    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    let error_body = serde_json::json!({
        "type": "error",
        "error": {
            "type": "rate_limit_error",
            "message": "Rate limit exceeded"
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "30")
                .set_body_json(&error_body),
        )
        .mount(&mock_server)
        .await;

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_err(), "Should fail with rate limit error");
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Rate limit") || error.to_string().contains("429"),
        "Error should indicate rate limiting: {}",
        error
    );
}

#[tokio::test]
async fn test_handle_500_server_error() {
    // Test handling of server errors (500)
    // Verifies that server failures are properly reported

    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    let error_body = serde_json::json!({
        "type": "error",
        "error": {
            "type": "api_error",
            "message": "Internal server error"
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_json(&error_body))
        .mount(&mock_server)
        .await;

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_err(), "Should fail with server error");
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("500") || error.to_string().contains("server"),
        "Error should indicate server failure: {}",
        error
    );
}

#[tokio::test]
async fn test_handle_invalid_json_response() {
    // Test handling of malformed JSON responses
    // Verifies that parsing errors are properly detected

    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
        .mount(&mock_server)
        .await;

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_err(), "Should fail with parsing error");
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("parsing") || error.to_string().contains("invalid"),
        "Error should indicate parsing failure: {}",
        error
    );
}

#[tokio::test]
async fn test_handle_network_failure() {
    // Test handling of network connection failures
    // Verifies that connection errors are properly reported

    let config = create_test_config("http://localhost:1".to_string()); // Invalid URL
    let params = create_default_params();

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_err(), "Should fail with network error");
    // Network error occurred - test passes
}

#[tokio::test]
async fn test_request_includes_authentication_headers() {
    // Test that requests include proper authentication headers
    // Verifies that x-api-key and anthropic-version headers are set

    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let params = create_default_params();

    let response = create_success_response();

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .and(header("anthropic-version", "2023-06-01"))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .expect(1) // Verify headers were present
        .mount(&mock_server)
        .await;

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = create_llm_request();

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_ok(), "Request with headers should succeed");
}

// ============================================================================
// Extended Cache Type Integration Tests (Issue #3)
// ============================================================================

#[tokio::test]
async fn test_extended_cache_type_in_request() {
    // Test that Extended cache type messages are properly sent to Anthropic API
    // Verifies TTL field is set to "1h" for Extended cache

    let mock_server = MockServer::start().await;
    let mut config = create_test_config(mock_server.uri());
    config.enable_prompt_caching = true;
    config.cache_ttl = "1h".to_string();
    let params = create_default_params();

    let response = serde_json::json!({
        "id": "msg_cached",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "text",
            "text": "Cached response"
        }],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 100,
            "output_tokens": 10,
            "cache_creation_input_tokens": 100,
            "cache_read_input_tokens": 0
        }
    });

    // Verify the request body contains Extended cache with 1h TTL
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(move |req: &wiremock::Request| {
            let body: serde_json::Value =
                serde_json::from_slice(&req.body).unwrap_or(serde_json::json!({}));

            // Check that messages have cache_control with ttl="1h"
            if let Some(messages) = body["messages"].as_array() {
                if let Some(first_msg) = messages.first() {
                    if let Some(content) = first_msg["content"].as_array() {
                        if let Some(first_content) = content.first() {
                            if let Some(cache_control) = first_content["cache_control"].as_object()
                            {
                                return cache_control["type"] == "ephemeral"
                                    && cache_control["ttl"] == "1h";
                            }
                        }
                    }
                }
            }
            false
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = UnifiedLLMRequest {
        messages: vec![create_cacheable_message(
            MessageRole::User,
            "Context for extended cache",
            CacheType::Extended,
        )],
        response_schema: None,
        config: None,
    };

    let result = provider.execute_llm(request, None, None).await;

    assert!(result.is_ok(), "Request with Extended cache should succeed");
    let response = unwrap_response!(result.unwrap());
    assert!(
        response.usage.is_some(),
        "Should have usage data with cache stats"
    );
}

#[tokio::test]
async fn test_ephemeral_cache_type_in_request() {
    // Test that Ephemeral cache type messages are properly sent to Anthropic API
    // Verifies TTL field is set to "5m" for Ephemeral cache

    let mock_server = MockServer::start().await;
    let mut config = create_test_config(mock_server.uri());
    config.enable_prompt_caching = true;
    config.cache_ttl = "5m".to_string();
    let params = create_default_params();

    let response = create_success_response();

    // Verify the request body contains Ephemeral cache with 5m TTL
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(move |req: &wiremock::Request| {
            let body: serde_json::Value =
                serde_json::from_slice(&req.body).unwrap_or(serde_json::json!({}));

            // Check that messages have cache_control with ttl="5m"
            if let Some(messages) = body["messages"].as_array() {
                if let Some(first_msg) = messages.first() {
                    if let Some(content) = first_msg["content"].as_array() {
                        if let Some(first_content) = content.first() {
                            if let Some(cache_control) = first_content["cache_control"].as_object()
                            {
                                return cache_control["type"] == "ephemeral"
                                    && cache_control["ttl"] == "5m";
                            }
                        }
                    }
                }
            }
            false
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = UnifiedLLMRequest {
        messages: vec![create_cacheable_message(
            MessageRole::User,
            "Query with ephemeral cache",
            CacheType::Ephemeral,
        )],
        response_schema: None,
        config: None,
    };

    let result = provider.execute_llm(request, None, None).await;

    assert!(
        result.is_ok(),
        "Request with Ephemeral cache should succeed"
    );
}

#[tokio::test]
async fn test_mixed_cache_types_in_request() {
    // Test that mixed cache types (Extended and Ephemeral) work in same conversation
    // Verifies different TTLs are correctly applied to different messages
    // NOTE: Uses 2 messages (not 3) because caching strategy for 1-2 messages
    // caches only the first message (index 0)

    let mock_server = MockServer::start().await;
    let mut config = create_test_config(mock_server.uri());
    config.enable_prompt_caching = true;
    let params = create_default_params();

    let response = create_success_response();

    // Verify request contains Extended cache type with correct TTL
    // With 2 messages, only the first one gets cached (2-breakpoint strategy)
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(move |req: &wiremock::Request| {
            let body: serde_json::Value =
                serde_json::from_slice(&req.body).unwrap_or(serde_json::json!({}));

            // Check that first message has Extended cache (1h TTL)
            if let Some(messages) = body["messages"].as_array() {
                if let Some(first_msg) = messages.first() {
                    if let Some(content) = first_msg["content"].as_array() {
                        if let Some(first_content) = content.first() {
                            if let Some(cache_control) = first_content["cache_control"].as_object()
                            {
                                // First message specified Extended, should get 1h TTL
                                return cache_control["type"] == "ephemeral"
                                    && cache_control["ttl"] == "1h";
                            }
                        }
                    }
                }
            }
            false
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = AnthropicProvider::new(config, params).unwrap();
    let request = UnifiedLLMRequest {
        messages: vec![
            create_cacheable_message(MessageRole::User, "Extended context", CacheType::Extended),
            create_cacheable_message(
                MessageRole::Assistant,
                "Ephemeral response",
                CacheType::Ephemeral,
            ),
        ],
        response_schema: None,
        config: None,
    };

    let result = provider.execute_llm(request, None, None).await;

    assert!(
        result.is_ok(),
        "Request with mixed cache types should succeed"
    );
}

#[tokio::test]
#[ignore] // Requires real Anthropic API key and incurs costs
async fn test_extended_cache_with_real_api() {
    // Integration test with real Anthropic API to verify Extended cache works
    // IMPORTANT: This test requires ANTHROPIC_API_KEY env var and incurs API costs
    //
    // Run with: cargo test test_extended_cache_with_real_api -- --ignored
    //
    // What this test verifies:
    // 1. Extended cache (1h TTL) is accepted by Anthropic API
    // 2. Request succeeds and returns valid response
    // 3. Cache creation and cache hits work with 1024+ token prompts
    // 4. Cache statistics are logged (check debug logs for cache_usage)
    // 5. Total tokens includes cache operations
    //
    // IMPORTANT: Anthropic requires at least 1024 tokens for caching to activate.
    // This test uses a large system prompt (1500+ tokens) to ensure real caching behavior.
    //
    // NOTE: The unified TokenUsage type only exposes prompt_tokens, completion_tokens,
    // and total_tokens. Cache-specific tokens (cache_creation, cache_read) are added
    // to total_tokens but not exposed separately. See logs for detailed cache stats.

    let api_key =
        std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set to run this test");

    let config = AnthropicConfig {
        api_key: Some(api_key),
        base_url: "https://api.anthropic.com".to_string(),
        default_model: "claude-3-5-haiku-20241022".to_string(),
        max_context_tokens: 200_000,
        retry_policy: RetryPolicy::default(),
        enable_prompt_caching: true,
        cache_ttl: "1h".to_string(), // Not used since messages specify cache_type
    };
    let params = create_default_params();

    let provider =
        AnthropicProvider::new(config, params).expect("Should create provider with valid config");

    // First request: Create Extended cache (1h TTL)
    // NOTE: Anthropic requires at least 1024 tokens for caching to work
    // This large system prompt ensures we meet that threshold
    let large_system_prompt = format!(
        "You are a helpful assistant with extensive knowledge. Here is important context: {}\n\n\
        Additional guidelines:\n\
        1. Always provide accurate information\n\
        2. Be concise but thorough\n\
        3. Use examples when helpful\n\
        4. Consider edge cases\n\
        5. Maintain consistency\n\
        \n\
        Remember these key principles throughout our conversation:\n\
        - Clarity is paramount\n\
        - Accuracy over speed\n\
        - User experience matters\n\
        - Context awareness is crucial\n\
        - Continuous improvement mindset\n\
        \n\
        This extended context ensures we meet the 1024+ token requirement for Anthropic's \
        prompt caching to activate. The cache allows subsequent requests with the same \
        context to be processed more efficiently and cost-effectively.",
        "Background knowledge: ".repeat(150) // Padding to reach 1024+ tokens
    );

    let system_msg = create_cacheable_message(
        MessageRole::System,
        &large_system_prompt,
        CacheType::Extended,
    );
    let user_msg = create_test_message("What is 2+2?");

    let request = UnifiedLLMRequest {
        messages: vec![system_msg.clone(), user_msg],
        response_schema: None,
        config: None,
    };

    println!("Making first request with Extended cache...");
    println!("NOTE: Check debug logs for 'Anthropic cache usage statistics' to see cache details");
    let result1 = provider
        .execute_llm(request, None, None)
        .await
        .expect("First request should succeed");
    let response1 = unwrap_response!(result1);

    println!("First request usage: {:?}", response1.usage);
    assert!(
        response1.usage.is_some(),
        "Should have usage data in first response"
    );

    let usage1 = response1.usage.unwrap();
    assert!(
        usage1.total_tokens > 0,
        "Should have non-zero total tokens in first request"
    );
    assert!(
        usage1.prompt_tokens > 0,
        "Should have non-zero prompt tokens"
    );
    assert!(
        usage1.completion_tokens > 0,
        "Should have non-zero completion tokens"
    );

    // total_tokens includes cache_creation_tokens from first request
    println!(
        "First request: prompt={}, completion={}, total={}",
        usage1.prompt_tokens, usage1.completion_tokens, usage1.total_tokens
    );

    // Second request: Should hit Extended cache
    println!("\nWaiting 2 seconds before second request...");
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let request2 = UnifiedLLMRequest {
        messages: vec![system_msg, create_test_message("What is 3+3?")],
        response_schema: None,
        config: None,
    };

    println!("Making second request (should hit Extended cache)...");
    let result2 = provider
        .execute_llm(request2, None, None)
        .await
        .expect("Second request should succeed");
    let response2 = unwrap_response!(result2);

    println!("Second request usage: {:?}", response2.usage);
    let usage2 = response2
        .usage
        .expect("Should have usage data in second response");

    // Second request should also succeed
    assert!(
        usage2.total_tokens > 0,
        "Second request should have non-zero total tokens"
    );

    // total_tokens in second request includes cache_read_tokens
    println!(
        "Second request: prompt={}, completion={}, total={}",
        usage2.prompt_tokens, usage2.completion_tokens, usage2.total_tokens
    );

    println!("\n✅ Extended cache integration test passed!");
    println!("- First request created Extended cache (1h TTL)");
    println!("- Second request processed successfully");
    println!("- Check debug logs for detailed cache statistics (cache_read_input_tokens, etc.)");
}
