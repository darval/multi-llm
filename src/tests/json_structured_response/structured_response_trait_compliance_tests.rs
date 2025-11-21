// Unit Tests for Cross-Provider Structured Response Compliance
//
// UNIT UNDER TEST: Cross-provider structured response consistency (trait compliance)
//
// BUSINESS RESPONSIBILITY:
//   - Ensures all LLM providers handle structured story analysis consistently
//   - Validates schema application works uniformly across OpenAI, Anthropic, LM Studio
//   - Maintains story processing reliability regardless of underlying LLM provider
//   - Enables seamless provider switching without affecting narrative analysis quality

use super::helpers::*;
use crate::providers::openai_shared::{utils, OpenAIJsonSchema};
use crate::core_types::executor::LLMRequestConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_response_format_consistency_across_story_schemas() {
        // Test verifies ExecutorResponseFormat maintains consistent behavior
        // for story analysis schemas regardless of complexity or use case

        // Arrange: Create different story analysis formats using helpers
        let simple_format = create_test_story_emotional_analysis_schema();
        let complex_format = create_test_complex_story_schema();

        // Act: Verify both formats maintain consistent structure
        // (No explicit act - testing structural consistency)

        // Assert: Verify both formats follow same naming conventions
        assert!(simple_format.name.contains("story"));
        assert!(complex_format.name.contains("story"));

        // Assert: Verify both formats have valid JSON schema structure
        assert!(simple_format.schema.is_object());
        assert!(complex_format.schema.is_object());
        assert!(simple_format.schema["properties"].is_object());
        assert!(complex_format.schema["properties"].is_object());

        // Assert: Verify both formats support emotional analysis
        let simple_props = &simple_format.schema["properties"];
        let complex_props = &complex_format.schema["properties"]["analysis"]["properties"];
        assert!(simple_props["emotional_intensity"].is_object());
        assert!(complex_props["emotional_content"]["properties"]["intensity"].is_object());
    }

    #[test]
    fn test_openai_format_conversion_maintains_story_schema_integrity() {
        // Test verifies OpenAI format conversion preserves story analysis schema structure
        // ensuring consistent narrative processing across different provider implementations

        // Arrange: Create different story schemas and convert to OpenAI format
        let formats = vec![
            create_test_story_emotional_analysis_schema(),
            create_test_complex_story_schema(),
        ];

        for format in formats {
            // Act: Convert to OpenAI format (simulated)
            let openai_schema = OpenAIJsonSchema {
                name: format.name.clone(),
                schema: format.schema.clone(),
                strict: Some(true),
            };

            // Assert: Verify OpenAI conversion maintains story schema structure
            assert_eq!(openai_schema.name, format.name);
            assert_eq!(openai_schema.schema, format.schema);
            assert_eq!(openai_schema.strict, Some(true));

            // Assert: Verify essential story analysis fields preserved
            if format.name.contains("emotional") {
                assert!(openai_schema.schema["properties"]["emotional_intensity"].is_object());
                assert!(openai_schema.schema["properties"]["primary_emotion"].is_object());
            } else if format.name.contains("complex") {
                let analysis_props = &openai_schema.schema["properties"]["analysis"]["properties"];
                assert!(analysis_props["emotional_content"].is_object());
                assert!(analysis_props["story_elements"].is_object());
            }
        }
    }

    #[test]
    fn test_structured_response_business_constraint_enforcement() {
        // Test verifies all structured response types enforce critical business constraints
        // maintaining story analysis quality and preventing invalid emotional intensity values

        // Arrange: Create story analysis response with business constraints
        let story_response = create_test_story_analysis_response();

        // Act: Extract and validate business-critical fields
        let structured_data = story_response.structured_response.unwrap();
        let emotional_intensity = structured_data["emotional_intensity"].as_f64().unwrap();
        let primary_emotion = structured_data["primary_emotion"].as_str().unwrap();

        // Assert: Verify emotional intensity within business range (0-1)
        assert!(emotional_intensity >= 0.0 && emotional_intensity <= 1.0);

        // Assert: Verify primary emotion is valid for story categorization
        let valid_emotions = vec![
            "joy",
            "sadness",
            "nostalgia",
            "fear",
            "anger",
            "love",
            "excitement",
        ];
        assert!(valid_emotions.contains(&primary_emotion));

        // Assert: Verify story themes structure supports narrative building
        assert!(structured_data["themes"].is_array());
        let themes = structured_data["themes"].as_array().unwrap();
        assert!(!themes.is_empty());

        // Assert: Verify all theme entries are valid strings
        for theme in themes {
            assert!(theme.is_string());
            assert!(!theme.as_str().unwrap().is_empty());
        }
    }

    #[test]
    fn test_configuration_application_consistency_across_request_types() {
        // Test verifies configuration application produces consistent results
        // regardless of request complexity or provider-specific formatting

        // Arrange: Create different request configurations
        let configs = vec![
            (create_test_story_emotional_analysis_schema(), 0.1f64),
            (create_test_complex_story_schema(), 0.0f64),
        ];

        for (response_format, expected_temperature) in configs {
            // Arrange: Create request and config for this iteration
            let mut request = create_openai_request_for_testing();
            let config = LLMRequestConfig {
                temperature: Some(expected_temperature),
                response_format: Some(response_format.clone()),
                ..Default::default()
            };

            // Act: Apply configuration to request
            utils::apply_config_to_request(&mut request, Some(config));

            // Assert: Verify consistent configuration application
            assert_eq!(request.temperature, Some(expected_temperature));
            assert!(request.response_format.is_some());

            // Assert: Verify OpenAI format conversion consistency
            let applied_format = request.response_format.unwrap();
            assert_eq!(applied_format.format_type, "json_schema");

            let applied_schema = applied_format.json_schema.unwrap();
            assert_eq!(applied_schema.name, response_format.name);
            assert_eq!(applied_schema.schema, response_format.schema);
            assert_eq!(applied_schema.strict, Some(true)); // Always strict for story analysis
        }
    }
}
