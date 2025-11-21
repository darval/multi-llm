//! Integration Test Suite for Complete LLM Client Flow
//!
//! Split into modules to maintain manageable file sizes
//! (following the <600 LOC requirement from module-refactoring-template.md).
//!
//! INTEGRATION UNDER TEST: Complete request flow from UnifiedLLMClient to HTTP mocks
//!
//! BUSINESS RESPONSIBILITY:
//!   - Validates end-to-end LLM request processing from client API to HTTP responses
//!   - Ensures proper error handling and retry behavior with realistic network conditions
//!   - Verifies token counting accuracy in real request/response scenarios
//!   - Tests provider switching and configuration management under load
//!   - Validates complete error propagation through all system layers
//!
//! MODULE BREAKDOWN:
//! - `helpers` - Common test utilities and mock response builders (95 LOC)
//! - `end_to_end_success_flow_tests` - Success path integration tests (140 LOC)
//! - `error_handling_integration_tests` - Error propagation and handling (85 LOC)
//! - `retry_logic_integration_tests` - Retry logic and circuit breaker tests (75 LOC)
//! - `token_management_integration_tests` - Token counting accuracy tests (85 LOC)
//! - `provider_switching_integration_tests` - Provider switching tests (45 LOC)

pub mod end_to_end_success_flow_tests;
pub mod error_handling_integration_tests;
pub mod helpers;
pub mod provider_switching_integration_tests;
pub mod retry_logic_integration_tests;
pub mod token_management_integration_tests;

// Re-export commonly used helpers for other test modules
pub use helpers::*;
