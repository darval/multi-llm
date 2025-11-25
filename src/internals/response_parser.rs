//! Response parsing and validation module for structured JSON responses
//!
//! Provides robust parsing with 3-tier fallback strategy to handle different
//! LLM output formats while ensuring structured responses.

use crate::error::{LlmError, LlmResult};
use crate::logging::{log_debug, log_warn};

use serde_json::Value;

/// Response parser with fallback strategies
pub struct ResponseParser;

impl ResponseParser {
    /// Parse LLM output into structured JSON with 3-tier fallback strategy
    ///
    /// 1. Try direct JSON parse
    /// 2. Clean known artifacts and retry
    /// 3. Extract JSON object from mixed content
    ///
    /// Fails with clear error if no valid JSON found
    pub fn parse_llm_output(raw: &str) -> LlmResult<Value> {
        log_debug!(
            content_length = raw.len(),
            content_preview = raw.chars().take(200).collect::<String>(),
            "Parsing LLM output for structured JSON"
        );

        // 1. Try direct JSON parse
        if let Ok(structured) = serde_json::from_str::<Value>(raw) {
            log_debug!("Successfully parsed JSON directly");
            return Self::validate_and_return(structured);
        }

        // 2. Clean known artifacts and retry
        let cleaned = Self::clean_artifacts(raw);
        if cleaned != raw {
            log_debug!(
                original_length = raw.len(),
                cleaned_length = cleaned.len(),
                "Cleaned artifacts from LLM response"
            );

            if let Ok(structured) = serde_json::from_str::<Value>(&cleaned) {
                log_debug!("Successfully parsed JSON after artifact cleaning");
                return Self::validate_and_return(structured);
            }
        }

        // 3. Extract JSON object from mixed content
        if let Some(json_str) = Self::extract_json_object(&cleaned) {
            log_debug!(
                extracted_length = json_str.len(),
                "Extracted JSON object from mixed content"
            );

            if let Ok(structured) = serde_json::from_str::<Value>(&json_str) {
                log_debug!("Successfully parsed JSON after extraction");
                return Self::validate_and_return(structured);
            }
        }

        // NO FALLBACK - must return error if parsing fails
        let preview = raw.chars().take(200).collect::<String>();
        log_warn!(
            content_preview = preview,
            "Failed to parse structured response from LLM output"
        );

        Err(LlmError::response_parsing_error(format!(
            "Could not parse structured JSON response from: {}{}",
            preview,
            if raw.len() > 200 { "..." } else { "" }
        )))
    }

    /// Validate parsed JSON structure
    fn validate_and_return(response: Value) -> LlmResult<Value> {
        // Basic validation - should be an object
        if !response.is_object() {
            return Err(LlmError::response_parsing_error(
                "Structured response must be a JSON object".to_string(),
            ));
        }

        // Check for required top-level structure (basic validation)
        if let Some(obj) = response.as_object() {
            if obj.is_empty() {
                return Err(LlmError::response_parsing_error(
                    "Structured response cannot be empty object".to_string(),
                ));
            }
        }

        Ok(response)
    }

    /// Clean known artifacts from LLM responses
    fn clean_artifacts(content: &str) -> String {
        let mut cleaned = content.to_string();

        // Remove common LLM artifacts
        cleaned = cleaned
            .replace("<|channel|>", "")
            .replace("```json", "")
            .replace("```JSON", "")
            .replace("```", "")
            .replace("<|end|>", "")
            .replace("<|start|>", "");

        // Remove leading/trailing whitespace and control characters
        cleaned = cleaned
            .trim()
            .chars()
            .filter(|c| !c.is_control() || c.is_whitespace())
            .collect();

        log_debug!(
            original_length = content.len(),
            cleaned_length = cleaned.len(),
            "Cleaned LLM response artifacts"
        );

        cleaned
    }

    /// Extract JSON object from mixed content (text + JSON)
    fn extract_json_object(content: &str) -> Option<String> {
        // Look for JSON object boundaries
        let start_idx = content.find('{')?;

        // Extract balanced JSON from the found starting position
        Self::extract_balanced_json(&content[start_idx..]).map(|(json_str, _)| json_str)
    }

    /// Extract balanced JSON from text, handling nested braces properly
    fn extract_balanced_json(text: &str) -> Option<(String, usize)> {
        let trimmed = text.trim_start();
        if !trimmed.starts_with('{') {
            return None;
        }

        let chars: Vec<char> = trimmed.chars().collect();
        let json_end = Self::find_balanced_json_end(&chars)?;

        let json_chars: String = chars[0..=json_end].iter().collect();
        let json_byte_len = json_chars.len();
        let offset = text.len() - trimmed.len(); // Account for leading whitespace
        Some((json_chars, offset + json_byte_len))
    }

    /// Find the index where balanced JSON ends
    fn find_balanced_json_end(chars: &[char]) -> Option<usize> {
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escaped = false;

        for (char_idx, ch) in chars.iter().enumerate() {
            match ch {
                '"' if !escaped => in_string = !in_string,
                '\\' if in_string => escaped = !escaped,
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        return Some(char_idx);
                    }
                }
                _ => escaped = false,
            }

            if *ch != '\\' {
                escaped = false;
            }
        }

        None // Unbalanced braces
    }
}
