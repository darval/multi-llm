//! Business event types for LLM operations.
//!
//! This module provides types for tracking LLM interactions for observability,
//! analytics, and debugging. Events can track request/response cycles, cache
//! hits/misses, errors, and custom application events.
//!
//! # Feature Flag
//!
//! This module requires the `events` feature:
//!
//! ```toml
//! [dependencies]
//! multi-llm = { version = "...", features = ["events"] }
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use multi_llm::{BusinessEvent, EventScope, event_types};
//!
//! // Create a custom event
//! let event = BusinessEvent::new(event_types::LLM_REQUEST)
//!     .with_metadata("model", "gpt-4")
//!     .with_metadata("prompt_tokens", 150);
//!
//! // Specify event scope
//! let scope = EventScope::User("user_123".to_string());
//! ```
//!
//! # Event Types
//!
//! Pre-defined event types are available in [`event_types`]:
//! - `LLM_REQUEST`: LLM request initiated
//! - `LLM_RESPONSE`: LLM response received
//! - `LLM_ERROR`: LLM request failed
//! - `CACHE_HIT`: Prompt cache hit
//! - `CACHE_MISS`: Prompt cache miss
//! - `ERROR`: Generic error event

#[cfg(feature = "events")]
use chrono::{DateTime, Utc};
#[cfg(feature = "events")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "events")]
use uuid::Uuid;

/// Business event for analytics and observability.
///
/// Events capture significant moments during LLM operations. Each event has:
/// - A unique ID for deduplication
/// - An event type for categorization
/// - Flexible metadata as JSON
/// - A creation timestamp
///
/// # Example
///
/// ```rust,ignore
/// use multi_llm::BusinessEvent;
///
/// let event = BusinessEvent::new("custom_event")
///     .with_metadata("user_id", "u123")
///     .with_metadata("action", "query")
///     .with_metadata("duration_ms", 150);
/// ```
///
/// # Feature Flag
///
/// Requires the `events` feature to be enabled.
#[cfg(feature = "events")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BusinessEvent {
    /// Unique identifier for this event (UUID v4).
    pub id: Uuid,

    /// Event type string (e.g., "llm_request", "cache_hit").
    ///
    /// Use constants from [`event_types`] for consistency.
    pub event_type: String,

    /// Flexible metadata as JSON object.
    ///
    /// Use [`with_metadata()`](Self::with_metadata) to add key-value pairs.
    pub metadata: serde_json::Value,

    /// Timestamp when the event was created (UTC).
    pub created_at: DateTime<Utc>,
}

#[cfg(feature = "events")]
impl BusinessEvent {
    /// Create a new business event with the given type
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: event_type.into(),
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            created_at: Utc::now(),
        }
    }

    /// Add metadata to this event
    ///
    /// # Example
    /// ```
    /// use multi_llm::BusinessEvent;
    ///
    /// let event = BusinessEvent::new("test_event")
    ///     .with_metadata("key1", "value1")
    ///     .with_metadata("count", 42);
    /// ```
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            if let Some(obj) = self.metadata.as_object_mut() {
                obj.insert(key.into(), v);
            }
        }
        self
    }
}

/// Scope determining where events should be stored.
///
/// Events can be scoped to individual users (for user-specific analytics)
/// or to the system level (for global metrics and monitoring).
///
/// # Example
///
/// ```rust,ignore
/// use multi_llm::EventScope;
///
/// // User-specific event
/// let user_scope = EventScope::User("user_123".to_string());
///
/// // System-wide event
/// let system_scope = EventScope::System;
/// ```
///
/// # Feature Flag
///
/// Requires the `events` feature to be enabled.
#[cfg(feature = "events")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventScope {
    /// User-scoped event for user-specific analytics.
    ///
    /// Contains the user ID. Events are stored in user-specific storage
    /// and can be used for personalized analytics.
    User(String),

    /// System-level event for global monitoring.
    ///
    /// Events are stored in system-wide storage and used for
    /// aggregate metrics and operational monitoring.
    System,
}

/// Pre-defined event type constants for consistency.
///
/// Use these constants when creating events to ensure consistent
/// event type strings across your application.
///
/// # Example
///
/// ```rust,ignore
/// use multi_llm::{BusinessEvent, event_types};
///
/// let event = BusinessEvent::new(event_types::LLM_REQUEST);
/// ```
///
/// # Feature Flag
///
/// Requires the `events` feature to be enabled.
#[cfg(feature = "events")]
pub mod event_types {
    /// LLM request initiated.
    pub const LLM_REQUEST: &str = "llm_request";

    /// LLM response received successfully.
    pub const LLM_RESPONSE: &str = "llm_response";

    /// LLM request failed with an error.
    pub const LLM_ERROR: &str = "llm_error";

    /// Prompt cache hit (content was cached).
    pub const CACHE_HIT: &str = "cache_hit";

    /// Prompt cache miss (content was not cached).
    pub const CACHE_MISS: &str = "cache_miss";

    /// Generic error event.
    pub const ERROR: &str = "error";
}
