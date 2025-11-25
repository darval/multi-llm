//! Unit Tests for Anthropic Caching Logic
//!
//! UNIT UNDER TEST: convert_executor_tools_to_anthropic function
//!
//! BUSINESS RESPONSIBILITY:
//!   - Convert Tool to Anthropic tool format
//!   - Apply cache control to last tool only (defines cache breakpoint)
//!   - Support ephemeral TTL configuration (5m or 1h)
//!   - Cache all tools as a single prefix following Anthropic's hierarchy
//!
//! TEST COVERAGE:
//!   - Empty tool array handling
//!   - Single tool with caching enabled
//!   - Single tool with caching disabled
//!   - Multiple tools - cache control on last tool only
//!   - Different TTL values (5m, 1h)
//!   - Tool format conversion (name, description, input_schema)

use super::super::caching::convert_executor_tools_to_anthropic;
use crate::provider::Tool;

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_tool(name: &str) -> Tool {
    Tool {
        name: name.to_string(),
        description: format!("Test tool: {}", name),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "arg": {"type": "string"}
            }
        }),
    }
}

// ============================================================================
// Empty Tool Array Tests
// ============================================================================

#[test]
fn test_empty_tools_returns_empty_array() {
    // Test verifies empty tool list produces empty result
    // Edge case: no tools provided

    let tools: Vec<Tool> = vec![];
    let result = convert_executor_tools_to_anthropic(&tools, true, "5m");

    assert!(result.is_empty(), "Empty tools should return empty array");
}

#[test]
fn test_empty_tools_with_caching_disabled() {
    // Test verifies empty tools with caching disabled
    // Should still return empty array

    let tools: Vec<Tool> = vec![];
    let result = convert_executor_tools_to_anthropic(&tools, false, "5m");

    assert!(result.is_empty());
}

// ============================================================================
// Single Tool Tests
// ============================================================================

#[test]
fn test_single_tool_with_caching_enabled() {
    // Test verifies single tool gets cache control when caching enabled
    // CRITICAL: Cache control on last tool defines cache breakpoint

    let tools = vec![create_test_tool("search")];
    let result = convert_executor_tools_to_anthropic(&tools, true, "5m");

    assert_eq!(result.len(), 1);

    let tool = &result[0];
    assert_eq!(tool["name"], "search");
    assert_eq!(tool["description"], "Test tool: search");
    assert!(tool["input_schema"].is_object());

    // Verify cache control is present
    assert!(
        tool["cache_control"].is_object(),
        "Single tool should have cache_control when enabled"
    );
    assert_eq!(tool["cache_control"]["type"], "ephemeral");
    assert_eq!(tool["cache_control"]["ttl"], "5m");
}

#[test]
fn test_single_tool_with_caching_disabled() {
    // Test verifies single tool has no cache control when caching disabled
    // Cache control should not be added if caching is disabled

    let tools = vec![create_test_tool("calculator")];
    let result = convert_executor_tools_to_anthropic(&tools, false, "5m");

    assert_eq!(result.len(), 1);

    let tool = &result[0];
    assert_eq!(tool["name"], "calculator");

    // Verify cache control is NOT present
    assert!(
        tool.get("cache_control").is_none() || tool["cache_control"].is_null(),
        "Single tool should not have cache_control when disabled"
    );
}

// ============================================================================
// Multiple Tools Tests
// ============================================================================

#[test]
fn test_multiple_tools_cache_only_last() {
    // Test verifies ONLY the last tool gets cache control
    // CRITICAL BUSINESS LOGIC: Anthropic's caching hierarchy
    // All tools are cached as a single prefix by marking only the last

    let tools = vec![
        create_test_tool("search"),
        create_test_tool("calculator"),
        create_test_tool("weather"),
    ];

    let result = convert_executor_tools_to_anthropic(&tools, true, "1h");

    assert_eq!(result.len(), 3);

    // First two tools should NOT have cache control
    assert!(
        result[0].get("cache_control").is_none() || result[0]["cache_control"].is_null(),
        "First tool should not have cache_control"
    );
    assert!(
        result[1].get("cache_control").is_none() || result[1]["cache_control"].is_null(),
        "Second tool should not have cache_control"
    );

    // Last tool MUST have cache control
    assert!(
        result[2]["cache_control"].is_object(),
        "Last tool must have cache_control to define cache breakpoint"
    );
    assert_eq!(result[2]["cache_control"]["type"], "ephemeral");
    assert_eq!(result[2]["cache_control"]["ttl"], "1h");
}

#[test]
fn test_multiple_tools_without_caching() {
    // Test verifies no tools get cache control when caching disabled
    // Even with multiple tools, caching can be disabled

    let tools = vec![
        create_test_tool("tool1"),
        create_test_tool("tool2"),
        create_test_tool("tool3"),
    ];

    let result = convert_executor_tools_to_anthropic(&tools, false, "1h");

    assert_eq!(result.len(), 3);

    // All tools should NOT have cache control
    for (i, tool) in result.iter().enumerate() {
        assert!(
            tool.get("cache_control").is_none() || tool["cache_control"].is_null(),
            "Tool {} should not have cache_control when caching disabled",
            i
        );
    }
}

// ============================================================================
// TTL Configuration Tests
// ============================================================================

#[test]
fn test_ttl_5m_configuration() {
    // Test verifies 5-minute TTL is correctly applied
    // Short TTL for frequently changing tools

    let tools = vec![create_test_tool("dynamic_tool")];
    let result = convert_executor_tools_to_anthropic(&tools, true, "5m");

    assert_eq!(result[0]["cache_control"]["ttl"], "5m");
}

#[test]
fn test_ttl_1h_configuration() {
    // Test verifies 1-hour TTL is correctly applied
    // Longer TTL for stable tools

    let tools = vec![create_test_tool("stable_tool")];
    let result = convert_executor_tools_to_anthropic(&tools, true, "1h");

    assert_eq!(result[0]["cache_control"]["ttl"], "1h");
}

// ============================================================================
// Tool Format Conversion Tests
// ============================================================================

#[test]
fn test_tool_format_conversion() {
    // Test verifies Tool converts to correct Anthropic format
    // Anthropic expects: name, description, input_schema

    let tool = Tool {
        name: "test_function".to_string(),
        description: "A test function with parameters".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "city": {"type": "string", "description": "City name"},
                "units": {"type": "string", "enum": ["celsius", "fahrenheit"]}
            },
            "required": ["city"]
        }),
    };

    let result = convert_executor_tools_to_anthropic(&[tool], false, "5m");

    assert_eq!(result.len(), 1);
    let anthropic_tool = &result[0];

    // Verify Anthropic format
    assert_eq!(anthropic_tool["name"], "test_function");
    assert_eq!(
        anthropic_tool["description"],
        "A test function with parameters"
    );

    // Verify input_schema contains the parameters
    assert!(anthropic_tool["input_schema"].is_object());
    assert_eq!(anthropic_tool["input_schema"]["type"], "object");
    assert_eq!(
        anthropic_tool["input_schema"]["properties"]["city"]["type"],
        "string"
    );
    assert_eq!(anthropic_tool["input_schema"]["required"][0], "city");
}

#[test]
fn test_all_tools_preserve_schema() {
    // Test verifies all tools preserve their input schema correctly
    // No information should be lost during conversion

    let tools = vec![
        Tool {
            name: "simple".to_string(),
            description: "Simple tool".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        },
        Tool {
            name: "complex".to_string(),
            description: "Complex tool".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "nested": {
                        "type": "object",
                        "properties": {
                            "deep": {"type": "number"}
                        }
                    }
                }
            }),
        },
    ];

    let result = convert_executor_tools_to_anthropic(&tools, true, "1h");

    // First tool
    assert_eq!(result[0]["name"], "simple");
    assert_eq!(result[0]["input_schema"]["type"], "object");

    // Second tool with nested schema
    assert_eq!(result[1]["name"], "complex");
    assert_eq!(
        result[1]["input_schema"]["properties"]["nested"]["properties"]["deep"]["type"],
        "number"
    );
}
