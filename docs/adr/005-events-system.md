# ADR-005: Events System Design

**Status**: Accepted
**Date**: 2025-11-22
**Authors**: multi-llm team
**Deciders**: Rick Duff

## Context

Applications using LLMs often need observability beyond basic logging:
- **Token usage tracking**: Cost calculation, quota management
- **Cache performance**: How often is Anthropic caching helping?
- **Provider performance**: Latency, error rates per provider
- **Business analytics**: Usage patterns, feature adoption

The library was originally extracted from a larger application (`myStory`) that had a business event system for these purposes. The events system was deeply integrated into the core library trait.

**Current state** (problematic):
```rust
#[async_trait]
pub trait ExecutorLLMProvider {
    async fn execute_llm(...)
        -> Result<(ExecutorLLMResponse, Vec<LLMBusinessEvent>), LlmError>;
        //                              ^^^^^^^^^^^^^^^^^^^^^^
        //                              Forced on all users!
}
```

**Problems**:
1. **Forces all users to handle events** even if they don't want them
2. **Adds dependencies** (`uuid`, `chrono`) that not all users need
3. **Couples library to application patterns** from parent project
4. **Violates library design principle**: Optional features should be opt-in

**Forces**:
- **Observability needs**: Many applications genuinely need structured event data
- **Library purity**: Libraries shouldn't force patterns on users
- **Performance**: Event creation/collection has overhead
- **Dependencies**: Each dependency affects all downstream users

## Decision

We will make the events system **optional via a Cargo feature flag**:

```toml
[features]
default = []
events = ["uuid", "chrono"]
```

**When feature is enabled**:
- Event types are compiled
- `Response` struct includes `events` field
- Providers emit structured events
- Users can consume event data

**When feature is disabled**:
- Event types not compiled (zero code size impact)
- `Response` struct has no `events` field
- No runtime overhead for event collection
- No `uuid` or `chrono` dependencies

**Implementation**:
```rust
#[cfg(feature = "events")]
pub struct BusinessEvent {
    pub event_id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub scope: EventScope,
    pub metadata: HashMap<String, Value>,
}

pub struct Response {
    pub content: String,
    pub role: MessageRole,
    pub tool_calls: Vec<ToolCall>,
    pub usage: TokenUsage,

    #[cfg(feature = "events")]
    pub events: Vec<BusinessEvent>,
}

// Provider implementation
impl LlmProvider for AnthropicProvider {
    async fn execute(...) -> Result<Response, LlmError> {
        // ... make request ...

        let mut response = Response {
            content: anthropic_response.content,
            // ... other fields ...
            #[cfg(feature = "events")]
            events: Vec::new(),
        };

        #[cfg(feature = "events")]
        {
            if let Some(cache_stats) = anthropic_response.usage.cache_read_input_tokens {
                response.events.push(BusinessEvent::cache_hit(cache_stats));
            }
        }

        Ok(response)
    }
}
```

## Consequences

### Positive

- ✅ **User choice**: Users opt-in to events, not forced
- ✅ **Zero cost**: When disabled, no code compiled, no runtime overhead
- ✅ **Dependency minimization**: `uuid` and `chrono` only required with feature
- ✅ **Library purity**: Maintains library-first design principle
- ✅ **Structured observability**: Users who need it get rich event data

### Negative

- ❌ **API fragmentation**: `Response` type differs based on feature flags
- ❌ **Testing complexity**: Must test with and without events feature
- ❌ **Documentation burden**: Must document both enabled/disabled behavior
- ❌ **Conditional compilation noise**: `#[cfg(feature = "events")]` throughout code

### Neutral

- ⚪ **Default behavior**: Events disabled by default (explicit opt-in)
- ⚪ **Migration path**: Existing users need to enable feature to keep current behavior

## Alternatives Considered

### Alternative 1: Callback/Handler Pattern

**Description**: Always compile event types, but make consumption optional via callback trait.

```rust
pub trait EventHandler: Send + Sync {
    fn on_event(&self, event: BusinessEvent);
}

// Users provide handler if they want events
let provider = OpenAIProvider::new(config)
    .with_event_handler(Arc::new(MyEventHandler));
```

**Pros**:
- No conditional compilation
- Users can implement any handling logic (logging, metrics, storage)
- Events always available for debugging

**Cons**:
- Event types always compiled (code size impact even if unused)
- Still requires `uuid` and `chrono` dependencies
- Performance overhead of event creation even if handler is no-op
- More complex API (handler registration, trait implementation)

**Why not chosen**: Doesn't achieve dependency minimization goal. Users who don't want events still pay dependency cost.

### Alternative 2: Remove Events Entirely

**Description**: Remove event system, recommend users use tracing/logging.

```rust
// Providers just log events
tracing::info!(
    cache_read_tokens = cache_stats,
    "Anthropic cache hit"
);

// Users subscribe to tracing events if they want
```

**Pros**:
- Simplest implementation
- No dependencies beyond `tracing` (already used)
- Standard Rust observability pattern
- No feature flags needed

**Cons**:
- Less structured than dedicated events (logs are strings, not typed data)
- Harder to consume programmatically (parsing logs vs deserializing events)
- Loss of existing event infrastructure from parent project
- Users need to implement their own structured event extraction

**Why not chosen**: Structured events provide real value for cost tracking and analytics. Tracing is great for debugging but not ideal for business metrics.

### Alternative 3: Separate Events Crate

**Description**: Move events to separate `multi-llm-events` crate.

```rust
// In multi-llm crate
pub struct Response {
    // ... no events field
}

// In multi-llm-events crate
pub trait EventEmitter {
    fn emit_event(&self, event: BusinessEvent);
}

impl EventEmitter for Response {
    // ... extract events from response
}
```

**Pros**:
- Complete separation of concerns
- Users only depend on events crate if needed
- Can version events independently

**Cons**:
- Requires maintaining separate crate
- Less convenient (separate dependency)
- Harder to emit events from within providers (tight coupling needed)
- Events crate would need visibility into internals

**Why not chosen**: Over-engineering for current needs. Feature flags achieve separation without additional crate complexity.

## Implementation Notes

**Event Types**:
```rust
#[cfg(feature = "events")]
pub enum EventType {
    CacheHit {
        tokens_saved: u32,
    },
    CacheWrite {
        tokens_written: u32,
    },
    TokenUsage {
        prompt: u32,
        completion: u32,
        total: u32,
    },
    ProviderCall {
        provider: String,
        duration_ms: u64,
        model: String,
    },
    ToolCall {
        tool_name: String,
        success: bool,
    },
}

#[cfg(feature = "events")]
pub enum EventScope {
    Request(String),  // Per-request ID
    Session(String),  // Per-session ID (application provides)
    User(String),     // Per-user ID (application provides)
}
```

**Why these events?**:
- **CacheHit/CacheWrite**: Cost optimization tracking (Anthropic caching)
- **TokenUsage**: Cost calculation, quota enforcement
- **ProviderCall**: Performance monitoring, SLA tracking
- **ToolCall**: Feature usage analytics, debugging

**Documentation requirements**:
- Clearly document that events are opt-in
- Show examples with and without events
- Explain dependencies added by events feature
- Provide migration guide for existing users

## References

- Rust API Guidelines - Feature flags: https://rust-lang.github.io/api-guidelines/flexibility.html#feature-flags
- Original myStory events system: (internal reference)
- Issue tracking events extraction: (to be created)

## Revision History

- 2025-11-22: Initial version
