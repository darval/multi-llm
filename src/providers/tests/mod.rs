//! Provider Tests Module
//!
//! This module contains comprehensive tests for all LLM provider implementations.
//! Tests follow the unit test template pattern and aim for 90%+ code coverage.
//!
//! ## Test Organization
//!
//! ### Trait Compliance Tests (`trait_compliance.rs`)
//! **CRITICAL**: These tests verify that ALL providers implement the ExecutorLLMProvider
//! trait consistently. This catches issues where one provider behaves differently from others.
//!
//! ### Configuration Tests (`configuration.rs`)
//! Tests that ALL providers handle ALL configuration options properly.
//! Ensures no provider silently ignores config fields.
//!
//! ### Provider-Specific Tests
//! Each provider has its own test module with 90%+ coverage:
//! - `anthropic/` - Anthropic provider tests
//! - `openai.rs` - OpenAI provider tests
//! - `lmstudio.rs` - LMStudio provider tests
//! - `ollama.rs` - Ollama provider tests
//! - `openai_shared/` - Shared OpenAI-compatible utilities tests
//!
//! ## Testing Strategy
//!
//! We use **wiremock for HTTP mocking** with **concrete provider instances**.
//! This gives us:
//! - Real code coverage (not trait mocks)
//! - Realistic testing of HTTP communication
//! - Real error handling paths tested
//! - Deterministic and fast tests
//!
//! ## Helper Functions
//!
//! Shared test utilities are in `crate::tests::helpers`:
//! - Provider creation: `create_concrete_*_provider(base_url)`
//! - Test data: `create_test_unified_request()`, `create_full_executor_config()`
//! - Mock responses: `create_successful_*_response()`, `create_*_error_response()`

// Re-export test helpers for convenience

// NOTE: HTTP integration tests are in tests/ directory
// These tests use MockServer and are slow, so they don't belong in unit tests:
// - trait_compliance.rs → tests/provider_trait_compliance_tests.rs
// - openai_provider.rs → tests/openai_provider_integration_tests.rs
// - lmstudio_provider.rs → tests/lmstudio_provider_integration_tests.rs
// - ollama_provider.rs → tests/ollama_provider_integration_tests.rs
