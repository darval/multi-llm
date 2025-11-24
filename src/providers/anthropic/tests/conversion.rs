//! Unit Tests for Anthropic Message Conversion Logic
//!
//! UNIT UNDER TEST: transform_unified_messages and related functions
//!
//! BUSINESS RESPONSIBILITY:
//!   - Convert UnifiedMessage format to Anthropic API format
//!   - Separate system messages from conversation messages
//!   - Apply cache control breakpoints following Anthropic hierarchy
//!   - Implement 2-breakpoint progressive sliding window caching strategy
//!   - Combine consecutive same-role messages
//!   - Handle empty text blocks (Anthropic rejects cache_control on empty text)
//!   - Convert tool calls and tool results to Anthropic format
//!
//! TEST COVERAGE:
//!   - Message separation (system vs conversation)
//!   - Cache breakpoint placement (2-breakpoint sliding window strategy)
//!   - Consecutive message combination
//!   - Empty text handling (no cache control on empty)
//!   - Tool role conversion (Tool → user in Anthropic)
//!   - Content merging for same-role consecutive messages
//!   - Caching enabled vs disabled scenarios
//!   - Different message counts (1-2, 3-5, 6+)

use super::super::conversion::transform_unified_messages;
use super::super::types::{AnthropicContent, AnthropicContentBlock};
use crate::config::AnthropicConfig;
use crate::core_types::messages::{
    CacheType, MessageAttributes, MessageCategory, MessageContent, MessageRole, UnifiedMessage,
};
use crate::retry::RetryPolicy;
use chrono::Utc;
use std::collections::HashMap;

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_config() -> AnthropicConfig {
    AnthropicConfig {
        api_key: Some("test-key".to_string()),
        base_url: "https://api.anthropic.com".to_string(),
        default_model: "claude-3-5-sonnet-20241022".to_string(),
        max_context_tokens: 200_000,
        retry_policy: RetryPolicy::default(),
        enable_prompt_caching: true,
        cache_ttl: "1h".to_string(),
    }
}

fn create_message(role: MessageRole, content: &str, cacheable: bool) -> UnifiedMessage {
    UnifiedMessage {
        role,
        content: MessageContent::Text(content.to_string()),
        attributes: MessageAttributes {
            priority: 0,
            cacheable,
            cache_type: None,
            cache_key: None,
            category: MessageCategory::Current,
            metadata: HashMap::new(),
        },
        timestamp: Utc::now(),
    }
}

fn create_message_with_cache_type(
    role: MessageRole,
    content: &str,
    cache_type: CacheType,
) -> UnifiedMessage {
    UnifiedMessage {
        role,
        content: MessageContent::Text(content.to_string()),
        attributes: MessageAttributes {
            priority: 0,
            cacheable: true,
            cache_type: Some(cache_type),
            cache_key: None,
            category: MessageCategory::Current,
            metadata: HashMap::new(),
        },
        timestamp: Utc::now(),
    }
}

fn create_tool_call_message(id: &str, name: &str) -> UnifiedMessage {
    UnifiedMessage {
        role: MessageRole::Assistant,
        content: MessageContent::ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments: serde_json::json!({"arg": "value"}),
        },
        attributes: MessageAttributes {
            priority: 0,
            cacheable: false,
            cache_type: None,
            cache_key: None,
            category: MessageCategory::Current,
            metadata: HashMap::new(),
        },
        timestamp: Utc::now(),
    }
}

fn create_tool_result_message(tool_call_id: &str, content: &str, is_error: bool) -> UnifiedMessage {
    UnifiedMessage {
        role: MessageRole::Tool,
        content: MessageContent::ToolResult {
            tool_call_id: tool_call_id.to_string(),
            content: content.to_string(),
            is_error,
        },
        attributes: MessageAttributes {
            priority: 0,
            cacheable: false,
            cache_type: None,
            cache_key: None,
            category: MessageCategory::Current,
            metadata: HashMap::new(),
        },
        timestamp: Utc::now(),
    }
}

// ============================================================================
// System/Conversation Separation Tests
// ============================================================================

#[test]
fn test_empty_messages() {
    // Test verifies empty message list returns empty results
    // Edge case: no messages provided

    let config = create_test_config();
    let messages: Vec<&UnifiedMessage> = vec![];

    let (system, conversation) = transform_unified_messages(&messages, &config, true);

    assert!(
        system.is_empty(),
        "Empty input should produce no system messages"
    );
    assert!(
        conversation.is_empty(),
        "Empty input should produce no conversation messages"
    );
}

#[test]
fn test_system_messages_separated() {
    // Test verifies system messages are separated from conversation
    // Anthropic API requires system messages in separate field

    let config = create_test_config();
    let system1 = create_message(MessageRole::System, "You are helpful.", false);
    let system2 = create_message(MessageRole::System, "Be concise.", false);
    let user = create_message(MessageRole::User, "Hello", false);

    let messages = vec![&system1, &system2, &user];
    let (system, conversation) = transform_unified_messages(&messages, &config, false);

    assert_eq!(system.len(), 2, "Should have 2 system messages");
    assert_eq!(system[0].text, "You are helpful.");
    assert_eq!(system[1].text, "Be concise.");
    assert_eq!(conversation.len(), 1, "Should have 1 conversation message");
}

#[test]
fn test_only_system_messages() {
    // Test verifies handling of system-only input
    // Some requests may only set system context

    let config = create_test_config();
    let system = create_message(MessageRole::System, "System prompt", false);

    let messages = vec![&system];
    let (sys_msgs, conv_msgs) = transform_unified_messages(&messages, &config, false);

    assert_eq!(sys_msgs.len(), 1);
    assert!(conv_msgs.is_empty(), "Should have no conversation messages");
}

#[test]
fn test_only_conversation_messages() {
    // Test verifies handling of conversation-only input
    // Most common case: no system prompt

    let config = create_test_config();
    let user = create_message(MessageRole::User, "Question", false);
    let assistant = create_message(MessageRole::Assistant, "Answer", false);

    let messages = vec![&user, &assistant];
    let (sys_msgs, conv_msgs) = transform_unified_messages(&messages, &config, false);

    assert!(sys_msgs.is_empty(), "Should have no system messages");
    assert_eq!(conv_msgs.len(), 2);
}

// ============================================================================
// Cache Breakpoint Strategy Tests (CRITICAL BUSINESS LOGIC)
// ============================================================================

#[test]
fn test_cache_strategy_single_message() {
    // Test verifies cache breakpoint for 1-message conversation
    // Strategy: Cache first message (index 0)

    let config = create_test_config();
    let msg = create_message(MessageRole::User, "Hello", true);

    let messages = vec![&msg];
    let (_, conversation) = transform_unified_messages(&messages, &config, true);

    assert_eq!(conversation.len(), 1);
    // First message should have cache control
    match &conversation[0].content {
        AnthropicContent::Blocks(blocks) => {
            assert!(
                blocks[0].has_cache_control(),
                "Single message should be cached"
            );
        }
        _ => panic!("Expected Blocks content"),
    }
}

#[test]
fn test_cache_strategy_two_messages() {
    // Test verifies cache breakpoint for 2-message conversation
    // Strategy: Cache first message (index 0)

    let config = create_test_config();
    let msg1 = create_message(MessageRole::User, "First", true);
    let msg2 = create_message(MessageRole::Assistant, "Second", true);

    let messages = vec![&msg1, &msg2];
    let (_, conversation) = transform_unified_messages(&messages, &config, true);

    assert_eq!(conversation.len(), 2);
    // First message should be cached
    if let AnthropicContent::Blocks(blocks) = &conversation[0].content {
        assert!(
            blocks[0].has_cache_control(),
            "First message should be cached"
        );
    }
}

#[test]
fn test_cache_strategy_three_to_five_messages() {
    // Test verifies cache breakpoint for 3-5 message conversations
    // Strategy: Cache messages at index 0 and 2
    // NOTE: Use alternating roles to prevent message combination

    let config = create_test_config();
    let messages_data: Vec<UnifiedMessage> = vec![
        create_message(MessageRole::User, "Message 0", true),
        create_message(MessageRole::Assistant, "Message 1", true),
        create_message(MessageRole::User, "Message 2", true),
        create_message(MessageRole::Assistant, "Message 3", true),
    ];
    let message_refs: Vec<&UnifiedMessage> = messages_data.iter().collect();

    let (_, conversation) = transform_unified_messages(&message_refs, &config, true);

    assert_eq!(
        conversation.len(),
        4,
        "Should have 4 messages with alternating roles"
    );
    // Messages at index 0 and 2 should be cached
    if let AnthropicContent::Blocks(blocks) = &conversation[0].content {
        assert!(blocks[0].has_cache_control(), "Message 0 should be cached");
    }
    if let AnthropicContent::Blocks(blocks) = &conversation[2].content {
        assert!(blocks[0].has_cache_control(), "Message 2 should be cached");
    }
}

#[test]
fn test_cache_strategy_six_plus_messages() {
    // Test verifies cache breakpoint for 6+ message conversations
    // Strategy: Cache last 2 messages (sliding window)
    // CRITICAL: This is the progressive sliding window strategy
    // NOTE: Use alternating roles to prevent message combination

    let config = create_test_config();
    let messages_data: Vec<UnifiedMessage> = vec![
        create_message(MessageRole::User, "Message 0", true),
        create_message(MessageRole::Assistant, "Message 1", true),
        create_message(MessageRole::User, "Message 2", true),
        create_message(MessageRole::Assistant, "Message 3", true),
        create_message(MessageRole::User, "Message 4", true),
        create_message(MessageRole::Assistant, "Message 5", true),
        create_message(MessageRole::User, "Message 6", true),
        create_message(MessageRole::Assistant, "Message 7", true),
    ];
    let message_refs: Vec<&UnifiedMessage> = messages_data.iter().collect();

    let (_, conversation) = transform_unified_messages(&message_refs, &config, true);

    assert_eq!(
        conversation.len(),
        8,
        "Should have 8 messages with alternating roles"
    );
    // Last 2 messages (index 6 and 7) should be cached
    if let AnthropicContent::Blocks(blocks) = &conversation[6].content {
        assert!(
            blocks[0].has_cache_control(),
            "Second-to-last message should be cached"
        );
    }
    if let AnthropicContent::Blocks(blocks) = &conversation[7].content {
        assert!(
            blocks[0].has_cache_control(),
            "Last message should be cached"
        );
    }
}

#[test]
fn test_caching_disabled_no_cache_control() {
    // Test verifies no cache control applied when caching disabled
    // Even if messages are cacheable, caching can be turned off

    let config = create_test_config();
    let msg = create_message(MessageRole::User, "Test", true);

    let messages = vec![&msg];
    let (_, conversation) = transform_unified_messages(&messages, &config, false); // caching disabled

    match &conversation[0].content {
        AnthropicContent::Text(_) => {
            // Expected: simple text without cache control
        }
        AnthropicContent::Blocks(_) => {
            panic!("Should not have blocks when caching disabled");
        }
    }
}

#[test]
fn test_non_cacheable_messages_not_cached() {
    // Test verifies messages with cacheable=false don't get cache control
    // Even with caching enabled, individual messages can be non-cacheable

    let config = create_test_config();
    let msg = create_message(MessageRole::User, "Test", false); // cacheable=false

    let messages = vec![&msg];
    let (_, conversation) = transform_unified_messages(&messages, &config, true);

    match &conversation[0].content {
        AnthropicContent::Text(_) => {
            // Expected: no cache control for non-cacheable message
        }
        AnthropicContent::Blocks(_) => {
            panic!("Non-cacheable message should not have cache control");
        }
    }
}

// ============================================================================
// Consecutive Message Combination Tests
// ============================================================================

#[test]
fn test_consecutive_same_role_messages_combined() {
    // Test verifies consecutive messages with same role are merged
    // Anthropic API requires alternating roles, so we combine same-role messages

    let config = create_test_config();
    let user1 = create_message(MessageRole::User, "First question", false);
    let user2 = create_message(MessageRole::User, "Second question", false);

    let messages = vec![&user1, &user2];
    let (_, conversation) = transform_unified_messages(&messages, &config, false);

    assert_eq!(
        conversation.len(),
        1,
        "Consecutive same-role messages should combine"
    );
    assert_eq!(conversation[0].role, "user");
    // Content should be merged
    match &conversation[0].content {
        AnthropicContent::Text(text) => {
            assert!(text.contains("First question"));
            assert!(text.contains("Second question"));
        }
        _ => panic!("Expected Text content"),
    }
}

#[test]
fn test_alternating_roles_not_combined() {
    // Test verifies alternating role messages stay separate
    // This is the normal conversation pattern

    let config = create_test_config();
    let user = create_message(MessageRole::User, "Question", false);
    let assistant = create_message(MessageRole::Assistant, "Answer", false);

    let messages = vec![&user, &assistant];
    let (_, conversation) = transform_unified_messages(&messages, &config, false);

    assert_eq!(
        conversation.len(),
        2,
        "Alternating roles should stay separate"
    );
    assert_eq!(conversation[0].role, "user");
    assert_eq!(conversation[1].role, "assistant");
}

// ============================================================================
// Empty Text Handling Tests
// ============================================================================

#[test]
fn test_empty_text_no_cache_control() {
    // Test verifies empty text doesn't get cache control
    // CRITICAL: Anthropic API rejects cache_control on empty text

    let config = create_test_config();
    let empty_msg = create_message(MessageRole::User, "", true); // Empty but cacheable

    let messages = vec![&empty_msg];
    let (_, conversation) = transform_unified_messages(&messages, &config, true);

    match &conversation[0].content {
        AnthropicContent::Text(text) => {
            assert_eq!(text, "", "Empty text should remain empty");
        }
        AnthropicContent::Blocks(_) => {
            panic!("Empty text should not create content blocks");
        }
    }
}

// ============================================================================
// Tool Role Conversion Tests
// ============================================================================

#[test]
fn test_tool_role_converted_to_user() {
    // Test verifies Tool role becomes user in Anthropic
    // Anthropic doesn't support Tool role, tool results become user messages

    let config = create_test_config();
    let tool_result = create_tool_result_message("tool_123", "Result", false);

    let messages = vec![&tool_result];
    let (_, conversation) = transform_unified_messages(&messages, &config, false);

    assert_eq!(conversation.len(), 1);
    assert_eq!(
        conversation[0].role, "user",
        "Tool role should convert to user"
    );
}

#[test]
fn test_tool_call_conversion() {
    // Test verifies tool calls convert to Anthropic ToolUse format
    // Assistant messages with tool calls become tool_use blocks

    let config = create_test_config();
    let tool_call = create_tool_call_message("call_123", "search");

    let messages = vec![&tool_call];
    let (_, conversation) = transform_unified_messages(&messages, &config, false);

    match &conversation[0].content {
        AnthropicContent::Blocks(blocks) => {
            assert_eq!(blocks.len(), 1);
            match &blocks[0] {
                AnthropicContentBlock::ToolUse { id, name, .. } => {
                    assert_eq!(id, "call_123");
                    assert_eq!(name, "search");
                }
                _ => panic!("Expected ToolUse block"),
            }
        }
        _ => panic!("Expected Blocks content"),
    }
}

#[test]
fn test_tool_result_conversion() {
    // Test verifies tool results convert to Anthropic ToolResult format
    // Tool messages become user messages with tool_result blocks

    let config = create_test_config();
    let tool_result = create_tool_result_message("call_456", "Found 3 results", false);

    let messages = vec![&tool_result];
    let (_, conversation) = transform_unified_messages(&messages, &config, false);

    assert_eq!(conversation[0].role, "user");
    match &conversation[0].content {
        AnthropicContent::Blocks(blocks) => match &blocks[0] {
            AnthropicContentBlock::ToolResult {
                tool_use_id,
                content,
                ..
            } => {
                assert_eq!(tool_use_id, "call_456");
                assert_eq!(content, "Found 3 results");
            }
            _ => panic!("Expected ToolResult block"),
        },
        _ => panic!("Expected Blocks content"),
    }
}

#[test]
fn test_tool_result_with_error() {
    // Test verifies error tool results are prefixed with "Error:"
    // Anthropic needs clear error indication

    let config = create_test_config();
    let error_result = create_tool_result_message("call_789", "Connection failed", true);

    let messages = vec![&error_result];
    let (_, conversation) = transform_unified_messages(&messages, &config, false);

    match &conversation[0].content {
        AnthropicContent::Blocks(blocks) => match &blocks[0] {
            AnthropicContentBlock::ToolResult { content, .. } => {
                assert!(
                    content.starts_with("Error:"),
                    "Error results should be prefixed"
                );
                assert!(content.contains("Connection failed"));
            }
            _ => panic!("Expected ToolResult block"),
        },
        _ => panic!("Expected Blocks content"),
    }
}

// ============================================================================
// System Message Cache Control Tests
// ============================================================================

#[test]
fn test_system_message_cache_control() {
    // Test verifies last cacheable system message gets cache control
    // Follows Anthropic's cache hierarchy: tools → system → conversation

    let config = create_test_config();
    let sys1 = create_message(MessageRole::System, "First", true);
    let sys2 = create_message(MessageRole::System, "Second", true);
    let user = create_message(MessageRole::User, "Hello", false);

    let messages = vec![&sys1, &sys2, &user];
    let (system, _) = transform_unified_messages(&messages, &config, true);

    assert_eq!(system.len(), 2);
    // Last system message should have cache control
    assert!(
        system[0].cache_control.is_none(),
        "First system msg should not be cached"
    );
    assert!(
        system[1].cache_control.is_some(),
        "Last system msg should be cached"
    );
    assert_eq!(
        system[1].cache_control.as_ref().unwrap().cache_type,
        "ephemeral"
    );
}

#[test]
fn test_non_cacheable_system_messages_not_cached() {
    // Test verifies non-cacheable system messages don't get cache control
    // Even if it's the last system message

    let config = create_test_config();
    let sys = create_message(MessageRole::System, "System", false); // not cacheable

    let messages = vec![&sys];
    let (system, _) = transform_unified_messages(&messages, &config, true);

    assert_eq!(system.len(), 1);
    assert!(
        system[0].cache_control.is_none(),
        "Non-cacheable system msg should not be cached"
    );
}

// Helper trait for checking cache control
trait HasCacheControl {
    fn has_cache_control(&self) -> bool;
    fn get_cache_ttl(&self) -> Option<String>;
}

impl HasCacheControl for AnthropicContentBlock {
    fn has_cache_control(&self) -> bool {
        match self {
            AnthropicContentBlock::Text { cache_control, .. } => cache_control.is_some(),
            _ => false,
        }
    }

    fn get_cache_ttl(&self) -> Option<String> {
        match self {
            AnthropicContentBlock::Text { cache_control, .. } => {
                cache_control.as_ref().and_then(|cc| cc.ttl.clone())
            }
            _ => None,
        }
    }
}

// ============================================================================
// Additional Coverage Tests - merge_content_blocks and extract_text_content
// ============================================================================

#[test]
fn test_merge_content_blocks_text_plus_blocks() {
    // Test merging: existing Text + new Blocks → result should be Blocks
    // This tests the Text + Blocks branch in merge_content_blocks

    let config = create_test_config();
    let msg1 = create_message(MessageRole::Assistant, "First text", false);
    let msg2 = create_tool_call_with_role(MessageRole::Assistant);

    let messages = vec![&msg1, &msg2];
    let (_, conversation) = transform_unified_messages(&messages, &config, false);

    // Should combine into single message with blocks
    assert_eq!(conversation.len(), 1);
    match &conversation[0].content {
        AnthropicContent::Blocks(blocks) => {
            assert_eq!(blocks.len(), 2, "Should have text block + tool use block");
            // First should be text, second should be tool use
            assert!(matches!(&blocks[0], AnthropicContentBlock::Text { .. }));
            assert!(matches!(&blocks[1], AnthropicContentBlock::ToolUse { .. }));
        }
        _ => panic!("Expected Blocks content"),
    }
}

#[test]
fn test_merge_content_blocks_blocks_plus_text() {
    // Test merging: existing Blocks + new Text → result should be Blocks
    // This tests the Blocks + Text branch in merge_content_blocks

    let config = create_test_config();
    let msg1 = create_tool_call_with_role(MessageRole::Assistant);
    let msg2 = create_message(MessageRole::Assistant, "Following text", false);

    let messages = vec![&msg1, &msg2];
    let (_, conversation) = transform_unified_messages(&messages, &config, false);

    // Should combine into single message with blocks
    assert_eq!(conversation.len(), 1);
    match &conversation[0].content {
        AnthropicContent::Blocks(blocks) => {
            assert_eq!(blocks.len(), 2, "Should have tool use block + text block");
            assert!(matches!(&blocks[0], AnthropicContentBlock::ToolUse { .. }));
            assert!(matches!(&blocks[1], AnthropicContentBlock::Text { .. }));
        }
        _ => panic!("Expected Blocks content"),
    }
}

#[test]
fn test_merge_content_blocks_text_plus_text() {
    // Test merging: existing Text + new Text → result should be combined Text
    // This tests the Text + Text branch in merge_content_blocks

    let config = create_test_config();
    let msg1 = create_message(MessageRole::User, "First", false);
    let msg2 = create_message(MessageRole::User, "Second", false);

    let messages = vec![&msg1, &msg2];
    let (_, conversation) = transform_unified_messages(&messages, &config, false);

    // Should combine into single text message
    assert_eq!(conversation.len(), 1);
    match &conversation[0].content {
        AnthropicContent::Text(text) => {
            assert!(text.contains("First"), "Should contain first text");
            assert!(text.contains("Second"), "Should contain second text");
            assert!(text.contains('\n'), "Should have newline separator");
        }
        _ => panic!("Expected Text content"),
    }
}

#[test]
fn test_extract_text_content_from_tool_call() {
    // Test extract_text_content handles ToolCall content
    // This covers the ToolCall branch in extract_text_content

    let config = create_test_config();
    let msg = create_tool_call_with_role(MessageRole::System);

    let messages = vec![&msg];
    let (system, _) = transform_unified_messages(&messages, &config, false);

    // System message should extract text representation of tool call
    assert_eq!(system.len(), 1);
    assert!(
        system[0].text.contains("Tool call:"),
        "Should format as 'Tool call:'"
    );
    assert!(
        system[0].text.contains("test_function"),
        "Should include function name"
    );
}

#[test]
fn test_extract_text_content_from_tool_result_success() {
    // Test extract_text_content handles ToolResult (success) content
    // This covers the ToolResult branch (is_error = false)

    let config = create_test_config();
    let msg = UnifiedMessage {
        role: MessageRole::System,
        content: MessageContent::ToolResult {
            tool_call_id: "call_123".to_string(),
            content: "Success result".to_string(),
            is_error: false,
        },
        attributes: MessageAttributes {
            priority: 0,
            cacheable: false,
            cache_type: None,
            cache_key: None,
            category: MessageCategory::Current,
            metadata: HashMap::new(),
        },
        timestamp: Utc::now(),
    };

    let messages = vec![&msg];
    let (system, _) = transform_unified_messages(&messages, &config, false);

    // Should extract result content without error prefix
    assert_eq!(system.len(), 1);
    assert_eq!(
        system[0].text, "Success result",
        "Should not prefix with 'Error:'"
    );
}

#[test]
fn test_extract_text_content_from_tool_result_error() {
    // Test extract_text_content handles ToolResult (error) content
    // This covers the ToolResult branch (is_error = true)

    let config = create_test_config();
    let msg = UnifiedMessage {
        role: MessageRole::System,
        content: MessageContent::ToolResult {
            tool_call_id: "call_123".to_string(),
            content: "Something failed".to_string(),
            is_error: true,
        },
        attributes: MessageAttributes {
            priority: 0,
            cacheable: false,
            cache_type: None,
            cache_key: None,
            category: MessageCategory::Current,
            metadata: HashMap::new(),
        },
        timestamp: Utc::now(),
    };

    let messages = vec![&msg];
    let (system, _) = transform_unified_messages(&messages, &config, false);

    // Should prefix error results with "Error:"
    assert_eq!(system.len(), 1);
    assert_eq!(
        system[0].text, "Error: Something failed",
        "Should prefix with 'Error:'"
    );
}

#[test]
fn test_cache_decision_logging_non_cacheable() {
    // Test determine_cache_decision when message is not cacheable
    // This covers the else branch (logging for non-cacheable messages)

    let config = create_test_config();
    let msg = create_message(MessageRole::User, "Test", false); // cacheable = false

    let messages = vec![&msg];
    let (_, conversation) = transform_unified_messages(&messages, &config, true); // caching enabled

    // Should not cache non-cacheable message
    assert_eq!(conversation.len(), 1);
    match &conversation[0].content {
        AnthropicContent::Text(_) => {} // Should be simple text, no cache control
        AnthropicContent::Blocks(blocks) => {
            // If blocks, none should have cache control
            for block in blocks {
                assert!(
                    !block.has_cache_control(),
                    "Non-cacheable message should not be cached"
                );
            }
        }
    }
}

#[test]
fn test_json_content_with_caching() {
    // Test convert_json_content with cache control
    // This covers the JSON content branch with should_cache = true

    let config = create_test_config();
    let json_msg = UnifiedMessage {
        role: MessageRole::User,
        content: MessageContent::Json(serde_json::json!({"key": "value"})),
        attributes: MessageAttributes {
            priority: 0,
            cacheable: true,
            cache_type: None,
            cache_key: None,
            category: MessageCategory::Current,
            metadata: HashMap::new(),
        },
        timestamp: Utc::now(),
    };

    let messages = vec![&json_msg];
    let (_, conversation) = transform_unified_messages(&messages, &config, true);

    assert_eq!(conversation.len(), 1);
    // Should have cache control since it's cacheable and caching is enabled
    match &conversation[0].content {
        AnthropicContent::Blocks(blocks) => {
            assert!(
                blocks[0].has_cache_control(),
                "JSON content should have cache control when cacheable"
            );
        }
        _ => panic!("Expected Blocks with cache control for cached JSON"),
    }
}

// ============================================================================
// Extended Cache Type Tests (Issue #3)
// ============================================================================

#[test]
fn test_extended_cache_type_conversation_message() {
    // Test verifies Extended cache type produces correct TTL (1h)
    // Extended cache is used for longer-lived context (e.g., system prompts)

    let config = create_test_config();
    let msg = create_message_with_cache_type(MessageRole::User, "Context", CacheType::Extended);

    let messages = vec![&msg];
    let (_, conversation) = transform_unified_messages(&messages, &config, true);

    assert_eq!(conversation.len(), 1);
    match &conversation[0].content {
        AnthropicContent::Blocks(blocks) => {
            assert!(
                blocks[0].has_cache_control(),
                "Extended cache message should have cache control"
            );
            assert_eq!(
                blocks[0].get_cache_ttl(),
                Some("1h".to_string()),
                "Extended cache should use 1h TTL"
            );
        }
        _ => panic!("Expected Blocks content with cache control"),
    }
}

#[test]
fn test_ephemeral_cache_type_conversation_message() {
    // Test verifies Ephemeral cache type produces correct TTL (5m)
    // Ephemeral cache is used for short-lived context

    let config = create_test_config();
    let msg = create_message_with_cache_type(MessageRole::User, "Query", CacheType::Ephemeral);

    let messages = vec![&msg];
    let (_, conversation) = transform_unified_messages(&messages, &config, true);

    assert_eq!(conversation.len(), 1);
    match &conversation[0].content {
        AnthropicContent::Blocks(blocks) => {
            assert!(
                blocks[0].has_cache_control(),
                "Ephemeral cache message should have cache control"
            );
            assert_eq!(
                blocks[0].get_cache_ttl(),
                Some("5m".to_string()),
                "Ephemeral cache should use 5m TTL"
            );
        }
        _ => panic!("Expected Blocks content with cache control"),
    }
}

#[test]
fn test_extended_cache_type_system_message() {
    // Test verifies Extended cache works for system messages
    // System messages can specify their own cache type

    let config = create_test_config();
    let sys =
        create_message_with_cache_type(MessageRole::System, "Long context", CacheType::Extended);

    let messages = vec![&sys];
    let (system, _) = transform_unified_messages(&messages, &config, true);

    assert_eq!(system.len(), 1);
    assert!(
        system[0].cache_control.is_some(),
        "Extended cache system message should have cache control"
    );
    let cache_control = system[0].cache_control.as_ref().unwrap();
    assert_eq!(cache_control.cache_type, "ephemeral");
    assert_eq!(
        cache_control.ttl,
        Some("1h".to_string()),
        "Extended cache system message should use 1h TTL"
    );
}

#[test]
fn test_ephemeral_cache_type_system_message() {
    // Test verifies Ephemeral cache works for system messages
    // System messages can specify their own cache type

    let config = create_test_config();
    let sys =
        create_message_with_cache_type(MessageRole::System, "Short context", CacheType::Ephemeral);

    let messages = vec![&sys];
    let (system, _) = transform_unified_messages(&messages, &config, true);

    assert_eq!(system.len(), 1);
    assert!(
        system[0].cache_control.is_some(),
        "Ephemeral cache system message should have cache control"
    );
    let cache_control = system[0].cache_control.as_ref().unwrap();
    assert_eq!(cache_control.cache_type, "ephemeral");
    assert_eq!(
        cache_control.ttl,
        Some("5m".to_string()),
        "Ephemeral cache system message should use 5m TTL"
    );
}

#[test]
fn test_mixed_cache_types_in_conversation() {
    // Test verifies different cache types can coexist
    // Some messages use Extended (1h), others use Ephemeral (5m)
    // Use 3 messages so both index 0 and 2 are cache breakpoints

    let config = create_test_config();
    let msg1 = create_message_with_cache_type(MessageRole::User, "Extended", CacheType::Extended);
    let msg2 =
        create_message_with_cache_type(MessageRole::Assistant, "Middle", CacheType::Ephemeral);
    let msg3 = create_message_with_cache_type(MessageRole::User, "Ephemeral", CacheType::Ephemeral);

    let messages = vec![&msg1, &msg2, &msg3];
    let (_, conversation) = transform_unified_messages(&messages, &config, true);

    assert_eq!(conversation.len(), 3);

    // Message at index 0 should have Extended cache (1h) - always a breakpoint
    match &conversation[0].content {
        AnthropicContent::Blocks(blocks) => {
            assert_eq!(
                blocks[0].get_cache_ttl(),
                Some("1h".to_string()),
                "First message should use Extended cache"
            );
        }
        _ => panic!("Expected Blocks content"),
    }

    // Message at index 1 should NOT be cached (not a breakpoint in 3-5 message range)
    match &conversation[1].content {
        AnthropicContent::Text(_) => {
            // Expected: no cache control for non-breakpoint message
        }
        AnthropicContent::Blocks(blocks) => {
            assert!(
                !blocks[0].has_cache_control(),
                "Middle message should not be cached (not a breakpoint)"
            );
        }
    }

    // Message at index 2 should have Ephemeral cache (5m) - breakpoint in 3-5 message range
    match &conversation[2].content {
        AnthropicContent::Blocks(blocks) => {
            assert_eq!(
                blocks[0].get_cache_ttl(),
                Some("5m".to_string()),
                "Third message should use Ephemeral cache"
            );
        }
        _ => panic!("Expected Blocks content"),
    }
}

#[test]
fn test_cache_type_overrides_config_ttl() {
    // Test verifies message-level cache_type overrides config.cache_ttl
    // IMPORTANT: Message-level control takes precedence

    let mut config = create_test_config();
    config.cache_ttl = "1h".to_string(); // Config says 1h

    // But message specifies Ephemeral (5m)
    let msg = create_message_with_cache_type(MessageRole::User, "Query", CacheType::Ephemeral);

    let messages = vec![&msg];
    let (_, conversation) = transform_unified_messages(&messages, &config, true);

    match &conversation[0].content {
        AnthropicContent::Blocks(blocks) => {
            assert_eq!(
                blocks[0].get_cache_ttl(),
                Some("5m".to_string()),
                "Message cache_type should override config.cache_ttl"
            );
        }
        _ => panic!("Expected Blocks content"),
    }
}

#[test]
fn test_no_cache_type_uses_config_ttl() {
    // Test verifies messages without cache_type fall back to config.cache_ttl
    // Backward compatibility: existing code continues to work

    let mut config = create_test_config();
    config.cache_ttl = "30m".to_string();

    let msg = create_message(MessageRole::User, "Query", true); // cacheable but no cache_type

    let messages = vec![&msg];
    let (_, conversation) = transform_unified_messages(&messages, &config, true);

    match &conversation[0].content {
        AnthropicContent::Blocks(blocks) => {
            assert_eq!(
                blocks[0].get_cache_ttl(),
                Some("30m".to_string()),
                "Should use config.cache_ttl when message has no cache_type"
            );
        }
        _ => panic!("Expected Blocks content"),
    }
}

#[test]
fn test_extended_cache_with_json_content() {
    // Test verifies Extended cache works with JSON content
    // JSON content should respect cache_type

    let config = create_test_config();
    let json_msg = UnifiedMessage {
        role: MessageRole::User,
        content: MessageContent::Json(serde_json::json!({"data": "value"})),
        attributes: MessageAttributes {
            priority: 0,
            cacheable: true,
            cache_type: Some(CacheType::Extended),
            cache_key: None,
            category: MessageCategory::Current,
            metadata: HashMap::new(),
        },
        timestamp: Utc::now(),
    };

    let messages = vec![&json_msg];
    let (_, conversation) = transform_unified_messages(&messages, &config, true);

    match &conversation[0].content {
        AnthropicContent::Blocks(blocks) => {
            assert_eq!(
                blocks[0].get_cache_ttl(),
                Some("1h".to_string()),
                "JSON content should respect Extended cache type"
            );
        }
        _ => panic!("Expected Blocks content"),
    }
}

fn create_tool_call_with_role(role: MessageRole) -> UnifiedMessage {
    UnifiedMessage {
        role,
        content: MessageContent::ToolCall {
            id: "call_123".to_string(),
            name: "test_function".to_string(),
            arguments: serde_json::json!({"param": "value"}),
        },
        attributes: MessageAttributes {
            priority: 0,
            cacheable: false,
            cache_type: None,
            cache_key: None,
            category: MessageCategory::Current,
            metadata: HashMap::new(),
        },
        timestamp: Utc::now(),
    }
}
