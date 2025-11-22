# Phase 2: Cleanup & Generalization Plan

## Context

Phase 1 extracted multi-llm as a standalone crate with minimal changes. Phase 2 will transform it into a production-ready, potentially crates.io-publishable library while maintaining semantic compatibility with the original project for easy Phase 3 migration.

## Immediate First Steps (Fix Compilation)

### Step 1: Fix Import Errors (~30 minutes)

**Problem**: Some files still import log macros from wrong paths.

**Files affected** (based on last compile check):
- Check all files for `use crate::core_types::log_*` imports
- Should be `use crate::{log_debug, log_error, log_warn, log_info}` instead

**Action**:
```bash
# Find files with incorrect imports
grep -r "use crate::core_types::log_" src/

# Fix pattern:
# FROM: use crate::core_types::log_debug;
# TO:   use crate::{log_debug, log_error, log_warn};
```

### Step 2: Clean Up Unused Imports (~15 minutes)

Remove unused `debug` imports from tracing:
- `src/providers/anthropic/provider.rs`
- `src/providers/openai.rs`
- `src/response_parser.rs`
- `src/retry.rs`
- `src/tokens.rs`

### Step 3: Verify Compilation (~5 minutes)

```bash
cargo check
cargo build --all-features
```

### Step 4: Run Tests (~10 minutes)

```bash
# Unit tests
cargo test --lib

# Integration tests (some may fail if they need Docker)
cargo test --tests
```

**Goal**: All tests that don't require Docker should pass.

---

## Main Phase 2 Tasks

### Task 2.1: Refactor core_types → Proper Public API Modules

**Duration**: 1-2 days

**Current**: `src/core_types/` contains extracted the original project types
**Target**: Move to proper top-level modules

**Actions**:
1. Move `core_types/messages.rs` → `src/messages.rs` (primary feature!)
2. Move `core_types/executor.rs` → `src/executor.rs`
3. Move `core_types/errors.rs` → `src/errors.rs`
4. Move `core_types/events.rs` → `src/events.rs` (or make optional)
5. Update all imports from `crate::core_types::` → `crate::`
6. Remove `src/core_types/` directory
7. Update `lib.rs` re-exports

**Rationale**: Core types should be first-class modules, not hidden in a sub-module.

---

### Task 2.2: Review and Improve Error Handling

**Duration**: 1 day

**Questions to Answer**:
1. Do we need the `the original projectError` trait, or is standard `Error` sufficient?
2. Should `ErrorCategory` and `ErrorSeverity` be kept or simplified?
3. Are `UserErrorCategory` variants too the original project-specific?

**Recommended Approach**:
- Keep trait-based approach similar to the original project for consistency
- Rename `the original projectError` → `LlmError` trait or similar
- Keep `ErrorCategory` and `ErrorSeverity` (useful for any LLM application)
- Review `UserErrorCategory` - keep generic ones, consider removing the original project-specific ones

**Files to Update**:
- `src/errors.rs` (formerly core_types/errors.rs)
- `src/error.rs` (the LlmError enum)
- Update documentation

---

### Task 2.3: Review Business Events System

**Duration**: 1 day

**Questions to Answer**:
1. Should business events be core to multi-llm or optional?
2. Are the event types generic enough for non-the original project use?
3. Should this be a feature flag?

**Options**:

**Option A: Make Optional (Recommended)**
```toml
[features]
events = []

[dependencies]
# Move event-related deps here if needed
```

**Option B: Keep as Core Feature**
- Rename event types to be more generic
- Better documentation about the events system
- Provide trait for custom event implementations

**Option C: Remove Entirely**
- Let consumers implement their own event logging
- Cleaner, more minimal library

**Recommendation**: Option A (feature flag) - maintains backward compatibility with the original project while being optional for others.

---

### Task 2.4: Enhance Logging System

**Duration**: 0.5 days

**Current**: `src/logging.rs` just re-exports tracing macros

**Enhancements**:
1. Document logging setup for consumers
2. Consider adding convenience functions for structured logging
3. Add examples of logging configuration

**Keep It Simple**: The current approach is fine for Phase 2. Don't over-engineer.

---

### Task 2.5: Unified Messages as Primary API

**Duration**: 1-2 days

**Goal**: Make `UnifiedMessage` and related types the star of the show

**Actions**:
1. Move `src/messages.rs` to be prominent in documentation
2. Add builder patterns for convenience:
   ```rust
   UnifiedMessage::user("Hello")
       .with_cache_key("user-greeting")
       .with_priority(10)
   ```
3. Add more examples in doc comments
4. Create `examples/message_construction.rs`

**Documentation to Add**:
- Why unified messages?
- How caching hints work
- Priority ordering explained
- Provider-agnostic benefits

---

### Task 2.6: Documentation & Examples

**Duration**: 2-3 days

**Required Files**:

**1. README.md** (comprehensive)
```markdown
# multi-llm

Unified multi-provider LLM client...

## Features
## Installation
## Quick Start
## Supported Providers
## Unified Messages (key feature)
## Examples
## Testing
## License
```

**2. examples/** directory
- `basic_usage.rs` - Simple request with OpenAI
- `provider_switching.rs` - Using multiple providers
- `tool_calling.rs` - Function calling example
- `anthropic_caching.rs` - Prompt caching with Anthropic
- `structured_output.rs` - JSON schema output
- `message_construction.rs` - Building unified messages

**3. CHANGELOG.md**
```markdown
# Changelog

## [0.1.0] - 2025-01-XX

### Added
- Initial extraction from the original project project
- Support for OpenAI, Anthropic, Ollama, LMStudio
- Unified message architecture with caching hints
- Tool/function calling support
```

**4. CONTRIBUTING.md** (if planning for contributions)

**5. API Documentation**
- Every public type, function, trait documented
- Examples in doc comments
- Link to examples where relevant

---

### Task 2.7: Prepare for crates.io Publication

**Duration**: 1 day

**Cargo.toml Checklist**:
- [ ] `version = "0.1.0"`
- [ ] `edition = "2021"`
- [ ] `license = "MIT OR Apache-2.0"` (add LICENSE files!)
- [ ] `repository = "https://github.com/your-username/multi-llm"`
- [ ] `documentation = "https://docs.rs/multi-llm"`
- [ ] `homepage` (optional)
- [ ] `keywords` (max 5)
- [ ] `categories` (from crates.io list)
- [ ] `description` (one-liner, max 100 chars)
- [ ] `readme = "README.md"`
- [ ] `authors` updated

**Add License Files**:
```bash
# Add MIT and Apache-2.0 license files
```

**Test Publishing**:
```bash
cargo publish --dry-run
```

**Review Output**:
- Check warnings
- Verify included files
- Ensure no secrets or private data

---

### Task 2.8: CI/CD Setup (Optional but Recommended)

**Duration**: 0.5-1 day

**If using GitHub**:
`.github/workflows/ci.yml`:
```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo fmt -- --check
      - run: cargo clippy -- -D warnings
      - run: cargo test --all-features
```

---

## Testing Strategy for Phase 2

### After Each Major Change:
1. `cargo check` - Fast compilation check
2. `cargo test --lib` - Unit tests
3. `cargo test --tests` - Integration tests
4. `cargo clippy -- -D warnings` - Linting
5. `cargo fmt -- --check` - Format check

### Before Completing Phase 2:
```bash
# Full test suite
./scripts/test-all.sh  # If exists from the original project, or:
cargo test --all-features --all-targets

# Documentation test
cargo doc --all-features --no-deps

# Dry-run publish
cargo publish --dry-run
```

---

## Success Criteria for Phase 2

### Must Have:
- [x] Compiles without errors or warnings
- [ ] All tests pass (except Docker-dependent ones)
- [ ] Comprehensive README.md
- [ ] At least 3 working examples
- [ ] All public API documented
- [ ] CHANGELOG.md created
- [ ] License files added
- [ ] `cargo publish --dry-run` succeeds

### Should Have:
- [ ] Core types refactored out of core_types/ module
- [ ] Business events reviewed (optional or removed)
- [ ] Error handling reviewed and improved
- [ ] CI/CD workflow set up

### Nice to Have:
- [ ] Additional examples
- [ ] Performance benchmarks
- [ ] Comparison with other LLM libraries

---

## Phase 3 Compatibility Notes

**Goal**: the original project should migrate with minimal changes

**What the original project Will Need to Do** (keep in mind):
1. Update imports: `the original project_core::messages` → `multi_llm::messages`
2. Update Cargo.toml: Replace multi-llm with multi-llm dependency
3. Re-export multi-llm types in external core library for backward compatibility
4. Update documentation references

**Ensure**:
- Type definitions remain semantically identical
- Function signatures unchanged where possible
- If breaking changes needed, document them clearly in CHANGELOG

---

## Open Questions for Phase 2 Session

1. **Project Name**: Is "multi-llm" final, or consider alternatives like "unified-llm"?
   - Current lean: "multi-llm" (highlights multiple provider support)

2. **License**: MIT, Apache-2.0, or dual license?
   - Recommendation: "MIT OR Apache-2.0" (Rust ecosystem standard)

3. **Repository**: Where will this be hosted?
   - Need to update repository URL in Cargo.toml

4. **Business Events**: Keep, make optional, or remove?
   - Recommendation: Feature flag (Option A above)

5. **Trait Name**: Keep `ExecutorLLMProvider` or rename to something simpler?
   - Consider: `LlmProvider` (simpler, this is now multi-llm's trait)

6. **Author Information**: Update in Cargo.toml

---

## Timeline Estimate

| Task | Duration | Priority |
|------|----------|----------|
| Fix compilation | 1 hour | Critical |
| Run tests | 1 hour | Critical |
| Refactor core_types | 1-2 days | High |
| Review errors | 1 day | High |
| Review events | 1 day | Medium |
| Documentation | 2-3 days | High |
| Examples | 1-2 days | High |
| Prepare for publish | 1 day | Medium |
| CI/CD | 0.5-1 day | Low |

**Total**: 7-12 days (1.5-2.5 weeks)

---

## Getting Started with Phase 2

**First Steps**:
1. Read [EXTRACTION.md](EXTRACTION.md) for context
2. Fix compilation errors (see Step 1 above)
3. Run tests to establish baseline
4. Review this plan and adjust priorities based on goals
5. Start with Task 2.1 (refactor core_types)

**Key Files to Understand**:
- `src/lib.rs` - Main entry point and re-exports
- `src/core_types/messages.rs` - The core unified message architecture (PRIMARY FEATURE)
- `src/client.rs` - Main UnifiedLLMClient
- `src/providers/` - Individual provider implementations

**Testing Approach**:
- Make small changes
- Test frequently
- Commit working states
- Use `cargo watch -x check` for fast feedback

---

*Good luck with Phase 2! The hard extraction work is done. Now make it shine!*
