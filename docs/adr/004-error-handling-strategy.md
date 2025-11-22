# ADR-004: Error Handling Strategy

**Status**: Accepted
**Date**: 2025-11-22
**Authors**: multi-llm team
**Deciders**: Rick Duff

## Context

The library interacts with multiple external LLM providers over HTTP. Errors can occur at many levels:
- **Configuration errors**: Invalid API keys, missing required fields
- **Network errors**: Connection failures, timeouts
- **Provider errors**: Rate limits, authentication failures, invalid requests
- **Parsing errors**: Malformed responses from providers
- **Validation errors**: Invalid message formats, tool schemas

**Requirements**:
- Errors must be **actionable** (users can tell what went wrong and how to fix it)
- Errors must be **retryable** (users can tell if retry makes sense)
- Errors must be **provider-transparent** (expose provider-specific details)
- Errors must be **type-safe** (no panics in library code)

**Library constraints**:
- Must return `Result<T, E>` everywhere (no panics)
- Must provide rich error context
- Should not use `anyhow::Error` in public API (that's for applications, not libraries)

**Forces**:
- **Simplicity vs Information**: Simple errors are easy to handle but lack context
- **Type safety vs Flexibility**: Strongly typed errors are harder to extend
- **Provider specifics vs Abstraction**: Too generic = lose info; too specific = leaky abstraction

## Decision

We will use a **rich enum-based error type** with provider-specific variants and retry metadata:

**Target design**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Provider '{provider}' error (status {status_code:?}): {message}")]
    Provider {
        provider: String,
        status_code: Option<u16>,
        message: String,
        retry_after: Option<Duration>,
    },

    #[error("Rate limit exceeded (retry after {retry_after:?})")]
    RateLimit {
        provider: String,
        retry_after: Option<Duration>,
    },

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Response parse error: {0}")]
    ResponseParse(String),
}

impl LlmError {
    /// Check if this error is potentially retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self,
            LlmError::Network(_) |
            LlmError::RateLimit { .. } |
            LlmError::Provider { status_code: Some(500..=599), .. }
        )
    }

    /// Get retry delay hint if available
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            LlmError::Provider { retry_after, .. } => *retry_after,
            LlmError::RateLimit { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Get provider name if error is provider-specific
    pub fn provider(&self) -> Option<&str> {
        match self {
            LlmError::Provider { provider, .. } => Some(provider),
            LlmError::RateLimit { provider, .. } => Some(provider),
            _ => None,
        }
    }
}
```

**Error construction helpers**:
```rust
impl LlmError {
    pub fn configuration(msg: impl Into<String>) -> Self {
        LlmError::Configuration(msg.into())
    }

    pub fn provider_error(
        provider: impl Into<String>,
        status_code: Option<u16>,
        message: impl Into<String>,
    ) -> Self {
        LlmError::Provider {
            provider: provider.into(),
            status_code,
            message: message.into(),
            retry_after: None,
        }
    }

    pub fn rate_limit(
        provider: impl Into<String>,
        retry_after: Option<Duration>,
    ) -> Self {
        LlmError::RateLimit {
            provider: provider.into(),
            retry_after,
        }
    }
}
```

**Usage in providers**:
```rust
// Configuration validation
if config.api_key.is_empty() {
    return Err(LlmError::configuration("API key is required"));
}

// Network errors (auto-converted via #[from])
let response = self.client.post(url).send().await?;

// Rate limits
if response.status() == StatusCode::TOO_MANY_REQUESTS {
    let retry_after = parse_retry_after_header(&response);
    return Err(LlmError::rate_limit("openai", retry_after));
}

// Provider errors
if !response.status().is_success() {
    let error_body = response.text().await?;
    return Err(LlmError::provider_error(
        "openai",
        Some(response.status().as_u16()),
        error_body,
    ));
}

// Parse errors
let parsed = response.json::<OpenAIResponse>()
    .await
    .map_err(|e| LlmError::ResponseParse(format!("Invalid JSON: {}", e)))?;
```

## Consequences

### Positive

- ✅ **Type safety**: Compile-time exhaustive matching on error variants
- ✅ **Actionable**: Users can inspect error type and decide how to handle
- ✅ **Retry logic**: `is_retryable()` and `retry_after()` guide retry decisions
- ✅ **Provider transparency**: Errors include provider name and HTTP status
- ✅ **No panics**: Library never panics, always returns `Result`
- ✅ **Rich context**: Error messages include details for debugging

### Negative

- ❌ **Verbosity**: More variants = more code to match on
- ❌ **Breaking changes**: Adding variants is breaking change (unless `#[non_exhaustive]`)
- ❌ **Provider leakage**: Error variants expose provider implementation details

### Neutral

- ⚪ **thiserror dependency**: Standard choice for library errors, minimal cost
- ⚪ **Error conversion**: Some boilerplate in provider implementations

## Alternatives Considered

### Alternative 1: anyhow::Error in Public API

**Description**: Use `anyhow::Error` for all library errors.

```rust
pub async fn execute(...) -> Result<Response, anyhow::Error> {
    let response = self.client.post(url).send().await?;
    // ...
}
```

**Pros**:
- Simple: any error converts automatically
- Flexible: easy to add context with `.context()`
- Less boilerplate

**Cons**:
- **Not library-appropriate**: `anyhow` is for applications, not libraries
- No type safety: users can't match on error types
- No structured access to error fields (status code, retry_after, etc.)
- Harder to document (what errors can occur?)

**Why not chosen**: Violates library design best practices. `anyhow` should only be used internally, not in public API.

### Alternative 2: Trait-Based Errors

**Description**: Define error trait instead of concrete enum.

```rust
pub trait LlmErrorTrait: std::error::Error {
    fn is_retryable(&self) -> bool;
    fn retry_after(&self) -> Option<Duration>;
    fn provider(&self) -> Option<&str>;
}

pub type LlmError = Box<dyn LlmErrorTrait>;
```

**Pros**:
- Extensible: providers can define custom error types
- Flexible: users can implement trait for their errors

**Cons**:
- Heap allocation for every error (Box)
- Dynamic dispatch (trait object)
- Can't pattern match on concrete types
- More complex for users

**Why not chosen**: Over-engineered. Concrete enum provides better type safety and performance.

### Alternative 3: Separate Error Types per Provider

**Description**: Each provider defines its own error type.

```rust
pub enum OpenAIError { /* ... */ }
pub enum AnthropicError { /* ... */ }

pub enum LlmError {
    OpenAI(OpenAIError),
    Anthropic(AnthropicError),
    // ...
}
```

**Pros**:
- Provider-specific error variants
- Full fidelity to provider error responses

**Cons**:
- Complex for users (must understand each provider's errors)
- Breaks abstraction (users handling provider-specific errors)
- More code to maintain

**Why not chosen**: Defeats purpose of unified abstraction. Users should handle errors generically.

### Alternative 4: Result<T> with Custom Error Trait

**Description**: Keep current trait-based approach from parent project.

```rust
pub trait MyStoryError: std::error::Error {
    fn category(&self) -> ErrorCategory;
    fn severity(&self) -> ErrorSeverity;
    fn is_retryable(&self) -> bool;
}

impl MyStoryError for LlmError { /* ... */ }
```

**Pros**:
- Consistent with parent project patterns
- Error categorization built-in

**Cons**:
- **Parent project name in library** (`MyStoryError` doesn't make sense here)
- Trait adds complexity without clear benefit for library users
- Category/severity may be application-specific concepts

**Why not chosen**: Legacy from parent project. For standalone library, concrete error type with helper methods is cleaner.

## Implementation Notes

**Provider error mapping examples**:

```rust
// OpenAI rate limit
if status == 429 {
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs);

    return Err(LlmError::rate_limit("openai", retry_after));
}

// Anthropic auth error
if status == 401 {
    return Err(LlmError::Authentication(
        "Invalid API key".to_string()
    ));
}

// Generic server error
if status >= 500 {
    return Err(LlmError::provider_error(
        "anthropic",
        Some(status),
        "Internal server error",
    ));
}
```

**User retry logic**:
```rust
let mut attempt = 0;
loop {
    match provider.execute(request.clone(), config.clone()).await {
        Ok(response) => return Ok(response),
        Err(e) if e.is_retryable() && attempt < 3 => {
            attempt += 1;
            if let Some(delay) = e.retry_after() {
                tokio::time::sleep(delay).await;
            } else {
                tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await;
            }
        }
        Err(e) => return Err(e),
    }
}
```

**Future considerations**:
- Add `#[non_exhaustive]` to allow adding variants without breaking changes
- Consider adding error codes for machine-readable error handling
- May add structured error context (HashMap of additional metadata)

## References

- Rust Error Handling Book: https://doc.rust-lang.org/book/ch09-00-error-handling.html
- thiserror crate: https://docs.rs/thiserror
- Rust API Guidelines - Error handling: https://rust-lang.github.io/api-guidelines/errors.html

## Revision History

- 2025-11-22: Initial version
