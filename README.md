# multi-llm

> Unified multi-provider LLM client library for Rust

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

A type-safe, async-first Rust library providing a unified interface for multiple Large Language Model providers. Write your code once, switch providers with a configuration change.

## Features

- 🔄 **Multi-Provider Support**: OpenAI, Anthropic, Ollama, LM Studio
- 🎯 **Unified Message Format**: Provider-agnostic message architecture
- ⚡ **Multiple Instances**: Run 1-N provider connections concurrently (even multiple instances of the same provider)
- 🎨 **Type-Safe**: Leverage Rust's type system to catch errors at compile time
- 🚀 **Async-First**: Built on Tokio for high-performance async I/O
- 💾 **Prompt Caching**: Native support for Anthropic's 5-minute and 1-hour caching
- 🔧 **Tool Calling**: First-class function/tool calling support
- 📊 **Optional Events**: Feature-gated business event logging for observability
- 🎚️ **KISS Principle**: Simple, maintainable solutions over complex abstractions

## Quick Start

```rust
use multi_llm::{Message, Request, OpenAIProvider, OpenAIConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = OpenAIConfig {
        api_key: std::env::var("OPENAI_API_KEY")?,
        model: "gpt-4".to_string(),
        ..Default::default()
    };

    let provider = OpenAIProvider::new(config)?;

    let request = Request {
        messages: vec![
            Message::user("What is the capital of France?"),
        ],
        config: None,
    };

    let response = provider.execute(request, None).await?;
    println!("Response: {}", response.content);

    Ok(())
}
```

## Multi-Provider Example

Switch between providers without code changes:

```rust
use multi_llm::{LlmProvider, AnthropicProvider, OpenAIProvider};

async fn ask_llm(provider: &dyn LlmProvider, question: &str) -> Result<String> {
    let request = Request {
        messages: vec![Message::user(question)],
        config: None,
    };
    let response = provider.execute(request, None).await?;
    Ok(response.content)
}

// Works with any provider
let openai = OpenAIProvider::new(openai_config)?;
let anthropic = AnthropicProvider::new(anthropic_config)?;

let answer1 = ask_llm(&openai, "What is 2+2?").await?;
let answer2 = ask_llm(&anthropic, "What is 2+2?").await?;
```

## Multi-Instance Pattern

Run multiple instances of the same provider with different configurations:

```rust
// Fast model for simple tasks
let anthropic_fast = AnthropicProvider::new(AnthropicConfig {
    model: "claude-3-haiku-20240307".to_string(),
    ..Default::default()
})?;

// Powerful model for complex tasks with 1-hour caching
let anthropic_smart = AnthropicProvider::new(AnthropicConfig {
    model: "claude-3-opus-20240229".to_string(),
    cache_ttl: Some("1h".to_string()),
    ..Default::default()
})?;
```

## Prompt Caching

Reduce costs with Anthropic's prompt caching (both 5-minute and 1-hour):

```rust
let msg = Message::user("Large context to cache")
    .with_cache_control(CacheControl::extended())  // 1-hour cache
    .build();
```

## Tool Calling

```rust
let tools = vec![
    Tool {
        name: "get_weather".to_string(),
        description: "Get current weather".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"}
            }
        }),
    },
];

let config = RequestConfig {
    tools,
    tool_choice: Some(ToolChoice::Auto),
    ..Default::default()
};

let response = provider.execute(request, Some(config)).await?;
```

## Optional Events

Enable business event logging for observability:

```toml
[dependencies]
multi-llm = { version = "0.1", features = ["events"] }
```

```rust
#[cfg(feature = "events")]
{
    for event in response.events {
        match event.event_type {
            EventType::CacheHit { tokens_saved } => {
                println!("Cache saved {} tokens", tokens_saved);
            }
            EventType::TokenUsage { prompt, completion } => {
                println!("Used {} + {} tokens", prompt, completion);
            }
            _ => {}
        }
    }
}
```

## Documentation

- **[Design Document](docs/DESIGN.md)** - Comprehensive architecture and design decisions
- **[Architecture Decision Records](docs/adr/)** - Detailed rationale for major decisions
- **[Phase 2 Plan](docs/PHASE2_PLAN.md)** - Current refactoring tasks
- **[Phase 3 Plan](docs/PHASE3_PLAN.md)** - Future integration plans

## Supported Providers

| Provider | Status | Caching | Tools | Streaming* |
|----------|--------|---------|-------|------------|
| **Anthropic** | ✅ | ✅ (5m + 1h) | ✅ | Post-1.0 |
| **OpenAI** | ✅ | ❌ | ✅ | Post-1.0 |
| **Ollama** | ✅ | ❌ | ⚠️ | Post-1.0 |
| **LM Studio** | ✅ | ❌ | ⚠️ | Post-1.0 |

*Streaming support deferred to post-1.0 release

## Design Philosophy

1. **KISS**: Simplicity over complexity - simple solutions are maintainable
2. **Multi-Provider by Design**: 1-N concurrent connections via config, not code
3. **Library-First**: Pure library with no application assumptions
4. **Type Safety**: Leverage Rust's type system to prevent errors
5. **Minimal Dependencies**: Every dependency impacts downstream users

See [Design Document](docs/DESIGN.md) for detailed philosophy and architecture.

## Project Status

**Current Phase**: Pre-1.0 Cleanup & Stabilization

**What Works**:
- ✅ All provider implementations (OpenAI, Anthropic, Ollama, LM Studio)
- ✅ Unified message architecture with caching hints
- ✅ Tool calling support
- ✅ Async I/O with Tokio
- ✅ Comprehensive error handling

**Pre-1.0 Tasks** (see [Phase 2 Plan](docs/PHASE2_PLAN.md)):
- 🔄 Remove legacy naming (`Executor*` → simpler names)
- 🔄 Feature-gate events system
- 🔄 Narrow public API surface
- 🔄 Remove parent project references
- 🔄 Comprehensive documentation and examples

## Requirements

- **Rust**: 1.75 or later
- **Edition**: 2021
- **Tokio**: Async runtime required

## Installation

```toml
[dependencies]
multi-llm = "0.1"

# With events feature
multi-llm = { version = "0.1", features = ["events"] }
```

## Testing

```bash
# Unit tests (fast)
cargo test --lib

# Integration tests (some require external services)
cargo test --tests

# Include ignored tests (require API keys)
cargo test -- --ignored
```

## Contributing

Contributions welcome! Before contributing:

1. Read the [Design Document](docs/DESIGN.md)
2. Review [Architecture Decision Records](docs/adr/)
3. Follow established patterns
4. Add tests for new functionality
5. Use `log_*!` macros for logging (not `println!`)

See [Appendix D: Contributing](docs/DESIGN.md#appendix-d-contributing) for detailed guidelines.

## Compatibility

Works with projects using any Rust edition (2015, 2018, 2021, 2024).

## License

MIT OR Apache-2.0

## Acknowledgments

Extracted from production use in myStory, refined as a standalone library.

---

**Status**: Pre-1.0 (Breaking changes expected before 1.0 release)
