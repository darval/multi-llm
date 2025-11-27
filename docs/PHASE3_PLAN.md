# Phase 3: myStory Migration to multi-llm

> **Status**: Planning
> **Location**: Execute in `../mystory` repository
> **Prerequisites**: Phase 2 complete, multi-llm v0.1.2+ available

## Overview

Phase 3 migrates the myStory project from its internal `mystory-llm` crate to using `multi-llm` as an external dependency. This completes the extraction cycle: myStory → multi-llm → back to myStory as dependency.

## Current State

### myStory's mystory-llm Crate

**Location**: `../mystory/mystory-llm/`

**Dependents** (crates that import mystory-llm):
- `mystory-agents` - Agent implementations
- `mystory-server` - HTTP server
- `mystory-exercisor` - Testing utility

**Key Exports from mystory-llm**:
```rust
// Client
pub use client::UnifiedLLMClient;

// Config
pub use config::{
    AnthropicConfig, DefaultLLMParams, DualLLMConfig, LLMConfig, LLMPath,
    LMStudioConfig, OllamaConfig, OpenAIConfig, ProviderConfig,
};

// Errors
pub use error::{LlmError, LlmResult};

// Providers
pub use providers::{AnthropicProvider, LMStudioProvider, OllamaProvider, OpenAIProvider};

// Token counting
pub use tokens::{AnthropicTokenCounter, OpenAITokenCounter, TokenCounter, TokenCounterFactory};

// Re-exports from mystory-core
pub use mystory_core::messages::{MessageContent, MessageRole, UnifiedMessage};
pub use mystory_core::executor::{ExecutorTool as Tool, ExecutorToolCall, ExecutorToolResult};

// Local types
pub use types::{LLMMetadata, LLMRequest, LLMToolCall, LLMUsage};
```

### multi-llm Equivalent Exports

All mystory-llm exports have equivalents in multi-llm:

| mystory-llm | multi-llm | Notes |
|-------------|-----------|-------|
| `UnifiedLLMClient` | `UnifiedLLMClient` | Same |
| `LLMConfig` | `LLMConfig` | Same |
| `AnthropicConfig` | `AnthropicConfig` | Same |
| `OpenAIConfig` | `OpenAIConfig` | Same |
| `OllamaConfig` | `OllamaConfig` | Same |
| `LMStudioConfig` | `LMStudioConfig` | Same |
| `DefaultLLMParams` | `DefaultLLMParams` | Same |
| `LlmError` | `LlmError` | Same |
| `LlmResult` | `LlmResult` | Same |
| `AnthropicProvider` | `AnthropicProvider` | Same |
| `OpenAIProvider` | `OpenAIProvider` | Same |
| `OllamaProvider` | `OllamaProvider` | Same |
| `LMStudioProvider` | `LMStudioProvider` | Same |
| `UnifiedMessage` | `UnifiedMessage` | Same |
| `MessageRole` | `MessageRole` | Same |
| `MessageContent` | `MessageContent` | Same |
| `TokenCounter` | `TokenCounter` | Same |
| `ExecutorTool` | `Tool` | Renamed |
| `ExecutorToolCall` | `ToolCall` | Renamed |
| `ExecutorToolResult` | `ToolResult` | Renamed |
| `LLMUsage` | `TokenUsage` | Renamed |

---

## Migration Steps

### Step 1: Add multi-llm Dependency

**File**: `mystory/Cargo.toml` (workspace root)

```toml
[workspace.dependencies]
# Remove or comment out mystory-llm
# mystory-llm = { path = "mystory-llm" }

# Add multi-llm
multi-llm = { version = "0.1.2", features = ["events"] }
```

### Step 2: Update Dependent Crates

#### mystory-agents/Cargo.toml

```toml
[dependencies]
# Remove
# mystory-llm = { path = "../mystory-llm" }

# Add
multi-llm = { workspace = true }
```

#### mystory-server/Cargo.toml

```toml
[dependencies]
# Remove
# mystory-llm = { path = "../mystory-llm" }

# Add
multi-llm = { workspace = true }
```

#### mystory-exercisor/Cargo.toml

```toml
[dependencies]
# Remove
# mystory-llm = { path = "../mystory-llm" }

# Add
multi-llm = { workspace = true }
```

### Step 3: Update Imports

#### Search and Replace Patterns

```bash
# Find all mystory_llm imports
rg "use mystory_llm::" --type rust

# Find all mystory-llm in Cargo.toml files
rg "mystory-llm" --type toml
```

#### Import Mappings

**Basic pattern**:
```rust
// Before
use mystory_llm::{UnifiedLLMClient, LLMConfig, AnthropicConfig};

// After
use multi_llm::{UnifiedLLMClient, LLMConfig, AnthropicConfig};
```

**Tool types** (renamed):
```rust
// Before
use mystory_llm::Tool;  // Was ExecutorTool
use mystory_llm::ExecutorToolCall;
use mystory_llm::ExecutorToolResult;

// After
use multi_llm::{Tool, ToolCall, ToolResult};
```

**Usage types** (renamed):
```rust
// Before
use mystory_llm::types::LLMUsage;

// After
use multi_llm::TokenUsage;
```

**Message types** (may have been from mystory-core):
```rust
// Before (if from mystory-core)
use mystory_core::messages::{UnifiedMessage, MessageRole, MessageContent};

// After
use multi_llm::{UnifiedMessage, MessageRole, MessageContent};
```

### Step 4: Handle mystory-core Dependencies

mystory-llm currently re-exports some types from mystory-core. After migration:

**Option A: Import directly from multi-llm** (recommended)
```rust
// All message and tool types now come from multi-llm
use multi_llm::{
    UnifiedMessage, MessageRole, MessageContent, MessageAttributes,
    Tool, ToolCall, ToolResult,
};
```

**Option B: Keep mystory-core for other types**

If mystory-core has other types not in multi-llm (like `AgentContext`), keep using mystory-core for those:
```rust
use multi_llm::{UnifiedMessage, MessageRole};  // LLM types
use mystory_core::agents::AgentContext;         // myStory-specific types
```

### Step 5: Update mystory-core (if needed)

If mystory-core currently defines message types that are now in multi-llm:

**Option A: Re-export from multi-llm**
```rust
// mystory-core/src/lib.rs
pub use multi_llm::{UnifiedMessage, MessageRole, MessageContent};
```

**Option B: Remove duplicates**

If mystory-core defined its own message types, remove them and use multi-llm's.

### Step 6: Remove mystory-llm from Workspace

**File**: `mystory/Cargo.toml`

```toml
[workspace]
members = [
    "mystory-core",
    "mystory-agents",
    # "mystory-llm",  # REMOVED
    "mystory-server",
    "mystory-exercisor",
    # ... other crates
]
```

### Step 7: Archive mystory-llm

```bash
cd ../mystory

# Option A: Move to archive directory
mkdir -p archived
mv mystory-llm archived/

# Option B: Delete (if confident)
rm -rf mystory-llm
```

---

## Verification Checklist

### Compilation

```bash
cd ../mystory

# Clean build
cargo clean
cargo build --workspace

# Should compile without errors
```

### Tests

```bash
# Run all tests
cargo test --workspace

# Run integration tests
cargo test --workspace -- --ignored
```

### Specific Checks

- [ ] `mystory-agents` compiles and tests pass
- [ ] `mystory-server` compiles and tests pass
- [ ] `mystory-exercisor` compiles and tests pass
- [ ] No references to `mystory_llm::` remain
- [ ] No `mystory-llm` in any Cargo.toml
- [ ] Server starts and handles LLM requests
- [ ] Prompt caching still works (Anthropic)
- [ ] Tool calling still works

---

## Potential Issues and Solutions

### Issue: AgentContext Not in multi-llm

**Symptom**: `AgentContext` import fails

**Solution**: `AgentContext` was myStory-specific. Keep it in mystory-core or mystory-agents:
```rust
// It was never meant to be in multi-llm
use mystory_core::agents::AgentContext;  // or wherever it lives now
```

### Issue: Type Mismatches

**Symptom**: Type errors when passing messages between crates

**Solution**: Ensure all crates use multi-llm types consistently:
```rust
// All crates should import from multi-llm, not re-exports
use multi_llm::UnifiedMessage;
```

### Issue: Feature Flag Differences

**Symptom**: Events not available

**Solution**: Ensure `features = ["events"]` is specified:
```toml
multi-llm = { version = "0.1.2", features = ["events"] }
```

### Issue: Local Types in mystory-llm

**Symptom**: `LLMRequest`, `LLMMetadata` not found

**Solution**: These were local convenience types. Either:
1. Recreate them in mystory-agents/mystory-core if needed
2. Refactor code to not need them (they were thin wrappers)

```rust
// LLMRequest was just:
pub struct LLMRequest {
    pub user_input: String,
    pub add_to_history: bool,
}

// Can be inlined or moved to mystory-core
```

---

## Rollback Plan

If migration fails:

```bash
cd ../mystory

# Restore mystory-llm to workspace
git checkout -- Cargo.toml
git checkout -- mystory-llm/

# Or if archived:
mv archived/mystory-llm .

# Rebuild
cargo clean
cargo build --workspace
```

---

## Post-Migration Tasks

1. **Update myStory documentation** to reference multi-llm
2. **Update any scripts** that referenced mystory-llm
3. **Consider contributing back** any myStory-specific improvements to multi-llm
4. **Monitor for issues** in production after deployment

---

## Timeline Estimate

| Task | Duration | Notes |
|------|----------|-------|
| Update Cargo.toml files | 15 min | Simple dependency changes |
| Update imports | 1-2 hours | Depends on codebase size |
| Fix type mismatches | 1-2 hours | Renamed types need attention |
| Run tests | 30 min | Full test suite |
| Manual verification | 1 hour | Test key workflows |
| **Total** | **4-6 hours** | For a focused session |

---

## Success Criteria

- [ ] All myStory crates compile with multi-llm
- [ ] All tests pass
- [ ] No mystory-llm references remain
- [ ] Server functions correctly with LLM features
- [ ] Prompt caching works
- [ ] Tool calling works
- [ ] mystory-llm archived or deleted

---

**Phase 3 completes the extraction cycle, making multi-llm a true standalone library while myStory becomes a consumer of that library.**
