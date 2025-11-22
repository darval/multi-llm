# Phase 3: Integration into Parent Project (Optional)

**Location**: This would be executed in a parent project repository if desired

This phase outlines how multi-llm could be integrated back into a larger project as an external dependency, if needed.

## Overview

Phase 3 integrates the standalone multi-llm library back into a parent project as an external dependency, replacing any inline LLM provider code.

This plan is **optional** and only relevant if multi-llm is being used within a larger workspace.

## Prerequisites

- Phase 1: ✅ Complete (multi-llm extracted)
- Phase 2: ⏳ In progress (cleanup and generalization)
- Phase 2 must be complete before Phase 3

## High-Level Steps

1. Add multi-llm as external dependency
2. Update core library to re-export multi-llm types (if needed for compatibility)
3. Archive any old inline LLM code
4. Update imports throughout project
5. Update documentation

## Detailed Steps

### Step 1: Add Dependency

**File**: Parent project `Cargo.toml`

```toml
[dependencies]
# Add multi-llm dependency
multi-llm = { version = "0.1", features = ["events"] }
```

Or if using local path during development:

```toml
[dependencies]
multi-llm = { path = "../multi-llm" }
```

**Core Library Cargo.toml** (if you have one):

```toml
[dependencies]
multi-llm = { version = "0.1", features = ["events"] }
```

### Step 2: Update Core Library Re-exports (if needed)

**File**: Core library `src/lib.rs`

```rust
// Re-export multi-llm types for backward compatibility
pub use multi_llm::{
    // Messages - the unified message architecture
    MessageAttributes, MessageCategory, MessageContent, MessageRole,
    UnifiedLLMRequest, UnifiedMessage,
    
    // Executor types
    ExecutorLLMConfig, ExecutorLLMProvider, ExecutorLLMResponse,
    ExecutorResponseFormat, ExecutorTool, ExecutorToolCall,
    ExecutorToolResult, ExecutorTokenUsage,
    
    // Configuration
    LLMConfig, AnthropicConfig, OpenAIConfig, OllamaConfig, LMStudioConfig,
    
    // Client
    UnifiedLLMClient,
    
    // Errors
    LlmError, LlmResult,
    ErrorCategory, ErrorSeverity,
    
    // Events (if using events feature)
    BusinessEvent, EventScope, LLMBusinessEvent,
};
```

### Step 3: Archive Old Code (if applicable)

If replacing inline LLM code:

```bash
# In parent repo
git mv old-llm-code old-llm-code-archived
```

**Workspace Cargo.toml Update**:

```toml
[workspace]
members = [
    "core",
    "agents",
    # "old-llm-code",  # REMOVED
    "storage",
    "server",
]
```

### Step 4: Update Imports Throughout Project

Search and replace imports:

```bash
# Find all old LLM imports
rg "use old_module::" --type rust

# Examples of conversions:
# old_module::UnifiedLLMClient → multi_llm::UnifiedLLMClient
# old_module::LLMConfig → multi_llm::LLMConfig
```

**Typical File Updates**:

Core library modules should mostly work via re-exports (see Step 2).

Main application/server code may need direct imports updated.

### Step 5: Update Executor (if applicable)

If you have an executor that uses LLM providers:

```rust
use multi_llm::UnifiedLLMClient;
use multi_llm::{ExecutorLLMProvider, UnifiedLLMRequest};

// Your executor implementation using multi_llm types
```

## Testing Integration

### Compilation Check

```bash
cd parent-project
cargo check --workspace
cargo build --workspace
```

### Run Tests

```bash
# Unit tests
cargo test --lib --workspace

# Integration tests
cargo test --tests --workspace

# Specific crate tests
cargo test -p core
cargo test -p agents
```

### Manual Testing

Test key workflows that use LLM functionality to ensure everything still works.

## Rollback Plan

If integration causes issues:

```bash
# Restore old code
git mv old-llm-code-archived old-llm-code

# Revert Cargo.toml changes
git checkout HEAD -- Cargo.toml */Cargo.toml

# Rebuild
cargo clean
cargo build
```

Keep archived code for a few weeks as safety net.

## Documentation Updates

Update relevant documentation:

1. **Architecture docs** - Note multi-llm is external library
2. **Setup guides** - Add multi-llm dependency installation
3. **Developer guides** - Update import examples

## Success Criteria

- [ ] Project compiles without errors
- [ ] All tests pass
- [ ] No old inline LLM code references remain (except archived)
- [ ] Documentation updated
- [ ] Manual testing successful
- [ ] Team reviewed changes

## Timeline Estimate

| Task | Duration | Notes |
|------|----------|-------|
| Add dependency | 15 min | Simple Cargo.toml changes |
| Update re-exports | 1-2 hours | Core library changes |
| Archive old code | 30 min | Git operations |
| Update imports | 2-4 hours | Depends on codebase size |
| Testing | 2-4 hours | Comprehensive verification |
| Documentation | 1-2 hours | Update guides and architecture docs |
| **Total** | **1-2 days** | Including review and fixes |

## Notes

- This phase is **optional** - multi-llm works fine standalone
- If multi-llm API changes in Phase 2, may need adapter layer
- Keep good version pinning to avoid surprise updates
- Consider feature flags if not all features needed

---

**Status**: Not yet started (Phase 2 in progress)
