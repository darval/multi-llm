// Unit Tests for Response and ResponseFormat
//
// UNIT UNDER TEST: Response (concrete type)
//
// BUSINESS RESPONSIBILITY:
//   - Captures structured AI analysis of user narratives for story processing
//   - Enables extraction of story elements (emotions, themes, characters, locations)
//   - Provides type-safe access to LLM-generated story insights and analysis
//   - Preserves both text content and structured data for flexible story processing
//
// TEST COVERAGE:
//   - Structured story analysis response creation and validation
//   - Schema definition for narrative analysis formats
//   - Edge cases: empty responses, missing structured data

use super::helpers::*;
use crate::provider::{Response, ResponseFormat, TokenUsage};
use serde_json::json;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structured_response_captures_story_emotional_analysis() {
        // Test verifies system can capture structured emotional analysis from LLM
        // for story element extraction and user narrative insights

        // Arrange: Create expected story analysis data from LLM
        let story_analysis_data = json!({
            "emotional_intensity": 0.85,
            "primary_emotion": "nostalgia",
            "themes": ["family", "childhood", "home"],
            "story_elements": {
                "characters": [{"name": "grandmother", "relationship": "family"}],
                "locations": [{"name": "old house", "type": "residential"}]
            }
        });

        // Act: Create Response with structured story analysis
        let response = Response {
            content: "The story shows strong nostalgic themes about family and childhood memories in the old house with grandmother.".to_string(),
            structured_response: Some(story_analysis_data.clone()),
            tool_calls: vec![],
            usage: Some(TokenUsage {
                prompt_tokens: 50,
                completion_tokens: 30,
                total_tokens: 80,
            }),
            model: Some("test-model".to_string()),
            raw_body: None,
        };

        // Assert: Verify story analysis data is captured for narrative processing
        assert_eq!(
            response.structured_response,
            Some(story_analysis_data.clone())
        );
        assert!(response.content.contains("nostalgic"));

        // Assert: Verify emotional intensity is within valid business range (0-1)
        let emotional_intensity = story_analysis_data["emotional_intensity"].as_f64().unwrap();
        assert!(emotional_intensity >= 0.0 && emotional_intensity <= 1.0);

        // Assert: Verify required story elements are present
        assert_eq!(story_analysis_data["primary_emotion"], "nostalgia");
        assert!(story_analysis_data["themes"].is_array());
    }

    #[test]
    fn test_response_format_defines_narrative_analysis_schema() {
        // Test verifies system can define schemas for story emotional analysis
        // which is critical for understanding user narrative emotional arcs

        // Arrange: Define schema for story emotional analysis
        let emotional_analysis_schema = json!({
            "type": "object",
            "properties": {
                "emotional_intensity": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 1,
                    "description": "Intensity of emotional content in user's story"
                },
                "primary_emotion": {
                    "type": "string",
                    "enum": ["joy", "sadness", "nostalgia", "fear", "anger", "love"],
                    "description": "Dominant emotion in narrative segment"
                },
                "story_themes": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Key themes extracted from user's narrative"
                }
            },
            "required": ["emotional_intensity", "primary_emotion"]
        });

        // Act: Create format for story emotional analysis
        let format = ResponseFormat {
            name: "story_emotional_analysis".to_string(),
            schema: emotional_analysis_schema.clone(),
        };

        // Assert: Verify schema enables story emotion extraction
        assert_eq!(format.name, "story_emotional_analysis");
        assert_eq!(format.schema, emotional_analysis_schema);

        // Assert: Verify schema contains required emotional analysis fields
        assert!(format.schema["properties"]["emotional_intensity"].is_object());
        assert!(format.schema["properties"]["primary_emotion"].is_object());

        // Assert: Verify emotional intensity has proper business constraints
        assert_eq!(
            format.schema["properties"]["emotional_intensity"]["minimum"],
            0
        );
        assert_eq!(
            format.schema["properties"]["emotional_intensity"]["maximum"],
            1
        );
    }

    #[test]
    fn test_story_schema_supports_character_and_location_extraction() {
        // Test verifies response format can define complex nested schemas
        // for comprehensive story element extraction (characters, locations, themes)

        // Arrange: Create comprehensive story analysis schema using helper
        let story_schema = create_test_complex_story_schema();

        // Act: Verify schema structure supports story element extraction
        let schema_properties = &story_schema.schema["properties"]["analysis"]["properties"];

        // Assert: Verify story elements schema contains character extraction fields
        assert!(schema_properties["story_elements"]["properties"]["characters"].is_object());
        assert!(schema_properties["story_elements"]["properties"]["locations"].is_object());

        // Assert: Verify emotional content analysis is properly structured
        assert!(
            schema_properties["emotional_content"]["properties"]["primary_emotion"].is_object()
        );
        assert!(schema_properties["emotional_content"]["properties"]["intensity"].is_object());

        // Assert: Verify required fields are enforced for story processing
        let required_fields = schema_properties["story_elements"]["required"]
            .as_array()
            .unwrap();
        assert!(required_fields.contains(&json!("characters")));
        assert!(required_fields.contains(&json!("locations")));
    }
}
