# multi-llm

> **Status**: Phase 1 Complete - Extraction from myStory ✅
> **Next**: Phase 2 - Cleanup & Generalization (see [PHASE2_PLAN.md](PHASE2_PLAN.md))

Unified multi-provider LLM client with support for OpenAI, Anthropic, Ollama, and LMStudio.

## Current State

This library has been **successfully extracted** from the myStory project and is **~95% complete** as a standalone crate.

**What Works**:
- ✅ All source code copied from mystory-llm
- ✅ Core types extracted from mystory-core
- ✅ Dependencies updated (no more mystory-core or mystory-logging)
- ✅ Most imports updated
- ✅ Logging module created

**What Needs Fixing**:
- ⚠️ ~43 compilation errors (mostly import path issues)
- ⚠️ Some unused imports to clean up
- ⏳ Tests not yet run

## Phase 2 Next Steps

**Immediate** (< 1 hour):
1. Fix remaining import errors (see [PHASE2_PLAN.md](PHASE2_PLAN.md) Step 1)
2. Clean up unused imports
3. Verify compilation: `cargo check`
4. Run tests: `cargo test --lib`

**Main Tasks** (1-2 weeks):
1. Refactor `core_types/` into proper public API modules
2. Review and improve error handling
3. Review business events system (make optional?)
4. Create comprehensive documentation and examples
5. Prepare for crates.io publication

See **[PHASE2_PLAN.md](PHASE2_PLAN.md)** for complete details.

## Key Features

- **Multiple Providers**: Seamless switching between OpenAI, Anthropic, Ollama, and LMStudio
- **Unified Messages**: Provider-agnostic message architecture with caching hints (core feature!)
- **Prompt Caching**: Native support for Anthropic prompt caching
- **Tool Calling**: First-class function/tool calling support
- **Resilience**: Built-in retry logic, rate limiting, and error handling

## Compatibility

- **Rust Edition**: 2021
- **MSRV**: Rust 1.75 or later
- **Edition Compatibility**: Works with projects using any Rust edition (2015, 2018, 2021, 2024)

## Project Structure

```
multi-llm/
├── src/
│   ├── core_types/        # Extracted types from mystory-core (Phase 2: refactor)
│   │   ├── errors.rs      # Error traits and types
│   │   ├── messages.rs    # ⭐ Unified message architecture (PRIMARY FEATURE)
│   │   ├── executor.rs    # Executor types and LLM provider trait
│   │   └── events.rs      # Business event logging
│   ├── logging.rs         # Log macro re-exports (log_debug, log_error, etc.)
│   ├── client.rs          # UnifiedLLMClient
│   ├── config.rs          # Configuration types
│   ├── providers/         # Provider implementations
│   │   ├── anthropic/     # Anthropic Claude
│   │   ├── openai.rs      # OpenAI GPT
│   │   ├── ollama.rs      # Ollama (local models)
│   │   └── lmstudio.rs    # LM Studio
│   └── ...
├── tests/                 # Integration tests
├── EXTRACTION.md          # ✅ Phase 1 completion summary
├── PHASE2_PLAN.md         # 📋 Detailed Phase 2 tasks
└── PHASE3_PLAN.md         # 📋 Integration back into myStory
```

## Documentation

- **[EXTRACTION.md](EXTRACTION.md)** - What was done in Phase 1, current state, notes for Phase 2
- **[PHASE2_PLAN.md](PHASE2_PLAN.md)** - Detailed plan for cleanup and generalization
- **[PHASE3_PLAN.md](PHASE3_PLAN.md)** - Plan for integrating back into myStory

## Getting Started (Phase 2)

```bash
# Navigate to the project
cd /Users/rick/git/multi-llm

# Read the context
cat EXTRACTION.md
cat PHASE2_PLAN.md

# Fix compilation errors (see PHASE2_PLAN.md Step 1)
# Then verify:
cargo check
cargo test --lib
```

## Architecture Highlights

### Unified Message Architecture

The core innovation of multi-llm is the **unified message** system that treats all providers consistently:

```rust
use multi_llm::{UnifiedMessage, MessageRole, MessageContent, MessageAttributes};

// Simple message
let msg = UnifiedMessage::user("Hello!");

// With caching hints (for Anthropic)
let system_msg = UnifiedMessage::system_instruction(
    "You are a helpful assistant",
    Some("system-v1".to_string())  // Cache key
);

// With priority ordering
let context_msg = UnifiedMessage::context(
    "User context...",
    Some("user-context".to_string())
);
```

### Provider Abstraction

```rust
use multi_llm::{UnifiedLLMClient, LLMConfig, OpenAIConfig};

let config = LLMConfig::openai(OpenAIConfig {
    api_key: "your-key".to_string(),
    model: "gpt-4".to_string(),
    ..Default::default()
});

let client = UnifiedLLMClient::new(config)?;
```

## Testing Strategy

**Unit Tests**: `cargo test --lib` (~2305 tests from mystory-llm)
**Integration Tests**: `cargo test --tests` (~107 tests, some require Docker)

Some integration tests are marked with `#[ignore]` and require external services:
```bash
# Run ignored tests
cargo test -- --ignored
```

## Origin

This library was extracted from the [myStory](../mystory) project to be a standalone, reusable multi-provider LLM client. The extraction was research-driven using Serena tools to identify minimal dependencies while maintaining semantic compatibility.

**Extraction Date**: 2025-01-21
**Original Crate**: `mystory-llm`
**Phase 1**: ✅ Complete
**Phase 2**: 🚧 In Progress

## License

MIT OR Apache-2.0 (to be added in Phase 2)

## Contributing

Phase 2 is currently in progress. After completion, contribution guidelines will be added.

---

**For Phase 2 Contributors**: Start by reading [PHASE2_PLAN.md](PHASE2_PLAN.md) for detailed tasks and priorities.
