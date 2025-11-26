//! Unit Tests for Anthropic Provider Pure Functions
//!
//! UNIT UNDER TEST: Pure utility functions in AnthropicProvider
//!
//! BUSINESS RESPONSIBILITY:
//!   - Parse authentication errors from Anthropic API responses
//!   - Extract and validate JSON from response content
//!   - Determine caching eligibility based on request config
//!   - Parse structured JSON responses
//!
//! TEST COVERAGE:
//!   - Authentication error detection and parsing
//!   - JSON extraction from content with trailing text
//!   - JSON brace matching for nested structures
//!   - Caching configuration logic
//!   - Structured response parsing

use super::super::provider::AnthropicProvider;
use crate::config::{AnthropicConfig, DefaultLLMParams};
use crate::provider::{RequestConfig, ResponseFormat, ToolCall};
use serde_json::json;

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Create an AnthropicProvider with default config for testing
fn create_test_provider() -> AnthropicProvider {
    let config = AnthropicConfig {
        api_key: Some("test-api-key".to_string()),
        base_url: "https://api.anthropic.com".to_string(),
        default_model: "claude-3-5-sonnet-20241022".to_string(),
        max_context_tokens: 200000,
        retry_policy: Default::default(),
        enable_prompt_caching: true,
        cache_ttl: "5m".to_string(),
    };
    let params = DefaultLLMParams::default();
    AnthropicProvider::new(config, params).expect("Failed to create test provider")
}

/// Create an AnthropicProvider with caching disabled
fn create_test_provider_no_caching() -> AnthropicProvider {
    let config = AnthropicConfig {
        api_key: Some("test-api-key".to_string()),
        base_url: "https://api.anthropic.com".to_string(),
        default_model: "claude-3-5-sonnet-20241022".to_string(),
        max_context_tokens: 200000,
        retry_policy: Default::default(),
        enable_prompt_caching: false,
        cache_ttl: "5m".to_string(),
    };
    let params = DefaultLLMParams::default();
    AnthropicProvider::new(config, params).expect("Failed to create test provider")
}

// ============================================================================
// Provider Construction Tests
// ============================================================================

#[test]
fn test_new_with_valid_config() {
    // Test verifies provider creation with valid config
    let config = AnthropicConfig {
        api_key: Some("test-api-key".to_string()),
        base_url: "https://api.anthropic.com".to_string(),
        default_model: "claude-3-5-sonnet-20241022".to_string(),
        max_context_tokens: 200000,
        retry_policy: Default::default(),
        enable_prompt_caching: true,
        cache_ttl: "5m".to_string(),
    };
    let params = DefaultLLMParams::default();

    let result = AnthropicProvider::new(config, params);

    assert!(result.is_ok(), "Should create provider with valid config");
}

#[test]
fn test_new_fails_without_api_key() {
    // Test verifies provider creation fails without API key
    //
    // Business rule: API key is required for Anthropic
    let config = AnthropicConfig {
        api_key: None,
        base_url: "https://api.anthropic.com".to_string(),
        default_model: "claude-3-5-sonnet-20241022".to_string(),
        max_context_tokens: 200000,
        retry_policy: Default::default(),
        enable_prompt_caching: true,
        cache_ttl: "5m".to_string(),
    };
    let params = DefaultLLMParams::default();

    let result = AnthropicProvider::new(config, params);

    assert!(result.is_err(), "Should fail without API key");
    let error = result.unwrap_err();
    assert!(
        format!("{}", error).contains("API key"),
        "Error should mention API key"
    );
}

// ============================================================================
// Authentication Error Tests
// ============================================================================

#[test]
fn test_is_auth_error_with_authentication_type() {
    // Test verifies detection of authentication error type
    let error_json = json!({
        "error": {
            "type": "authentication_error",
            "message": "Invalid API key"
        }
    });

    assert!(
        AnthropicProvider::is_auth_error(&error_json),
        "Should detect authentication error"
    );
}

#[test]
fn test_is_auth_error_with_invalid_api_key_type() {
    // Test verifies detection of invalid_api_key error type
    let error_json = json!({
        "error": {
            "type": "invalid_api_key",
            "message": "The API key is invalid"
        }
    });

    assert!(
        AnthropicProvider::is_auth_error(&error_json),
        "Should detect invalid_api_key error"
    );
}

#[test]
fn test_is_auth_error_with_rate_limit_error() {
    // Test verifies rate limit errors are NOT auth errors
    let error_json = json!({
        "error": {
            "type": "rate_limit_error",
            "message": "Rate limit exceeded"
        }
    });

    assert!(
        !AnthropicProvider::is_auth_error(&error_json),
        "Rate limit should NOT be auth error"
    );
}

#[test]
fn test_is_auth_error_with_missing_error_field() {
    // Test verifies graceful handling of missing error field
    let error_json = json!({
        "message": "Some error"
    });

    assert!(
        !AnthropicProvider::is_auth_error(&error_json),
        "Missing error field should return false"
    );
}

#[test]
fn test_is_auth_error_with_missing_type_field() {
    // Test verifies graceful handling of missing type field
    let error_json = json!({
        "error": {
            "message": "Some error"
        }
    });

    assert!(
        !AnthropicProvider::is_auth_error(&error_json),
        "Missing type field should return false"
    );
}

#[test]
fn test_parse_auth_error_with_valid_json() {
    // Test verifies auth error parsing from valid JSON
    let error_text = r#"{"error": {"type": "authentication_error", "message": "Invalid key"}}"#;

    let error = AnthropicProvider::parse_auth_error(error_text);

    // Should return authentication error
    assert!(
        format!("{}", error)
            .to_lowercase()
            .contains("authentication")
            || format!("{}", error).to_lowercase().contains("auth"),
        "Should contain authentication in error message"
    );
}

#[test]
fn test_parse_auth_error_with_invalid_json() {
    // Test verifies fallback for invalid JSON
    let error_text = "not valid json";

    let error = AnthropicProvider::parse_auth_error(error_text);

    // Should return generic auth error
    assert!(
        format!("{}", error)
            .to_lowercase()
            .contains("authentication")
            || format!("{}", error).to_lowercase().contains("auth"),
        "Should still return authentication error"
    );
}

#[test]
fn test_parse_auth_error_with_non_auth_error() {
    // Test verifies fallback for non-auth errors
    let error_text = r#"{"error": {"type": "rate_limit", "message": "Too many requests"}}"#;

    let error = AnthropicProvider::parse_auth_error(error_text);

    // Should return generic auth error (function always returns auth error)
    assert!(
        format!("{}", error)
            .to_lowercase()
            .contains("authentication")
            || format!("{}", error).to_lowercase().contains("auth"),
        "Should return authentication error"
    );
}

// ============================================================================
// Caching Configuration Tests
// ============================================================================

#[test]
fn test_should_enable_caching_enabled_no_config() {
    // Test verifies caching enabled when provider has it enabled and no config override
    let provider = create_test_provider();

    let result = provider.should_enable_caching(None);

    assert!(result, "Should enable caching with no config override");
}

#[test]
fn test_should_enable_caching_disabled_in_provider() {
    // Test verifies caching disabled when provider has it disabled
    let provider = create_test_provider_no_caching();

    let result = provider.should_enable_caching(None);

    assert!(
        !result,
        "Should disable caching when provider config disables it"
    );
}

#[test]
fn test_should_enable_caching_disabled_for_nlp_llm() {
    // Test verifies caching disabled for nlp_llm path
    let provider = create_test_provider();
    let config = RequestConfig {
        llm_path: Some("nlp_llm".to_string()),
        ..Default::default()
    };

    let result = provider.should_enable_caching(Some(&config));

    assert!(!result, "Should disable caching for nlp_llm path");
}

#[test]
fn test_should_enable_caching_enabled_for_user_llm() {
    // Test verifies caching enabled for user_llm path
    let provider = create_test_provider();
    let config = RequestConfig {
        llm_path: Some("user_llm".to_string()),
        ..Default::default()
    };

    let result = provider.should_enable_caching(Some(&config));

    assert!(result, "Should enable caching for user_llm path");
}

// ============================================================================
// Structured Response Parsing Tests
// ============================================================================

#[test]
fn test_parse_structured_response_no_format() {
    // Test verifies passthrough when no response format specified
    let provider = create_test_provider();
    let content = "Just regular text".to_string();
    let tool_calls: Vec<ToolCall> = vec![];

    let (result_content, json_opt) =
        provider.parse_structured_response(content.clone(), &tool_calls, None);

    assert_eq!(result_content, content);
    assert!(json_opt.is_none(), "Should return None when no format");
}

#[test]
fn test_parse_structured_response_from_tool_call() {
    // Test verifies extraction from structured_response tool call
    let provider = create_test_provider();
    let content = "Some content".to_string();
    let tool_calls = vec![ToolCall {
        id: "tool_123".to_string(),
        name: "structured_response".to_string(),
        arguments: json!({"result": "extracted"}),
    }];
    let response_format = Some(ResponseFormat {
        name: "test".to_string(),
        schema: json!({}),
    });

    let (_, json_opt) = provider.parse_structured_response(content, &tool_calls, response_format);

    assert!(json_opt.is_some(), "Should extract from tool call");
    assert_eq!(json_opt.unwrap()["result"], "extracted");
}

#[test]
fn test_parse_structured_response_fallback_to_content() {
    // Test verifies fallback to parsing content as JSON
    let provider = create_test_provider();
    let content = r#"{"parsed": "from content"}"#.to_string();
    let tool_calls: Vec<ToolCall> = vec![];
    let response_format = Some(ResponseFormat {
        name: "test".to_string(),
        schema: json!({}),
    });

    let (_, json_opt) = provider.parse_structured_response(content, &tool_calls, response_format);

    assert!(json_opt.is_some(), "Should parse content as JSON");
    assert_eq!(json_opt.unwrap()["parsed"], "from content");
}

#[test]
fn test_parse_json_content_valid_json() {
    // Test verifies parsing valid JSON content
    let provider = create_test_provider();
    let content = r#"{"valid": true, "count": 42}"#.to_string();

    let (result_content, json_opt) = provider.parse_json_content(content);

    assert!(json_opt.is_some(), "Should parse valid JSON");
    assert_eq!(json_opt.unwrap()["valid"], true);
    assert!(result_content.starts_with('{'));
}

#[test]
fn test_parse_json_content_prepends_brace() {
    // Test verifies brace prepended when content doesn't start with {
    let provider = create_test_provider();
    let content = r#""key": "value"}"#.to_string();

    let (result_content, json_opt) = provider.parse_json_content(content);

    assert!(result_content.starts_with('{'), "Should prepend brace");
    assert!(json_opt.is_some(), "Should parse after prepending");
}

#[test]
fn test_parse_json_content_invalid_json() {
    // Test verifies graceful handling of invalid JSON
    let provider = create_test_provider();
    let content = "not json at all".to_string();

    let (result_content, json_opt) = provider.parse_json_content(content.clone());

    assert!(json_opt.is_none(), "Should return None for invalid JSON");
    assert_eq!(result_content, content, "Should return original content");
}
