# ADR-001: Unified Message Architecture

**Status**: Accepted
**Date**: 2025-11-22
**Authors**: multi-llm team
**Deciders**: Rick Duff

## Context

The library needs to support multiple LLM providers (OpenAI, Anthropic, Ollama, LM Studio) with different message formats:

- **OpenAI**: `{"role": "user", "content": "text"}` format, supports tools
- **Anthropic**: Similar but with `cache_control` blocks for prompt caching
- **Ollama**: Simplified format, variable tool support depending on model
- **LM Studio**: OpenAI-compatible but with limitations

Users want to write application code once and switch providers via configuration, not code changes. This requires a provider-agnostic message format.

**Constraints**:
- Must support all common message types: text, tool calls, tool results
- Must support provider-specific features (like Anthropic caching) without breaking abstraction
- Conversions must be lossless (round-trip without data loss)
- Performance: Minimal allocation overhead in conversions

**Forces**:
- **Abstraction vs Features**: Too abstract = can't use provider-specific features; too specific = defeats purpose
- **Type Safety vs Flexibility**: Strongly typed = catches errors early; flexible = harder to use incorrectly
- **Ergonomics vs Explicitness**: Builder patterns are nice but add code; explicit construction is verbose

## Decision

We will use a **single unified message format** that:

1. **Supports all common patterns** through enum-based content types
2. **Extends via metadata** for provider-specific features
3. **Provides builder methods** for ergonomic construction

**Core types**:

```rust
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

pub enum MessageContent {
    Text(String),
    ToolCall(ToolCallContent),
    ToolResult(ToolResultContent),
}

pub struct MessageAttributes {
    pub cache_control: Option<CacheControl>,  // Anthropic caching
    pub priority: i32,                        // Message ordering
    pub metadata: HashMap<String, Value>,     // Provider-specific escape hatch
}

pub struct Message {
    pub role: MessageRole,
    pub content: MessageContent,
    pub attributes: MessageAttributes,
}
```

**Builder pattern**:
```rust
Message::user("Hello")
    .cacheable()
    .with_priority(10)
    .build()
```

**Conversion responsibility**: Each provider implements conversions from `Message` to their native format and vice versa.

## Consequences

### Positive

- ✅ **Provider switching**: Change config, not code - same `Message` works everywhere
- ✅ **Type safety**: `MessageRole` and `MessageContent` enums catch mistakes at compile time
- ✅ **Extensibility**: New providers can map to existing types; new features add to `MessageAttributes`
- ✅ **Ergonomics**: Builder pattern makes common cases concise
- ✅ **Feature parity**: Anthropic caching works via `cache_control`, doesn't affect other providers

### Negative

- ❌ **Conversion overhead**: Every request/response requires type conversion (but allocations are minimal)
- ❌ **Abstraction leaks**: Some provider features hard to expose (e.g., Anthropic's thinking blocks)
- ❌ **Learning curve**: Users must learn our types, not just provider APIs

### Neutral

- ⚪ **Maintenance**: New providers need conversion implementations (but pattern is established)
- ⚪ **Testing**: Conversions must be tested for each provider (increases test surface)

## Alternatives Considered

### Alternative 1: Per-Provider Message Types

**Description**: Each provider exposes its native message format; users construct provider-specific types.

```rust
// OpenAI
let msg = OpenAIMessage { role: "user", content: "hello" };

// Anthropic
let msg = AnthropicMessage { role: "user", content: vec![TextBlock { text: "hello" }] };
```

**Pros**:
- No conversion overhead
- Users can access 100% of provider features directly
- Simple implementation (no abstraction layer)

**Cons**:
- Switching providers requires rewriting all message construction code
- Users must learn every provider's API
- No code reuse across providers

**Why not chosen**: Defeats the primary goal of provider-agnostic code. Users could just use provider SDKs directly if they want this.

### Alternative 2: Trait-Based Polymorphism

**Description**: Define a `Message` trait; providers implement it with their types.

```rust
pub trait Message {
    fn role(&self) -> MessageRole;
    fn content(&self) -> &str;
}

impl Message for OpenAIMessage { /* ... */ }
impl Message for AnthropicMessage { /* ... */ }
```

**Pros**:
- Polymorphic code via trait objects
- Each provider can optimize its own representation

**Cons**:
- Trait objects require `Box<dyn Message>` (heap allocation, dynamic dispatch)
- Can't pattern match on concrete types
- Awkward to construct (which concrete type do users create?)
- Tool calls and results don't fit trait abstraction well

**Why not chosen**: Too much indirection, doesn't leverage Rust's strength (sum types/enums). Less ergonomic than concrete types.

### Alternative 3: JSON-Based Messages

**Description**: Messages are just `serde_json::Value`, conversions happen via serialization.

```rust
let msg = json!({
    "role": "user",
    "content": "hello"
});
```

**Pros**:
- Maximally flexible
- Easy to add fields
- Mirrors provider APIs closely

**Cons**:
- **No type safety**: Easy to construct invalid messages (caught at runtime, not compile time)
- No IDE autocomplete
- Conversion errors happen at runtime (serialization failures)
- Harder to document (what fields are valid?)

**Why not chosen**: Sacrifices Rust's type safety for flexibility. Runtime errors are expensive (debugging, user experience). Doesn't leverage the type system.

## Implementation Notes

**Conversion pattern** (example for OpenAI):
```rust
// Message -> OpenAI format
fn convert_to_openai(msg: &Message) -> OpenAIMessage {
    match &msg.content {
        MessageContent::Text(text) => OpenAIMessage {
            role: msg.role.to_string(),
            content: text.clone(),
            ..Default::default()
        },
        MessageContent::ToolCall(tc) => OpenAIMessage {
            role: "assistant",
            tool_calls: Some(vec![convert_tool_call(tc)]),
            ..Default::default()
        },
        // ... other variants
    }
}
```

**Cache control handling**:
```rust
// Only Anthropic uses this; other providers ignore it
if let Some(cache_control) = &msg.attributes.cache_control {
    anthropic_msg.cache_control = Some(anthropic_cache_control_from(cache_control));
}
```

## References

- OpenAI Chat API: https://platform.openai.com/docs/api-reference/chat
- Anthropic Messages API: https://docs.anthropic.com/claude/reference/messages
- Original design discussion: (link to issue/PR when available)

## Revision History

- 2025-11-22: Initial version
