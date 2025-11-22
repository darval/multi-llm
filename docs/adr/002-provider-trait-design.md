# ADR-002: Provider Trait Design

**Status**: Accepted
**Date**: 2025-11-22
**Authors**: multi-llm team
**Deciders**: Rick Duff

## Context

The library needs a common interface that all LLM provider implementations must satisfy. This trait defines the contract for:
- Executing LLM requests
- Returning unified responses
- Handling errors consistently
- Supporting async operations

**Requirements**:
- Must work with different provider APIs (OpenAI, Anthropic, Ollama, LM Studio)
- Must be async (all providers are HTTP-based, I/O-bound)
- Must support both simple requests and complex ones (tool calling, structured output)
- Must be extensible for future providers

**Constraints**:
- Rust's trait system (no async in traits without `async-trait` crate)
- Need `Send + Sync` for multi-threaded async runtime (Tokio)
- Different providers have different capabilities (not all support tools, caching, etc.)

**Forces**:
- **Simplicity vs Capability**: Simple trait = limited features; complex trait = harder to implement
- **Async patterns**: Native async traits unstable; `async-trait` adds `Box<dyn Future>` overhead
- **Error handling**: Generic errors vs provider-specific errors
- **Configuration**: Per-request vs per-provider config

## Decision

We will define a **simple, async-first trait** using `async-trait`:

**Current** (legacy naming):
```rust
#[async_trait]
pub trait ExecutorLLMProvider: Send + Sync {
    async fn execute_llm(
        &self,
        request: UnifiedLLMRequest,
        config: Option<ExecutorLLMConfig>,
    ) -> Result<ExecutorLLMResponse, LlmError>;

    fn provider_name(&self) -> &'static str;
}
```

**Target** (post-cleanup):
```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Execute a request against this LLM provider
    async fn execute(
        &self,
        request: Request,
        config: Option<RequestConfig>,
    ) -> Result<Response, LlmError>;

    /// Provider identifier (e.g., "openai", "anthropic")
    fn provider_name(&self) -> &'static str;

    /// Optional: Capability detection
    fn supports_streaming(&self) -> bool { false }
    fn supports_tools(&self) -> bool { true }
    fn supports_caching(&self) -> bool { false }
}
```

**Design principles**:
1. **Single responsibility**: Trait only defines request execution
2. **Provider-agnostic**: Request/Response types are unified, not provider-specific
3. **Async-only**: No blocking API (all providers are I/O-bound)
4. **Send + Sync**: Required for multi-threaded async runtimes
5. **Capability discovery**: Optional methods reveal provider features

## Consequences

### Positive

- ✅ **Simple interface**: One primary method, easy to understand
- ✅ **Consistent error handling**: All providers return `Result<Response, LlmError>`
- ✅ **Async-first**: Natural for HTTP-based providers
- ✅ **Extensible**: New providers just implement trait, no library changes
- ✅ **Testable**: Easy to mock for testing (trait objects or `mockall`)
- ✅ **Capability discovery**: Users can check features before using them

### Negative

- ❌ **async-trait overhead**: Boxing futures has small allocation cost
- ❌ **No streaming** (yet): Single response model, streaming deferred to post-1.0
- ❌ **Generic capabilities**: Capability methods are optional, require checking

### Neutral

- ⚪ **Configuration split**: Some config is provider-specific (construction), some is per-request
- ⚪ **Provider lifecycle**: Providers are constructed once, reused for many requests

## Alternatives Considered

### Alternative 1: Concrete Type with Enum Dispatch

**Description**: No trait; single concrete type that dispatches to providers via enum.

```rust
pub enum Provider {
    OpenAI(OpenAIProvider),
    Anthropic(AnthropicProvider),
    Ollama(OllamaProvider),
    LMStudio(LMStudioProvider),
}

impl Provider {
    pub async fn execute(&self, request: Request) -> Result<Response, LlmError> {
        match self {
            Provider::OpenAI(p) => p.execute(request).await,
            Provider::Anthropic(p) => p.execute(request).await,
            // ...
        }
    }
}
```

**Pros**:
- No trait object overhead (static dispatch)
- Simpler for users (one type, not trait)
- Easier to add provider-specific methods

**Cons**:
- Must modify `Provider` enum for each new provider (not extensible)
- Third-party providers can't integrate without forking library
- All providers compiled even if only using one

**Why not chosen**: Not extensible to user-defined providers. Library should be open for extension.

### Alternative 2: Separate Methods for Each Operation

**Description**: Split trait into multiple methods for different operations.

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn execute_text(&self, request: TextRequest) -> Result<TextResponse, LlmError>;
    async fn execute_with_tools(&self, request: ToolRequest) -> Result<ToolResponse, LlmError>;
    async fn execute_structured(&self, request: StructuredRequest) -> Result<StructuredResponse, LlmError>;
}
```

**Pros**:
- Type-specific requests/responses (more precise)
- Clear capabilities (if provider doesn't support tools, method can return error)

**Cons**:
- Complex trait (multiple methods to implement)
- Duplication in implementations (much overlap between methods)
- Harder to use (which method do I call?)
- Response types differ (harder to work generically)

**Why not chosen**: Over-complicated. Single unified request/response better serves most use cases.

### Alternative 3: Builder Pattern in Trait

**Description**: Trait returns builder for incremental request construction.

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn request(&self) -> RequestBuilder;
}

impl RequestBuilder {
    pub fn message(self, msg: Message) -> Self { /* ... */ }
    pub fn tool(self, tool: Tool) -> Self { /* ... */ }
    pub async fn execute(self) -> Result<Response, LlmError> { /* ... */ }
}
```

**Pros**:
- Ergonomic API for building complex requests
- Method chaining

**Cons**:
- Builder lifetime tied to provider (borrow checker issues)
- More complex trait surface
- Builders need generic parameter for provider type

**Why not chosen**: Over-engineering. Users can build `Request` directly, don't need trait-provided builder.

## Implementation Notes

**Provider implementation pattern**:
```rust
pub struct OpenAIProvider {
    config: OpenAIConfig,
    client: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new(config: OpenAIConfig) -> Result<Self, LlmError> {
        // Validate config
        config.validate()?;

        // Build HTTP client
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(LlmError::network_error)?;

        Ok(Self { config, client })
    }
}

#[async_trait]
impl LlmProvider for OpenAIProvider {
    async fn execute(
        &self,
        request: Request,
        config: Option<RequestConfig>,
    ) -> Result<Response, LlmError> {
        // 1. Merge request config with provider defaults
        let merged_config = self.merge_config(config);

        // 2. Convert unified request to provider format
        let openai_request = self.convert_request(&request, &merged_config)?;

        // 3. Make HTTP request
        let response = self.client
            .post(&self.config.endpoint)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&openai_request)
            .send()
            .await
            .map_err(LlmError::network_error)?;

        // 4. Handle HTTP errors
        if !response.status().is_success() {
            return Err(self.handle_error_response(response).await);
        }

        // 5. Parse response
        let openai_response = response
            .json::<OpenAIResponse>()
            .await
            .map_err(|e| LlmError::response_parse_error(format!("Parse error: {}", e)))?;

        // 6. Convert to unified response
        self.convert_response(openai_response)
    }

    fn provider_name(&self) -> &'static str {
        "openai"
    }

    fn supports_tools(&self) -> bool {
        true
    }
}
```

**Error handling**:
- Map HTTP errors to `LlmError::Network`
- Map rate limits to `LlmError::RateLimit` with `retry_after`
- Map auth errors to `LlmError::Authentication`
- Map provider-specific errors to `LlmError::Provider`

**Testing**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::mock;

    mock! {
        Provider {}

        #[async_trait]
        impl LlmProvider for Provider {
            async fn execute(
                &self,
                request: Request,
                config: Option<RequestConfig>,
            ) -> Result<Response, LlmError>;

            fn provider_name(&self) -> &'static str;
        }
    }

    #[tokio::test]
    async fn test_generic_code_with_provider() {
        let mut mock = MockProvider::new();
        mock.expect_execute()
            .returning(|_, _| Ok(Response::default()));

        let response = mock.execute(Request::default(), None).await.unwrap();
        assert_eq!(response.role, MessageRole::Assistant);
    }
}
```

## References

- async-trait crate: https://docs.rs/async-trait
- Rust Async Book: https://rust-lang.github.io/async-book/
- Provider implementations: [src/providers/](../../src/providers/)

## Revision History

- 2025-11-22: Initial version
