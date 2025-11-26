//! Unit Tests for Messages Module
//!
//! UNIT UNDER TEST: CacheType enum and UnifiedMessage cache builder methods
//!
//! BUSINESS RESPONSIBILITY:
//!   - Provides cache type enum for Anthropic prompt caching (Ephemeral/Extended)
//!   - Provides builder methods to mark messages for specific cache types
//!   - Ensures messages can be marked for 5-minute (ephemeral) or 1-hour (extended) caching
//!
//! TEST COVERAGE:
//!   - CacheType enum variants and default behavior
//!   - Builder methods set correct cache_type and cacheable flags
//!   - Builder methods are chainable and preserve message data
//!   - Edge cases: builder methods on messages with different roles and content types

use crate::messages::{CacheType, MessageRole, UnifiedMessage};

// ============================================================================
// CacheType Enum Tests
// ============================================================================

#[test]
fn test_cache_type_default_is_ephemeral() {
    // Test verifies CacheType::default() returns Ephemeral variant
    //
    // Business rule: Ephemeral (5-minute) cache is the default for safety

    // Act
    let cache_type = CacheType::default();

    // Assert
    assert_eq!(cache_type, CacheType::Ephemeral);
}

#[test]
fn test_cache_type_variants_exist() {
    // Test verifies both cache type variants are available
    //
    // Business rule: Support both Anthropic cache TTL options

    // Act & Assert - verify both variants compile and are distinct
    let ephemeral = CacheType::Ephemeral;
    let extended = CacheType::Extended;

    assert_ne!(ephemeral, extended, "Cache types should be distinct");
}

// ============================================================================
// Builder Method Tests - with_ephemeral_cache()
// ============================================================================

#[test]
fn test_with_ephemeral_cache_sets_cache_type() {
    // Test verifies with_ephemeral_cache() sets cache_type to Ephemeral
    //
    // Business rule: Ephemeral cache marks content for 5-minute caching

    // Arrange
    let message = UnifiedMessage::user("Test content");

    // Act
    let cached_message = message.with_ephemeral_cache();

    // Assert
    assert_eq!(
        cached_message.attributes.cache_type,
        Some(CacheType::Ephemeral),
        "cache_type should be set to Ephemeral"
    );
}

#[test]
fn test_with_ephemeral_cache_sets_cacheable_true() {
    // Test verifies with_ephemeral_cache() enables caching
    //
    // Business rule: Cache type implies message is cacheable

    // Arrange
    let message = UnifiedMessage::user("Test content");
    assert!(!message.attributes.cacheable, "Should start non-cacheable");

    // Act
    let cached_message = message.with_ephemeral_cache();

    // Assert
    assert!(
        cached_message.attributes.cacheable,
        "cacheable flag should be set to true"
    );
}

#[test]
fn test_with_ephemeral_cache_preserves_message_content() {
    // Test verifies builder method doesn't alter message content
    //
    // Business rule: Cache control is metadata, not content modification

    // Arrange
    let original_content = "Important message content";
    let message = UnifiedMessage::user(original_content);

    // Act
    let cached_message = message.with_ephemeral_cache();

    // Assert
    match cached_message.content {
        crate::messages::MessageContent::Text(ref text) => {
            assert_eq!(text, original_content, "Content should be preserved");
        }
        _ => panic!("Expected text content"),
    }
}

#[test]
fn test_with_ephemeral_cache_preserves_role() {
    // Test verifies builder method doesn't alter message role
    //
    // Business rule: Cache control doesn't change message semantics

    // Arrange
    let message = UnifiedMessage::system("System prompt");

    // Act
    let cached_message = message.with_ephemeral_cache();

    // Assert
    assert_eq!(
        cached_message.role,
        MessageRole::System,
        "Role should be preserved"
    );
}

// ============================================================================
// Builder Method Tests - with_extended_cache()
// ============================================================================

#[test]
fn test_with_extended_cache_sets_cache_type() {
    // Test verifies with_extended_cache() sets cache_type to Extended
    //
    // Business rule: Extended cache marks content for 1-hour caching

    // Arrange
    let message = UnifiedMessage::user("Test content");

    // Act
    let cached_message = message.with_extended_cache();

    // Assert
    assert_eq!(
        cached_message.attributes.cache_type,
        Some(CacheType::Extended),
        "cache_type should be set to Extended"
    );
}

#[test]
fn test_with_extended_cache_sets_cacheable_true() {
    // Test verifies with_extended_cache() enables caching
    //
    // Business rule: Cache type implies message is cacheable

    // Arrange
    let message = UnifiedMessage::user("Test content");
    assert!(!message.attributes.cacheable, "Should start non-cacheable");

    // Act
    let cached_message = message.with_extended_cache();

    // Assert
    assert!(
        cached_message.attributes.cacheable,
        "cacheable flag should be set to true"
    );
}

#[test]
fn test_with_extended_cache_preserves_message_content() {
    // Test verifies builder method doesn't alter message content
    //
    // Business rule: Cache control is metadata, not content modification

    // Arrange
    let original_content = "Important message content";
    let message = UnifiedMessage::user(original_content);

    // Act
    let cached_message = message.with_extended_cache();

    // Assert
    match cached_message.content {
        crate::messages::MessageContent::Text(ref text) => {
            assert_eq!(text, original_content, "Content should be preserved");
        }
        _ => panic!("Expected text content"),
    }
}

// ============================================================================
// Builder Method Tests - Chaining and Edge Cases
// ============================================================================

#[test]
fn test_cache_builders_are_distinct() {
    // Test verifies ephemeral and extended builders produce different results
    //
    // Business rule: Two cache types must be distinguishable

    // Arrange
    let message = UnifiedMessage::user("Test content");

    // Act
    let ephemeral_cached = message.clone().with_ephemeral_cache();
    let extended_cached = message.with_extended_cache();

    // Assert
    assert_ne!(
        ephemeral_cached.attributes.cache_type, extended_cached.attributes.cache_type,
        "Different builders should produce different cache types"
    );
}

#[test]
fn test_cache_builder_on_system_message() {
    // Test verifies cache builders work with system messages
    //
    // Business rule: System prompts are prime candidates for caching

    // Arrange
    let message = UnifiedMessage::system("You are a helpful assistant");

    // Act
    let cached_message = message.with_extended_cache();

    // Assert
    assert_eq!(cached_message.role, MessageRole::System);
    assert_eq!(
        cached_message.attributes.cache_type,
        Some(CacheType::Extended)
    );
    assert!(cached_message.attributes.cacheable);
}

#[test]
fn test_cache_builder_on_assistant_message() {
    // Test verifies cache builders work with assistant messages
    //
    // Business rule: All message roles support caching

    // Arrange
    let message = UnifiedMessage::assistant("Previous response");

    // Act
    let cached_message = message.with_ephemeral_cache();

    // Assert
    assert_eq!(cached_message.role, MessageRole::Assistant);
    assert_eq!(
        cached_message.attributes.cache_type,
        Some(CacheType::Ephemeral)
    );
    assert!(cached_message.attributes.cacheable);
}

#[test]
fn test_overwriting_cache_type() {
    // Test verifies calling a second builder overwrites the first
    //
    // Business rule: Last cache type specification wins

    // Arrange
    let message = UnifiedMessage::user("Test content");

    // Act - apply ephemeral then extended
    let cached_message = message.with_ephemeral_cache().with_extended_cache();

    // Assert - should be extended (last applied)
    assert_eq!(
        cached_message.attributes.cache_type,
        Some(CacheType::Extended),
        "Second builder should overwrite first"
    );
}

// ============================================================================
// MessageRole Display Tests
// ============================================================================

#[test]
fn test_message_role_display_system() {
    // Test verifies MessageRole::System displays as "system"
    assert_eq!(format!("{}", MessageRole::System), "system");
}

#[test]
fn test_message_role_display_user() {
    // Test verifies MessageRole::User displays as "user"
    assert_eq!(format!("{}", MessageRole::User), "user");
}

#[test]
fn test_message_role_display_assistant() {
    // Test verifies MessageRole::Assistant displays as "assistant"
    assert_eq!(format!("{}", MessageRole::Assistant), "assistant");
}

#[test]
fn test_message_role_display_tool() {
    // Test verifies MessageRole::Tool displays as "tool"
    assert_eq!(format!("{}", MessageRole::Tool), "tool");
}

// ============================================================================
// MessageContent Display Tests
// ============================================================================

use crate::messages::MessageContent;

#[test]
fn test_message_content_display_text() {
    // Test verifies MessageContent::Text displays the text content
    let content = MessageContent::Text("Hello, world!".to_string());
    assert_eq!(format!("{}", content), "Hello, world!");
}

#[test]
fn test_message_content_display_json() {
    // Test verifies MessageContent::Json displays pretty-printed JSON
    let content = MessageContent::Json(serde_json::json!({"key": "value"}));
    let display = format!("{}", content);
    assert!(display.contains("key"));
    assert!(display.contains("value"));
}

#[test]
fn test_message_content_display_tool_call() {
    // Test verifies MessageContent::ToolCall displays name and args
    let content = MessageContent::ToolCall {
        id: "call_123".to_string(),
        name: "get_weather".to_string(),
        arguments: serde_json::json!({"city": "London"}),
    };
    let display = format!("{}", content);
    assert!(display.contains("get_weather"));
    assert!(display.contains("London"));
}

#[test]
fn test_message_content_display_tool_result_success() {
    // Test verifies MessageContent::ToolResult displays content for success
    let content = MessageContent::ToolResult {
        tool_call_id: "call_123".to_string(),
        content: "Sunny, 22°C".to_string(),
        is_error: false,
    };
    assert_eq!(format!("{}", content), "Sunny, 22°C");
}

#[test]
fn test_message_content_display_tool_result_error() {
    // Test verifies MessageContent::ToolResult displays "Error:" prefix for errors
    let content = MessageContent::ToolResult {
        tool_call_id: "call_123".to_string(),
        content: "API timeout".to_string(),
        is_error: true,
    };
    let display = format!("{}", content);
    assert!(display.starts_with("Error:"));
    assert!(display.contains("API timeout"));
}

// ============================================================================
// Semantic Constructor Tests
// ============================================================================

use crate::messages::{MessageAttributes, MessageCategory};

#[test]
fn test_unified_message_with_attributes() {
    // Test verifies with_attributes creates message with custom attributes
    let attrs = MessageAttributes {
        priority: 10,
        cacheable: true,
        cache_type: Some(CacheType::Extended),
        cache_key: Some("custom-key".to_string()),
        category: MessageCategory::Context,
        metadata: std::collections::HashMap::new(),
    };

    let message = UnifiedMessage::with_attributes(
        MessageRole::System,
        MessageContent::Text("Test content".to_string()),
        attrs,
    );

    assert_eq!(message.role, MessageRole::System);
    assert_eq!(message.attributes.priority, 10);
    assert!(message.attributes.cacheable);
    assert_eq!(message.attributes.cache_type, Some(CacheType::Extended));
    assert_eq!(message.attributes.cache_key, Some("custom-key".to_string()));
    assert_eq!(message.attributes.category, MessageCategory::Context);
}

#[test]
fn test_system_instruction_constructor() {
    // Test verifies system_instruction creates cacheable high-priority message
    //
    // Business rule: System instructions are priority 0 and cacheable
    let message =
        UnifiedMessage::system_instruction("You are helpful.".to_string(), Some("v1".to_string()));

    assert_eq!(message.role, MessageRole::System);
    assert_eq!(message.attributes.priority, 0);
    assert!(message.attributes.cacheable);
    assert_eq!(message.attributes.cache_key, Some("v1".to_string()));
    assert_eq!(
        message.attributes.category,
        MessageCategory::SystemInstruction
    );
}

#[test]
fn test_system_instruction_without_cache_key() {
    // Test verifies system_instruction works without cache key
    let message = UnifiedMessage::system_instruction("You are helpful.".to_string(), None);

    assert_eq!(message.attributes.cache_key, None);
    assert!(message.attributes.cacheable);
}

#[test]
fn test_tool_definition_constructor() {
    // Test verifies tool_definition creates cacheable priority-1 message
    //
    // Business rule: Tool definitions come after system instructions
    let message =
        UnifiedMessage::tool_definition("Tool schema".to_string(), Some("tools-v1".to_string()));

    assert_eq!(message.role, MessageRole::System);
    assert_eq!(message.attributes.priority, 1);
    assert!(message.attributes.cacheable);
    assert_eq!(message.attributes.category, MessageCategory::ToolDefinition);
}

#[test]
fn test_context_constructor() {
    // Test verifies context creates cacheable medium-priority message
    //
    // Business rule: Context is priority 5
    let message = UnifiedMessage::context("User preferences".to_string(), None);

    assert_eq!(message.role, MessageRole::System);
    assert_eq!(message.attributes.priority, 5);
    assert!(message.attributes.cacheable);
    assert_eq!(message.attributes.category, MessageCategory::Context);
}

#[test]
fn test_history_constructor() {
    // Test verifies history creates cacheable lower-priority message
    //
    // Business rule: History is priority 20
    let message = UnifiedMessage::history(MessageRole::User, "Previous question".to_string());

    assert_eq!(message.role, MessageRole::User);
    assert_eq!(message.attributes.priority, 20);
    assert!(message.attributes.cacheable);
    assert_eq!(message.attributes.category, MessageCategory::History);
}

#[test]
fn test_current_user_constructor() {
    // Test verifies current_user creates non-cacheable lowest-priority message
    //
    // Business rule: Current user input is priority 30 and not cached
    let message = UnifiedMessage::current_user("What's the weather?".to_string());

    assert_eq!(message.role, MessageRole::User);
    assert_eq!(message.attributes.priority, 30);
    assert!(!message.attributes.cacheable);
    assert_eq!(message.attributes.category, MessageCategory::Current);
}

#[test]
fn test_tool_call_constructor() {
    // Test verifies tool_call creates assistant message with ToolCall content
    let message = UnifiedMessage::tool_call(
        "call_abc".to_string(),
        "get_weather".to_string(),
        serde_json::json!({"city": "London"}),
    );

    assert_eq!(message.role, MessageRole::Assistant);
    assert_eq!(message.attributes.priority, 25);
    assert!(!message.attributes.cacheable);
    match message.content {
        MessageContent::ToolCall {
            id,
            name,
            arguments,
        } => {
            assert_eq!(id, "call_abc");
            assert_eq!(name, "get_weather");
            assert_eq!(arguments["city"], "London");
        }
        _ => panic!("Expected ToolCall content"),
    }
}

#[test]
fn test_tool_result_constructor() {
    // Test verifies tool_result creates tool message with ToolResult content
    let message =
        UnifiedMessage::tool_result("call_abc".to_string(), "Sunny, 22°C".to_string(), false);

    assert_eq!(message.role, MessageRole::Tool);
    assert_eq!(message.attributes.priority, 26);
    assert!(!message.attributes.cacheable);
    match message.content {
        MessageContent::ToolResult {
            tool_call_id,
            content,
            is_error,
        } => {
            assert_eq!(tool_call_id, "call_abc");
            assert_eq!(content, "Sunny, 22°C");
            assert!(!is_error);
        }
        _ => panic!("Expected ToolResult content"),
    }
}

#[test]
fn test_tool_result_constructor_with_error() {
    // Test verifies tool_result handles error flag correctly
    let message = UnifiedMessage::tool_result(
        "call_abc".to_string(),
        "Connection failed".to_string(),
        true,
    );

    match message.content {
        MessageContent::ToolResult { is_error, .. } => {
            assert!(is_error);
        }
        _ => panic!("Expected ToolResult content"),
    }
}

// ============================================================================
// UnifiedLLMRequest Tests
// ============================================================================

use crate::messages::UnifiedLLMRequest;
use crate::provider::RequestConfig;

#[test]
fn test_unified_llm_request_new() {
    // Test verifies new() creates request with messages only
    let messages = vec![
        UnifiedMessage::system("System prompt"),
        UnifiedMessage::user("User input"),
    ];

    let request = UnifiedLLMRequest::new(messages);

    assert_eq!(request.messages.len(), 2);
    assert!(request.response_schema.is_none());
    assert!(request.config.is_none());
}

#[test]
fn test_unified_llm_request_with_schema() {
    // Test verifies with_schema() creates request with response schema
    let messages = vec![UnifiedMessage::user("Extract data")];
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        }
    });

    let request = UnifiedLLMRequest::with_schema(messages, schema.clone());

    assert_eq!(request.messages.len(), 1);
    assert_eq!(request.response_schema, Some(schema));
    assert!(request.config.is_none());
}

#[test]
fn test_unified_llm_request_with_config() {
    // Test verifies with_config() creates request with config override
    let messages = vec![UnifiedMessage::user("Hello")];
    let config = RequestConfig {
        temperature: Some(0.7),
        max_tokens: Some(1000),
        ..Default::default()
    };

    let request = UnifiedLLMRequest::with_config(messages, config);

    assert_eq!(request.messages.len(), 1);
    assert!(request.response_schema.is_none());
    assert!(request.config.is_some());
    assert_eq!(request.config.as_ref().unwrap().temperature, Some(0.7));
}

#[test]
fn test_sort_messages_by_priority() {
    // Test verifies sort_messages() orders by priority (lower = first)
    let messages = vec![
        UnifiedMessage::current_user("User input".to_string()), // priority 30
        UnifiedMessage::system_instruction("System".to_string(), None), // priority 0
        UnifiedMessage::context("Context".to_string(), None),   // priority 5
    ];

    let mut request = UnifiedLLMRequest::new(messages);
    request.sort_messages();

    assert_eq!(request.messages[0].attributes.priority, 0);
    assert_eq!(request.messages[1].attributes.priority, 5);
    assert_eq!(request.messages[2].attributes.priority, 30);
}

#[test]
fn test_get_sorted_messages_does_not_modify_original() {
    // Test verifies get_sorted_messages() returns sorted view without modifying
    let messages = vec![
        UnifiedMessage::current_user("User input".to_string()), // priority 30
        UnifiedMessage::system_instruction("System".to_string(), None), // priority 0
    ];

    let request = UnifiedLLMRequest::new(messages);
    let sorted = request.get_sorted_messages();

    // Original unchanged
    assert_eq!(request.messages[0].attributes.priority, 30);
    // Sorted view is ordered
    assert_eq!(sorted[0].attributes.priority, 0);
    assert_eq!(sorted[1].attributes.priority, 30);
}

#[test]
fn test_get_system_messages() {
    // Test verifies get_system_messages() filters to system role only
    let messages = vec![
        UnifiedMessage::system("System 1"),
        UnifiedMessage::user("User"),
        UnifiedMessage::system("System 2"),
        UnifiedMessage::assistant("Assistant"),
    ];

    let request = UnifiedLLMRequest::new(messages);
    let system_msgs = request.get_system_messages();

    assert_eq!(system_msgs.len(), 2);
    assert!(system_msgs.iter().all(|m| m.role == MessageRole::System));
}

#[test]
fn test_get_conversation_messages() {
    // Test verifies get_conversation_messages() excludes system messages
    let messages = vec![
        UnifiedMessage::system("System"),
        UnifiedMessage::user("User"),
        UnifiedMessage::assistant("Assistant"),
        UnifiedMessage::tool_result("id".to_string(), "result".to_string(), false),
    ];

    let request = UnifiedLLMRequest::new(messages);
    let conv_msgs = request.get_conversation_messages();

    assert_eq!(conv_msgs.len(), 3);
    assert!(conv_msgs.iter().all(|m| m.role != MessageRole::System));
}

#[test]
fn test_get_cacheable_messages() {
    // Test verifies get_cacheable_messages() filters to cacheable only
    let messages = vec![
        UnifiedMessage::system_instruction("Cacheable system".to_string(), None), // cacheable
        UnifiedMessage::current_user("Not cached".to_string()),                   // not cacheable
        UnifiedMessage::context("Cacheable context".to_string(), None),           // cacheable
    ];

    let request = UnifiedLLMRequest::new(messages);
    let cacheable = request.get_cacheable_messages();

    assert_eq!(cacheable.len(), 2);
    assert!(cacheable.iter().all(|m| m.attributes.cacheable));
}
