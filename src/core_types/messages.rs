//! Unified message architecture for LLM interactions
//!
//! This is a core feature of multi-llm - provider-agnostic message handling with caching hints.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Message roles for LLM interactions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::System => write!(f, "system"),
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::Tool => write!(f, "tool"),
        }
    }
}

/// Message content types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageContent {
    /// Plain text content
    Text(String),
    /// Structured JSON content
    Json(serde_json::Value),
    /// Tool call request
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    /// Tool execution result
    ToolResult {
        tool_call_id: String,
        content: String,
        is_error: bool,
    },
}

impl std::fmt::Display for MessageContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageContent::Text(text) => write!(f, "{}", text),
            MessageContent::Json(value) => {
                write!(
                    f,
                    "{}",
                    serde_json::to_string_pretty(value).unwrap_or_default()
                )
            }
            MessageContent::ToolCall {
                name, arguments, ..
            } => {
                write!(
                    f,
                    "Tool call: {} with args: {}",
                    name,
                    serde_json::to_string(arguments).unwrap_or_default()
                )
            }
            MessageContent::ToolResult {
                content, is_error, ..
            } => {
                if *is_error {
                    write!(f, "Error: {}", content)
                } else {
                    write!(f, "{}", content)
                }
            }
        }
    }
}

/// Message categories for semantic grouping
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageCategory {
    /// Core system prompts and instructions
    SystemInstruction,
    /// Tool/function definitions
    ToolDefinition,
    /// Story/user context
    Context,
    /// Conversation history
    History,
    /// Current turn
    Current,
    /// Tool execution results
    ToolResult,
}

/// Message attributes that guide provider behavior
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageAttributes {
    /// Priority for message ordering (lower = higher priority)
    pub priority: u8,
    /// Whether this message content is static and cacheable
    pub cacheable: bool,
    /// Optional cache key for deduplication
    pub cache_key: Option<String>,
    /// Message category for provider-specific handling
    pub category: MessageCategory,
    /// Custom metadata for future extensions
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for MessageAttributes {
    fn default() -> Self {
        Self {
            priority: 50,
            cacheable: false,
            cache_key: None,
            category: MessageCategory::Current,
            metadata: HashMap::new(),
        }
    }
}

/// Universal message for LLM interactions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedMessage {
    /// Message role
    pub role: MessageRole,
    /// Message content
    pub content: MessageContent,
    /// Message attributes for provider optimization
    pub attributes: MessageAttributes,
    /// Timestamp for ordering if not using priority
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl UnifiedMessage {
    /// Create a new message with default attributes
    pub fn new(role: MessageRole, content: MessageContent) -> Self {
        Self {
            role,
            content,
            attributes: MessageAttributes::default(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a new message with custom attributes
    pub fn with_attributes(
        role: MessageRole,
        content: MessageContent,
        attributes: MessageAttributes,
    ) -> Self {
        Self {
            role,
            content,
            attributes,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a system instruction message (cacheable, high priority)
    pub fn system_instruction(content: String, cache_key: Option<String>) -> Self {
        Self::with_attributes(
            MessageRole::System,
            MessageContent::Text(content),
            MessageAttributes {
                priority: 0,
                cacheable: true,
                cache_key,
                category: MessageCategory::SystemInstruction,
                metadata: HashMap::new(),
            },
        )
    }

    /// Create a tool definition message (cacheable, high priority)
    pub fn tool_definition(content: String, cache_key: Option<String>) -> Self {
        Self::with_attributes(
            MessageRole::System,
            MessageContent::Text(content),
            MessageAttributes {
                priority: 1,
                cacheable: true,
                cache_key,
                category: MessageCategory::ToolDefinition,
                metadata: HashMap::new(),
            },
        )
    }

    /// Create a context message (cacheable, medium priority)
    pub fn context(content: String, cache_key: Option<String>) -> Self {
        Self::with_attributes(
            MessageRole::System,
            MessageContent::Text(content),
            MessageAttributes {
                priority: 5,
                cacheable: true,
                cache_key,
                category: MessageCategory::Context,
                metadata: HashMap::new(),
            },
        )
    }

    /// Create a history message (cacheable, lower priority)
    pub fn history(role: MessageRole, content: String) -> Self {
        Self::with_attributes(
            role,
            MessageContent::Text(content),
            MessageAttributes {
                priority: 20,
                cacheable: true,
                cache_key: None,
                category: MessageCategory::History,
                metadata: HashMap::new(),
            },
        )
    }

    /// Create a current user message (not cacheable, lowest priority)
    pub fn current_user(content: String) -> Self {
        Self::with_attributes(
            MessageRole::User,
            MessageContent::Text(content),
            MessageAttributes {
                priority: 30,
                cacheable: false,
                cache_key: None,
                category: MessageCategory::Current,
                metadata: HashMap::new(),
            },
        )
    }

    /// Create a tool call message
    pub fn tool_call(id: String, name: String, arguments: serde_json::Value) -> Self {
        Self::with_attributes(
            MessageRole::Assistant,
            MessageContent::ToolCall {
                id,
                name,
                arguments,
            },
            MessageAttributes {
                priority: 25,
                cacheable: false,
                cache_key: None,
                category: MessageCategory::ToolResult,
                metadata: HashMap::new(),
            },
        )
    }

    /// Create a tool result message
    pub fn tool_result(tool_call_id: String, content: String, is_error: bool) -> Self {
        Self::with_attributes(
            MessageRole::Tool,
            MessageContent::ToolResult {
                tool_call_id,
                content,
                is_error,
            },
            MessageAttributes {
                priority: 26,
                cacheable: false,
                cache_key: None,
                category: MessageCategory::ToolResult,
                metadata: HashMap::new(),
            },
        )
    }

    // Convenience constructors

    /// Create a simple text message
    pub fn simple(role: MessageRole, content: impl Into<String>) -> Self {
        Self::new(role, MessageContent::Text(content.into()))
    }

    /// Create a simple user message
    pub fn user(content: impl Into<String>) -> Self {
        Self::simple(MessageRole::User, content)
    }

    /// Create a simple assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::simple(MessageRole::Assistant, content)
    }

    /// Create a simple system message
    pub fn system(content: impl Into<String>) -> Self {
        Self::simple(MessageRole::System, content)
    }
}

/// Unified request for LLM operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedLLMRequest {
    /// All messages in priority order
    pub messages: Vec<UnifiedMessage>,
    /// Optional response schema for structured output
    pub response_schema: Option<serde_json::Value>,
    /// Configuration for this request
    pub config: Option<crate::core_types::provider::RequestConfig>,
}

impl UnifiedLLMRequest {
    /// Create a new request with messages
    pub fn new(messages: Vec<UnifiedMessage>) -> Self {
        Self {
            messages,
            response_schema: None,
            config: None,
        }
    }

    /// Create a new request with schema
    pub fn with_schema(messages: Vec<UnifiedMessage>, schema: serde_json::Value) -> Self {
        Self {
            messages,
            response_schema: Some(schema),
            config: None,
        }
    }

    /// Create a new request with config
    pub fn with_config(
        messages: Vec<UnifiedMessage>,
        config: crate::core_types::provider::RequestConfig,
    ) -> Self {
        Self {
            messages,
            response_schema: None,
            config: Some(config),
        }
    }

    /// Sort messages by priority and timestamp
    pub fn sort_messages(&mut self) {
        self.messages.sort_by(|a, b| {
            a.attributes
                .priority
                .cmp(&b.attributes.priority)
                .then_with(|| a.timestamp.cmp(&b.timestamp))
        });
    }

    /// Get messages sorted by priority (does not modify original)
    pub fn get_sorted_messages(&self) -> Vec<&UnifiedMessage> {
        let mut sorted: Vec<&UnifiedMessage> = self.messages.iter().collect();
        sorted.sort_by(|a, b| {
            a.attributes
                .priority
                .cmp(&b.attributes.priority)
                .then_with(|| a.timestamp.cmp(&b.timestamp))
        });
        sorted
    }

    /// Get system messages
    pub fn get_system_messages(&self) -> Vec<&UnifiedMessage> {
        self.messages
            .iter()
            .filter(|msg| msg.role == MessageRole::System)
            .collect()
    }

    /// Get non-system messages
    pub fn get_conversation_messages(&self) -> Vec<&UnifiedMessage> {
        self.messages
            .iter()
            .filter(|msg| msg.role != MessageRole::System)
            .collect()
    }

    /// Get cacheable messages
    pub fn get_cacheable_messages(&self) -> Vec<&UnifiedMessage> {
        self.messages
            .iter()
            .filter(|msg| msg.attributes.cacheable)
            .collect()
    }
}
