# Unit Test Template for multi-llm Library

Unit tests should only test the unit under test. Use mocking or an appropriate test interface for dependent functions so only the unit you are testing is tested. Test the main run path, any branches and error paths to ensure good coverage. Test edge cases as appropriate.

**Note**: This is a Rust library crate, not an application. Tests focus on public API behavior and library internals.

## Test File Placement and Organization

### File Placement Rules (CRITICAL)

**Golden Rule**: Unit tests live in the same module hierarchy as the code they test, in a `tests` submodule.

### Test File Naming Convention

**Standard Pattern**: For source file `src/module/filename.rs`, tests go in `src/module/tests/filename.rs`
- **NOT** `src/module/tests/filename_tests.rs`
- Test file has **same base name** as source file, no `_tests` suffix

**Multi-File Pattern**: When one source file needs multiple test files, split by purpose:
- Source: `src/module/filename.rs`
- Tests: `src/module/tests/filename_<purpose>.rs`
- Examples: `filename_validation.rs`, `filename_errors.rs`, `filename_integration.rs`

#### ✅ Correct Placement Patterns

**IMPORTANT**: The `tests/` directory goes at the same level as the code being tested:
- Top-level files (`src/error.rs`) → Tests in `src/tests/error.rs`
- Module files (`src/domain/user.rs`) → Tests in `src/domain/tests/user.rs`

```
src/
├── lib.rs
├── client.rs                      # Top-level code under test
├── config.rs                      # Top-level code under test
├── tests/                         # Tests for TOP-LEVEL files
│   ├── mod.rs                     # Declares: mod client; mod config;
│   ├── client.rs                  # Tests for src/client.rs
│   └── config.rs                  # Tests for src/config.rs
├── core_types/
│   ├── mod.rs
│   ├── messages.rs                # Module-level code under test
│   ├── provider.rs                # Module-level code under test
│   ├── errors.rs                  # Module-level code under test
│   └── tests/                     # Tests for core_types/* files
│       ├── mod.rs                 # Declares: mod messages; mod provider; mod errors;
│       ├── messages.rs            # Tests for core_types/messages.rs (NOT messages_tests.rs)
│       ├── provider.rs            # Tests for core_types/provider.rs
│       └── errors.rs              # Tests for core_types/errors.rs
├── providers/
│   ├── mod.rs
│   ├── openai.rs                  # Module-level code under test
│   ├── ollama.rs                  # Module-level code under test
│   └── anthropic/                 # Submodule with multiple files
│       ├── mod.rs
│       ├── provider.rs            # Submodule code under test
│       ├── types.rs               # Submodule code under test
│       ├── conversion.rs          # Submodule code under test
│       └── tests/                 # Tests for anthropic/* files
│           ├── mod.rs             # Declares: mod provider; mod types; mod conversion;
│           ├── provider.rs        # Tests for anthropic/provider.rs
│           ├── types.rs           # Tests for anthropic/types.rs
│           └── conversion.rs      # Tests for anthropic/conversion.rs
```

#### ❌ Incorrect Placement Patterns

```
# WRONG: Top-level tests directory for unit tests
tests/
├── user_domain.rs                 # Should be src/domain/tests/user.rs
├── story_domain.rs                # Should be src/domain/tests/story.rs
└── session_domain.rs              # Should be src/domain/tests/session.rs

# WRONG: Using _tests suffix on test files
src/domain/tests/
├── user_tests.rs                  # Should be user.rs
├── story_tests.rs                 # Should be story.rs
└── session_tests.rs               # Should be session.rs

# WRONG: Inline tests for complex modules
src/domain/user.rs
    #[cfg(test)]                   # Should use separate tests/user.rs file
    mod tests { ... }              # when tests are substantial
```

### Import Pattern Reference

#### For Tests in `src/tests/file.rs` (Top-Level Files)

```rust
// Testing top-level files: src/tests/config.rs tests src/config.rs

use super::super::config::AnthropicConfig;    // Go up to src/ then into config module
// OR more commonly:
use crate::config::AnthropicConfig;           // From crate root (clearer)

// External crate imports
use serde_json;
use tokio;
```

#### For Tests in `src/module/tests/file.rs` (Module Files)

```rust
// Correct import patterns by test file location:

// In src/core_types/tests/messages.rs (testing src/core_types/messages.rs)
use super::super::messages::{CacheType, MessageRole, UnifiedMessage}; // Go up to core_types module
use chrono::Utc;

// In src/providers/anthropic/tests/conversion.rs (testing src/providers/anthropic/conversion.rs)
use super::super::{transform_unified_messages, AnthropicMessage};  // Go up to anthropic module
use crate::core_types::messages::{MessageRole, UnifiedMessage};    // From crate root

// In src/providers/tests/openai.rs (testing src/providers/openai.rs)
use super::super::openai::{OpenAIProvider, OpenAIConfig}; // Go up to providers module

// External crate imports (always the same)
use chrono::Utc;
use serde_json;
use tokio;
```

#### Common Import Mistakes to Avoid

```rust
// ❌ WRONG: Trying to import the crate itself from within the crate
use multi_llm::{UnifiedMessage, MessageRole};       // Error: unresolved module

// ❌ WRONG: Wrong number of super:: levels
use super::{UnifiedMessage};                        // Error: UnifiedMessage not in tests module

// ✅ CORRECT: Navigate module hierarchy properly
use super::super::{UnifiedMessage};                 // Go up two levels to core_types module
```

### File Size Management (Critical for Maintainability)

#### File Size Requirements (from module-refactoring-template.md):
- **Target**: ≤500 LOC per file (recommended for optimal readability)
- **Warning**: 500-599 LOC (strongly consider splitting)
- **Mandatory**: ≥600 LOC (MUST split - no exceptions)

#### When Test Files Exceed Limits:
1. **Split by business responsibility** (preferred approach):
   - Group tests by the aspect of functionality they verify
   - Example: Split a 1400 LOC context.rs into:
     - `creation_tests.rs` - Constructor and initialization
     - `user_context_tests.rs` - User-specific functionality
     - `sliding_window_tests.rs` - Message buffer management
     - `emotional_state_tests.rs` - State detection logic
     - `error_path_tests.rs` - All error scenarios

2. **Create a test module structure**:
   ```
   tests/
   ├── context/           # Module for context manager tests
   │   ├── mod.rs        # Module organization and documentation
   │   ├── helpers.rs    # Shared test utilities
   │   ├── creation_tests.rs
   │   ├── user_context_tests.rs
   │   └── error_path_tests.rs
   ```

3. **Document the split** in mod.rs:
   ```rust
   //! ContextManagerAgent Test Suite
   //!
   //! Split into modules to maintain manageable file sizes
   //! (following the <600 LOC requirement from module-refactoring-template.md).
   //!
   //! - `creation_tests` - Constructor tests (3 tests, 69 LOC)
   //! - `user_context_tests` - User management (3 tests, 125 LOC)
   //! - `error_path_tests` - Error scenarios (9 tests, 371 LOC)
   ```

### Decision Framework: Test Organization

#### Standard Rule: ALWAYS Use Separate `tests/` Directory

**ALL tests MUST be in separate test files**, not inline with source code:

- **Top-level files** (`src/error.rs`) → Tests in `src/tests/error.rs`
- **Module files** (`src/domain/user.rs`) → Tests in `src/domain/tests/user.rs`
- **Provider implementations** (`src/providers/resend.rs`) → Tests in `src/providers/tests/resend.rs`

**Rationale:**
- Keeps source files focused on implementation only
- Clear separation of concerns (business logic vs verification)
- Easier to navigate and maintain
- Consistent structure across entire codebase
- `mod.rs` and `lib.rs` files contain ONLY module declarations and re-exports, NO functional code

```rust
// Example: Domain module structure
// src/domain/user.rs - User, UserId, UserPreferences (business logic only)
// src/domain/tests/user.rs - All tests for user.rs
```

#### ❌ NEVER Use Inline `#[cfg(test)]`

**Do NOT put tests inline in source files:**

```rust
// ❌ WRONG: Tests inline with source code
// src/domain/user.rs
pub struct User { /* ... */ }

#[cfg(test)]
mod tests {  // NO - move to separate file
    use super::*;

    #[test]
    fn test_user_creation() { /* ... */ }
}
```

**Exception:** Simple assertion-level tests for trivial helpers MAY use inline tests, but this should be extremely rare. When in doubt, use separate files.

#### File Size Thresholds
- **Split test files**: Required when single test file ≥600 LOC
- **Recommended split**: Consider splitting at >500 LOC for better maintainability
- **Ideal test file size**: 100-400 LOC per file for optimal readability

### Module Declaration Patterns

#### In Parent Module (`src/domain/mod.rs`)
```rust
pub mod user;
pub mod story;
pub mod session;

// Declare tests submodule
#[cfg(test)]
mod tests;

// Re-export public items
pub use user::*;
pub use story::*;
pub use session::*;
```

#### In Test Module (`src/domain/tests/mod.rs`)
```rust
//! Tests for domain types
//!
//! Tests are organized by domain entity for clear separation and maintainability

mod user;    // src/domain/tests/user.rs
mod story;   // src/domain/tests/story.rs  
mod session; // src/domain/tests/session.rs
```

### Visual Directory Structure Examples

#### ✅ Well-Organized Crate Structure
```
mystory-core/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── error.rs
    ├── logging.rs
    └── domain/
        ├── mod.rs                 # Declares submodules and tests
        ├── user.rs               # User domain types
        ├── story.rs              # Story domain types  
        ├── session.rs            # Session domain types
        └── tests/                # Domain test directory
            ├── mod.rs            # Test module declarations
            ├── user.rs           # Tests for domain::user
            ├── story.rs          # Tests for domain::story
            └── session.rs        # Tests for domain::session
```

#### ✅ Agent Crate Structure
```
mystory-agents/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── context.rs                # ContextManagerAgent
    ├── conversation.rs           # ConversationAgent
    ├── memory.rs                 # StoryMemoryAgent
    ├── coordinator.rs            # AgentCoordinator
    └── tests/
        ├── mod.rs
        ├── context.rs            # ContextManagerAgent tests
        ├── conversation.rs       # ConversationAgent tests
        ├── memory.rs             # StoryMemoryAgent tests
        └── coordinator.rs        # AgentCoordinator tests
```

## TODO Comments and Test Requirements

**CRITICAL RULE**: Every TODO comment that affects production functionality MUST have a corresponding test that:
1. **Documents the missing functionality** in test comments
2. **Tests the current behavior** (even if incomplete)
3. **Marks expected future behavior** with clear assertions

### Example: Testing TODO Functionality

```rust
#[test]
fn test_anthropic_structured_response_todo_implementation() {
    // DOCUMENTS: Anthropic provider currently returns None for structured_response
    // TODO: Remove this test when structured response is implemented
    // TRACKS: Issue #123 - Implement Anthropic structured responses
    
    let provider = create_anthropic_provider();
    let config = LLMRequestConfig {
        response_format: Some(create_json_schema()),
        ..Default::default()
    };
    
    let response = provider.complete(messages, Some(config)).await.unwrap();
    
    // Current behavior (TODO not implemented)
    assert!(
        response.structured_response.is_none(),
        "Anthropic structured response not yet implemented - see TODO at line 634"
    );
    
    // FUTURE: When implemented, this should pass:
    // assert!(response.structured_response.is_some());
    // assert_eq!(response.structured_response.unwrap()["field"], expected_value);
}
```

**Benefits**:
- Makes TODOs visible in test output
- Prevents silent feature gaps
- Creates failing tests when TODOs are completed (prompting test updates)
- Documents expected behavior for future implementation

## Cross-Provider Configuration Testing

**MANDATORY**: Test that ALL configuration options are handled by ALL providers:

```rust
#[cfg(test)]
mod configuration_consistency_tests {
    use super::*;
    
    /// Verify ALL providers handle ALL configuration options
    #[test]
    fn test_all_config_options_handled_by_all_providers() {
        let full_config = LLMRequestConfig {
            temperature: Some(0.5),
            max_tokens: Some(1000),
            top_p: Some(0.9),
            top_k: Some(40),
            frequency_penalty: Some(0.1),
            presence_penalty: Some(0.1),
            response_format: Some(create_test_json_schema()), // CRITICAL
            tools: Some(vec![create_test_tool()]),
            stop_sequences: Some(vec!["END".to_string()]),
            // Add ALL config options here
        };
        
        // Test each provider
        let providers = get_all_provider_implementations();
        
        for provider in providers {
            let request = provider.apply_config(full_config.clone());
            
            // Verify EVERY config field is applied (not ignored)
            assert_config_fully_applied(&request, &full_config);
        }
    }
}
```

## Implementation-Test Symmetry Pattern

**CRITICAL PRINCIPLE**: Test file structure MUST mirror implementation structure to prevent coverage gaps.

### The Problem This Solves

**Real Example**: `json_structured_response.rs` only tested OpenAI format conversion, while three providers existed. The test file structure didn't reflect the multi-provider reality.

### Solution: Symmetric Test Structure

```
Implementation Structure:          Test Structure:
providers/                        tests/
├── openai.rs                    ├── json_structured_response.rs
├── anthropic.rs                 │   ├── openai_tests
├── lmstudio.rs                  │   ├── anthropic_tests  // MISSING - caught by symmetry check
                                 │   └── lmstudio_tests   // MISSING - caught by symmetry check
```

### Enforcement Pattern

```rust
// At the top of shared feature test files:
#[cfg(test)]
mod provider_coverage_verification {
    #[test]
    fn verify_all_providers_have_test_modules() {
        // This test ensures test symmetry with implementation
        let provider_impls = vec!["openai", "anthropic", "lmstudio"];
        let test_modules = vec!["openai_tests", "anthropic_tests", "lmstudio_tests"];
        
        for provider in provider_impls {
            assert!(
                test_modules.contains(&format!("{}_tests", provider).as_str()),
                "Missing test module for {} provider", provider
            );
        }
    }
}
```

## Proactive Test Gap Detection

### Pattern 1: Feature Matrix Testing

Create a feature matrix that ensures complete coverage:

```rust
#[cfg(test)]
mod feature_matrix_tests {
    const PROVIDERS: &[&str] = &["openai", "anthropic", "lmstudio"];
    const FEATURES: &[&str] = &[
        "basic_completion",
        "streaming", 
        "tool_calling",
        "structured_responses",  // Would have caught the gap
        "retry_logic",
        "rate_limiting",
    ];
    
    #[test]
    fn verify_feature_matrix_complete() {
        for provider in PROVIDERS {
            for feature in FEATURES {
                assert!(
                    test_exists_for(provider, feature),
                    "Missing test: {}_{}_test", provider, feature
                );
            }
        }
    }
}
```

### Pattern 2: Configuration Field Coverage

```rust
#[test]
fn verify_all_config_fields_tested() {
    // Use reflection or macro to get all fields of LLMRequestConfig
    let config_fields = get_struct_fields::<LLMRequestConfig>();
    
    for field in config_fields {
        assert!(
            field_has_test_coverage(field),
            "Config field '{}' has no test coverage", field
        );
    }
}
```

## PRIMARY PRINCIPLE: Test Business Logic First

**CRITICAL**: Template compliance is important for maintainability, but the fundamental purpose is to verify that the unit's business logic works correctly. A perfectly template-compliant test that tests the wrong behavior or misses key functionality is counterproductive.

### ⚠️ MANDATORY: Research Type Structures Before Writing Tests

**The Single Most Common Mistake**: Writing tests based on assumptions about types rather than reading actual type definitions.

**Why This Matters**:
- Incorrect assumptions lead to 100s of LOC that don't compile
- Wastes significant development time
- Tests can't verify business logic if they're testing the wrong interfaces
- Mock implementations that don't match real interfaces provide false confidence

**Real Example of the Problem**:
```rust
// WRONG: Assumed from memory/intuition
let user_context = AgentUserContext::new(user);  // Compilation error
let style = profile.conversation_style;           // Field doesn't exist
let state = context.emotional_state();           // Method doesn't exist

// CORRECT: After reading actual type definitions
let user_context = AgentUserContext::new(Arc::new(user));  // Takes Arc<User>
let style = profile.writing_style;                          // Actual field name
let state = context.conversation_state();                  // Actual method name
```

### Research Checklist: Before Writing ANY Test

Complete this checklist **BEFORE** writing the first line of test code:

#### 1. Read the Source File
- [ ] Read the entire source file being tested (`src/module/filename.rs`)
- [ ] Identify all public functions/methods to test
- [ ] Understand dependencies and their trait bounds
- [ ] Note any generic type parameters and their constraints

#### 2. Read Type Definitions
- [ ] Read ALL type definitions used by the unit (`types.rs`, domain types, etc.)
- [ ] Verify struct field names and types (don't assume)
- [ ] Check method signatures and return types
- [ ] Understand ownership patterns (plain types vs `Arc<T>` vs `&T`)

#### 3. Review Existing Test Patterns
- [ ] Find a similar test file in the codebase
- [ ] Study how it creates test instances
- [ ] Copy mock/helper patterns that work
- [ ] Match the import structure

#### 4. Validate Dependencies
- [ ] List all dependencies the unit requires
- [ ] Find the trait definitions for each dependency
- [ ] Understand the trait interface methods
- [ ] Check if test utilities exist for these dependencies

#### 5. Start Simple
- [ ] Write ONE simple test first (e.g., constructor test)
- [ ] Verify it compiles and runs
- [ ] Only then expand to comprehensive coverage

### Example: Proper Research Workflow

```rust
// Step 1: Read the source file (session_manager.rs)
// Found: SessionManager::new(executor: Arc<dyn Executor>, config: ManagerConfig)
// Found: SessionManager::create_session(user_context: &AgentUserContext)
// Found: Dependencies: Executor trait, AgentUserContext type

// Step 2: Read type definitions
// From types.rs:
//   - AgentUserContext::new(user: Arc<User>, profile: SessionUserProfile)
//   - SessionUserProfile has: writing_style, collected_insights, interaction_patterns
//   - AgentUserContext methods: conversation_state(), get_profile()

// Step 3: Review existing tests
// From helpers.rs:
//   - create_test_user() returns User (not Arc<User>)
//   - create_test_executor() returns Arc<MockExecutor>
//   - Existing tests wrap user: Arc::new(create_test_user())

// Step 4: Validate dependencies
// Executor trait needs: execute_async(), blocking_execute()
// MockExecutor needs to implement both methods

// Step 5: Write first simple test
#[test]
fn test_session_manager_creation() {
    // Test verifies basic construction with correct types
    let executor = create_test_executor();  // From helpers.rs
    let config = ManagerConfig::default();
    let manager = SessionManager::new(executor, config);
    assert!(manager.is_ok());
}

// ONLY AFTER THIS COMPILES, expand to more complex tests
```

### Warning Signs You're Making Assumptions

Stop and research if you find yourself:
- ❌ "I think this takes a User, let me try..."
- ❌ "This field should be called conversation_style..."
- ❌ "I remember this method returns Option<T>..."
- ❌ Writing 100+ lines before first compile attempt
- ❌ Creating mock implementations from memory

### Correct Approach

- ✅ "Let me read the actual AgentUserContext definition"
- ✅ "Let me check what fields SessionUserProfile actually has"
- ✅ "Let me look at how helpers.rs creates this type"
- ✅ Writing 20-30 lines, then compiling to validate
- ✅ Copying mock patterns from existing working tests

### Required Unit Documentation

**Each unit being tested** MUST have a documentation comment that clearly specifies:

1. **What unit is being tested** (the concrete implementation)
2. **What the unit's business responsibility is** (its core function/purpose)
3. **How the tests verify the unit works correctly** (what behaviors are being validated)

**Important**: A single test file may test multiple units - each unit should have its own documentation block.

#### Single Unit Example:
```rust
// Unit Tests for ConversationAgent
//
// UNIT UNDER TEST: ConversationAgentImpl (concrete implementation)
// 
// BUSINESS RESPONSIBILITY:
//   - Processes user input using ReAct pattern for story conversation
//   - Coordinates with Memory and Context Manager agents
//   - Extracts story elements from conversation flow
//   - Manages conversation state and user guidance
//
// TEST COVERAGE:
//   - Message processing with ReAct pattern execution
//   - Agent coordination and handoff scenarios
//   - Story element extraction and validation
//   - Error handling for LLM timeouts and invalid responses
//   - Edge cases: empty context, token limit exceeded, malformed input
//
#[cfg(test)]
mod tests {
    use super::*;
    use mystory_core::log_info;
    // ... test implementation
}
```

#### Multiple Units Example:
```rust
// Unit Tests for LLM Provider Implementations
//
// UNIT UNDER TEST: AnthropicProvider
// BUSINESS RESPONSIBILITY:
//   - Handles communication with Anthropic Claude API
//   - Converts between myStory types and provider-specific formats
//   - Manages API rate limiting and error recovery
// TEST COVERAGE:
//   - Request/response conversion, error handling, rate limiting

// UNIT UNDER TEST: OpenAIProvider  
// BUSINESS RESPONSIBILITY:
//   - Handles communication with OpenAI GPT API
//   - Implements tool calling and structured output parsing
// TEST COVERAGE:
//   - Tool call handling, structured output validation, timeout scenarios

// UNIT UNDER TEST: LMStudioProvider
// BUSINESS RESPONSIBILITY:
//   - Provides local LLM communication for development
//   - Handles connection management and fallback behavior
// TEST COVERAGE:
//   - Connection management, fallback scenarios, local API compatibility

#[cfg(test)]
mod tests {
    use super::*;
    use mystory_core::{log_info, log_debug};
    
    mod anthropic_tests { /* tests for AnthropicProvider */ }
    mod openai_tests { /* tests for OpenAIProvider */ }  
    mod lm_studio_tests { /* tests for LMStudioProvider */ }
}
```

**Bad Example (Missing Business Focus):**
```rust
// Tests for ConversationAgent
// Tests the constructor and basic methods
```

**Good Example (Business Logic Focused):**
```rust
// Unit Tests for ContextManagerAgent
//
// UNIT UNDER TEST: ContextManagerImpl
//
// BUSINESS RESPONSIBILITY:
//   - Builds optimal context windows for LLM requests
//   - Manages token limits and auto-compaction strategies
//   - Balances recent messages with relevant story context
//   - Optimizes for LLM response quality and performance
//
// TEST COVERAGE:
//   - Context assembly with different story complexity levels
//   - Token limit enforcement and auto-compaction triggers
//   - Relevance-based story element selection algorithms
//   - Performance optimization for sub-second context building
//   - Error cases: corrupted story data, extreme token usage
```

## Project-Specific Testing Patterns

**Critical Requirements Reinforced During Implementation:**

1. **Test Comments Must Reflect Business Responsibilities**: Every test must have a comment describing the exact business responsibility being verified
2. **Consistent Helper Function Naming**: Use `create_concrete_<unit_under_test>()` for the actual unit being tested and `create_mock_<dependency>()` for dependencies
3. **Traits Over Concrete Types**: Always prefer trait-based dependencies for better testability and loose coupling
4. **Trait Compliance Testing**: When multiple implementations share a trait, test trait behavior consistency across all implementations

## Trait Compliance Testing (Architectural Consistency)

**CRITICAL PRINCIPLE**: When multiple implementations share a trait interface, test the trait behavior to ensure architectural consistency across all implementations.

### The Problem This Solves

**Real Example**: During LLM provider retry logic integration, the Anthropic provider was missed because:
- OpenAI/LM Studio used shared retry infrastructure  
- Anthropic used direct HTTP calls without retry
- Unit tests were implementation-specific and missed the inconsistency
- Integration tests came too late to catch the architectural gap

### Solution: Comprehensive Trait-Based Compliance Tests

Add trait compliance tests that verify **all implementations behave consistently** for the same interface and implement **complete functionality**:

```rust
// Trait Compliance Tests - Test ALL implementations of LLMClientCore
//
// PURPOSE: Ensures architectural consistency across provider implementations
// CRITICAL: Catches when one implementation lacks features others have
//
#[cfg(test)]
mod trait_compliance_tests {
    use super::*;
    use crate::client::LLMClientCore;
    use mystory_test_utils::{create_test_config, MockServer};
    
    /// Test retry behavior consistency across ALL LLM providers
    async fn test_provider_retry_behavior<T: LLMClientCore>(provider: T) {
        // Test verifies ALL providers handle retries consistently
        // This would have caught the Anthropic retry gap immediately
        
        // Arrange: Mock server that fails then succeeds
        let mock_server = MockServer::with_retry_scenario().await;
        
        // Act: Make request that triggers retry logic
        let result = provider.send_message("test message").await;
        
        // Assert: All providers should handle retries the same way
        assert!(result.is_ok(), "All providers must handle retries consistently");
        
        // Verify retry behavior happened (implementation-agnostic)
        assert!(mock_server.request_count() > 1, "Provider should have retried failed requests");
    }
    
    /// Test authentication error handling across ALL providers
    async fn test_provider_auth_error_consistency<T: LLMClientCore>(provider: T) {
        // Test ensures ALL providers return same error type for auth failures
        
        // Arrange: Mock server returns 401 Unauthorized
        let mock_server = MockServer::with_auth_error().await;
        
        // Act: Make request with invalid credentials
        let result = provider.send_message("test").await;
        
        // Assert: ALL providers must return AuthenticationFailed error
        match result {
            Err(LlmError::AuthenticationFailed { .. }) => {
                // Expected - all providers should behave this way
            },
            _ => panic!("All providers must return AuthenticationFailed for 401 errors"),
        }
    }
    
    /// Test configuration completeness across ALL trait implementations
    async fn test_trait_configuration_completeness<T: LLMClientCore>(implementation: T, impl_name: &str) {
        // Test verifies ALL implementations handle ALL configuration options from the trait
        // CRITICAL: This would have caught the Anthropic response_format gap immediately
        
        // Arrange: Create configuration with ALL possible options
        let full_config = LLMRequestConfig {
            temperature: Some(0.5),
            max_tokens: Some(1000),
            top_p: Some(0.9),
            response_format: Some(create_test_json_schema()), // CRITICAL - Anthropic ignored this
            tools: Some(vec![create_test_tool()]),
            // Add ALL configuration fields here
        };
        
        // Act: Send request with full configuration
        let result = implementation.execute_chat_with_config(full_config.clone()).await;
        
        // Assert: Implementation MUST handle all configuration options
        match result {
            Ok(response) => {
                // Verify structured response when response_format is set
                if full_config.response_format.is_some() {
                    assert!(
                        response.structured_response.is_some(),
                        "{} implementation MUST return structured_response when response_format is configured",
                        impl_name
                    );
                }
                
                // Add assertions for other config options
                // Temperature, max_tokens, etc. should be reflected in the response
            },
            Err(e) => panic!("{} implementation failed with full config: {}", impl_name, e),
        }
    }
    
    /// Test feature completeness across ALL trait implementations
    async fn test_trait_feature_completeness<T: LLMClientCore>(implementation: T, impl_name: &str) {
        // Test matrix of ALL features that the trait contract implies
        let features = vec![
            ("basic_completion", test_basic_completion_feature),
            ("tool_calling", test_tool_calling_feature), 
            ("structured_responses", test_structured_response_feature),
            ("retry_logic", test_retry_logic_feature),
            ("error_handling", test_error_handling_feature),
        ];
        
        for (feature_name, test_fn) in features {
            test_fn(&implementation).await
                .unwrap_or_else(|_| panic!("{} implementation lacks {} feature", impl_name, feature_name));
        }
    }

    #[tokio::test]
    async fn test_all_trait_implementations_complete_compliance() {
        // Test verifies ALL trait implementations follow complete trait contracts
        // CRITICAL: This comprehensive approach would have caught the Anthropic gaps
        
        // Arrange: Create ALL implementations of the trait
        let implementations: Vec<(String, Box<dyn LLMClientCore>)> = vec![
            ("OpenAI".to_string(), Box::new(OpenAIProvider::new(create_test_config("openai")).unwrap())),
            ("Anthropic".to_string(), Box::new(AnthropicProvider::new(create_test_config("anthropic")).unwrap())),
            ("LMStudio".to_string(), Box::new(LMStudioProvider::new(create_test_config("lmstudio")).unwrap())),
        ];
        
        // Act & Assert: Test each implementation against COMPLETE trait contract
        for (impl_name, implementation) in implementations {
            // Test behavioral consistency
            test_provider_retry_behavior(implementation.as_ref()).await
                .unwrap_or_else(|_| panic!("{} failed retry compliance", impl_name));
                
            test_provider_auth_error_consistency(implementation.as_ref()).await
                .unwrap_or_else(|_| panic!("{} failed auth error compliance", impl_name));
            
            // Test configuration completeness - NEW: This would have caught the Anthropic gap
            test_trait_configuration_completeness(implementation.as_ref(), &impl_name).await;
            
            // Test feature completeness - NEW: Systematic feature coverage
            test_trait_feature_completeness(implementation.as_ref(), &impl_name).await;
        }
    }
}
```

### When to Add Trait Compliance Tests

**MANDATORY** when:
- Multiple implementations of the same trait exist
- The trait represents critical system behavior (authentication, retry logic, error handling, configuration handling)
- Cross-implementation consistency is required for system reliability
- Any trait method accepts configuration parameters or returns structured data
- A TODO comment exists in any implementation that affects trait contract compliance

**OPTIONAL** when:
- Only one implementation exists (but consider adding for future-proofing)
- Implementation differences are expected and acceptable

### Key Compliance Testing Dimensions

When testing trait implementations, verify these dimensions:

1. **Behavioral Consistency**: All implementations behave the same way for the same inputs
2. **Configuration Completeness**: All implementations handle ALL configuration options
3. **Feature Completeness**: All implementations support ALL features implied by the trait
4. **Error Handling Consistency**: All implementations return consistent error types
5. **TODO Implementation**: All implementations actually implement features (not hardcoded placeholders)

### Generic Trait Compliance Test Structure

```rust
#[cfg(test)]
mod trait_compliance_tests {
    use super::*;
    
    /// Generic behavioral consistency test - works with ANY implementation
    async fn test_trait_behavior_consistency<T: TraitName>(implementation: T, impl_name: &str) {
        // Test the trait contract behavior, not implementation details
        // This ensures ALL implementations behave consistently for the same interface
    }
    
    /// Generic configuration completeness test - works with ANY trait that accepts config
    async fn test_trait_configuration_completeness<T: TraitName>(implementation: T, impl_name: &str) {
        // Test that implementation handles ALL configuration options from the trait
        // CRITICAL: This pattern would have caught configuration gaps immediately
        
        let full_config = create_full_configuration(); // ALL possible config options
        let result = implementation.execute_with_config(full_config.clone()).await;
        
        // Assert ALL config options are handled (not ignored)
        assert_config_fully_applied(&result, &full_config, impl_name);
    }
    
    /// Generic feature completeness test - works with ANY trait
    async fn test_trait_feature_completeness<T: TraitName>(implementation: T, impl_name: &str) {
        // Test matrix of ALL features that the trait contract implies
        let trait_features = get_all_trait_features(); // Based on trait definition
        
        for feature in trait_features {
            assert!(
                implementation.supports_feature(&feature),
                "{} implementation MUST support {} feature required by trait",
                impl_name, feature
            );
        }
    }
    
    #[tokio::test] 
    async fn test_all_trait_implementations_complete_compliance() {
        // Create ALL implementations of the trait
        let implementations: Vec<(String, Box<dyn TraitName>)> = vec![
            ("Implementation1".to_string(), Box::new(Implementation1::new())),
            ("Implementation2".to_string(), Box::new(Implementation2::new())),
            ("Implementation3".to_string(), Box::new(Implementation3::new())),
        ];
        
        // Test each implementation against COMPLETE trait contract
        for (impl_name, implementation) in implementations {
            // Test all dimensions of trait compliance
            test_trait_behavior_consistency(implementation.as_ref(), &impl_name).await;
            test_trait_configuration_completeness(implementation.as_ref(), &impl_name).await;
            test_trait_feature_completeness(implementation.as_ref(), &impl_name).await;
        }
    }
}
```

### Integration with Testing Hierarchy

1. **Trait Compliance Tests** (Architectural consistency) - Test interface contracts across implementations
2. **Unit Tests** (Implementation details) - Test provider-specific implementation details  
3. **Integration Tests** (End-to-end) - Test complete workflows through system boundaries

**Key Insight**: Trait compliance tests catch architectural inconsistencies that unit tests miss and integration tests find too late.


### 1. Unit Under Test vs Dependencies

**Key Principle**: When testing a unit, use concrete objects for the unit itself, but use traits and dependency injection for its dependencies.

#### Testing the Unit Directly (Use Concrete Objects)

When testing `ServiceA` itself, create a concrete instance because that's what we're actually testing:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory_store::MemoryStore;
    use std::sync::Arc;

    // Helper function to create the concrete unit under test
    fn create_concrete_conversation_agent() -> ConversationAgentImpl {
        let llm_client = Arc::new(create_mock_llm_client());
        let memory = Arc::new(create_mock_memory_agent());
        let context = Arc::new(create_mock_context_manager());
        ConversationAgentImpl::new(llm_client, memory, context)
    }

    #[tokio::test]
    async fn test_conversation_agent_processes_message() {
        // Arrange: Create concrete instance of the unit under test
        let agent = create_concrete_conversation_agent();
        let input = ConversationInput::new("Tell me about your childhood");
        
        // Act & Assert: Test the actual implementation
        let result = agent.process_turn(input).await;
        assert!(result.is_ok());
        assert!(result.unwrap().response.len() > 0);
    }
}
```

#### Testing with Dependencies (Use Traits and Test Doubles)

When `ServiceA` depends on `ServiceB`, inject test doubles via traits to isolate the unit under test:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mystory_test_utils::{create_mock_llm_client, create_mock_storage};
    
    #[tokio::test]
    async fn test_agent_with_dependencies() {
        // Arrange: Use test doubles for dependencies
        let mock_llm = create_mock_llm_client(); // From test-utils crate
        let mock_storage = create_mock_storage(); // From test-utils crate
        let agent = AgentImpl::new(mock_llm, mock_storage); // Inject via traits
        
        // Act: Exercise the unit under test
        let result = agent.process_with_llm_and_storage(input).await;
        
        // Assert: Verify behavior of unit under test
        assert!(result.is_ok());
        // Can also verify interactions with the mocks if needed
    }
}
```

#### Unit Test Helper Architecture Pattern (Key Learning)

**Critical Pattern**: The optimal architecture for unit test helpers was discovered during the conversation_manager_tests.rs implementation:

**Pattern**: 
- `create_concrete_<unit_under_test>()` helpers live in **local test files**
- `create_mock_<dependent_units>()` helpers live in **test-utils crate**
- Concrete helpers internally call all needed mock helpers for dependencies

```rust
// In local test file (e.g., conversation_agent_tests.rs):
fn create_concrete_conversation_agent() -> ConversationAgentImpl {
    // This helper creates the concrete unit under test with ALL mocked dependencies
    let mock_llm = create_mock_llm_client();        // from test-utils
    let mock_memory = create_mock_memory_agent();   // from test-utils  
    let mock_context = create_mock_context_manager(); // from test-utils
    
    ConversationAgentImpl::new(
        mock_llm,
        mock_memory,
        mock_context,
    )
}

// In test-utils/src/mocks.rs:
pub fn create_mock_llm_client() -> Arc<dyn LLMClientCore> {
    let mut mock = MockLLMClient::new();
    mock.expect_complete()
        .returning(|_request| {
            Box::pin(async move {
                // Realistic mock response
                Ok(CompletionResponse {
                    content: "Mock LLM response".to_string(),
                    tokens_used: 50,
                })
            })
        });
    Arc::new(mock)
}
```

**Benefits of This Pattern**:
- **Clear separation**: Unit test creates concrete implementation, test-utils provides mocks
- **Naming consistency**: `create_concrete_*()` for units under test, `create_mock_*()` for dependencies
- **Reusability**: Mock helpers can be reused across multiple test files
- **Maintainability**: Changes to mock behavior centralized in test-utils
- **Business logic focus**: Unit tests focus on testing the actual implementation with controlled dependencies

#### Test Utilities for Shared Test Doubles

Use the `test-utils` crate to provide reusable test doubles and builders:

```rust
// In test-utils crate - Mock helper functions
pub fn create_mock_storage() -> Arc<dyn StorageTrait> {
    // Return a mock/fake implementation
}

pub fn create_test_ai_config<T>() -> T {
    // Return a test configuration
}

// In your test module - Local concrete helper
use mystory_test_utils::{create_mock_storage, create_test_ai_config};

fn create_concrete_service() -> ServiceImpl<MockStorage, MockConfig> {
    // Use shared test doubles from test-utils
    let mock_storage = create_mock_storage();    // from test-utils
    let test_config = create_test_ai_config::<AiConfig>(); // from test-utils
    
    // Create concrete instance of unit under test
    ServiceImpl::new(mock_storage, test_config)
}

#[test]
fn test_unit_with_dependencies() {
    // Test verifies the unit handles dependency interactions correctly
    
    // Arrange
    let service = create_concrete_service();
    
    // Act
    let result = service.process();
    
    // Assert
    assert!(result.is_ok());
}
```

### 2. Agent Configuration Testing

When testing agent components, distinguish between the agent itself and its configuration dependencies:

```rust
// Testing the ConversationAgent itself (concrete object)
fn create_concrete_conversation_agent() -> ConversationAgentImpl {
    let config = mystory_test_utils::create_test_agent_config();
    let llm_client = create_mock_llm_client(); // Test double for dependency
    let memory = create_mock_memory_agent();   // Test double for dependency
    let context = create_mock_context_manager(); // Test double for dependency
    
    ConversationAgentImpl::new_with_config(
        llm_client,
        memory,
        context,
        config, // Test configuration
    )
}
```

### 3. User ID Consistency

Always use `create_test_user()` and capture the actual user ID rather than hardcoding:

```rust
#[tokio::test]
async fn test_with_proper_user_setup() {
    let test_user = create_test_user();
    let user_id = test_user.id.clone(); // Capture actual ID
    storage.create_user(test_user).await.unwrap();
    
    // Use captured user_id instead of hardcoded "test_user"
    let result = service.process_for_user(&user_id).await;
    assert!(result.is_ok());
}
```

## Standard Rust Unit Test Template

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name_with_valid_input() {
        // Arrange
        let input = "expected_input";
        let expected = "expected_output";
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected);
    }

    #[test]
    fn test_function_name_with_invalid_input() {
        // Arrange
        let invalid_input = "invalid_input";
        
        // Act & Assert
        assert!(function_under_test(invalid_input).is_err());
        // Or for panics:
        // let result = std::panic::catch_unwind(|| function_under_test(invalid_input));
        // assert!(result.is_err());
    }

    #[test]
    fn test_function_name_edge_case() {
        // Arrange
        let edge_case_input = "";
        let expected = "expected_edge_case_result";
        
        // Act
        let result = function_under_test(edge_case_input);
        
        // Assert
        assert_eq!(result, expected);
    }

    #[test]
    #[should_panic(expected = "specific error message")]
    fn test_function_name_should_panic() {
        // Arrange
        let panic_input = "panic_trigger";
        
        // Act (this should panic)
        function_under_test(panic_input);
    }

    // For testing private functions or methods
    #[test]
    fn test_private_function() {
        // Arrange
        let input = "test_input";
        
        // Act
        let result = super::private_function(input);
        
        // Assert
        assert!(result.is_ok());
    }
}
```

## Test Organization Best Practices

### Module Structure
```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod unit_tests {
        use super::*;
        // Individual function tests
    }

    mod integration_tests {
        use super::*;
        // Cross-component interaction tests
    }

    mod edge_case_tests {
        use super::*;
        // Boundary and edge case tests
    }
}
```

### Process Definition Testing

Use real process definitions instead of test builders:

```rust
#[test]
fn test_process_workflow() {
    // GOOD: Use real process definition
    let process_def = get_default_process_definition();
    
    // AVOID: Using test builders that don't match production
    // let process_state = ProcessStateBuilder::new().build();
}
```

## Reference Example: process_state_integrator_tests.rs

**See `backend/src/ai/tool_interface/process_state_integrator_tests.rs`** for a complete reference implementation that demonstrates all template requirements:

### What Makes It Exemplary:

1. **Complete Unit Documentation**: Clear header specifying unit under test, business responsibility, and test coverage
2. **Trait-Based Architecture**: Uses concrete `ProcessStateIntegrator` for the unit under test, traits for dependencies (`LLMToolInterfaceCore`, `StoryElementExtractorCore`)
3. **Business Logic Validation**: Tests validate actual business behavior (deviation detection thresholds, step completion analysis) not just success/failure
4. **Helper Functions**: Reusable setup functions eliminate code duplication
5. **Bracketing Tests**: Tests boundary conditions between success/failure states
6. **Proper Dependency Injection**: Uses `TestProcessState` from test-utils instead of concrete production types
7. **Meaningful Assertions**: Every test validates specific business outcomes with contextual error messages

### Architecture Pattern:
```rust
// Unit under test: concrete implementation
fn create_test_integrator() -> ProcessStateIntegrator<MockLLMToolInterface, MockStoryElementExtractor> {
    let mock_llm = create_mock_llm_tool_interface();     // Trait dependency
    let mock_extractor = create_mock_story_element_extractor(); // Trait dependency  
    ProcessStateIntegrator::new(memory_store, mock_llm, mock_extractor)
}

// Test validates business logic, not just technical success
assert!(analysis.process_deviation_detected, "Poor validation score should trigger deviation detection");
assert_ne!(step1.completion_percentage, step2.completion_percentage, 
           "Sessions with different data should have different completion percentages");
```

This file successfully revealed implementation gaps through proper business logic testing, demonstrating the template's effectiveness.

## Key Best Practices Demonstrated

1. **`#[cfg(test)]`** - Ensures test code is only compiled during testing
2. **`mod tests`** - Separates test code into its own module
3. **`use super::*`** - Imports the parent module's items for testing
4. **Descriptive test names** - Clearly indicates what's being tested and the scenario
5. **AAA Pattern** - Arrange, Act, Assert for clear test structure
6. **Single responsibility** - Each test focuses on one specific behavior
7. **Unit vs Dependencies distinction**:
   - **Unit under test**: Use concrete implementations 
   - **Dependencies**: Use traits and test doubles/mocks
8. **Shared test utilities** - Use `test-utils` crate for reusable test doubles
9. **User ID consistency** - Use captured IDs from test utilities
10. **Real data structures** - Use production data structures, not test builders

## Common Assertions for myStory Types

```rust
// For floating point comparisons (use test utility)
use mystory_test_utils::assert_float_eq;
assert_float_eq(actual, expected, "description");

// For JSON validation tests (ensure exact string matching)
invalid_response = invalid_response.replace(
    r#""field_name":0.8"#,  // Match actual JSON without spaces
    r#""field_name":-0.1"#,
);

// For collections
assert!(vec.contains(&item));
assert_eq!(vec.len(), 3);

// For Options and Results
assert!(result.is_some());
assert!(result.is_ok());
assert_eq!(result.unwrap(), expected);

// For custom error messages
assert_eq!(result, expected, "Custom failure message: {}", context);

// For story status (use proper enum values)
assert_eq!(story.status, mystory_types::StoryStatus::Draft as i32);

// For enum serialization tests
let json = serde_json::to_string(&data).unwrap();
assert!(json.contains("moderate")); // DetailLevel::Moderate serializes to "moderate"
```

## Test Review Checklist

When reviewing existing tests or writing new ones:

### Business Logic Focus (MOST IMPORTANT)
- [ ] **Unit documentation**: Each unit has clear documentation of what's being tested, business responsibility, and test coverage
- [ ] **Multiple units handled properly**: If file tests multiple units, each has its own documentation block
- [ ] **Business logic coverage**: Tests verify the unit's core business functionality, not just implementation details
- [ ] **Meaningful assertions**: Tests validate expected business outcomes, not just that methods don't crash
- [ ] **Real scenarios**: Tests cover actual use cases the unit will encounter in production

### Unit Testing Architecture
- [ ] **Unit under test**: Uses concrete implementation (the actual thing being tested)
- [ ] **Dependencies**: Uses traits and test doubles/mocks from `test-utils` crate
- [ ] Clearly distinguishes between what's being tested vs what's helping test it

### Trait Compliance Testing (CRITICAL for Multi-Implementation Traits)
- [ ] **Trait compliance tests exist**: When multiple implementations share a trait, tests verify consistent behavior across ALL implementations
- [ ] **Behavioral consistency**: All implementations behave the same way for the same trait interface calls
- [ ] **Configuration completeness**: ALL implementations handle ALL configuration parameters (none ignored)
- [ ] **Feature completeness**: ALL implementations support ALL features implied by the trait contract
- [ ] **Error handling consistency**: All implementations return consistent error types for the same failure scenarios
- [ ] **TODO implementation verification**: All implementations actually implement features (no hardcoded placeholders with TODO comments)
- [ ] **Implementation enumeration**: Tests explicitly list and test ALL implementations of each trait
- [ ] **Interface contracts**: Tests verify trait behavior through the trait interface, not implementation details
- [ ] **Cross-implementation validation**: Generic test functions work with any implementation of the trait


### Project-Specific Patterns  
- [ ] Captures actual user IDs from `create_test_user()` instead of hardcoding
- [ ] Uses real process definitions from `get_default_process_definition()`
- [ ] JSON string replacements match actual serialized structure (no spaces)
- [ ] Enum expectations match current enum variants (e.g., `DetailLevel::Moderate`)
- [ ] Float comparisons use `assert_float_eq` helper

### General Test Quality
- [ ] Test names clearly describe the scenario being tested
- [ ] Follows AAA pattern (Arrange, Act, Assert)
- [ ] Tests both success and error paths
- [ ] Includes edge cases and boundary conditions
- [ ] Single responsibility - each test focuses on one specific behavior

### File Size and Organization
- [ ] **File size compliance**: All test files <600 LOC (mandatory limit)
- [ ] **Optimal size**: Test files ideally 100-400 LOC for maintainability
- [ ] **Split strategy**: Large files split by business responsibility, not arbitrary boundaries
- [ ] **Module documentation**: Split test modules have clear mod.rs with file purposes and test counts
- [ ] **Shared helpers**: Common test utilities extracted to helpers.rs or test-utils crate
- [ ] **Test count preservation**: No tests lost during file reorganization
- [ ] **Helper naming**: Uses `create_concrete_<unit>()` for units under test, `create_mock_<dependency>()` for mocks

### Code Coverage (CRITICAL Quality Gate)
- [ ] **Coverage measured**: Run `python3 scripts/verify-test-coverage.py <crate> --test-filter <module>::`
- [ ] **Goal met (≥90%)**: Module achieves excellent coverage threshold
- [ ] **Minimum met (≥80%)**: All files meet hard limit (blocks merge if not met)
- [ ] **Low coverage investigated**: Files below 90% reviewed for missing business logic tests
- [ ] **Business responsibilities verified**: TEST COVERAGE documentation lists all responsibilities from source code
- [ ] **HTML report reviewed**: For files below goal, check `target/llvm-cov/html/index.html` to find untested code paths
- [ ] **Gap tests added**: Missing tests added for uncovered business logic (not just to hit metrics)

## Code Coverage Verification

**CRITICAL**: Code coverage is a quality gate that ensures tests actually verify all business responsibilities. Low coverage is a trigger to re-examine tests for completeness.

### Coverage Requirements

- **Goal**: ≥90% line coverage (target for excellent coverage)
- **Minimum**: ≥80% line coverage (hard limit)
- **Tool**: `cargo-llvm-cov` (install: `cargo install cargo-llvm-cov`)

### Running Coverage Analysis

```bash
# Check coverage for entire crate
python3 scripts/verify-test-coverage.py mystory-core

# Check coverage for specific module (runs only matching tests)
python3 scripts/verify-test-coverage.py mystory-core --test-filter domain::

# Generate HTML report for detailed analysis
python3 scripts/verify-test-coverage.py mystory-core --test-filter domain:: --html

# Show per-file coverage breakdown (filtered to specified crate)
# Shows only files from mystory-core with clean paths (e.g., domain/user.rs)
# Also shows crate-specific coverage statistics
python3 scripts/verify-test-coverage.py mystory-core --show-files

# Show per-file coverage including all dependency crates
# Useful for mystory-agents which includes mystory-core, mystory-llm, etc.
# Shows full paths (e.g., mystory-agents/src/context/types.rs) and overall statistics
python3 scripts/verify-test-coverage.py mystory-agents --show-all-files

# Custom thresholds
python3 scripts/verify-test-coverage.py mystory-core --goal 95 --minimum 85
```

### Coverage-Driven Test Improvement

**When coverage is below goal (90%)**, follow this process:

1. **Identify Low-Coverage Files**
   ```bash
   python3 scripts/verify-test-coverage.py mystory-core --test-filter domain:: --show-files
   ```
   Look for files with ⚠️ (80-90%) or ❌ (<80%) markers.

2. **Analyze Business Responsibilities**
   - Open the source file (e.g., `mystory-core/src/domain/session.rs`)
   - Review the file's public API and business logic
   - Check the BUSINESS RESPONSIBILITY section in test file
   - **Ask**: Does the test coverage section list all business responsibilities?

3. **Find Untested Code Paths**
   - Generate HTML coverage report: `--html`
   - Open `target/llvm-cov/html/index.html` in browser
   - Red/yellow highlighted lines = not covered by tests
   - **Focus on business logic**, not boilerplate

4. **Add Missing Tests**
   - For each uncovered business responsibility:
     - Add test to TEST COVERAGE documentation
     - Write test following AAA pattern
     - Verify new test executes uncovered code
   - Re-run coverage to confirm improvement

5. **Validate Coverage Improvement**
   ```bash
   python3 scripts/verify-test-coverage.py mystory-core --test-filter domain:: --show-files
   ```
   Confirm file now shows ✅ (≥90%)

### Example: Improving session.rs Coverage

```bash
# Current coverage shows session.rs at 80.33%
$ python3 scripts/verify-test-coverage.py mystory-core --test-filter domain:: --show-files

Per-File Coverage:
------------------------------------------------------------
Domain Files:
  ⚠️  domain/session.rs                                   80.33%

# Generate HTML report to see what's not covered
$ python3 scripts/verify-test-coverage.py mystory-core --test-filter domain:: --html
# Open target/llvm-cov/html/index.html

# Find session.rs shows:
# - Line 45-52: is_stale() with custom threshold (NOT TESTED)
# - Line 67-71: duration_minutes() calculation (NOT TESTED)

# Review BUSINESS RESPONSIBILITY in tests/session.rs:
# - Missing: "Supports configurable staleness thresholds"
# - Missing: "Calculates session duration in minutes"

# Add missing tests:
#[test]
fn test_session_staleness_with_custom_threshold() {
    // Test verifies configurable staleness detection
    // ...
}

#[test]
fn test_session_duration_in_minutes() {
    // Test verifies duration calculation for reporting
    // ...
}

# Verify coverage improvement
$ python3 scripts/verify-test-coverage.py mystory-core --test-filter domain:: --show-files
  ✅ domain/session.rs                                   92.15%
```

### Coverage Best Practices

1. **Coverage is a Tool, Not a Goal**
   - 100% coverage doesn't guarantee quality
   - Focus on testing business responsibilities
   - Skip trivial getters/setters if no business logic

2. **Test Business Logic, Not Implementation**
   - Cover all error paths (not just success)
   - Cover boundary conditions (empty, max, negative)
   - Cover state transitions (Draft → InProgress → Complete)

3. **Use Coverage to Find Gaps**
   - Low coverage = potential missing tests
   - Review business responsibilities
   - Add tests for uncovered business logic

4. **Don't Game the Metrics**
   - Don't write tests just to increase coverage
   - Write tests that verify business value
   - Use coverage to validate completeness

### Integration with CI/CD

Coverage checks run automatically in CI:
- Goal (90%): Warning if not met, but doesn't fail build
- Minimum (80%): Fails build if not met
- Report available in CI artifacts

```bash
# Local pre-commit check
python3 scripts/verify-test-coverage.py mystory-core --test-filter domain::
# Exit code: 0 = excellent, 1 = warning, 2 = violation
```

## Development Command Reference

Always use the project's development script for testing:

```bash
# Run all tests
./scripts/dev.sh test

# Run specific crate tests
cargo test -p mystory-agents --lib
cargo test -p mystory-llm --lib
cargo test -p mystory-storage --lib

# Run specific test module
cargo test -p mystory-agents --lib conversation_agent

# Run with output for debugging
cargo test -p mystory-agents --lib test_name -- --nocapture

# Run trait compliance tests specifically
cargo test -p mystory-llm --lib trait_compliance

# Run configuration completeness tests
cargo test -p mystory-llm --lib configuration_completeness

# Run feature completeness tests
cargo test -p mystory-llm --lib feature_completeness

# Check for TODO-related tests
cargo test -p mystory-llm --lib -- --ignored todo_implementation

# Verify code coverage (IMPORTANT)
python3 scripts/verify-test-coverage.py <crate-path> --test-filter <module>::
python3 scripts/verify-test-coverage.py mystory-core --test-filter domain::
```

This template reflects the patterns established during the comprehensive test recovery effort and provides guidance for maintaining high-quality, consistent tests throughout the codebase.