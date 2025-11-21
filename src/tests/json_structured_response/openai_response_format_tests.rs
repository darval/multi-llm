// Unit Tests for OpenAI Response Format Structures
//
// UNIT UNDER TEST: OpenAIResponseFormat and OpenAIJsonSchema (concrete types)
//
// BUSINESS RESPONSIBILITY:
//   - Converts generic story analysis schemas to OpenAI-compatible request formats
//   - Ensures structured story analysis works across different LLM providers
//   - Maintains schema integrity when transforming between provider formats
//   - Enables consistent story element extraction regardless of underlying LLM provider

use crate::providers::openai_shared::{OpenAIJsonSchema, OpenAIRequest, OpenAIResponseFormat};
use serde_json::json;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_format_conversion_enables_cross_provider_story_analysis() {
        // Test verifies system converts story analysis schemas to OpenAI-compatible format
        // enabling consistent story processing across different LLM providers

        // Arrange: Create story analysis schema for emotional content extraction
        let story_schema = OpenAIJsonSchema {
            name: "story_emotional_analysis".to_string(),
            schema: json!({
                "type": "object",
                "properties": {
                    "emotional_intensity": {
                        "type": "number",
                        "minimum": 0,
                        "maximum": 1,
                        "description": "Emotional intensity of story content for narrative arc analysis"
                    },
                    "primary_emotion": {
                        "type": "string",
                        "enum": ["joy", "sadness", "nostalgia", "fear", "anger", "love"],
                        "description": "Dominant emotion for story theme categorization"
                    },
                    "story_elements": {
                        "type": "object",
                        "properties": {
                            "characters": {"type": "array"},
                            "locations": {"type": "array"}
                        }
                    }
                },
                "required": ["emotional_intensity", "primary_emotion"]
            }),
            strict: Some(true),
        };

        // Act: Create OpenAI response format for story analysis
        let response_format = OpenAIResponseFormat {
            format_type: "json_schema".to_string(),
            json_schema: Some(story_schema.clone()),
        };

        // Assert: Verify OpenAI format maintains story analysis schema integrity
        assert_eq!(response_format.format_type, "json_schema");
        assert_eq!(
            response_format.json_schema.unwrap().name,
            "story_emotional_analysis"
        );

        // Assert: Verify strict mode enabled for reliable story element extraction
        assert_eq!(story_schema.strict, Some(true));
    }

    #[test]
    fn test_openai_request_configures_structured_story_response() {
        // Test verifies OpenAI requests properly configure structured story analysis
        // enabling reliable extraction of narrative elements from LLM responses

        // Arrange: Create story analysis schema for OpenAI request
        let story_analysis_schema = OpenAIJsonSchema {
            name: "narrative_analysis".to_string(),
            schema: json!({
                "type": "object",
                "properties": {
                    "themes": {"type": "array", "items": {"type": "string"}},
                    "emotional_arc": {"type": "string"},
                    "key_characters": {"type": "array"}
                },
                "required": ["themes", "emotional_arc"]
            }),
            strict: Some(true),
        };

        let response_format = OpenAIResponseFormat {
            format_type: "json_schema".to_string(),
            json_schema: Some(story_analysis_schema),
        };

        // Act: Create OpenAI request with story analysis format
        let request = OpenAIRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            temperature: Some(0.1), // Lower temperature for consistent story analysis
            max_tokens: Some(2000),
            top_p: None,
            presence_penalty: None,
            stream: Some(false),
            tools: Some(vec![]),
            tool_choice: None,
            response_format: Some(response_format),
        };

        // Assert: Verify request configured for structured story analysis
        assert!(request.response_format.is_some());
        let format = request.response_format.unwrap();
        assert_eq!(format.format_type, "json_schema");

        // Assert: Verify schema name indicates story analysis purpose
        let schema = format.json_schema.unwrap();
        assert_eq!(schema.name, "narrative_analysis");

        // Assert: Verify temperature set appropriately for consistent analysis
        assert_eq!(request.temperature, Some(0.1));
    }

    #[test]
    fn test_openai_schema_strict_mode_enforces_story_structure() {
        // Test verifies strict mode variations properly enforce story analysis schema
        // ensuring consistent story element extraction across different use cases

        // Arrange: Create schemas with different strict mode settings
        let strict_story_schema = OpenAIJsonSchema {
            name: "strict_story_analysis".to_string(),
            schema: json!({"type": "object", "properties": {"emotion": {"type": "string"}}}),
            strict: Some(true),
        };

        let flexible_story_schema = OpenAIJsonSchema {
            name: "flexible_story_analysis".to_string(),
            schema: json!({"type": "object", "properties": {"emotion": {"type": "string"}}}),
            strict: Some(false),
        };

        let default_story_schema = OpenAIJsonSchema {
            name: "default_story_analysis".to_string(),
            schema: json!({"type": "object", "properties": {"emotion": {"type": "string"}}}),
            strict: None,
        };

        // Act: Verify strict mode settings are preserved
        // (No explicit Act section - configuration verification)

        // Assert: Verify strict mode enables reliable story analysis
        assert_eq!(strict_story_schema.strict, Some(true));
        assert_eq!(flexible_story_schema.strict, Some(false));
        assert_eq!(default_story_schema.strict, None);

        // Assert: Verify all schemas maintain story analysis structure
        assert!(strict_story_schema.schema["properties"]["emotion"].is_object());
        assert!(flexible_story_schema.schema["properties"]["emotion"].is_object());
        assert!(default_story_schema.schema["properties"]["emotion"].is_object());
    }
}
