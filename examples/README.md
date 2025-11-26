# multi-llm Examples

Runnable examples demonstrating all major features of the multi-llm library.

## Quick Start

```bash
# List all available examples
cargo run --example 2>&1 | grep -A 100 "Available examples"

# Run a specific example
cargo run --example basic_openai
```

## Examples Overview

### Basic Provider Examples

Simple request/response patterns for each supported provider.

| Example | Provider | Required Env Var |
|---------|----------|------------------|
| `basic_openai` | OpenAI | `OPENAI_API_KEY` |
| `basic_anthropic` | Anthropic | `ANTHROPIC_API_KEY` |
| `basic_ollama` | Ollama | None (local) |
| `basic_lmstudio` | LM Studio | None (local) |

**Usage:**
```bash
export OPENAI_API_KEY="sk-..."
cargo run --example basic_openai
```

### Feature Examples

Examples demonstrating specific features.

| Example | Feature | Required Env Var |
|---------|---------|------------------|
| `provider_switching` | Same code, different providers | `OPENAI_API_KEY` or `ANTHROPIC_API_KEY` |
| `multi_instance` | Multiple simultaneous LLMs | `ANTHROPIC_API_KEY` |
| `prompt_caching` | Anthropic prompt caching | `ANTHROPIC_API_KEY` |
| `tool_calling` | Function/tool calling | `ANTHROPIC_API_KEY` (default) or `OPENAI_API_KEY` |
| `error_handling` | Error types and retry logic | None (demo only) |

## Example Details

### basic_openai / basic_anthropic / basic_ollama / basic_lmstudio

Shows the minimal setup for each provider:
- Creating provider configuration
- Building a simple conversation
- Executing a request
- Reading the response and token usage

### provider_switching

Demonstrates the unified API - same application code works with different providers:
```rust
// Configuration determines the provider, code stays the same
let client = UnifiedLLMClient::from_config(config)?;
let response = client.execute_llm(request, None, None).await?;
```

### prompt_caching

Demonstrates Anthropic's prompt caching feature for 90% cost savings:
- Ephemeral cache (5-minute TTL, 1.25x write cost)
- Extended cache (1-hour TTL, 2x write cost)
- Cache hit/miss tracking with the `events` feature

**Note:** Requires minimum token thresholds:
- Claude 3.5 Sonnet: 1,024 tokens
- Claude 3.5 Haiku: 2,048 tokens

```bash
# Run with events feature to see cache statistics
cargo run --example prompt_caching --features events
```

### multi_instance

Demonstrates using multiple LLM instances simultaneously:

1. **DualLLMConfig**: Separate configs for user-facing vs background NLP
2. **Parallel requests**: Using `tokio::join!` for concurrent execution
3. **Multiple providers**: Mix cloud (Anthropic) and local (Ollama) models
4. **Draft & polish pattern**: Fast model for drafts, smart model for refinement

```bash
cargo run --example multi_instance

# With local Ollama support
OLLAMA_MODEL=qwen2.5-coder cargo run --example multi_instance
```

### tool_calling

Complete tool/function calling workflow:
1. Define tools with JSON Schema parameters
2. Send request with tools attached
3. Handle tool calls from LLM response
4. Execute tools and return results
5. Continue conversation with tool results

### error_handling

Demonstrates the rich error handling API:
- Error types and their categories
- `error.category()` for routing decisions
- `error.is_retryable()` for retry logic
- `error.user_message()` for safe user-facing messages

```bash
# Run demo without API calls
cargo run --example error_handling

# Run with live API test (uses invalid key to trigger error)
cargo run --example error_handling -- --live
```

## Running Examples

### Cloud Providers (OpenAI, Anthropic)

1. Set the required environment variable:
   ```bash
   export OPENAI_API_KEY="sk-..."
   # or
   export ANTHROPIC_API_KEY="sk-ant-..."
   ```

2. Run the example:
   ```bash
   cargo run --example basic_openai
   ```

### Local Providers (Ollama, LM Studio)

1. Start the local server:
   - **Ollama:** `ollama serve` (default port 11434)
   - **LM Studio:** Start the server in LM Studio (default port 1234)

2. Load a model:
   - **Ollama:** `ollama pull llama2`
   - **LM Studio:** Load a model in the GUI

3. Run the example:
   ```bash
   cargo run --example basic_ollama
   # or
   cargo run --example basic_lmstudio
   ```

## Building All Examples

```bash
# Build without events feature
cargo build --examples

# Build with events feature
cargo build --examples --features events
```

## Events Feature

Some examples support the `events` feature for detailed metrics:

```bash
cargo run --example prompt_caching --features events
```

This enables:
- Cache hit/miss tracking
- Token usage by cache type
- Request/response event logging
