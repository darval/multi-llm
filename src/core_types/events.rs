//! Business event types extracted from mystory-core
//!
//! Phase 2 will review whether these should be:
//! - Optional via feature flag
//! - Generalized for non-mystory use
//! - Provided as a trait for consumers to implement

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Business event for analytics and observability
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

impl BusinessEvent {
    /// Create a new business event
    pub fn new(event_type: impl Into<String>, metadata: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: event_type.into(),
            metadata,
            created_at: Utc::now(),
        }
    }
}

/// Event scope - determines which storage backend to use
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventScope {
    /// User-scoped event (written to user storage)
    User(String),
    /// System-level event (written to system storage)
    System,
}

/// Pre-defined event types for consistency
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
