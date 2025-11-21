//! Agent context types for the three-agent system
//!
//! This module provides agent-aware context types that enhance the core LLM client
//! with agent-specific monitoring and logging.
//!
//! Following KISS principles: simple context tracking, no premature optimizations.

use crate::retry::RetryPolicy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Agent context for request tracking and customization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    /// Name of the agent making the request ("ConversationAgent", "StoryMemoryAgent", "ContextManagerAgent")
    pub agent_name: String,
    /// Session identifier for request correlation
    pub session_id: String,
    /// Agent-specific metadata for logging and monitoring
    pub metadata: HashMap<String, serde_json::Value>,
    /// Optional agent-specific retry policy override
    pub retry_policy: Option<RetryPolicy>,
}

impl AgentContext {
    /// Create context for ConversationAgent
    pub fn conversation(session_id: impl Into<String>) -> Self {
        Self {
            agent_name: "ConversationAgent".to_string(),
            session_id: session_id.into(),
            metadata: HashMap::from([
                ("pattern".into(), serde_json::json!("ReAct")),
                ("priority".into(), serde_json::json!("real-time")),
            ]),
            retry_policy: None,
        }
    }

    /// Create context for StoryMemoryAgent
    pub fn story_memory(session_id: impl Into<String>) -> Self {
        Self {
            agent_name: "StoryMemoryAgent".to_string(),
            session_id: session_id.into(),
            metadata: HashMap::from([
                ("function".into(), serde_json::json!("extraction")),
                ("priority".into(), serde_json::json!("accuracy")),
            ]),
            retry_policy: None,
        }
    }

    /// Create context for ContextManagerAgent
    pub fn context_manager(session_id: impl Into<String>) -> Self {
        Self {
            agent_name: "ContextManagerAgent".to_string(),
            session_id: session_id.into(),
            metadata: HashMap::from([
                ("function".into(), serde_json::json!("context-selection")),
                ("priority".into(), serde_json::json!("efficiency")),
            ]),
            retry_policy: None,
        }
    }

    /// Add custom metadata entry
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Set custom retry policy for this agent request
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }
}
