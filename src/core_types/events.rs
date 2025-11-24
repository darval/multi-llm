//! Business event types for LLM operations
//!
//! These events track LLM interactions for observability and debugging.
//!
//! This module is only available when the `events` feature is enabled.
//! Enable with: `cargo add multi-llm --features events`

#[cfg(feature = "events")]
use chrono::{DateTime, Utc};
#[cfg(feature = "events")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "events")]
use uuid::Uuid;

/// Business event for analytics and observability
///
/// Only available with the `events` feature enabled.
#[cfg(feature = "events")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BusinessEvent {
    /// Unique identifier for this event
    pub id: Uuid,
    /// Event type (e.g., "llm_request", "llm_response")
    pub event_type: String,
    /// Flexible metadata as JSON
    pub metadata: serde_json::Value,
    /// Timestamp when event was created
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

/// Event scope - determines which storage backend to use
///
/// Only available with the `events` feature enabled.
#[cfg(feature = "events")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventScope {
    /// User-scoped event (written to user storage)
    User(String),
    /// System-level event (written to system storage)
    System,
}

/// Pre-defined event types for consistency
///
/// Only available with the `events` feature enabled.
#[cfg(feature = "events")]
pub mod event_types {
    /// LLM request initiated
    pub const LLM_REQUEST: &str = "llm_request";
    /// LLM response received
    pub const LLM_RESPONSE: &str = "llm_response";
    /// LLM request failed
    pub const LLM_ERROR: &str = "llm_error";
    /// Cache hit
    pub const CACHE_HIT: &str = "cache_hit";
    /// Cache miss
    pub const CACHE_MISS: &str = "cache_miss";
    /// Generic error event
    pub const ERROR: &str = "error";
}
