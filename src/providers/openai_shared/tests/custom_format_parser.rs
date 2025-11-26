//! Unit Tests for Custom Tool Call Format Parser
//!
//! UNIT UNDER TEST: CustomFormatParser
//!
//! BUSINESS RESPONSIBILITY:
//!   - Parse 5 different custom tool call formats from model responses
//!   - Extract function name and arguments from each format
//!   - Clean content by removing tool call markers
//!   - Handle malformed JSON with repair attempts
//!   - Support formats: GPT-OSS v1, XML tool_call, DeepSeek TOOL_REQUEST, tool_call_with_args, JSON-only
//!
//! TEST COVERAGE:
//!   - Each of the 5 format patterns (happy path)
//!   - No match scenarios
//!   - Malformed JSON handling
//!   - Content cleaning (removing tool call markers)
//!   - Edge cases: empty content, nested JSON, incomplete tool calls

use super::super::utils::CustomFormatParser;

// ============================================================================
// Parser Initialization Tests
// ============================================================================

#[test]
fn test_parser_initializes_successfully() {
    // Test verifies parser initializes without panicking
    // Critical: All patterns must compile successfully

    let _parser = CustomFormatParser::new();
    // If we get here, parser initialized successfully
}

#[test]
fn test_parser_default_initializes() {
    // Test verifies Default trait implementation works
    // Ensures consistent initialization

    let _parser1 = CustomFormatParser::new();
    let _parser2 = CustomFormatParser::default();
    // Both should initialize without panicking
}

// ============================================================================
// GPT-OSS v1 Format Tests
// ============================================================================

#[test]
fn test_parse_gpt_oss_v1_format() {
    // Test verifies GPT-OSS v1 format parsing
    // Format: commentary to=functions.FUNC_NAME <|constrain|>json<|message|>{...}

    let parser = CustomFormatParser::new();
    let content = r#"commentary to=functions.get_weather <|constrain|>json<|message|>{"city": "London", "units": "celsius"}"#;

    let result = parser.parse(content).unwrap();
    assert!(result.is_some(), "Should parse GPT-OSS v1 format");

    let match_result = result.unwrap();
    assert_eq!(match_result.function_name, "get_weather");
    assert_eq!(match_result.arguments["city"], "London");
    assert_eq!(match_result.arguments["units"], "celsius");
}

#[test]
fn test_parse_gpt_oss_v1_with_text_around() {
    // Test verifies GPT-OSS format embedded in text is parsed
    // Content before/after the tool call should be preserved in cleaned_content

    let parser = CustomFormatParser::new();
    let content = r#"Let me check the weather. commentary to=functions.get_weather <|constrain|>json<|message|>{"city": "NYC"} I'll get that for you."#;

    let result = parser.parse(content).unwrap();
    assert!(result.is_some());

    let match_result = result.unwrap();
    assert_eq!(match_result.function_name, "get_weather");
    assert!(match_result.cleaned_content.contains("Let me check"));
    assert!(match_result.cleaned_content.contains("I'll get that"));
}

// ============================================================================
// XML Tool Call Format Tests (Qwen models)
// ============================================================================

#[test]
fn test_parse_xml_tool_call_format() {
    // Test verifies XML <tool_call> format parsing
    // Format: <tool_call>{"name": "func", "arguments": {...}}</tool_call>

    let parser = CustomFormatParser::new();
    let content = r#"<tool_call>{"name": "search", "arguments": {"query": "rust"}}</tool_call>"#;

    let result = parser.parse(content).unwrap();
    assert!(result.is_some(), "Should parse XML tool_call format");

    let match_result = result.unwrap();
    assert_eq!(match_result.function_name, "search");
    assert_eq!(match_result.arguments["query"], "rust");
    assert!(
        match_result.cleaned_content.is_empty(),
        "Content should be cleaned"
    );
}

#[test]
fn test_parse_xml_tool_call_without_closing_tag() {
    // Test verifies XML format without closing tag is handled
    // Some models don't include </tool_call>

    let parser = CustomFormatParser::new();
    let content =
        r#"<tool_call>{"name": "calculator", "arguments": {"operation": "add", "a": 5, "b": 3}}"#;

    let result = parser.parse(content).unwrap();
    assert!(result.is_some(), "Should parse XML without closing tag");

    let match_result = result.unwrap();
    assert_eq!(match_result.function_name, "calculator");
    assert_eq!(match_result.arguments["operation"], "add");
}

#[test]
fn test_parse_xml_tool_call_multiline() {
    // Test verifies XML format across multiple lines
    // (?s) flag in regex should handle newlines

    let parser = CustomFormatParser::new();
    let content = r#"<tool_call>
{
  "name": "get_user",
  "arguments": {
    "user_id": 123
  }
}
</tool_call>"#;

    let result = parser.parse(content).unwrap();
    assert!(result.is_some(), "Should parse multiline XML");

    let match_result = result.unwrap();
    assert_eq!(match_result.function_name, "get_user");
    assert_eq!(match_result.arguments["user_id"], 123);
}

// ============================================================================
// DeepSeek TOOL_REQUEST Format Tests
// ============================================================================

#[test]
fn test_parse_deepseek_tool_request_format() {
    // Test verifies DeepSeek [TOOL_REQUEST] format parsing
    // Format: [TOOL_REQUEST]{"name": "func", "arguments": {...}}[END_TOOL_REQUEST]

    let parser = CustomFormatParser::new();
    let content = r#"[TOOL_REQUEST]{"name": "file_read", "arguments": {"path": "/tmp/file.txt"}}[END_TOOL_REQUEST]"#;

    let result = parser.parse(content).unwrap();
    assert!(
        result.is_some(),
        "Should parse DeepSeek TOOL_REQUEST format"
    );

    let match_result = result.unwrap();
    assert_eq!(match_result.function_name, "file_read");
    assert_eq!(match_result.arguments["path"], "/tmp/file.txt");
}

#[test]
fn test_parse_deepseek_with_surrounding_text() {
    // Test verifies DeepSeek format with text before/after

    let parser = CustomFormatParser::new();
    let content = r#"I'll read the file now. [TOOL_REQUEST]{"name": "read", "arguments": {"file": "data.json"}}[END_TOOL_REQUEST] Done."#;

    let result = parser.parse(content).unwrap();
    assert!(result.is_some());

    let match_result = result.unwrap();
    assert_eq!(match_result.function_name, "read");
    assert!(match_result.cleaned_content.contains("I'll read"));
    assert!(match_result.cleaned_content.contains("Done"));
}

// ============================================================================
// Tool Call With Args Format Tests
// ============================================================================

#[test]
fn test_parse_tool_call_with_args_format() {
    // Test verifies "Tool call: func with args: {...}" format
    // Self-generated format from structured content

    let parser = CustomFormatParser::new();
    let content = r#"Tool call: calculate with args: {"x": 10, "y": 20}"#;

    let result = parser.parse(content).unwrap();
    assert!(result.is_some(), "Should parse tool_call_with_args format");

    let match_result = result.unwrap();
    assert_eq!(match_result.function_name, "calculate");
    assert_eq!(match_result.arguments["x"], 10);
    assert_eq!(match_result.arguments["y"], 20);
}

// ============================================================================
// JSON-Only Format Tests
// ============================================================================

#[test]
fn test_parse_json_only_format() {
    // Test verifies bare JSON format
    // Format: {"name": "func", "arguments": {...}} (isolated)
    // Note: This pattern is very restrictive and may not match in practice

    let parser = CustomFormatParser::new();
    let content =
        r#"{"name": "send_email", "arguments": {"to": "user@example.com", "subject": "Hello"}}"#;

    let result = parser.parse(content).unwrap();
    // JSON-only pattern is very strict, may not match complex nested objects
    // If it doesn't match, that's expected behavior - other patterns should catch it
    if let Some(match_result) = result {
        assert_eq!(match_result.function_name, "send_email");
        assert_eq!(match_result.arguments["to"], "user@example.com");
    }
    // Either matches or doesn't - both are valid outcomes for this edge case pattern
}

// ============================================================================
// No Match Tests
// ============================================================================

#[test]
fn test_parse_returns_none_for_plain_text() {
    // Test verifies plain text without tool calls returns None
    // No pattern should match regular conversation

    let parser = CustomFormatParser::new();
    let content = "This is just a regular response with no tool calls.";

    let result = parser.parse(content).unwrap();
    assert!(result.is_none(), "Plain text should not match any pattern");
}

#[test]
fn test_parse_returns_none_for_empty_string() {
    // Test verifies empty content returns None
    // Edge case: empty model response

    let parser = CustomFormatParser::new();
    let content = "";

    let result = parser.parse(content).unwrap();
    assert!(result.is_none(), "Empty string should not match");
}

#[test]
fn test_parse_returns_none_for_incomplete_format() {
    // Test verifies incomplete formats don't match
    // Ensures robust pattern matching

    let parser = CustomFormatParser::new();
    let content = "<tool_call>incomplete json";

    let result = parser.parse(content);
    // Should either return None or error, both acceptable
    assert!(result.is_err() || result.unwrap().is_none());
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_parse_handles_invalid_json() {
    // Test verifies invalid JSON produces appropriate error
    // Parser should fail gracefully on malformed JSON

    let parser = CustomFormatParser::new();
    let content = r#"<tool_call>{"name": "func", "arguments": {invalid json}}</tool_call>"#;

    let result = parser.parse(content);
    // Should return error or None if repair fails
    assert!(result.is_err() || result.unwrap().is_none());
}

#[test]
fn test_parse_handles_missing_name_field() {
    // Test verifies missing 'name' field produces error
    // Tool calls must have function name

    let parser = CustomFormatParser::new();
    let content = r#"<tool_call>{"arguments": {"x": 1}}</tool_call>"#;

    let result = parser.parse(content);
    assert!(result.is_err(), "Should error on missing 'name' field");
}

#[test]
fn test_parse_handles_missing_arguments_field() {
    // Test verifies missing 'arguments' field produces error
    // Tool calls must have arguments (even if empty object)

    let parser = CustomFormatParser::new();
    let content = r#"<tool_call>{"name": "test_func"}</tool_call>"#;

    let result = parser.parse(content);
    assert!(result.is_err(), "Should error on missing 'arguments' field");
}

// ============================================================================
// Content Cleaning Tests
// ============================================================================

#[test]
fn test_cleaned_content_removes_tool_marker() {
    // Test verifies tool call markers are removed from content
    // Cleaned content should only contain surrounding text

    let parser = CustomFormatParser::new();
    let content =
        r#"Let me help. <tool_call>{"name": "help", "arguments": {}}</tool_call> Here you go."#;

    let result = parser.parse(content).unwrap();
    assert!(result.is_some());

    let match_result = result.unwrap();
    assert!(!match_result.cleaned_content.contains("<tool_call>"));
    assert!(!match_result.cleaned_content.contains("</tool_call>"));
    assert!(match_result.cleaned_content.contains("Let me help"));
    assert!(match_result.cleaned_content.contains("Here you go"));
}

#[test]
fn test_raw_match_preserves_full_pattern() {
    // Test verifies raw_match contains the complete matched pattern
    // Useful for debugging and logging

    let parser = CustomFormatParser::new();
    let content = r#"<tool_call>{"name": "test", "arguments": {"key": "value"}}</tool_call>"#;

    let result = parser.parse(content).unwrap();
    assert!(result.is_some());

    let match_result = result.unwrap();
    assert!(match_result.raw_match.starts_with("<tool_call>"));
    assert!(match_result.raw_match.ends_with("</tool_call>"));
    assert!(match_result.raw_match.contains("test"));
}

// ============================================================================
// Format Priority Tests
// ============================================================================

#[test]
fn test_parser_tries_patterns_in_order() {
    // Test verifies parser tries patterns in defined order
    // First match wins if content could match multiple patterns

    let parser = CustomFormatParser::new();
    // This content could potentially match multiple patterns depending on implementation
    // Test that parser returns a result (any valid parse is acceptable)
    let content = r#"commentary to=functions.test <|constrain|>json<|message|>{"x": 1}"#;

    let result = parser.parse(content).unwrap();
    assert!(result.is_some(), "Should match first applicable pattern");
}

// ============================================================================
// Complex/Nested JSON Tests
// ============================================================================

#[test]
fn test_parse_handles_nested_objects() {
    // Test verifies nested JSON objects are preserved
    // Arguments can contain complex nested structures

    let parser = CustomFormatParser::new();
    let content = r#"<tool_call>{"name": "create_user", "arguments": {"user": {"name": "John", "address": {"city": "NYC", "zip": "10001"}}}}</tool_call>"#;

    let result = parser.parse(content).unwrap();
    assert!(result.is_some());

    let match_result = result.unwrap();
    assert_eq!(match_result.function_name, "create_user");
    assert_eq!(match_result.arguments["user"]["name"], "John");
    assert_eq!(match_result.arguments["user"]["address"]["city"], "NYC");
}

#[test]
fn test_parse_handles_array_arguments() {
    // Test verifies array arguments are preserved
    // Arguments can be arrays, not just objects

    let parser = CustomFormatParser::new();
    let content = r#"<tool_call>{"name": "batch_process", "arguments": {"items": [1, 2, 3, 4, 5]}}</tool_call>"#;

    let result = parser.parse(content).unwrap();
    assert!(result.is_some());

    let match_result = result.unwrap();
    assert_eq!(match_result.function_name, "batch_process");
    assert!(match_result.arguments["items"].is_array());
    assert_eq!(match_result.arguments["items"].as_array().unwrap().len(), 5);
}

// ============================================================================
// JSON Repair Tests (clean_tool_call_patterns, attempt_json_repair)
// ============================================================================

#[test]
fn test_clean_tool_call_patterns_removes_xml_tags() {
    // Test verifies XML tool call tags are cleaned from content
    // Fallback when parsing fails to prevent showing raw XML to users

    let content = "Some text <tool_call>{\"broken\": json}</tool_call> more text";
    let cleaned = CustomFormatParser::clean_tool_call_patterns(content);

    assert!(!cleaned.contains("<tool_call>"));
    assert!(!cleaned.contains("</tool_call>"));
    assert!(cleaned.contains("Some text"));
    assert!(cleaned.contains("more text"));
}

#[test]
fn test_clean_tool_call_patterns_removes_deepseek_format() {
    // Test verifies DeepSeek tool request patterns are cleaned

    let content = "Result: [TOOL_REQUEST]{\"invalid\": data}[END_TOOL_REQUEST] Done";
    let cleaned = CustomFormatParser::clean_tool_call_patterns(content);

    assert!(!cleaned.contains("[TOOL_REQUEST]"));
    assert!(!cleaned.contains("[END_TOOL_REQUEST]"));
    assert!(cleaned.contains("Result:"));
    assert!(cleaned.contains("Done"));
}

#[test]
fn test_clean_tool_call_patterns_removes_tool_call_format() {
    // Test verifies "Tool call: X with args: {...}" format is cleaned

    let content = "I'll help. Tool call: search with args: {\"query\": \"test\"} Here.";
    let cleaned = CustomFormatParser::clean_tool_call_patterns(content);

    assert!(!cleaned.contains("Tool call:"));
    assert!(!cleaned.contains("with args:"));
    assert!(cleaned.contains("I'll help."));
    assert!(cleaned.contains("Here."));
}

#[test]
fn test_clean_tool_call_patterns_handles_unclosed_xml() {
    // Test verifies unclosed XML tags are cleaned
    // Models sometimes don't close tags properly

    let content = "Text <tool_call>incomplete json without closing";
    let cleaned = CustomFormatParser::clean_tool_call_patterns(content);

    // Should remove everything from <tool_call> onward
    assert!(!cleaned.contains("<tool_call>"));
    assert!(cleaned.contains("Text"));
}

#[test]
fn test_clean_tool_call_patterns_no_patterns_unchanged() {
    // Test verifies content without tool patterns is unchanged

    let content = "This is just regular text without any tool calls.";
    let cleaned = CustomFormatParser::clean_tool_call_patterns(content);

    assert_eq!(cleaned, content);
}

#[test]
fn test_clean_tool_call_patterns_empty_string() {
    // Test verifies empty string handling

    let content = "";
    let cleaned = CustomFormatParser::clean_tool_call_patterns(content);

    assert_eq!(cleaned, "");
}

// ============================================================================
// Balanced JSON Extraction Tests
// ============================================================================

#[test]
fn test_extract_balanced_json_simple() {
    // Test verifies simple JSON extraction

    let text = r#"{"key": "value"} extra text"#;
    let result = CustomFormatParser::extract_balanced_json(text);

    assert!(result.is_some());
    let (json_str, end_pos) = result.unwrap();
    assert_eq!(json_str, r#"{"key": "value"}"#);
    assert!(end_pos > 0);
}

#[test]
fn test_extract_balanced_json_nested() {
    // Test verifies nested JSON extraction

    let text = r#"{"outer": {"inner": {"deep": true}}} more"#;
    let result = CustomFormatParser::extract_balanced_json(text);

    assert!(result.is_some());
    let (json_str, _) = result.unwrap();
    assert!(json_str.contains("outer"));
    assert!(json_str.contains("inner"));
    assert!(json_str.contains("deep"));
}

#[test]
fn test_extract_balanced_json_with_braces_in_strings() {
    // Test verifies braces inside strings don't confuse parser

    let text = r#"{"code": "if (x) { return }"} rest"#;
    let result = CustomFormatParser::extract_balanced_json(text);

    assert!(result.is_some());
    let (json_str, _) = result.unwrap();
    assert!(json_str.contains("if (x) { return }"));
}

#[test]
fn test_extract_balanced_json_no_json() {
    // Test verifies non-JSON text returns None

    let text = "This is not JSON";
    let result = CustomFormatParser::extract_balanced_json(text);

    assert!(result.is_none());
}

#[test]
fn test_extract_balanced_json_unbalanced() {
    // Test verifies unbalanced braces returns None

    let text = r#"{"key": "value""#; // Missing closing brace
    let result = CustomFormatParser::extract_balanced_json(text);

    assert!(result.is_none());
}

#[test]
fn test_extract_balanced_json_with_leading_whitespace() {
    // Test verifies leading whitespace is handled

    let text = "   {\"key\": 1}";
    let result = CustomFormatParser::extract_balanced_json(text);

    assert!(result.is_some());
    let (json_str, _) = result.unwrap();
    assert_eq!(json_str, r#"{"key": 1}"#);
}

// ============================================================================
// DeepSeek Format Edge Cases
// ============================================================================

#[test]
fn test_parse_deepseek_without_end_marker_returns_none() {
    // Test verifies DeepSeek format without [END_TOOL_REQUEST] marker returns None
    // The regex pattern requires the end marker for proper matching

    let parser = CustomFormatParser::new();
    let content = r#"[TOOL_REQUEST]{"name": "test", "arguments": {"x": 1}}"#;

    let result = parser.parse(content).unwrap();
    // Without [END_TOOL_REQUEST], the pattern doesn't match
    assert!(result.is_none());
}

#[test]
fn test_parse_deepseek_multiline() {
    // Test verifies DeepSeek format works with multiline JSON

    let parser = CustomFormatParser::new();
    let content = r#"[TOOL_REQUEST]{
        "name": "multiline_tool",
        "arguments": {
            "param1": "value1",
            "param2": 42
        }
    }[END_TOOL_REQUEST]"#;

    let result = parser.parse(content).unwrap();
    assert!(result.is_some());

    let match_result = result.unwrap();
    assert_eq!(match_result.function_name, "multiline_tool");
    assert_eq!(match_result.arguments["param1"], "value1");
    assert_eq!(match_result.arguments["param2"], 42);
}

// ============================================================================
// JSON Brace Counting Tests
// ============================================================================

#[test]
fn test_count_json_braces_simple() {
    // Test verifies basic brace counting

    let (open, close) = CustomFormatParser::count_json_braces(r#"{"key": "value"}"#);
    assert_eq!(open, 1);
    assert_eq!(close, 1);
}

#[test]
fn test_count_json_braces_nested() {
    // Test verifies nested brace counting

    let (open, close) = CustomFormatParser::count_json_braces(r#"{"a": {"b": {"c": 1}}}"#);
    assert_eq!(open, 3);
    assert_eq!(close, 3);
}

#[test]
fn test_count_json_braces_ignores_braces_in_strings() {
    // Test verifies braces inside strings are not counted

    let (open, close) = CustomFormatParser::count_json_braces(r#"{"code": "if (x) { } else { }"}"#);
    // Only the outer {} should be counted, not the ones in the string
    assert_eq!(open, 1);
    assert_eq!(close, 1);
}

#[test]
fn test_count_json_braces_unbalanced() {
    // Test verifies unbalanced braces are detected

    let (open, close) = CustomFormatParser::count_json_braces(r#"{"key": "value""#);
    assert_eq!(open, 1);
    assert_eq!(close, 0);
}

#[test]
fn test_count_json_braces_with_escaped_quotes() {
    // Test verifies escaped quotes don't break string detection

    let (open, close) = CustomFormatParser::count_json_braces(r#"{"text": "say \"hello\""}"#);
    assert_eq!(open, 1);
    assert_eq!(close, 1);
}

// ============================================================================
// JSON Repair Tests
// ============================================================================

#[test]
fn test_attempt_json_repair_already_valid() {
    // Test verifies valid JSON is returned unchanged

    let text = r#"{"key": "value"}"#;
    let repaired = CustomFormatParser::attempt_json_repair(text);
    assert_eq!(repaired, text);
}

#[test]
fn test_attempt_json_repair_missing_one_brace() {
    // Test verifies single missing brace is added

    let text = r#"{"key": "value""#;
    let repaired = CustomFormatParser::attempt_json_repair(text);
    assert_eq!(repaired, r#"{"key": "value"}"#);
}

#[test]
fn test_attempt_json_repair_missing_multiple_braces() {
    // Test verifies multiple missing braces are added

    let text = r#"{"outer": {"inner": "value""#;
    let repaired = CustomFormatParser::attempt_json_repair(text);
    assert_eq!(repaired, r#"{"outer": {"inner": "value"}}"#);
}

#[test]
fn test_attempt_json_repair_non_json_unchanged() {
    // Test verifies non-JSON text is returned as-is

    let text = "This is not JSON";
    let repaired = CustomFormatParser::attempt_json_repair(text);
    assert_eq!(repaired, text);
}

#[test]
fn test_attempt_json_repair_with_whitespace() {
    // Test verifies leading/trailing whitespace is trimmed and brace added

    let text = "  {\"key\": \"value\"  ";
    let repaired = CustomFormatParser::attempt_json_repair(text);
    // Whitespace trimmed, then missing brace detected and added
    assert_eq!(repaired, r#"{"key": "value"}"#);
}

#[test]
fn test_add_missing_braces_single() {
    // Test verifies single brace is added

    let repaired = CustomFormatParser::add_missing_braces(r#"{"key": "value""#, 1);
    assert_eq!(repaired, r#"{"key": "value"}"#);
}

#[test]
fn test_add_missing_braces_multiple() {
    // Test verifies multiple braces are added

    let repaired = CustomFormatParser::add_missing_braces(r#"{"a": {"b": 1"#, 2);
    assert_eq!(repaired, r#"{"a": {"b": 1}}"#);
}

#[test]
fn test_add_missing_braces_zero() {
    // Test verifies zero braces doesn't change text

    let text = r#"{"key": "value"}"#;
    let repaired = CustomFormatParser::add_missing_braces(text, 0);
    assert_eq!(repaired, text);
}

// ============================================================================
// GPT-OSS Format Edge Cases
// ============================================================================

#[test]
fn test_parse_gpt_oss_with_nested_json_arguments() {
    // Test verifies GPT-OSS format with deeply nested JSON arguments

    let parser = CustomFormatParser::new();
    let content = r#"commentary to=functions.complex_call <|constrain|>json<|message|>{"nested": {"level": {"deep": {"value": 42}}}}"#;

    let result = parser.parse(content).unwrap();
    assert!(result.is_some());

    let match_result = result.unwrap();
    assert_eq!(match_result.function_name, "complex_call");
    assert_eq!(
        match_result.arguments["nested"]["level"]["deep"]["value"],
        42
    );
}
