//! Caching control logic for Anthropic prompt caching

use crate::core_types::provider::Tool;
use serde_json::Value;

/// Convert executor tools to Anthropic format with caching support
pub(super) fn convert_executor_tools_to_anthropic(
    tools: &[Tool],
    enable_caching: bool,
    cache_ttl: &str,
) -> Vec<Value> {
    let mut tool_defs: Vec<Value> = tools
        .iter()
        .map(|tool| {
            serde_json::json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": tool.parameters
            })
        })
        .collect();

    // Add cache control ONLY to the last tool to define the cache breakpoint
    // This caches all tools as a single prefix following Anthropic's hierarchy
    if enable_caching && !tool_defs.is_empty() {
        if let Some(last_tool) = tool_defs.last_mut() {
            last_tool["cache_control"] = serde_json::json!({
                "type": "ephemeral",
                "ttl": cache_ttl
            });
        }
    }

    tool_defs
}
