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

**Previous state** (problematic):
```rust
#[async_trait]
pub trait ExecutorLLMProvider {
    async fn execute_llm(...)
        -> Result<(ExecutorLLMResponse, Vec<LLMBusinessEvent>), LlmError>;
        //                              ^^^^^^^^^^^^^^^^^^^^^^
        //                              Forced on all users!
}
```

**Problems identified**:
1. **Forces all users to handle events** even if they don't want them
2. **Adds dependencies** (`uuid`, `chrono`) that not all users need
3. **Couples library to application patterns** from parent project
4. **Violates library design principle**: Optional features should be opt-in

**Current state** (✅ implemented):
The events system is now feature-gated. The `LlmProvider` trait has two implementations:
- With `events` feature: Returns `(Response, Vec<LLMBusinessEvent>)`
- Without `events` feature: Returns just `Response`

All providers properly emit cache statistics when events are enabled.

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

**Event Types** (actual implementation):
```rust
#[cfg(feature = "events")]
pub struct BusinessEvent {
    pub id: Uuid,
    pub event_type: String,  // e.g., "llm_request", "llm_response"
    pub metadata: serde_json::Value,  // Flexible key-value data
    pub created_at: DateTime<Utc>,
}

#[cfg(feature = "events")]
pub enum EventScope {
    User(String),  // User-scoped (written to user storage)
    System,        // System-level (written to system storage)
}

#[cfg(feature = "events")]
pub mod event_types {
    pub const LLM_REQUEST: &str = "llm_request";
    pub const LLM_RESPONSE: &str = "llm_response";
    pub const LLM_ERROR: &str = "llm_error";
    pub const CACHE_HIT: &str = "cache_hit";
    pub const CACHE_MISS: &str = "cache_miss";
    pub const ERROR: &str = "error";
}
```

**Event metadata structure** (Anthropic provider example):
```rust
// LLM_RESPONSE event metadata:
{
    "provider": "anthropic",
    "model": "claude-3-5-sonnet-20241022",
    "input_tokens": 1500,
    "output_tokens": 250,
    "duration_ms": 1234,

    // Cache-specific fields (if applicable)
    "cache_creation_tokens": 1024,  // Tokens written to cache
    "cache_read_tokens": 476,       // Tokens read from cache

    // Optional context
    "session_id": "...",
    "llm_path": "..."
}
```

**Why this design?**:
- **Flexible metadata**: Different providers can add provider-specific fields
- **Type safety**: Predefined constants for common event types
- **Cache observability**: Cache statistics in metadata enable cost analysis
- **Extensible**: New fields can be added without breaking changes

**Documentation requirements**:
- Clearly document that events are opt-in
- Show examples with and without events
- Explain dependencies added by events feature
- Provide migration guide for existing users

## Cache Monitoring and Tuning with Events

One of the primary use cases for the events system is monitoring and optimizing Anthropic prompt caching. This section provides practical guidance on using events to tune cache performance.

### Understanding Cache Statistics

When the `events` feature is enabled, Anthropic responses include cache statistics in the `LLM_RESPONSE` event metadata:

```rust
// Example event metadata from cached request
{
    "provider": "anthropic",
    "model": "claude-3-5-sonnet-20241022",
    "input_tokens": 2000,           // Total input tokens
    "output_tokens": 150,
    "cache_creation_tokens": 1024,  // Tokens written to new cache
    "cache_read_tokens": 976,       // Tokens read from existing cache
    "duration_ms": 823
}
```

**Key metrics**:
- `cache_creation_tokens`: New content written to cache (incurs write cost)
- `cache_read_tokens`: Content served from cache (90% cost savings)
- `input_tokens`: Total input = cache_creation + cache_read + uncached tokens

### Minimum Token Requirements

Anthropic caching has model-specific minimum token thresholds:

| Model | Minimum Tokens for Caching |
|-------|---------------------------|
| Claude 3.5 Sonnet | 1,024 tokens |
| Claude 3.0 Opus | 1,024 tokens |
| Claude 3.7 Sonnet | 1,024 tokens |
| Claude 3.0 Haiku | 2,048 tokens |
| **Claude 4.5 Haiku** | **4,096 tokens** |

**Important**: If you mark content as cacheable below these thresholds, the request will succeed but caching won't occur. Use events to detect this:

```rust
#[cfg(feature = "events")]
fn analyze_cache_effectiveness(events: &[LLMBusinessEvent]) {
    for event in events {
        if event.event.event_type == "llm_response" {
            let metadata = &event.event.metadata;

            let cache_created = metadata.get("cache_creation_tokens")
                .and_then(|v| v.as_u64()).unwrap_or(0);
            let cache_read = metadata.get("cache_read_tokens")
                .and_then(|v| v.as_u64()).unwrap_or(0);

            if cache_created == 0 && cache_read == 0 {
                // Warning: Content marked cacheable but not cached
                // Likely below minimum token threshold
                log::warn!("Cacheable content not cached - may be below threshold");
            }
        }
    }
}
```

### Cost Analysis Example

Use events to calculate actual cache savings:

```rust
#[cfg(feature = "events")]
fn calculate_cache_savings(events: &[LLMBusinessEvent]) -> f64 {
    let mut total_savings = 0.0;
    let base_cost_per_token = 0.003 / 1000.0; // $3 per 1M tokens

    for event in events {
        if event.event.event_type == "llm_response" {
            let metadata = &event.event.metadata;

            // Tokens read from cache saved 90%
            if let Some(cache_read) = metadata.get("cache_read_tokens")
                .and_then(|v| v.as_u64())
            {
                let savings = cache_read as f64 * base_cost_per_token * 0.9;
                total_savings += savings;
            }

            // Account for cache write premium
            if let Some(cache_write) = metadata.get("cache_creation_tokens")
                .and_then(|v| v.as_u64())
            {
                // Check if ephemeral (1.25x) or extended (2x)
                // For simplicity, assuming ephemeral (25% premium)
                let write_cost = cache_write as f64 * base_cost_per_token * 0.25;
                total_savings -= write_cost;
            }
        }
    }

    total_savings
}
```

### Optimizing Cache Breakpoints

Use events to identify optimal cache breakpoint placement:

```rust
#[cfg(feature = "events")]
fn analyze_cache_patterns(events: &[LLMBusinessEvent]) {
    for event in events {
        if event.event.event_type == "llm_response" {
            let metadata = &event.event.metadata;

            let input = metadata.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
            let cache_created = metadata.get("cache_creation_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
            let cache_read = metadata.get("cache_read_tokens").and_then(|v| v.as_u64()).unwrap_or(0);

            // Calculate cache hit rate
            let cache_hit_rate = if input > 0 {
                (cache_read as f64 / input as f64) * 100.0
            } else {
                0.0
            };

            // Analyze cache effectiveness
            if cache_hit_rate > 50.0 {
                log::info!("High cache efficiency: {:.1}% hit rate", cache_hit_rate);
            } else if cache_created > 0 && cache_read == 0 {
                log::warn!("Cache created but not used - consider removing cache breakpoint");
            }
        }
    }
}
```

### Choosing Between Ephemeral and Extended Cache

The cache type decision depends on three key factors, especially for human-in-the-loop scenarios:

#### Factor 1: Session Duration (Overall Conversation Length)

How long does a typical session last? This determines if system prompts and tool definitions stay cached.

- **Short sessions (<5 min)**: Ephemeral sufficient, extended overkill
- **Medium sessions (5-30 min)**: Extended cache valuable for static content (tools, system prompts)
- **Long sessions (>30 min)**: Extended cache essential, recoups 2x write cost easily

**Human-in-loop impact**: Users think/read between responses. A 10-turn conversation might span 20-30 minutes, not 5 minutes. Extended cache keeps system context hot.

#### Factor 2: Turnaround Time (Query → Reply → Query)

How quickly do users respond to the LLM? This is the critical difference between automated and human workflows.

- **Automated (seconds)**: 5-minute ephemeral works, many round-trips within TTL
- **Chatbot (1-3 minutes/turn)**: 5 minutes expires after 2-3 exchanges!
- **Thoughtful conversation (5-10 min/turn)**: Only extended cache survives between turns

**Example**: User asks question → LLM responds in 2s → User reads for 4 minutes → User follows up. That's 4+ minutes! Second query likely misses ephemeral cache.

#### Factor 3: Content Stability (What Changes vs What Stays)

What parts of your context are static vs dynamic?

- **System prompts**: Usually 100% static → Always cache (extended if sessions >5min)
- **Tool definitions**: Very static → Always cache (extended if sessions >5min)
- **Conversation history**: Grows each turn → Cache recent history only
- **Dynamic context**: Changes frequently → Don't cache, waste of write cost

**Breakpoint strategy**:
```rust
// Good: Cache static content with extended, dynamic content uncached
[
    system_prompt,           // ✅ Extended cache (static, reused entire session)
    tool_definitions,        // ✅ Extended cache (static, reused entire session)
    conversation_history,    // ❌ No cache (grows every turn)
    current_user_query,      // ❌ No cache (unique every time)
]
```

### Using Events to Analyze Your Usage Pattern

```rust
#[cfg(feature = "events")]
fn analyze_session_characteristics(session_events: &[LLMBusinessEvent]) {
    let mut request_times = vec![];

    for event in session_events {
        if event.event.event_type == "llm_response" {
            request_times.push(event.event.created_at);
        }
    }

    if request_times.len() < 2 {
        return;
    }

    // Factor 1: Session duration
    let session_duration = request_times.last().unwrap()
        .signed_duration_since(*request_times.first().unwrap())
        .num_minutes();

    // Factor 2: Average turnaround time
    let mut turnaround_times = vec![];
    for i in 1..request_times.len() {
        let gap = request_times[i]
            .signed_duration_since(request_times[i-1])
            .num_seconds();
        turnaround_times.push(gap);
    }
    let avg_turnaround = turnaround_times.iter().sum::<i64>() / turnaround_times.len() as i64;

    // Factor 3: Cache effectiveness (from metadata)
    let mut static_content_hits = 0;
    let mut total_cache_reads = 0;

    for event in session_events {
        if let Some(cache_read) = event.event.metadata.get("cache_read_tokens")
            .and_then(|v| v.as_u64())
        {
            if cache_read > 0 {
                total_cache_reads += 1;
                // If cache hit on first request, indicates static content working
                if event == &session_events[1] {
                    static_content_hits += 1;
                }
            }
        }
    }

    // Recommendation logic
    println!("Session Analysis:");
    println!("  Duration: {} minutes", session_duration);
    println!("  Avg turnaround: {} seconds", avg_turnaround);
    println!("  Cache hits: {}/{} requests", total_cache_reads, request_times.len());

    if session_duration > 5 && avg_turnaround > 60 {
        println!("✅ Extended cache recommended: Long sessions with human think-time");
    } else if session_duration > 5 && avg_turnaround < 30 {
        println!("🤔 Extended cache beneficial: Long session, but fast turnaround. Consider ephemeral.");
    } else if avg_turnaround > 300 {
        println!("⚠️  Turnaround >5min: Ephemeral cache likely expiring between requests!");
    } else {
        println!("✅ Ephemeral cache sufficient: Short sessions, quick turnaround");
    }
}
```

### Decision Matrix

| Scenario | Session Length | Turnaround | Content | Recommendation |
|----------|---------------|------------|---------|----------------|
| **API automation** | Any | <30s | Mixed | Ephemeral (many hits within 5min) |
| **Chatbot (casual)** | 10-20 min | 2-4 min | System+Tools static | Extended (human think-time kills ephemeral) |
| **Chatbot (support)** | 5-10 min | 1-2 min | System+Tools static | Extended or Ephemeral (monitor hit rates) |
| **Long-form assistant** | 30-60 min | 5-10 min | Large static context | Extended (essential for long sessions) |
| **Code review** | 20-40 min | 3-5 min | Codebase context static | Extended (context survives review time) |
| **Quick Q&A** | <5 min | <1 min | Small prompts | Neither (below minimum tokens) |

### Break-Even Reality Check

The "5-6 reads for extended" math assumes immediate consecutive requests:
- **Automated scenario**: 6 requests in 30 seconds → Ephemeral works fine
- **Human-in-loop**: 6 requests across 25 minutes → Ephemeral expires after request 2!

**Use events to calculate YOUR actual break-even**:
- Track time between requests (turnaround time)
- Count cache hits vs misses over session lifetime
- Factor in that extended cache survives human think-time

### Monitoring Cache Performance Over Time

Build a cache efficiency dashboard using event data:

```rust
#[cfg(feature = "events")]
struct CacheStats {
    total_requests: u32,
    cache_hits: u32,
    cache_writes: u32,
    total_tokens_saved: u64,
    total_cost_savings: f64,
}

#[cfg(feature = "events")]
fn build_cache_dashboard(events: &[LLMBusinessEvent]) -> CacheStats {
    let mut stats = CacheStats::default();

    for event in events {
        if event.event.event_type == "llm_response" {
            stats.total_requests += 1;

            let metadata = &event.event.metadata;
            if let Some(cache_read) = metadata.get("cache_read_tokens").and_then(|v| v.as_u64()) {
                if cache_read > 0 {
                    stats.cache_hits += 1;
                    stats.total_tokens_saved += cache_read;
                }
            }

            if let Some(cache_write) = metadata.get("cache_creation_tokens").and_then(|v| v.as_u64()) {
                if cache_write > 0 {
                    stats.cache_writes += 1;
                }
            }
        }
    }

    // Calculate overall hit rate and savings
    let base_cost = 0.003 / 1000.0; // $3/M tokens
    stats.total_cost_savings = stats.total_tokens_saved as f64 * base_cost * 0.9;

    stats
}
```

### Best Practices

1. **Start with ephemeral cache**: Lower risk, faster to iterate
2. **Monitor for 24-48 hours**: Collect event data across usage patterns
3. **Analyze cache hit rates**: >50% hit rate indicates good cache placement
4. **Watch for cache misses**: Frequent writes with no reads = wasted cost
5. **Consider extended cache**: If you see 6+ requests within the 5-minute TTL
6. **Respect minimum thresholds**: Don't cache content below model minimums
7. **Track cost savings**: Use events to prove ROI of caching strategy

## References

- Rust API Guidelines - Feature flags: https://rust-lang.github.io/api-guidelines/flexibility.html#feature-flags
- Anthropic Prompt Caching: https://platform.claude.com/docs/en/build-with-claude/prompt-caching
- Anthropic Minimum Token Requirements: https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching
- [ADR-003: Caching Hints Architecture](./003-caching-hints.md) - Cache type design
- Original myStory events system: (internal reference)

## Revision History

- 2025-11-24: Added comprehensive cache monitoring and tuning section with emphasis on human-in-loop factors
- 2025-11-22: Initial version with feature-gating design
