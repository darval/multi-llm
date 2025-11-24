# ADR-003: Caching Hints Architecture

**Status**: Accepted
**Date**: 2025-11-22
**Authors**: multi-llm team
**Deciders**: Rick Duff

## Context

Anthropic Claude supports prompt caching to reduce costs and latency when reusing large prompt contexts. This feature allows portions of prompts to be cached and reused across requests, significantly reducing token usage for repeated content.

**Anthropic's caching system**:
- **Ephemeral cache**: 5-minute TTL, 1.25x write cost (25% premium), 0.1x read cost (90% savings)
- **Extended cache**: 1-hour TTL, 2x write cost (100% premium), 0.1x read cost (90% savings)
- Cache control blocks attached to specific messages
- Both cache types offer 90% savings on reads; extended cache has higher upfront cost but longer TTL
- Used in production by myStory with extended cache for long-lived contexts

**Observability requirement**: Caching is a cost optimization feature that requires monitoring to tune effectively. The events system (see [ADR-005](./005-events-system.md)) captures cache statistics (cache hits, cache writes, tokens saved) to enable:
- **Cost analysis**: Understanding actual savings from caching
- **Cache tuning**: Identifying which content benefits most from caching
- **Performance monitoring**: Tracking cache hit rates over time
- **Visualization**: Displaying cache efficiency in dashboards

While the events system is optional (feature-gated), caching hints are always available - the two features work together but are not coupled.

**Challenge**: How to expose provider-specific caching in a provider-agnostic library?

**Forces**:
- **Provider-specific feature**: Currently only Anthropic supports this
- **Future compatibility**: Other providers may add caching with different models
- **Production requirement**: Extended (1-hour) cache is actively used and must be supported from day one
- **Abstraction level**: Don't want to leak Anthropic implementation details into core types
- **Opt-in behavior**: Not all messages should be cached
- **Observability**: Need to capture cache statistics for cost optimization (via events)

## Decision

We will expose caching through **optional message attributes** with support for both Anthropic cache types from the start:

```rust
pub struct CacheControl {
    pub cache_type: CacheType,
}

pub enum CacheType {
    Ephemeral,   // Anthropic: 5-minute cache
    Extended,    // Anthropic: 1-hour cache
    // Future: could add provider-specific variants or Custom(Duration)
}

pub struct MessageAttributes {
    pub cache_control: Option<CacheControl>,
    pub priority: i32,
    pub metadata: HashMap<String, Value>,
}

pub struct Message {
    pub role: MessageRole,
    pub content: MessageContent,
    pub attributes: MessageAttributes,
}

// Usage
let msg = Message::user("Large context to cache")
    .with_cache_control(CacheControl::extended())
    .build();
```

**Provider behavior**:
- **Anthropic**: Maps to native cache control blocks (both ephemeral and extended)
- **Other providers**: Silently ignore (no error, treated as regular message)
- **Future providers**: Can implement if they add caching support

**Observability integration**:
When events feature is enabled, providers emit cache-related events:
```rust
#[cfg(feature = "events")]
{
    response.events.push(BusinessEvent::cache_hit(tokens_saved));
    response.events.push(BusinessEvent::cache_write(tokens_written));
}
```

This allows applications to track cache efficiency and optimize which content to cache.

## Consequences

### Positive

- ✅ **Production ready**: Supports extended (1-hour) cache from day one
- ✅ **Provider agnostic**: Doesn't couple library to Anthropic terminology
- ✅ **Optional**: Messages without cache control work normally
- ✅ **Extensible**: Can add new cache types without breaking changes
- ✅ **No runtime cost**: Providers that don't support caching just ignore the hint
- ✅ **Type safe**: Enum ensures valid cache types at compile time
- ✅ **Observable**: Works with events system to enable cost/performance monitoring

### Negative

- ❌ **Provider differences hidden**: Users might not know which providers support caching
- ❌ **Silent failures**: Cache control ignored by unsupported providers (could be confusing)
- ❌ **Abstraction limitations**: Future provider caching models might not map to enum

### Neutral

- ⚪ **Documentation burden**: Must clearly document which providers support which cache types
- ⚪ **Testing complexity**: Need to verify caching works for Anthropic, ignored for others
- ⚪ **Events coupling**: Cache observability requires events feature (but not required for caching itself)

## Alternatives Considered

### Alternative 1: Provider-Specific Metadata

**Description**: Use metadata escape hatch for caching hints.

```rust
let mut msg = Message::user("content");
msg.attributes.metadata.insert(
    "anthropic:cache_control".into(),
    json!({"type": "ephemeral"})
);
```

**Pros**:
- Maximum flexibility
- No commitment to specific cache model
- Easy to add provider-specific nuances

**Cons**:
- Stringly-typed (no compile-time safety)
- Different API for each provider
- Harder to discover (not in type system)
- Users must know provider-specific formats
- No events integration without custom parsing

**Why not chosen**: Caching is important enough to deserve first-class support. Metadata should be escape hatch, not primary API.

### Alternative 2: Trait-Based Caching

**Description**: Define caching trait, providers opt-in.

```rust
pub trait Cacheable {
    fn cache_ttl(&self) -> Option<Duration>;
    fn cache_key(&self) -> Option<String>;
}

impl Cacheable for Message {
    // ... implementation
}
```

**Pros**:
- Clean separation of concerns
- Providers can implement caching differently

**Cons**:
- Over-engineered for current needs
- Trait objects or generics add complexity
- Harder for users to understand and use
- Complicates events integration

**Why not chosen**: KISS principle - simple enum in attributes is sufficient.

### Alternative 3: Anthropic-Only Feature

**Description**: Make caching Anthropic-specific, not abstract.

```rust
pub struct AnthropicCacheControl {
    pub cache_type: AnthropicCacheType,
}

pub enum AnthropicCacheType {
    Ephemeral,
    Extended,
}
```

**Pros**:
- Honest about what's supported
- No pretense of provider agnosticism
- Simpler implementation

**Cons**:
- Locks users into Anthropic terminology
- Harder to migrate if other providers add caching
- Violates library's provider-agnostic philosophy
- Makes events system provider-specific too

**Why not chosen**: We want to be provider-agnostic even if only one provider currently supports a feature.

## Implementation Notes

**Anthropic provider implementation**:
```rust
fn convert_to_anthropic_message(msg: &Message) -> AnthropicMessage {
    let mut anthropic_msg = AnthropicMessage {
        role: msg.role.to_string(),
        content: convert_content(&msg.content),
    };

    // Map cache control if present
    if let Some(cache_control) = &msg.attributes.cache_control {
        anthropic_msg.cache_control = Some(AnthropicCacheControl {
            type_: match cache_control.cache_type {
                CacheType::Ephemeral => "ephemeral",
                CacheType::Extended => "extended",
            }.to_string(),
        });
    }

    anthropic_msg
}

// Emit cache events if feature enabled
#[cfg(feature = "events")]
fn emit_cache_events(response: &mut Response, anthropic_usage: &Usage) {
    if let Some(cache_read) = anthropic_usage.cache_read_input_tokens {
        response.events.push(BusinessEvent::cache_hit(cache_read));
    }
    if let Some(cache_write) = anthropic_usage.cache_creation_input_tokens {
        response.events.push(BusinessEvent::cache_write(cache_write));
    }
}
```

**OpenAI/Ollama/LMStudio providers**:
```rust
fn convert_to_openai_message(msg: &Message) -> OpenAIMessage {
    // Cache control is ignored - not supported
    OpenAIMessage {
        role: msg.role.to_string(),
        content: msg.content.to_string(),
    }
}
```

**Documentation requirements**:
- Clearly document in Message rustdocs which providers support caching
- Include examples showing both cache types
- Explain cost implications (cache write vs read costs)
- Note that unsupported providers silently ignore
- Document events integration for cache monitoring

**Pricing details** (as of 2025):
- **Ephemeral writes**: 1.25x base input token cost
- **Extended writes**: 2x base input token cost
- **Cache reads (both)**: 0.1x base input token cost
- Example (Claude Sonnet 4.5): Base=$3/M, Ephemeral write=$3.75/M, Extended write=$6/M, Cache read=$0.30/M
- Break-even: Ephemeral profitable after 1-2 reads, Extended profitable after 5-6 reads
- Source: https://platform.claude.com/docs/en/build-with-claude/prompt-caching

## Future Considerations

**If other providers add caching**:
1. Assess if their model fits existing `CacheType` enum
2. If yes: Document support, implement conversion, emit cache events
3. If no: Consider adding new variant or provider-specific metadata

**Potential extensions**:
```rust
pub enum CacheType {
    Ephemeral,
    Extended,
    Custom {
        ttl: Duration,
        provider_hints: HashMap<String, Value>,
    },
}
```

**Events system evolution**:
- Consider adding cache efficiency metrics (hit rate, cost savings)
- Provide aggregation utilities for cache statistics
- Enable cache performance dashboards

## References

- Anthropic Prompt Caching: https://docs.anthropic.com/claude/docs/prompt-caching
- myStory production usage: Uses extended (1-hour) cache extensively
- [ADR-005: Events System Design](./005-events-system.md) - Observable cache statistics
- Design discussion: Extended cache must be supported from day one

## Revision History

- 2025-11-22: Initial version with both ephemeral and extended cache types, events integration documented
