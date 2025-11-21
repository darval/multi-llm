//! Message conversion between unified format and Anthropic API format

use super::types::{
    AnthropicContent, AnthropicContentBlock, AnthropicMessage, CacheControl, SystemMessage,
};
use crate::config::AnthropicConfig;
use crate::log_debug;
use crate::core_types::messages::{MessageContent, MessageRole, UnifiedMessage};

/// Convert unified messages to Anthropic format, applying caching properly per Anthropic hierarchy
pub(super) fn transform_unified_messages(
    messages: &[&UnifiedMessage],
    config: &AnthropicConfig,
    enable_caching: bool,
) -> (Vec<SystemMessage>, Vec<AnthropicMessage>) {
    let (mut system_messages, conversation_msg_sources) =
        separate_system_and_conversation_messages(messages);
    let conversation_messages = convert_and_combine_conversation_messages(
        &conversation_msg_sources,
        config,
        enable_caching,
    );
    apply_system_cache_control(&mut system_messages, messages, config, enable_caching);

    (system_messages, conversation_messages)
}

fn separate_system_and_conversation_messages<'a>(
    messages: &'a [&'a UnifiedMessage],
) -> (Vec<SystemMessage>, Vec<&'a UnifiedMessage>) {
    let mut system_messages = Vec::new();
    let mut conversation_msg_sources = Vec::new();

    for msg in messages {
        match msg.role {
            MessageRole::System => {
                system_messages.push(SystemMessage {
                    message_type: "text".to_string(),
                    text: extract_text_content(&msg.content),
                    cache_control: None,
                });
            }
            MessageRole::User | MessageRole::Assistant | MessageRole::Tool => {
                conversation_msg_sources.push(*msg);
            }
        }
    }

    (system_messages, conversation_msg_sources)
}

fn convert_and_combine_conversation_messages(
    conversation_msg_sources: &[&UnifiedMessage],
    config: &AnthropicConfig,
    enable_caching: bool,
) -> Vec<AnthropicMessage> {
    let mut combined_messages: Vec<AnthropicMessage> = Vec::new();

    for (index, msg) in conversation_msg_sources.iter().enumerate() {
        let should_cache =
            determine_cache_decision(msg, index, conversation_msg_sources.len(), enable_caching);
        let anthropic_msg =
            unified_message_to_anthropic_with_cache_control(msg, config, should_cache);

        combine_or_add_message(&mut combined_messages, anthropic_msg);
    }

    combined_messages
}

fn determine_cache_decision(
    msg: &UnifiedMessage,
    index: usize,
    total_messages: usize,
    enable_caching: bool,
) -> bool {
    if msg.attributes.cacheable && enable_caching {
        let should_cache =
            should_place_cache_breakpoint_at_conversation_index(index, total_messages);
        log_debug!(
            provider = "anthropic",
            msg_index = index,
            total_messages = total_messages,
            should_cache = should_cache,
            role = ?msg.role,
            "Cache breakpoint decision for conversation message"
        );
        should_cache
    } else {
        log_debug!(
            provider = "anthropic",
            msg_index = index,
            should_cache = false,
            role = ?msg.role,
            "Message not cacheable or caching disabled"
        );
        false
    }
}

fn combine_or_add_message(
    combined_messages: &mut Vec<AnthropicMessage>,
    anthropic_msg: AnthropicMessage,
) {
    if let Some(last_msg) = combined_messages.last_mut() {
        if last_msg.role == anthropic_msg.role {
            log_debug!(
                provider = "anthropic",
                role = %anthropic_msg.role,
                "Combining consecutive messages with same role"
            );
            merge_content_blocks(&mut last_msg.content, anthropic_msg.content);
        } else {
            combined_messages.push(anthropic_msg);
        }
    } else {
        combined_messages.push(anthropic_msg);
    }
}

fn merge_content_blocks(existing: &mut AnthropicContent, new: AnthropicContent) {
    match (&mut *existing, new) {
        (AnthropicContent::Blocks(existing_blocks), AnthropicContent::Blocks(new)) => {
            existing_blocks.extend(new);
        }
        (AnthropicContent::Blocks(existing_blocks), AnthropicContent::Text(text)) => {
            existing_blocks.push(AnthropicContentBlock::Text {
                text,
                cache_control: None,
            });
        }
        (AnthropicContent::Text(existing_text), AnthropicContent::Text(new_text)) => {
            existing_text.push('\n');
            existing_text.push_str(&new_text);
        }
        (AnthropicContent::Text(existing_text), AnthropicContent::Blocks(blocks)) => {
            let mut all_blocks = vec![AnthropicContentBlock::Text {
                text: existing_text.clone(),
                cache_control: None,
            }];
            all_blocks.extend(blocks);
            *existing = AnthropicContent::Blocks(all_blocks);
        }
    }
}

fn apply_system_cache_control(
    system_messages: &mut [SystemMessage],
    messages: &[&UnifiedMessage],
    config: &AnthropicConfig,
    enable_caching: bool,
) {
    if !enable_caching || system_messages.is_empty() {
        return;
    }

    let system_msgs: Vec<_> = messages
        .iter()
        .filter(|m| m.role == MessageRole::System)
        .collect();

    if let Some(last_cacheable_index) = system_msgs.iter().rposition(|msg| msg.attributes.cacheable)
    {
        if let Some(system_msg) = system_messages.get_mut(last_cacheable_index) {
            system_msg.cache_control = Some(CacheControl {
                cache_type: "ephemeral".to_string(),
                ttl: Some(config.cache_ttl.clone()),
            });
        }
    }
}

/// Convert a UnifiedMessage to AnthropicMessage with proper cache control
fn unified_message_to_anthropic_with_cache_control(
    msg: &UnifiedMessage,
    config: &AnthropicConfig,
    should_cache: bool,
) -> AnthropicMessage {
    let role = convert_message_role(&msg.role);
    let content = convert_message_content(&msg.content, &role, config, should_cache);
    AnthropicMessage { role, content }
}

/// Convert UnifiedMessage role to Anthropic role
fn convert_message_role(role: &MessageRole) -> String {
    match role {
        MessageRole::User => "user".to_string(),
        MessageRole::Assistant => "assistant".to_string(),
        MessageRole::Tool => "user".to_string(), // Tool results become user messages in Anthropic
        MessageRole::System => "user".to_string(), // Should not happen here
    }
}

/// Convert MessageContent to AnthropicContent with cache control
fn convert_message_content(
    content: &MessageContent,
    role: &str,
    config: &AnthropicConfig,
    should_cache: bool,
) -> AnthropicContent {
    match content {
        MessageContent::Text(text) => convert_text_content(text, role, config, should_cache),
        MessageContent::Json(value) => convert_json_content(value, config, should_cache),
        MessageContent::ToolCall {
            id,
            name,
            arguments,
        } => convert_tool_call(id, name, arguments),
        MessageContent::ToolResult {
            tool_call_id,
            content,
            is_error,
        } => convert_tool_result(tool_call_id, content, *is_error),
    }
}

/// Convert text content with optional caching
fn convert_text_content(
    text: &str,
    role: &str,
    config: &AnthropicConfig,
    should_cache: bool,
) -> AnthropicContent {
    // NEVER apply cache_control to empty text blocks (Anthropic API rejects this)
    if should_cache && !text.is_empty() {
        log_debug!(
            provider = "anthropic",
            role = %role,
            text_preview = %text.chars().take(100).collect::<String>(),
            "Applying cache_control to conversation message"
        );
        AnthropicContent::Blocks(vec![AnthropicContentBlock::Text {
            text: text.to_string(),
            cache_control: Some(CacheControl {
                cache_type: "ephemeral".to_string(),
                ttl: Some(config.cache_ttl.clone()),
            }),
        }])
    } else {
        log_debug!(
            provider = "anthropic",
            role = %role,
            text_preview = %text.chars().take(100).collect::<String>(),
            "NOT applying cache_control to conversation message"
        );
        AnthropicContent::Text(text.to_string())
    }
}

/// Convert JSON content with optional caching
fn convert_json_content(
    value: &serde_json::Value,
    config: &AnthropicConfig,
    should_cache: bool,
) -> AnthropicContent {
    let json_text = serde_json::to_string_pretty(value).unwrap_or_default();
    // NEVER apply cache_control to empty text blocks (Anthropic API rejects this)
    if should_cache && !json_text.is_empty() {
        AnthropicContent::Blocks(vec![AnthropicContentBlock::Text {
            text: json_text,
            cache_control: Some(CacheControl {
                cache_type: "ephemeral".to_string(),
                ttl: Some(config.cache_ttl.clone()),
            }),
        }])
    } else {
        AnthropicContent::Text(json_text)
    }
}

/// Convert tool call to Anthropic format
fn convert_tool_call(id: &str, name: &str, arguments: &serde_json::Value) -> AnthropicContent {
    log_debug!(
        provider = "anthropic",
        tool_id = %id,
        tool_name = %name,
        "Converting ToolCall to Anthropic format for tool result pairing"
    );

    AnthropicContent::Blocks(vec![AnthropicContentBlock::ToolUse {
        id: id.to_string(),
        name: name.to_string(),
        input: arguments.clone(),
    }])
}

/// Convert tool result to Anthropic format
fn convert_tool_result(tool_call_id: &str, content: &str, is_error: bool) -> AnthropicContent {
    log_debug!(
        provider = "anthropic",
        tool_call_id = %tool_call_id,
        is_error = is_error,
        content_len = content.len(),
        content_preview = %if content.len() > 200 {
            format!("{}...", &content[..200])
        } else {
            content.to_string()
        },
        "DEBUG: Converting ToolResult to Anthropic format"
    );

    AnthropicContent::Blocks(vec![AnthropicContentBlock::ToolResult {
        tool_use_id: tool_call_id.to_string(),
        content: if is_error {
            format!("Error: {}", content)
        } else {
            content.to_string()
        },
    }])
}

/// Extract text content from any MessageContent type
fn extract_text_content(content: &MessageContent) -> String {
    match content {
        MessageContent::Text(text) => text.clone(),
        MessageContent::Json(value) => serde_json::to_string_pretty(value).unwrap_or_default(),
        MessageContent::ToolCall {
            name, arguments, ..
        } => {
            format!(
                "Tool call: {} with args: {}",
                name,
                serde_json::to_string(arguments).unwrap_or_default()
            )
        }
        MessageContent::ToolResult {
            content, is_error, ..
        } => {
            if *is_error {
                format!("Error: {}", content)
            } else {
                content.clone()
            }
        }
    }
}

/// Implements 2-breakpoint progressive sliding window strategy for optimal cache utilization
/// Based on testing: achieves significant cache improvement while respecting Anthropic's limits
/// Uses Anthropic's 4 cache block limit: 1 for tools + 1 for system + 2 for conversation sliding window
fn should_place_cache_breakpoint_at_conversation_index(
    index: usize,
    total_messages: usize,
) -> bool {
    // Strategy: Use 2 cache breakpoints that progressively move forward through conversation
    // This ensures cache grows by including more recent conversation history

    match total_messages {
        // For short conversations, cache strategically to build up cache
        1..=2 => {
            // Cache first message to start building cache
            index == 0
        }
        3..=5 => {
            // Cache messages 0, 2 to build cache with good spacing
            index == 0 || index == 2
        }
        _ => {
            // Simple incremental caching strategy: always cache the last 2 messages
            // This mimics the working test pattern where cache grows incrementally
            // Last message becomes "previous last" next time, ensuring cache growth

            if total_messages >= 2 {
                // Cache last 2 messages
                index == total_messages - 1 || index == total_messages - 2
            } else {
                // For single message, cache it
                index == total_messages - 1
            }
        }
    }
}
