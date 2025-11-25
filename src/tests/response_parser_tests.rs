//! Tests for ResponseParser
//!
//! Testing 3-tier fallback strategy for parsing LLM output into structured JSON.

use crate::internals::response_parser::ResponseParser;

#[test]
fn test_parse_direct_json() {
    let input = r#"{"test": "value", "number": 42}"#;
    let result = ResponseParser::parse_llm_output(input).unwrap();
    assert_eq!(result["test"], "value");
    assert_eq!(result["number"], 42);
}

#[test]
fn test_parse_with_artifacts() {
    let input = r#"```json
{"test": "value", "number": 42}
```"#;
    let result = ResponseParser::parse_llm_output(input).unwrap();
    assert_eq!(result["test"], "value");
    assert_eq!(result["number"], 42);
}

#[test]
fn test_parse_mixed_content() {
    let input = r#"Here is the response: {"test": "value", "number": 42} and some trailing text"#;
    let result = ResponseParser::parse_llm_output(input).unwrap();
    assert_eq!(result["test"], "value");
    assert_eq!(result["number"], 42);
}

#[test]
fn test_parse_nested_json() {
    let input = r#"{"outer": {"inner": {"value": "test"}}, "array": [1, 2, 3]}"#;
    let result = ResponseParser::parse_llm_output(input).unwrap();
    assert_eq!(result["outer"]["inner"]["value"], "test");
    assert_eq!(result["array"][0], 1);
}

#[test]
fn test_parse_invalid_json_fails() {
    let input = r#"This is just text without any JSON"#;
    let result = ResponseParser::parse_llm_output(input);
    assert!(result.is_err());
}

#[test]
fn test_parse_empty_object_fails() {
    let input = r#"{}"#;
    let result = ResponseParser::parse_llm_output(input);
    assert!(result.is_err());
}

#[test]
fn test_parse_non_object_fails() {
    let input = r#"["not", "an", "object"]"#;
    let result = ResponseParser::parse_llm_output(input);
    assert!(result.is_err());
}
