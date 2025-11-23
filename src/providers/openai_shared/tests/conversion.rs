//! Unit Tests for OpenAI-Shared Conversion Functions
//!
//! UNIT UNDER TEST: Conversion utility functions in openai_shared/utils.rs
//!
//! BUSINESS RESPONSIBILITY:
//!   - Convert neutral message formats to OpenAI-compatible messages
//!   - Convert neutral tool definitions to OpenAI tool format
//!   - Apply executor LLM configuration to OpenAI requests
//!   - Convert OpenAI tool calls to executor tool call format
//!   - Handle tool calls with custom format detection and content cleaning
//!   - Estimate tokens for logging and diagnostics
//!
//! TEST COVERAGE:
//!   - Message conversion: text, JSON, tool calls, tool results
//!   - Tool definition conversion
//!   - Config application: temperature, max_tokens, top_p, tools, tool_choice, response_format
//!   - Tool call conversion: standard and custom formats
//!   - Token estimation: simple text and message arrays
//!   - Edge cases: empty arrays, None values, invalid JSON

use super::super::types::*;
use super::super::utils::*;
use crate::core_types::messages::MessageAttributes;
use crate::core_types::provider::{RequestConfig, ResponseFormat, Tool, ToolChoice};
use crate::{MessageContent, MessageRole, UnifiedMessage};
use chrono::Utc;
use serde_json::json;

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Create a simple text message with default attributes
fn create_message(role: MessageRole, content: &str) -> UnifiedMessage {
    UnifiedMessage {
        role,
        content: MessageContent::Text(content.to_string()),
        attributes: MessageAttributes::default(),
        timestamp: Utc::now(),
    }
}

/// Create a message with JSON content
fn create_json_message(role: MessageRole, json: serde_json::Value) -> UnifiedMessage {
    UnifiedMessage {
        role,
        content: MessageContent::Json(json),
        attributes: MessageAttributes::default(),
        timestamp: Utc::now(),
    }
}

/// Create a message with tool call content
fn create_tool_call_message(id: &str, name: &str, arguments: serde_json::Value) -> UnifiedMessage {
    UnifiedMessage {
        role: MessageRole::Assistant,
        content: MessageContent::ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
        },
        attributes: MessageAttributes::default(),
        timestamp: Utc::now(),
    }
}

/// Create a message with tool result content
fn create_tool_result_message(tool_call_id: &str, content: &str, is_error: bool) -> UnifiedMessage {
    UnifiedMessage {
        role: MessageRole::Tool,
        content: MessageContent::ToolResult {
            tool_call_id: tool_call_id.to_string(),
            content: content.to_string(),
            is_error,
        },
        attributes: MessageAttributes::default(),
        timestamp: Utc::now(),
    }
}

/// Create test RequestConfig with minimal fields
fn create_test_executor_config() -> RequestConfig {
    RequestConfig {
        temperature: None,
        max_tokens: None,
        top_p: None,
        top_k: None,
        min_p: None,
        presence_penalty: None,
        response_format: None,
        tools: vec![],
        tool_choice: None,
        user_id: None,
        session_id: None,
        llm_path: Some("user_llm".to_string()),
    }
}

// ============================================================================
// Message Conversion Tests
// ============================================================================

#[test]
fn test_convert_text_messages() {
    // Test verifies basic text message conversion across all role types
    // Ensures role mapping and text content preservation

    let messages = vec![
        create_message(MessageRole::System, "System prompt"),
        create_message(MessageRole::User, "User question"),
        create_message(MessageRole::Assistant, "Assistant response"),
    ];

    let converted = convert_neutral_messages_to_openai(&messages);

    assert_eq!(converted.len(), 3);
    assert_eq!(converted[0].role, "system");
    assert_eq!(converted[0].content, "System prompt");
    assert_eq!(converted[1].role, "user");
    assert_eq!(converted[1].content, "User question");
    assert_eq!(converted[2].role, "assistant");
    assert_eq!(converted[2].content, "Assistant response");
}

#[test]
fn test_convert_json_message_content() {
    // Test verifies JSON content is serialized to pretty string format
    // Important for structured responses and debugging

    let json_value = json!({
        "field1": "value1",
        "field2": 42,
        "nested": {"key": "value"}
    });

    let messages = vec![create_json_message(
        MessageRole::Assistant,
        json_value.clone(),
    )];

    let converted = convert_neutral_messages_to_openai(&messages);

    assert_eq!(converted.len(), 1);
    assert_eq!(converted[0].role, "assistant");
    assert!(converted[0].content.contains("field1"));
    assert!(converted[0].content.contains("value1"));
    assert!(converted[0].content.contains("field2"));
    assert!(converted[0].content.contains("42"));
}

#[test]
fn test_convert_tool_call_message_content() {
    // Test verifies tool call messages convert to text format
    // Note: Tool calls FROM the LLM shouldn't be in messages TO the LLM,
    // but we convert for compatibility

    let messages = vec![create_tool_call_message(
        "call_123",
        "get_weather",
        json!({"location": "Seattle"}),
    )];

    let converted = convert_neutral_messages_to_openai(&messages);

    assert_eq!(converted.len(), 1);
    assert_eq!(converted[0].role, "assistant");
    assert!(converted[0].content.contains("Tool call:"));
    assert!(converted[0].content.contains("get_weather"));
    assert!(converted[0].content.contains("Seattle"));
}

#[test]
fn test_convert_tool_result_success() {
    // Test verifies successful tool results format correctly

    let messages = vec![create_tool_result_message(
        "call_123",
        "Weather is sunny, 72°F",
        false,
    )];

    let converted = convert_neutral_messages_to_openai(&messages);

    assert_eq!(converted.len(), 1);
    assert_eq!(converted[0].role, "tool");
    assert!(converted[0].content.starts_with("Tool result:"));
    assert!(converted[0].content.contains("Weather is sunny"));
}

#[test]
fn test_convert_tool_result_error() {
    // Test verifies error tool results are prefixed correctly

    let messages = vec![create_tool_result_message("call_123", "API timeout", true)];

    let converted = convert_neutral_messages_to_openai(&messages);

    assert_eq!(converted.len(), 1);
    assert_eq!(converted[0].role, "tool");
    assert!(converted[0].content.starts_with("Tool error:"));
    assert!(converted[0].content.contains("API timeout"));
}

#[test]
fn test_convert_empty_message_array() {
    // Test verifies empty message array handling

    let messages: Vec<UnifiedMessage> = vec![];
    let converted = convert_neutral_messages_to_openai(&messages);

    assert_eq!(converted.len(), 0);
}

#[test]
fn test_convert_mixed_message_types() {
    // Test verifies mixed message types in conversation

    let messages = vec![
        create_message(MessageRole::System, "System"),
        create_message(MessageRole::User, "User"),
        create_json_message(MessageRole::Assistant, json!({"response": "data"})),
        create_tool_result_message("call_1", "result", false),
    ];

    let converted = convert_neutral_messages_to_openai(&messages);

    assert_eq!(converted.len(), 4);
    assert_eq!(converted[0].role, "system");
    assert_eq!(converted[1].role, "user");
    assert_eq!(converted[2].role, "assistant");
    assert_eq!(converted[3].role, "tool");
}

// ============================================================================
// Tool Conversion Tests
// ============================================================================

#[test]
fn test_convert_single_tool() {
    // Test verifies single tool conversion to OpenAI format

    let tools = vec![Tool {
        name: "get_weather".to_string(),
        description: "Get current weather".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"}
            }
        }),
    }];

    let converted = convert_neutral_tools_to_openai(&tools);

    assert_eq!(converted.len(), 1);
    assert_eq!(converted[0]["type"], "function");
    assert_eq!(converted[0]["function"]["name"], "get_weather");
    assert_eq!(
        converted[0]["function"]["description"],
        "Get current weather"
    );
    assert_eq!(
        converted[0]["function"]["parameters"]["properties"]["location"]["type"],
        "string"
    );
}

#[test]
fn test_convert_multiple_tools() {
    // Test verifies multiple tool conversion

    let tools = vec![
        Tool {
            name: "tool1".to_string(),
            description: "First tool".to_string(),
            parameters: json!({"type": "object"}),
        },
        Tool {
            name: "tool2".to_string(),
            description: "Second tool".to_string(),
            parameters: json!({"type": "object"}),
        },
    ];

    let converted = convert_neutral_tools_to_openai(&tools);

    assert_eq!(converted.len(), 2);
    assert_eq!(converted[0]["function"]["name"], "tool1");
    assert_eq!(converted[1]["function"]["name"], "tool2");
}

#[test]
fn test_convert_empty_tool_array() {
    // Test verifies empty tool array handling

    let tools: Vec<Tool> = vec![];
    let converted = convert_neutral_tools_to_openai(&tools);

    assert_eq!(converted.len(), 0);
}

#[test]
fn test_convert_tool_with_complex_parameters() {
    // Test verifies complex parameter schemas are preserved

    let tools = vec![Tool {
        name: "complex_tool".to_string(),
        description: "Tool with complex params".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "simple": {"type": "string"},
                "array": {
                    "type": "array",
                    "items": {"type": "number"}
                },
                "nested": {
                    "type": "object",
                    "properties": {
                        "deep": {"type": "boolean"}
                    }
                }
            },
            "required": ["simple"]
        }),
    }];

    let converted = convert_neutral_tools_to_openai(&tools);

    let params = &converted[0]["function"]["parameters"];
    assert_eq!(params["properties"]["simple"]["type"], "string");
    assert_eq!(params["properties"]["array"]["type"], "array");
    assert_eq!(params["properties"]["nested"]["type"], "object");
    assert_eq!(params["required"][0], "simple");
}

// ============================================================================
// Config Application Tests
// ============================================================================

#[test]
fn test_apply_config_with_none() {
    // Test verifies no-op when config is None

    let mut request = OpenAIRequest {
        model: "gpt-4".to_string(),
        messages: vec![],
        temperature: Some(1.0),
        max_tokens: None,
        top_p: None,
        presence_penalty: None,
        stream: None,
        tools: None,
        tool_choice: None,
        response_format: None,
    };

    let original_temp = request.temperature;
    apply_config_to_request(&mut request, None);

    assert_eq!(request.temperature, original_temp);
    assert!(request.max_tokens.is_none());
}

#[test]
fn test_apply_llm_parameters() {
    // Test verifies temperature, max_tokens, top_p, presence_penalty application

    let mut request = OpenAIRequest {
        model: "gpt-4".to_string(),
        messages: vec![],
        temperature: None,
        max_tokens: None,
        top_p: None,
        presence_penalty: None,
        stream: None,
        tools: None,
        tool_choice: None,
        response_format: None,
    };

    let mut config = create_test_executor_config();
    config.temperature = Some(0.7);
    config.max_tokens = Some(1000);
    config.top_p = Some(0.9);
    config.presence_penalty = Some(0.5);

    apply_config_to_request(&mut request, Some(config));

    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.max_tokens, Some(1000));
    assert_eq!(request.top_p, Some(0.9));
    assert_eq!(request.presence_penalty, Some(0.5));
}

#[test]
fn test_apply_tools_for_user_llm() {
    // Test verifies tools are applied when llm_path is "user_llm"

    let mut request = OpenAIRequest {
        model: "gpt-4".to_string(),
        messages: vec![],
        temperature: None,
        max_tokens: None,
        top_p: None,
        presence_penalty: None,
        stream: None,
        tools: None,
        tool_choice: None,
        response_format: None,
    };

    let mut config = create_test_executor_config();
    config.tools = vec![Tool {
        name: "test_tool".to_string(),
        description: "Test".to_string(),
        parameters: json!({}),
    }];

    apply_config_to_request(&mut request, Some(config));

    assert!(request.tools.is_some());
    assert_eq!(request.tools.as_ref().unwrap().len(), 1);
}

#[test]
fn test_skip_tools_for_non_user_llm() {
    // Test verifies tools are NOT applied when llm_path is not "user_llm"
    // This prevents sending tools to internal/utility LLMs

    let mut request = OpenAIRequest {
        model: "gpt-4".to_string(),
        messages: vec![],
        temperature: None,
        max_tokens: None,
        top_p: None,
        presence_penalty: None,
        stream: None,
        tools: None,
        tool_choice: None,
        response_format: None,
    };

    let mut config = create_test_executor_config();
    config.llm_path = Some("internal_llm".to_string());
    config.tools = vec![Tool {
        name: "test_tool".to_string(),
        description: "Test".to_string(),
        parameters: json!({}),
    }];

    apply_config_to_request(&mut request, Some(config));

    assert!(request.tools.is_none());
}

#[test]
fn test_apply_tool_choice_auto() {
    // Test verifies ToolChoice::Auto converts to "auto"

    let mut request = OpenAIRequest {
        model: "gpt-4".to_string(),
        messages: vec![],
        temperature: None,
        max_tokens: None,
        top_p: None,
        presence_penalty: None,
        stream: None,
        tools: None,
        tool_choice: None,
        response_format: None,
    };

    let mut config = create_test_executor_config();
    config.tool_choice = Some(ToolChoice::Auto);

    apply_config_to_request(&mut request, Some(config));

    assert_eq!(request.tool_choice, Some("auto".to_string()));
}

#[test]
fn test_apply_tool_choice_none() {
    // Test verifies ToolChoice::None converts to "none"

    let mut request = OpenAIRequest {
        model: "gpt-4".to_string(),
        messages: vec![],
        temperature: None,
        max_tokens: None,
        top_p: None,
        presence_penalty: None,
        stream: None,
        tools: None,
        tool_choice: None,
        response_format: None,
    };

    let mut config = create_test_executor_config();
    config.tool_choice = Some(ToolChoice::None);

    apply_config_to_request(&mut request, Some(config));

    assert_eq!(request.tool_choice, Some("none".to_string()));
}

#[test]
fn test_apply_tool_choice_required() {
    // Test verifies ToolChoice::Required converts to "required"

    let mut request = OpenAIRequest {
        model: "gpt-4".to_string(),
        messages: vec![],
        temperature: None,
        max_tokens: None,
        top_p: None,
        presence_penalty: None,
        stream: None,
        tools: None,
        tool_choice: None,
        response_format: None,
    };

    let mut config = create_test_executor_config();
    config.tool_choice = Some(ToolChoice::Required);

    apply_config_to_request(&mut request, Some(config));

    assert_eq!(request.tool_choice, Some("required".to_string()));
}

#[test]
fn test_apply_tool_choice_specific() {
    // Test verifies ToolChoice::Specific passes through tool name

    let mut request = OpenAIRequest {
        model: "gpt-4".to_string(),
        messages: vec![],
        temperature: None,
        max_tokens: None,
        top_p: None,
        presence_penalty: None,
        stream: None,
        tools: None,
        tool_choice: None,
        response_format: None,
    };

    let mut config = create_test_executor_config();
    config.tool_choice = Some(ToolChoice::Specific("get_weather".to_string()));

    apply_config_to_request(&mut request, Some(config));

    assert_eq!(request.tool_choice, Some("get_weather".to_string()));
}

#[test]
fn test_apply_response_format() {
    // Test verifies response format with JSON schema

    let mut request = OpenAIRequest {
        model: "gpt-4".to_string(),
        messages: vec![],
        temperature: None,
        max_tokens: None,
        top_p: None,
        presence_penalty: None,
        stream: None,
        tools: None,
        tool_choice: None,
        response_format: None,
    };

    let schema = json!({
        "type": "object",
        "properties": {
            "answer": {"type": "string"}
        }
    });

    let mut config = create_test_executor_config();
    config.response_format = Some(ResponseFormat {
        name: "answer_schema".to_string(),
        schema: schema.clone(),
    });

    apply_config_to_request(&mut request, Some(config));

    assert!(request.response_format.is_some());
    let format = request.response_format.unwrap();
    assert_eq!(format.format_type, "json_schema");
    assert!(format.json_schema.is_some());
    let json_schema = format.json_schema.unwrap();
    assert_eq!(json_schema.name, "answer_schema");
    assert_eq!(json_schema.schema, schema);
    assert_eq!(json_schema.strict, Some(true));
}

#[test]
fn test_apply_all_config_options() {
    // Test verifies all config options apply together correctly

    let mut request = OpenAIRequest {
        model: "gpt-4".to_string(),
        messages: vec![],
        temperature: None,
        max_tokens: None,
        top_p: None,
        presence_penalty: None,
        stream: None,
        tools: None,
        tool_choice: None,
        response_format: None,
    };

    let mut config = create_test_executor_config();
    config.temperature = Some(0.8);
    config.max_tokens = Some(2000);
    config.top_p = Some(0.95);
    config.presence_penalty = Some(0.3);
    config.tools = vec![Tool {
        name: "tool1".to_string(),
        description: "Tool".to_string(),
        parameters: json!({}),
    }];
    config.tool_choice = Some(ToolChoice::Auto);
    config.response_format = Some(ResponseFormat {
        name: "schema".to_string(),
        schema: json!({"type": "object"}),
    });

    apply_config_to_request(&mut request, Some(config));

    assert_eq!(request.temperature, Some(0.8));
    assert_eq!(request.max_tokens, Some(2000));
    assert_eq!(request.top_p, Some(0.95));
    assert_eq!(request.presence_penalty, Some(0.3));
    assert!(request.tools.is_some());
    assert_eq!(request.tool_choice, Some("auto".to_string()));
    assert!(request.response_format.is_some());
}

// ============================================================================
// Tool Call Conversion Tests
// ============================================================================

#[test]
fn test_convert_single_tool_call() {
    // Test verifies single tool call conversion from OpenAI format

    let openai_calls = vec![OpenAIToolCall {
        id: "call_123".to_string(),
        call_type: "function".to_string(),
        function: OpenAIToolFunction {
            name: "get_weather".to_string(),
            arguments: r#"{"location":"Seattle"}"#.to_string(),
        },
    }];

    let converted = convert_tool_calls(&openai_calls);

    assert_eq!(converted.len(), 1);
    assert_eq!(converted[0].id, "call_123");
    assert_eq!(converted[0].name, "get_weather");
    assert_eq!(converted[0].arguments["location"], "Seattle");
}

#[test]
fn test_convert_multiple_tool_calls() {
    // Test verifies multiple tool calls conversion

    let openai_calls = vec![
        OpenAIToolCall {
            id: "call_1".to_string(),
            call_type: "function".to_string(),
            function: OpenAIToolFunction {
                name: "tool1".to_string(),
                arguments: r#"{"arg1":"value1"}"#.to_string(),
            },
        },
        OpenAIToolCall {
            id: "call_2".to_string(),
            call_type: "function".to_string(),
            function: OpenAIToolFunction {
                name: "tool2".to_string(),
                arguments: r#"{"arg2":"value2"}"#.to_string(),
            },
        },
    ];

    let converted = convert_tool_calls(&openai_calls);

    assert_eq!(converted.len(), 2);
    assert_eq!(converted[0].id, "call_1");
    assert_eq!(converted[0].name, "tool1");
    assert_eq!(converted[1].id, "call_2");
    assert_eq!(converted[1].name, "tool2");
}

#[test]
fn test_convert_tool_call_with_invalid_json() {
    // Test verifies invalid JSON in arguments results in empty object

    let openai_calls = vec![OpenAIToolCall {
        id: "call_bad".to_string(),
        call_type: "function".to_string(),
        function: OpenAIToolFunction {
            name: "bad_tool".to_string(),
            arguments: "not valid json".to_string(),
        },
    }];

    let converted = convert_tool_calls(&openai_calls);

    assert_eq!(converted.len(), 1);
    assert_eq!(converted[0].id, "call_bad");
    assert_eq!(converted[0].name, "bad_tool");
    assert_eq!(converted[0].arguments, json!({}));
}

#[test]
fn test_convert_empty_tool_call_array() {
    // Test verifies empty tool call array handling

    let openai_calls: Vec<OpenAIToolCall> = vec![];
    let converted = convert_tool_calls(&openai_calls);

    assert_eq!(converted.len(), 0);
}

// ============================================================================
// Token Estimation Tests
// ============================================================================

#[test]
fn test_estimate_tokens_empty_string() {
    // Test verifies empty string returns 0 tokens

    let tokens = estimate_tokens("");
    assert_eq!(tokens, 0);
}

#[test]
fn test_estimate_tokens_simple_text() {
    // Test verifies basic token estimation (~4 chars per token)

    let text = "Hello world, this is a test."; // 29 chars
    let tokens = estimate_tokens(text);

    assert_eq!(tokens, 7); // 29 / 4 = 7
}

#[test]
fn test_estimate_tokens_longer_text() {
    // Test verifies estimation scales linearly

    let text = "a".repeat(400); // 400 chars
    let tokens = estimate_tokens(&text);

    assert_eq!(tokens, 100); // 400 / 4 = 100
}

#[test]
fn test_estimate_message_tokens_empty() {
    // Test verifies empty message array returns 0 tokens

    let messages: Vec<OpenAIMessage> = vec![];
    let tokens = estimate_message_tokens(&messages);

    assert_eq!(tokens, 0);
}

#[test]
fn test_estimate_message_tokens_single_message() {
    // Test verifies single message with formatting overhead

    let messages = vec![OpenAIMessage {
        role: "user".to_string(),
        content: "Test message".to_string(), // 12 chars
    }];

    let tokens = estimate_message_tokens(&messages);

    // "user: Test message\n" = ~17 chars = 4 tokens + 8 overhead = 12 tokens
    assert_eq!(tokens, 12);
}

#[test]
fn test_estimate_message_tokens_multiple_messages() {
    // Test verifies multiple messages include per-message overhead

    let messages = vec![
        OpenAIMessage {
            role: "system".to_string(),
            content: "System".to_string(), // 6 chars
        },
        OpenAIMessage {
            role: "user".to_string(),
            content: "User".to_string(), // 4 chars
        },
        OpenAIMessage {
            role: "assistant".to_string(),
            content: "Assistant".to_string(), // 9 chars
        },
    ];

    let tokens = estimate_message_tokens(&messages);

    // Total content: ~40 chars = 10 tokens
    // Per-message overhead: 3 * 8 = 24 tokens
    // Total: 35 tokens (actual measurement)
    assert_eq!(tokens, 35);
}

// ============================================================================
// Handle Tool Calls Tests
// ============================================================================

#[test]
fn test_handle_tool_calls_with_standard_format() {
    // Test verifies standard OpenAI tool calls are processed correctly

    let message = OpenAIResponseMessage {
        role: "assistant".to_string(),
        content: "".to_string(),
        tool_calls: Some(vec![OpenAIToolCall {
            id: "call_123".to_string(),
            call_type: "function".to_string(),
            function: OpenAIToolFunction {
                name: "get_weather".to_string(),
                arguments: r#"{"location":"NYC"}"#.to_string(),
            },
        }]),
    };

    let result = handle_tool_calls(&message).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, "call_123");
    assert_eq!(result[0].name, "get_weather");
    assert_eq!(result[0].arguments["location"], "NYC");
}

#[test]
fn test_handle_tool_calls_with_no_tool_calls() {
    // Test verifies no tool calls returns empty array

    let message = OpenAIResponseMessage {
        role: "assistant".to_string(),
        content: "Just a regular response".to_string(),
        tool_calls: None,
    };

    let result = handle_tool_calls(&message).unwrap();

    assert_eq!(result.len(), 0);
}

#[test]
fn test_handle_tool_calls_with_custom_format() {
    // Test verifies custom format detection (XML tool_call)

    let message = OpenAIResponseMessage {
        role: "assistant".to_string(),
        content: r#"<tool_call>{"name": "get_weather", "arguments": {"location": "Seattle"}}</tool_call>"#.to_string(),
        tool_calls: None,
    };

    let result = handle_tool_calls(&message).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "get_weather");
    assert_eq!(result[0].arguments["location"], "Seattle");
    assert!(result[0].id.starts_with("custom_"));
}

#[test]
fn test_handle_tool_calls_with_content_cleaning() {
    // Test verifies content cleaning when custom format detected

    let message = OpenAIResponseMessage {
        role: "assistant".to_string(),
        content: r#"Here is the result: <tool_call>{"name": "test", "arguments": {}}</tool_call>"#
            .to_string(),
        tool_calls: None,
    };

    let result = handle_tool_calls_with_content_cleaning(&message).unwrap();

    assert_eq!(result.tool_calls.len(), 1);
    assert!(result.cleaned_content.is_some());
    let cleaned = result.cleaned_content.unwrap();
    assert_eq!(cleaned, "Here is the result:");
    assert!(!cleaned.contains("<tool_call>"));
}

#[test]
fn test_handle_tool_calls_standard_takes_precedence() {
    // Test verifies standard tool calls take precedence over custom formats
    // Important: Don't parse content if standard tool_calls exist

    let message = OpenAIResponseMessage {
        role: "assistant".to_string(),
        content: r#"<tool_call>{"name": "custom_tool", "arguments": {}}</tool_call>"#.to_string(),
        tool_calls: Some(vec![OpenAIToolCall {
            id: "standard_123".to_string(),
            call_type: "function".to_string(),
            function: OpenAIToolFunction {
                name: "standard_tool".to_string(),
                arguments: "{}".to_string(),
            },
        }]),
    };

    let result = handle_tool_calls_with_content_cleaning(&message).unwrap();

    // Should get standard tool call, not custom
    assert_eq!(result.tool_calls.len(), 1);
    assert_eq!(result.tool_calls[0].id, "standard_123");
    assert_eq!(result.tool_calls[0].name, "standard_tool");
    assert!(result.cleaned_content.is_none());
}
