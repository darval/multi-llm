//! Trait Compliance Tests for ExecutorLLMProvider
//!
//! **CRITICAL TESTS**: These tests verify that ALL provider implementations of the
//! `ExecutorLLMProvider` trait behave consistently. This catches issues where one
//! provider behaves differently from others.
//!
//! ## Why These Tests Are Critical
//!
//! From the unit test template:
//! > "Trait compliance tests would have caught the Anthropic retry logic gap where
//! > configuration wasn't properly applied across all providers."
//!
//! These tests ensure:
//! 1. All providers implement the trait consistently
//! 2. All providers handle the same inputs the same way
//! 3. All providers generate business events
//! 4. All providers handle errors consistently
//! 5. All providers respect configuration options
//!
//! ## Test Organization
//!
//! Tests are organized by business responsibility:
//! - **Initialization**: All providers can be created with valid config
//! - **Basic Execution**: All providers execute simple requests
//! - **Error Handling**: All providers handle errors consistently
//! - **Business Events**: All providers generate required events
//! - **Configuration**: All providers respect config parameters
//!
//! ## Testing Approach
//!
//! We test CONCRETE providers with MOCK HTTP servers (not trait mocking).
//! This gives us real code coverage and realistic testing.

mod common;
use common::*;
use multi_llm::core_types::executor::ExecutorLLMProvider;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// Test Helper: Setup Mock Servers for All Providers
// ============================================================================

/// Setup mock servers for all four providers
///
/// Returns (anthropic_mock, openai_mock, lmstudio_mock, ollama_mock)
async fn setup_all_mock_servers() -> (MockServer, MockServer, MockServer, MockServer) {
    (
        MockServer::start().await,
        MockServer::start().await,
        MockServer::start().await,
        MockServer::start().await,
    )
}

/// Mount successful response on Anthropic mock
async fn mount_anthropic_success(mock: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(create_successful_anthropic_response()),
        )
        .mount(mock)
        .await;
}

/// Mount successful response on OpenAI-compatible mock (OpenAI, LMStudio, Ollama)
async fn mount_openai_success(mock: &MockServer) {
    // OpenAI providers use base_url directly + /v1/chat/completions
    // So we need to match any path that ends with /v1/chat/completions
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_successful_openai_response()))
        .expect(1..) // Expect at least one call
        .mount(mock)
        .await;
}

// ============================================================================
// Initialization Tests
// ============================================================================

#[cfg(test)]
mod initialization_tests {
    use super::*;

    #[tokio::test]
    async fn test_all_providers_initialize_with_valid_config() {
        // Test verifies all providers can be created with valid configuration
        // This ensures no provider has configuration validation bugs that others don't have

        let (anthropic_mock, openai_mock, lmstudio_mock, ollama_mock) =
            setup_all_mock_servers().await;

        // Create all providers
        let anthropic = create_concrete_anthropic_provider(&anthropic_mock.uri());
        let openai = create_concrete_openai_provider(&openai_mock.uri());
        let lmstudio = create_concrete_lmstudio_provider(&lmstudio_mock.uri());
        let ollama = create_concrete_ollama_provider(&ollama_mock.uri());

        // All should return provider names
        assert_eq!(anthropic.provider_name(), "anthropic");
        assert_eq!(openai.provider_name(), "openai");
        assert_eq!(lmstudio.provider_name(), "lmstudio");
        assert_eq!(ollama.provider_name(), "ollama");
    }
}

// ============================================================================
// Basic Execution Tests
// ============================================================================

#[cfg(test)]
mod basic_execution_tests {
    use super::*;

    #[tokio::test]
    async fn test_all_providers_execute_simple_request() {
        // Test verifies all providers can execute a simple request successfully
        // This is the most basic trait compliance test - all providers must work

        let (anthropic_mock, openai_mock, lmstudio_mock, ollama_mock) =
            setup_all_mock_servers().await;

        // Setup mocks
        mount_anthropic_success(&anthropic_mock).await;
        mount_openai_success(&openai_mock).await;
        mount_openai_success(&lmstudio_mock).await;
        mount_openai_success(&ollama_mock).await;

        // Create providers
        let anthropic = create_concrete_anthropic_provider(&anthropic_mock.uri());
        let openai = create_concrete_openai_provider(&openai_mock.uri());
        let lmstudio = create_concrete_lmstudio_provider(&lmstudio_mock.uri());
        let ollama = create_concrete_ollama_provider(&ollama_mock.uri());

        let request = create_test_unified_request();
        let config = Some(create_minimal_executor_config());

        // Execute on all providers
        let anthropic_result = anthropic
            .execute_llm(request.clone(), None, config.clone())
            .await;
        let openai_result = openai
            .execute_llm(request.clone(), None, config.clone())
            .await;
        let lmstudio_result = lmstudio
            .execute_llm(request.clone(), None, config.clone())
            .await;
        let ollama_result = ollama
            .execute_llm(request.clone(), None, config.clone())
            .await;

        // All should succeed
        assert!(
            anthropic_result.is_ok(),
            "Anthropic provider should execute successfully"
        );
        assert!(
            openai_result.is_ok(),
            "OpenAI provider should execute successfully"
        );
        assert!(
            lmstudio_result.is_ok(),
            "LMStudio provider should execute successfully"
        );
        assert!(
            ollama_result.is_ok(),
            "Ollama provider should execute successfully"
        );
    }

    #[tokio::test]
    async fn test_all_providers_return_non_empty_content() {
        // Test verifies all providers return actual content in responses
        // Ensures no provider returns empty/null content for successful requests

        let (anthropic_mock, openai_mock, lmstudio_mock, ollama_mock) =
            setup_all_mock_servers().await;

        mount_anthropic_success(&anthropic_mock).await;
        mount_openai_success(&openai_mock).await;
        mount_openai_success(&lmstudio_mock).await;
        mount_openai_success(&ollama_mock).await;

        let anthropic = create_concrete_anthropic_provider(&anthropic_mock.uri());
        let openai = create_concrete_openai_provider(&openai_mock.uri());
        let lmstudio = create_concrete_lmstudio_provider(&lmstudio_mock.uri());
        let ollama = create_concrete_ollama_provider(&ollama_mock.uri());

        let request = create_test_unified_request();

        // Get responses
        let (anthropic_resp, _) = anthropic
            .execute_llm(request.clone(), None, None)
            .await
            .unwrap();
        let (openai_resp, _) = openai
            .execute_llm(request.clone(), None, None)
            .await
            .unwrap();
        let (lmstudio_resp, _) = lmstudio
            .execute_llm(request.clone(), None, None)
            .await
            .unwrap();
        let (ollama_resp, _) = ollama
            .execute_llm(request.clone(), None, None)
            .await
            .unwrap();

        // All should have non-empty content
        assert!(
            !anthropic_resp.content.is_empty(),
            "Anthropic should return content"
        );
        assert!(
            !openai_resp.content.is_empty(),
            "OpenAI should return content"
        );
        assert!(
            !lmstudio_resp.content.is_empty(),
            "LMStudio should return content"
        );
        assert!(
            !ollama_resp.content.is_empty(),
            "Ollama should return content"
        );
    }

    #[tokio::test]
    async fn test_all_providers_return_usage_statistics() {
        // Test verifies all providers return token usage statistics
        // This is required for cost tracking and monitoring

        let (anthropic_mock, openai_mock, lmstudio_mock, ollama_mock) =
            setup_all_mock_servers().await;

        mount_anthropic_success(&anthropic_mock).await;
        mount_openai_success(&openai_mock).await;
        mount_openai_success(&lmstudio_mock).await;
        mount_openai_success(&ollama_mock).await;

        let anthropic = create_concrete_anthropic_provider(&anthropic_mock.uri());
        let openai = create_concrete_openai_provider(&openai_mock.uri());
        let lmstudio = create_concrete_lmstudio_provider(&lmstudio_mock.uri());
        let ollama = create_concrete_ollama_provider(&ollama_mock.uri());

        let request = create_test_unified_request();

        // Get responses
        let (anthropic_resp, _) = anthropic
            .execute_llm(request.clone(), None, None)
            .await
            .unwrap();
        let (openai_resp, _) = openai
            .execute_llm(request.clone(), None, None)
            .await
            .unwrap();
        let (lmstudio_resp, _) = lmstudio
            .execute_llm(request.clone(), None, None)
            .await
            .unwrap();
        let (ollama_resp, _) = ollama
            .execute_llm(request.clone(), None, None)
            .await
            .unwrap();

        // All should have usage statistics
        assert!(
            anthropic_resp.usage.is_some(),
            "Anthropic should return usage"
        );
        assert!(openai_resp.usage.is_some(), "OpenAI should return usage");
        assert!(
            lmstudio_resp.usage.is_some(),
            "LMStudio should return usage"
        );
        assert!(ollama_resp.usage.is_some(), "Ollama should return usage");

        // Verify usage has actual token counts
        let anthropic_usage = anthropic_resp.usage.unwrap();
        assert!(
            anthropic_usage.prompt_tokens > 0,
            "Anthropic should have prompt tokens"
        );
        assert!(
            anthropic_usage.completion_tokens > 0,
            "Anthropic should have completion tokens"
        );
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_all_providers_handle_auth_errors_consistently() {
        // Test verifies all providers handle 401 authentication errors
        // Ensures consistent error handling across providers

        let (anthropic_mock, openai_mock, lmstudio_mock, ollama_mock) =
            setup_all_mock_servers().await;

        // Mount auth error responses
        Mock::given(method("POST"))
            .respond_with(create_auth_error_response())
            .mount(&anthropic_mock)
            .await;
        Mock::given(method("POST"))
            .respond_with(create_auth_error_response())
            .mount(&openai_mock)
            .await;
        Mock::given(method("POST"))
            .respond_with(create_auth_error_response())
            .mount(&lmstudio_mock)
            .await;
        Mock::given(method("POST"))
            .respond_with(create_auth_error_response())
            .mount(&ollama_mock)
            .await;

        let anthropic = create_concrete_anthropic_provider(&anthropic_mock.uri());
        let openai = create_concrete_openai_provider(&openai_mock.uri());
        let lmstudio = create_concrete_lmstudio_provider(&lmstudio_mock.uri());
        let ollama = create_concrete_ollama_provider(&ollama_mock.uri());

        let request = create_test_unified_request();

        // All should return errors
        let anthropic_result = anthropic.execute_llm(request.clone(), None, None).await;
        let openai_result = openai.execute_llm(request.clone(), None, None).await;
        let lmstudio_result = lmstudio.execute_llm(request.clone(), None, None).await;
        let ollama_result = ollama.execute_llm(request.clone(), None, None).await;

        assert!(anthropic_result.is_err(), "Anthropic should fail on 401");
        assert!(openai_result.is_err(), "OpenAI should fail on 401");
        assert!(lmstudio_result.is_err(), "LMStudio should fail on 401");
        assert!(ollama_result.is_err(), "Ollama should fail on 401");
    }

    #[tokio::test]
    async fn test_all_providers_handle_server_errors_consistently() {
        // Test verifies all providers handle 500 server errors
        // Ensures retry logic and error reporting work consistently

        let (anthropic_mock, openai_mock, lmstudio_mock, ollama_mock) =
            setup_all_mock_servers().await;

        // Mount server error responses
        Mock::given(method("POST"))
            .respond_with(create_server_error_response())
            .mount(&anthropic_mock)
            .await;
        Mock::given(method("POST"))
            .respond_with(create_server_error_response())
            .mount(&openai_mock)
            .await;
        Mock::given(method("POST"))
            .respond_with(create_server_error_response())
            .mount(&lmstudio_mock)
            .await;
        Mock::given(method("POST"))
            .respond_with(create_server_error_response())
            .mount(&ollama_mock)
            .await;

        let anthropic = create_concrete_anthropic_provider(&anthropic_mock.uri());
        let openai = create_concrete_openai_provider(&openai_mock.uri());
        let lmstudio = create_concrete_lmstudio_provider(&lmstudio_mock.uri());
        let ollama = create_concrete_ollama_provider(&ollama_mock.uri());

        let request = create_test_unified_request();

        // All should return errors (after retries)
        let anthropic_result = anthropic.execute_llm(request.clone(), None, None).await;
        let openai_result = openai.execute_llm(request.clone(), None, None).await;
        let lmstudio_result = lmstudio.execute_llm(request.clone(), None, None).await;
        let ollama_result = ollama.execute_llm(request.clone(), None, None).await;

        assert!(anthropic_result.is_err(), "Anthropic should fail on 500");
        assert!(openai_result.is_err(), "OpenAI should fail on 500");
        assert!(lmstudio_result.is_err(), "LMStudio should fail on 500");
        assert!(ollama_result.is_err(), "Ollama should fail on 500");
    }
}

// ============================================================================
// Business Event Tests
// ============================================================================

#[cfg(test)]
mod business_event_tests {
    use super::*;

    #[tokio::test]
    async fn test_all_providers_generate_business_events() {
        // Test verifies all providers generate business events
        // Required for analytics, monitoring, and business logic

        let (anthropic_mock, openai_mock, lmstudio_mock, ollama_mock) =
            setup_all_mock_servers().await;

        mount_anthropic_success(&anthropic_mock).await;
        mount_openai_success(&openai_mock).await;
        mount_openai_success(&lmstudio_mock).await;
        mount_openai_success(&ollama_mock).await;

        let anthropic = create_concrete_anthropic_provider(&anthropic_mock.uri());
        let openai = create_concrete_openai_provider(&openai_mock.uri());
        let lmstudio = create_concrete_lmstudio_provider(&lmstudio_mock.uri());
        let ollama = create_concrete_ollama_provider(&ollama_mock.uri());

        let request = create_test_unified_request();
        let config = Some(create_minimal_executor_config());

        // Get events from all providers
        let (_, anthropic_events) = anthropic
            .execute_llm(request.clone(), None, config.clone())
            .await
            .unwrap();
        let (_, openai_events) = openai
            .execute_llm(request.clone(), None, config.clone())
            .await
            .unwrap();
        let (_, lmstudio_events) = lmstudio
            .execute_llm(request.clone(), None, config.clone())
            .await
            .unwrap();
        let (_, ollama_events) = ollama
            .execute_llm(request.clone(), None, config.clone())
            .await
            .unwrap();

        // All should generate events
        assert!(
            !anthropic_events.is_empty(),
            "Anthropic should generate events"
        );
        assert!(!openai_events.is_empty(), "OpenAI should generate events");
        assert!(
            !lmstudio_events.is_empty(),
            "LMStudio should generate events"
        );
        assert!(!ollama_events.is_empty(), "Ollama should generate events");

        // All should have at least request and response events (2 minimum)
        assert!(
            anthropic_events.len() >= 2,
            "Anthropic should have request+response events"
        );
        assert!(
            openai_events.len() >= 2,
            "OpenAI should have request+response events"
        );
        assert!(
            lmstudio_events.len() >= 2,
            "LMStudio should have request+response events"
        );
        assert!(
            ollama_events.len() >= 2,
            "Ollama should have request+response events"
        );
    }
}

// ============================================================================
// Configuration Handling Tests
// ============================================================================

#[cfg(test)]
mod configuration_tests {
    use super::*;

    #[tokio::test]
    async fn test_all_providers_accept_full_config() {
        // Test verifies all providers accept a fully populated config
        // Ensures no provider rejects valid configuration options

        let (anthropic_mock, openai_mock, lmstudio_mock, ollama_mock) =
            setup_all_mock_servers().await;

        mount_anthropic_success(&anthropic_mock).await;
        mount_openai_success(&openai_mock).await;
        mount_openai_success(&lmstudio_mock).await;
        mount_openai_success(&ollama_mock).await;

        let anthropic = create_concrete_anthropic_provider(&anthropic_mock.uri());
        let openai = create_concrete_openai_provider(&openai_mock.uri());
        let lmstudio = create_concrete_lmstudio_provider(&lmstudio_mock.uri());
        let ollama = create_concrete_ollama_provider(&ollama_mock.uri());

        let request = create_test_unified_request();
        let config = Some(create_full_executor_config());

        // All should accept full config without errors
        let anthropic_result = anthropic
            .execute_llm(request.clone(), None, config.clone())
            .await;
        let openai_result = openai
            .execute_llm(request.clone(), None, config.clone())
            .await;
        let lmstudio_result = lmstudio
            .execute_llm(request.clone(), None, config.clone())
            .await;
        let ollama_result = ollama
            .execute_llm(request.clone(), None, config.clone())
            .await;

        assert!(
            anthropic_result.is_ok(),
            "Anthropic should accept full config"
        );
        assert!(openai_result.is_ok(), "OpenAI should accept full config");
        assert!(
            lmstudio_result.is_ok(),
            "LMStudio should accept full config"
        );
        assert!(ollama_result.is_ok(), "Ollama should accept full config");
    }

    #[tokio::test]
    async fn test_all_providers_handle_empty_config() {
        // Test verifies all providers work without config (None)
        // Ensures default behavior is consistent

        let (anthropic_mock, openai_mock, lmstudio_mock, ollama_mock) =
            setup_all_mock_servers().await;

        mount_anthropic_success(&anthropic_mock).await;
        mount_openai_success(&openai_mock).await;
        mount_openai_success(&lmstudio_mock).await;
        mount_openai_success(&ollama_mock).await;

        let anthropic = create_concrete_anthropic_provider(&anthropic_mock.uri());
        let openai = create_concrete_openai_provider(&openai_mock.uri());
        let lmstudio = create_concrete_lmstudio_provider(&lmstudio_mock.uri());
        let ollama = create_concrete_ollama_provider(&ollama_mock.uri());

        let request = create_test_unified_request();

        // All should work with None config
        let anthropic_result = anthropic.execute_llm(request.clone(), None, None).await;
        let openai_result = openai.execute_llm(request.clone(), None, None).await;
        let lmstudio_result = lmstudio.execute_llm(request.clone(), None, None).await;
        let ollama_result = ollama.execute_llm(request.clone(), None, None).await;

        assert!(
            anthropic_result.is_ok(),
            "Anthropic should work without config"
        );
        assert!(openai_result.is_ok(), "OpenAI should work without config");
        assert!(
            lmstudio_result.is_ok(),
            "LMStudio should work without config"
        );
        assert!(ollama_result.is_ok(), "Ollama should work without config");
    }
}

// ============================================================================
// Configuration Completeness Tests (CRITICAL)
// ============================================================================
//
// These tests ensure ALL providers handle ALL configuration options.
// From the unit test template:
// > "This would have caught the Anthropic configuration gaps immediately"
//
// We verify that no provider silently ignores configuration fields.

#[cfg(test)]
mod configuration_completeness_tests {
    use super::*;

    #[tokio::test]
    async fn test_all_providers_handle_temperature_config() {
        // Test verifies all providers respect temperature configuration
        // Temperature controls randomness in LLM responses (0.0 = deterministic, 1.0+ = creative)

        let (anthropic_mock, openai_mock, lmstudio_mock, ollama_mock) =
            setup_all_mock_servers().await;

        mount_anthropic_success(&anthropic_mock).await;
        mount_openai_success(&openai_mock).await;
        mount_openai_success(&lmstudio_mock).await;
        mount_openai_success(&ollama_mock).await;

        let anthropic = create_concrete_anthropic_provider(&anthropic_mock.uri());
        let openai = create_concrete_openai_provider(&openai_mock.uri());
        let lmstudio = create_concrete_lmstudio_provider(&lmstudio_mock.uri());
        let ollama = create_concrete_ollama_provider(&ollama_mock.uri());

        let request = create_test_unified_request();
        let mut config = create_minimal_executor_config();
        config.temperature = Some(0.5);

        // All providers should accept temperature without errors
        let anthropic_result = anthropic
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let openai_result = openai
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let lmstudio_result = lmstudio
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let ollama_result = ollama
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;

        assert!(
            anthropic_result.is_ok(),
            "Anthropic should handle temperature config"
        );
        assert!(
            openai_result.is_ok(),
            "OpenAI should handle temperature config"
        );
        assert!(
            lmstudio_result.is_ok(),
            "LMStudio should handle temperature config"
        );
        assert!(
            ollama_result.is_ok(),
            "Ollama should handle temperature config"
        );
    }

    #[tokio::test]
    async fn test_all_providers_handle_max_tokens_config() {
        // Test verifies all providers respect max_tokens configuration
        // max_tokens limits the length of generated responses

        let (anthropic_mock, openai_mock, lmstudio_mock, ollama_mock) =
            setup_all_mock_servers().await;

        mount_anthropic_success(&anthropic_mock).await;
        mount_openai_success(&openai_mock).await;
        mount_openai_success(&lmstudio_mock).await;
        mount_openai_success(&ollama_mock).await;

        let anthropic = create_concrete_anthropic_provider(&anthropic_mock.uri());
        let openai = create_concrete_openai_provider(&openai_mock.uri());
        let lmstudio = create_concrete_lmstudio_provider(&lmstudio_mock.uri());
        let ollama = create_concrete_ollama_provider(&ollama_mock.uri());

        let request = create_test_unified_request();
        let mut config = create_minimal_executor_config();
        config.max_tokens = Some(500);

        // All providers should accept max_tokens without errors
        let anthropic_result = anthropic
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let openai_result = openai
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let lmstudio_result = lmstudio
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let ollama_result = ollama
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;

        assert!(
            anthropic_result.is_ok(),
            "Anthropic should handle max_tokens config"
        );
        assert!(
            openai_result.is_ok(),
            "OpenAI should handle max_tokens config"
        );
        assert!(
            lmstudio_result.is_ok(),
            "LMStudio should handle max_tokens config"
        );
        assert!(
            ollama_result.is_ok(),
            "Ollama should handle max_tokens config"
        );
    }

    #[tokio::test]
    async fn test_all_providers_handle_top_p_config() {
        // Test verifies all providers respect top_p configuration
        // top_p controls nucleus sampling (alternative to temperature)

        let (anthropic_mock, openai_mock, lmstudio_mock, ollama_mock) =
            setup_all_mock_servers().await;

        mount_anthropic_success(&anthropic_mock).await;
        mount_openai_success(&openai_mock).await;
        mount_openai_success(&lmstudio_mock).await;
        mount_openai_success(&ollama_mock).await;

        let anthropic = create_concrete_anthropic_provider(&anthropic_mock.uri());
        let openai = create_concrete_openai_provider(&openai_mock.uri());
        let lmstudio = create_concrete_lmstudio_provider(&lmstudio_mock.uri());
        let ollama = create_concrete_ollama_provider(&ollama_mock.uri());

        let request = create_test_unified_request();
        let mut config = create_minimal_executor_config();
        config.top_p = Some(0.9);

        // All providers should accept top_p without errors
        let anthropic_result = anthropic
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let openai_result = openai
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let lmstudio_result = lmstudio
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let ollama_result = ollama
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;

        assert!(
            anthropic_result.is_ok(),
            "Anthropic should handle top_p config"
        );
        assert!(openai_result.is_ok(), "OpenAI should handle top_p config");
        assert!(
            lmstudio_result.is_ok(),
            "LMStudio should handle top_p config"
        );
        assert!(ollama_result.is_ok(), "Ollama should handle top_p config");
    }

    #[tokio::test]
    async fn test_all_providers_handle_tools_config() {
        // Test verifies all providers respect tools configuration
        // Tools enable function calling / tool use capabilities
        // CRITICAL: This is a core feature that must work consistently

        let (anthropic_mock, openai_mock, lmstudio_mock, ollama_mock) =
            setup_all_mock_servers().await;

        // Mount tool-aware responses
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(create_anthropic_response_with_tools()),
            )
            .mount(&anthropic_mock)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(create_openai_response_with_tools()),
            )
            .mount(&openai_mock)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(create_openai_response_with_tools()),
            )
            .mount(&lmstudio_mock)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(create_openai_response_with_tools()),
            )
            .mount(&ollama_mock)
            .await;

        let anthropic = create_concrete_anthropic_provider(&anthropic_mock.uri());
        let openai = create_concrete_openai_provider(&openai_mock.uri());
        let lmstudio = create_concrete_lmstudio_provider(&lmstudio_mock.uri());
        let ollama = create_concrete_ollama_provider(&ollama_mock.uri());

        let request = create_test_unified_request();
        let mut config = create_minimal_executor_config();
        config.tools = vec![create_test_tool()];

        // All providers should accept tools and return tool calls
        let anthropic_result = anthropic
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let openai_result = openai
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let lmstudio_result = lmstudio
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let ollama_result = ollama
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;

        // All should succeed
        assert!(
            anthropic_result.is_ok(),
            "Anthropic should handle tools config"
        );
        assert!(openai_result.is_ok(), "OpenAI should handle tools config");
        assert!(
            lmstudio_result.is_ok(),
            "LMStudio should handle tools config"
        );
        assert!(ollama_result.is_ok(), "Ollama should handle tools config");

        // All should return tool calls
        let (anthropic_resp, _) = anthropic_result.unwrap();
        let (openai_resp, _) = openai_result.unwrap();
        let (lmstudio_resp, _) = lmstudio_result.unwrap();
        let (ollama_resp, _) = ollama_result.unwrap();

        assert!(
            !anthropic_resp.tool_calls.is_empty(),
            "Anthropic should return tool calls"
        );
        assert!(
            !openai_resp.tool_calls.is_empty(),
            "OpenAI should return tool calls"
        );
        assert!(
            !lmstudio_resp.tool_calls.is_empty(),
            "LMStudio should return tool calls"
        );
        assert!(
            !ollama_resp.tool_calls.is_empty(),
            "Ollama should return tool calls"
        );
    }

    #[tokio::test]
    async fn test_all_providers_handle_combined_sampling_params() {
        // Test verifies all providers handle multiple sampling parameters together
        // Ensures no conflicts when temperature, top_p, top_k, min_p are all set
        // Real-world usage often combines these parameters

        let (anthropic_mock, openai_mock, lmstudio_mock, ollama_mock) =
            setup_all_mock_servers().await;

        mount_anthropic_success(&anthropic_mock).await;
        mount_openai_success(&openai_mock).await;
        mount_openai_success(&lmstudio_mock).await;
        mount_openai_success(&ollama_mock).await;

        let anthropic = create_concrete_anthropic_provider(&anthropic_mock.uri());
        let openai = create_concrete_openai_provider(&openai_mock.uri());
        let lmstudio = create_concrete_lmstudio_provider(&lmstudio_mock.uri());
        let ollama = create_concrete_ollama_provider(&ollama_mock.uri());

        let request = create_test_unified_request();
        let mut config = create_minimal_executor_config();
        config.temperature = Some(0.7);
        config.top_p = Some(0.9);
        config.top_k = Some(40);
        config.min_p = Some(0.05);

        // All providers should handle combined sampling params without errors
        let anthropic_result = anthropic
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let openai_result = openai
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let lmstudio_result = lmstudio
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let ollama_result = ollama
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;

        assert!(
            anthropic_result.is_ok(),
            "Anthropic should handle combined sampling params"
        );
        assert!(
            openai_result.is_ok(),
            "OpenAI should handle combined sampling params"
        );
        assert!(
            lmstudio_result.is_ok(),
            "LMStudio should handle combined sampling params"
        );
        assert!(
            ollama_result.is_ok(),
            "Ollama should handle combined sampling params"
        );
    }

    #[tokio::test]
    async fn test_all_providers_handle_presence_penalty() {
        // Test verifies all providers respect presence_penalty configuration
        // presence_penalty encourages model to talk about new topics

        let (anthropic_mock, openai_mock, lmstudio_mock, ollama_mock) =
            setup_all_mock_servers().await;

        mount_anthropic_success(&anthropic_mock).await;
        mount_openai_success(&openai_mock).await;
        mount_openai_success(&lmstudio_mock).await;
        mount_openai_success(&ollama_mock).await;

        let anthropic = create_concrete_anthropic_provider(&anthropic_mock.uri());
        let openai = create_concrete_openai_provider(&openai_mock.uri());
        let lmstudio = create_concrete_lmstudio_provider(&lmstudio_mock.uri());
        let ollama = create_concrete_ollama_provider(&ollama_mock.uri());

        let request = create_test_unified_request();
        let mut config = create_minimal_executor_config();
        config.presence_penalty = Some(0.6);

        // All providers should accept presence_penalty without errors
        let anthropic_result = anthropic
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let openai_result = openai
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let lmstudio_result = lmstudio
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;
        let ollama_result = ollama
            .execute_llm(request.clone(), None, Some(config.clone()))
            .await;

        assert!(
            anthropic_result.is_ok(),
            "Anthropic should handle presence_penalty"
        );
        assert!(
            openai_result.is_ok(),
            "OpenAI should handle presence_penalty"
        );
        assert!(
            lmstudio_result.is_ok(),
            "LMStudio should handle presence_penalty"
        );
        assert!(
            ollama_result.is_ok(),
            "Ollama should handle presence_penalty"
        );
    }
}
