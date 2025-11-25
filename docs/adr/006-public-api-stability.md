# ADR-006: Public API Stability and Surface Area

**Status**: Implemented
**Date**: 2025-11-22 (Updated: 2025-11-25)
**Authors**: multi-llm team
**Deciders**: Rick Duff

## Context

The library was extracted from a larger application (`myStory`) and carries legacy design choices. The current public API surface is too broad, exposing many internal types and using legacy naming from the parent project.

**Current problems** (pre-implementation):
1. **Too many public exports**: `lib.rs` re-exports 45+ types, many internal
2. **Legacy naming**: `ExecutorLLMProvider`, `ExecutorLLMConfig`, `MyStoryError`
3. **No clear boundaries**: Everything is `pub mod`, internals exposed
4. **Duplicate types**: `LLMToolCall` vs `ExecutorToolCall`, `LLMUsage` vs `ExecutorTokenUsage`
5. **Parent project concepts**: `AgentContext`, `EventScope::User` assume application patterns

**Requirements for library-grade API**:
- **Minimal surface**: Only expose what users need directly
- **Clear stability tiers**: What's locked vs what can change
- **Generic naming**: No parent project references
- **No duplication**: One canonical type for each concept

**Forces**:
- **Stability vs Flexibility**: Narrow API is more stable but less flexible
- **Migration pain**: Renaming breaks existing users (pre-1.0 is acceptable)
- **Discoverability**: Too minimal = users don't know what's available

## Decision

We **narrowed the public API** to ~28 core types and removed legacy naming:

### Public API (Stable after 1.0)

**Core abstractions**:
```rust
// lib.rs
pub use messages::{Message, MessageRole, MessageContent, MessageAttributes};
pub use provider::LlmProvider;
pub use response::{Response, TokenUsage, FinishReason};
pub use error::LlmError;
```

**Request/Configuration types**:
```rust
pub use config::{
    Request, RequestConfig,
    Tool, ToolChoice, ToolCall, ToolResult,
    ResponseFormat,
};
```

**Provider configurations** (for construction):
```rust
pub use config::{
    OpenAIConfig, AnthropicConfig, OllamaConfig, LMStudioConfig,
};
```

**Provider implementations** (for construction):
```rust
pub use providers::{
    OpenAIProvider, AnthropicProvider, OllamaProvider, LMStudioProvider,
};
```

**Events** (feature-gated):
```rust
#[cfg(feature = "events")]
pub use events::{BusinessEvent, EventType, EventScope};
```

**Total: ~20 public types** (down from 28+)

### Internal APIs (Can change freely)

**Not exported** from `lib.rs`:
- `logging` module - Internal tracing macros (`pub(crate)`)
- `response_parser` module - Response parsing helpers (`pub(crate)`)
- `retry` internals - `CircuitBreaker`, `CircuitState`, `RetryExecutor` (`pub(crate)`)
- Error classification - `ErrorCategory`, `ErrorSeverity`, `UserErrorCategory`
- Internal types - `ToolCallingRound`
- Provider conversion modules - Implementation details
- HTTP client utilities - Internal plumbing
- `types` module - **Removed** (duplicates core types: `LLMToolCall`, `LLMUsage`, `LLMRequest`, `LLMMetadata`)

### Naming Changes

| Old (0.1.x) | New (1.0) | Rationale |
|-------------|-----------|-----------|
| `ExecutorLLMProvider` | `LlmProvider` | Remove "Executor" legacy prefix |
| `ExecutorLLMRequest` | `Request` | Simplify, scoped to module |
| `ExecutorLLMResponse` | `Response` | Simplify, scoped to module |
| `ExecutorLLMConfig` | `RequestConfig` | Clarify it's per-request, not per-provider |
| `ExecutorTool` | `Tool` | Simplify |
| `ExecutorToolCall` | `ToolCall` | Simplify |
| `ExecutorToolResult` | `ToolResult` | Simplify |
| `ExecutorTokenUsage` | `TokenUsage` | Simplify |
| `MyStoryError` trait | **Removed** | Parent project name, unnecessary trait |
| `AgentContext` | **Removed** | Parent project concept |
| `UnifiedLLMRequest` | `Request` | "Unified" is redundant in unified library |
| `UnifiedMessage` | `Message` | Keep or simplify (TBD) |

### Module Structure (After Cleanup)

```
src/
├── lib.rs               # Minimal re-exports (PUBLIC API)
├── messages.rs          # Message types (PUBLIC)
├── provider.rs          # LlmProvider trait (PUBLIC)
├── response.rs          # Response types (PUBLIC)
├── error.rs             # LlmError (PUBLIC)
├── config.rs            # Config types (PUBLIC)
├── providers/           # Implementations (INTERNAL, types public for construction)
│   ├── anthropic/
│   ├── openai/
│   ├── ollama/
│   └── lmstudio/
└── internals/           # NOT exported
    ├── retry.rs
    ├── tokens.rs
    ├── response_parser.rs
    └── events.rs        # Feature-gated
```

## Consequences

### Positive

- ✅ **Stability**: Small API surface = easier to maintain stability guarantees
- ✅ **Clarity**: Clear what's public (stable) vs internal (can change)
- ✅ **Generic naming**: No parent project references confusing users
- ✅ **No duplication**: One canonical type for each concept
- ✅ **Faster compilation**: Smaller public API = less to compile against
- ✅ **Better documentation**: Focused on what users actually need

### Negative

- ❌ **Breaking changes**: Existing 0.1.x users must update code
- ❌ **Migration work**: Need to provide migration guide
- ❌ **Less flexibility**: Internal types harder to access (intentional)

### Neutral

- ⚪ **Pre-1.0**: Changes acceptable before 1.0 release
- ⚪ **Documentation**: Must document all public types well

## Alternatives Considered

### Alternative 1: Keep Broad API, Deprecate Gradually

**Description**: Keep current exports, mark deprecated, remove in 2.0.

```rust
#[deprecated(since = "0.2.0", note = "Use `LlmProvider` instead")]
pub use core_types::ExecutorLLMProvider;

pub use provider::LlmProvider;
```

**Pros**:
- No immediate breaking changes
- Users can migrate gradually

**Cons**:
- Maintains complexity during 1.0 lifecycle
- Users might never migrate (deprecated code stays forever)
- Confusing to have two names for same thing

**Why not chosen**: Pre-1.0 is the right time for breaking changes. Clean start better than gradual deprecation.

### Alternative 2: Namespace-Based Exports

**Description**: Group exports in modules to reduce top-level clutter.

```rust
pub mod messages { /* ... */ }
pub mod providers { /* ... */ }
pub mod config { /* ... */ }

// No top-level re-exports
```

**Pros**:
- Organized namespacing
- Clear module boundaries

**Cons**:
- Verbose imports: `use multi_llm::messages::Message` vs `use multi_llm::Message`
- Less ergonomic for common types
- Rust convention is to re-export common types

**Why not chosen**: Top-level re-exports are more ergonomic. Users import from `multi_llm::*` for common types.

### Alternative 3: Prelude Pattern

**Description**: Provide prelude module with common imports.

```rust
pub mod prelude {
    pub use crate::{Message, LlmProvider, Response, LlmError};
}

// Users do: use multi_llm::prelude::*;
```

**Pros**:
- Common pattern in Rust ecosystem
- One import for most use cases

**Cons**:
- Wildcard imports discouraged in some codebases
- Still need to decide what goes in prelude

**Why not chosen**: For small library, direct re-exports from `lib.rs` sufficient. Can add prelude later if needed.

## Implementation Notes

### Migration Guide for 0.1.x Users

**Dependency update**:
```toml
# Before
[dependencies]
multi-llm = "0.1"

# After
[dependencies]
multi-llm = "1.0"
multi-llm = { version = "1.0", features = ["events"] }  # If using events
```

**Type renames**:
```rust
// Before (0.1.x)
use multi_llm::{ExecutorLLMProvider, ExecutorLLMRequest, ExecutorLLMResponse};

// After (1.0)
use multi_llm::{LlmProvider, Request, Response};
```

**Events**:
```rust
// Before (0.1.x)
let (response, events) = provider.execute_llm(request, config).await?;

// After (1.0) with events feature
let response = provider.execute(request, config).await?;
#[cfg(feature = "events")]
let events = response.events;

// After (1.0) without events feature
let response = provider.execute(request, config).await?;
// No events field
```

**Removed types**:
```rust
// Before: AgentContext
let context = AgentContext {
    agent_name: "MyAgent",
    // ...
};

// After: Build your own context type
struct AppContext {
    session_id: String,
    // ... whatever your app needs
}
```

### Stability Tiers Documentation

Add to rustdoc:
```rust
//! # Stability Guarantees
//!
//! ## Public API (Stable after 1.0)
//!
//! These types and traits are part of the stable public API.
//! Breaking changes require a major version bump (2.0).
//!
//! - [`Message`], [`MessageRole`], [`MessageContent`]
//! - [`LlmProvider`] trait
//! - [`Response`], [`TokenUsage`], [`FinishReason`]
//! - [`LlmError`]
//! - [`Request`], [`RequestConfig`]
//! - [`Tool`], [`ToolChoice`], [`ToolCall`]
//!
//! ## Provider Implementations (Semi-Stable)
//!
//! Provider types are public for construction but internal implementation
//! may change in minor versions:
//!
//! - [`OpenAIProvider`], [`AnthropicProvider`], etc.
//!
//! ## Internal APIs (Unstable)
//!
//! Not exported from `lib.rs`. Can change in any version.
```

### Pre-1.0 Checklist

- [x] Rename all `Executor*` types (Issue #1)
- [x] Remove `MyStoryError` trait (Issue #1)
- [x] Remove `AgentContext` (Issue #1)
- [x] Remove duplicate types (Issue #4: `LLMToolCall`, `LLMUsage`, `LLMRequest`, `LLMMetadata`)
- [x] Hide internals - `response_parser` (`pub(crate)`), `logging` (`pub(crate)`)
- [x] Hide retry internals - `CircuitBreaker`, `CircuitState`, `RetryExecutor` (`pub(crate)`)
- [x] Remove error classification from exports - `ErrorCategory`, `ErrorSeverity`, `UserErrorCategory`
- [x] Remove internal types from exports - `ToolCallingRound`
- [x] Feature-gate events (Issue #2)
- [x] Update documentation (DESIGN.md Section 6, ADR-006)
- [ ] Write migration guide
- [ ] Update examples
- [ ] Update integration tests

## References

- Rust API Guidelines - Naming: https://rust-lang.github.io/api-guidelines/naming.html
- Rust API Guidelines - Interoperability: https://rust-lang.github.io/api-guidelines/interoperability.html
- Cargo Book - SemVer Compatibility: https://doc.rust-lang.org/cargo/reference/semver.html

## Revision History

- 2025-11-25: Marked as Implemented, updated checklist with completed items
- 2025-11-22: Initial version
