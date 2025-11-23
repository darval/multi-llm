// Unit Tests for Configuration Application to LLM Requests
//
// UNIT UNDER TEST: utils::apply_config_to_request (function)
//
// BUSINESS RESPONSIBILITY:
//   - Configures LLM providers to return structured story analysis responses
//   - Ensures consistent response format across different AI providers (OpenAI, Anthropic, LM Studio)
//   - Enables schema enforcement for reliable story element extraction
//   - Transforms generic schema definitions into provider-specific request formats

use super::helpers::*;
use crate::providers::openai_shared::utils;
use crate::core_types::provider::LLMRequestConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_enables_structured_story_analysis_across_providers() {
        // Test verifies configuration system applies story analysis schemas to LLM requests
        // enabling consistent narrative processing across different AI providers

        // Arrange: Create base OpenAI request and story analysis configuration
        let mut request = create_openai_request_for_testing();
        let story_analysis_format = create_test_story_emotional_analysis_schema();

        let config = LLMRequestConfig {
            temperature: Some(0.1), // Low temperature for consistent story analysis
            max_tokens: Some(2000),
            response_format: Some(story_analysis_format.clone()),
            ..Default::default()
        };

        // Act: Apply story analysis configuration to request
        utils::apply_config_to_request(&mut request, Some(config));

        // Assert: Verify request configured for structured story analysis
        assert_eq!(request.temperature, Some(0.1));
        assert_eq!(request.max_tokens, Some(2000));
        assert!(request.response_format.is_some());

        // Assert: Verify story analysis schema properly converted to OpenAI format
        let openai_format = request.response_format.unwrap();
        assert_eq!(openai_format.format_type, "json_schema");
        assert!(openai_format.json_schema.is_some());

        // Assert: Verify schema maintains story analysis structure and strict mode
        let schema = openai_format.json_schema.unwrap();
        assert_eq!(schema.name, "story_emotional_analysis");
        assert_eq!(schema.strict, Some(true));

        // Assert: Verify emotional analysis fields preserved in schema conversion
        let schema_props = &schema.schema["properties"];
        assert!(schema_props["emotional_intensity"].is_object());
        assert!(schema_props["primary_emotion"].is_object());
    }

    #[test]
    fn test_config_preserves_request_when_no_story_schema_provided() {
        // Test verifies configuration system gracefully handles requests without story analysis
        // maintaining existing request parameters for standard text-based processing

        // Arrange: Create request with existing configuration
        let mut request = create_openai_request_for_testing();
        request.temperature = Some(0.7); // Pre-existing configuration
        request.max_tokens = Some(1000);

        let config = LLMRequestConfig {
            temperature: Some(0.8), // Should update existing value
            response_format: None,  // No story analysis schema
            ..Default::default()
        };

        // Act: Apply configuration without story analysis schema
        utils::apply_config_to_request(&mut request, Some(config));

        // Assert: Verify standard configuration applied without structured response
        assert_eq!(request.temperature, Some(0.8));
        assert!(request.response_format.is_none());

        // Assert: Verify request remains valid for text-based story processing
        assert_eq!(request.model, "test-model");
        assert!(request.stream.is_some());
    }

    #[test]
    fn test_complex_story_schema_application_preserves_narrative_structure() {
        // Test verifies complex nested story analysis schemas maintain their structure
        // during configuration application for comprehensive narrative element extraction

        // Arrange: Create request and complex story analysis schema using helper
        let mut request = create_openai_request_for_testing();
        let complex_story_schema = create_test_complex_story_schema();

        let config = LLMRequestConfig {
            temperature: Some(0.0), // Deterministic for story analysis
            response_format: Some(complex_story_schema.clone()),
            ..Default::default()
        };

        // Act: Apply complex story analysis configuration
        utils::apply_config_to_request(&mut request, Some(config));

        // Assert: Verify complex schema applied successfully
        assert!(request.response_format.is_some());
        assert_eq!(request.temperature, Some(0.0));

        // Assert: Verify nested story analysis structure preserved
        let openai_format = request.response_format.unwrap();
        let applied_schema = openai_format.json_schema.unwrap();
        assert_eq!(applied_schema.schema, complex_story_schema.schema);
        assert_eq!(applied_schema.name, "complex_story_analysis");

        // Assert: Verify strict mode enabled for reliable story element extraction
        assert_eq!(applied_schema.strict, Some(true));

        // Assert: Verify nested emotional content and story elements structure preserved
        let analysis_props = &applied_schema.schema["properties"]["analysis"]["properties"];
        assert!(analysis_props["emotional_content"]["properties"]["primary_emotion"].is_object());
        assert!(analysis_props["emotional_content"]["properties"]["intensity"].is_object());
        assert!(analysis_props["story_elements"]["properties"]["characters"].is_object());
        assert!(analysis_props["story_elements"]["properties"]["locations"].is_object());
    }

    #[test]
    fn test_configuration_handles_missing_config_gracefully() {
        // Test verifies system maintains stability when no configuration provided
        // ensuring story processing can continue with default LLM settings

        // Arrange: Create base request with default settings
        let mut original_request = create_openai_request_for_testing();
        original_request.temperature = Some(0.5);
        let mut request = original_request.clone();

        // Act: Apply None configuration (simulates missing config scenario)
        utils::apply_config_to_request(&mut request, None);

        // Assert: Verify original request preserved when no config provided
        assert_eq!(request.temperature, original_request.temperature);
        assert_eq!(request.max_tokens, original_request.max_tokens);
        assert_eq!(request.response_format, original_request.response_format);
        assert_eq!(request.model, original_request.model);

        // Assert: Verify request remains valid for story processing
        assert!(request.response_format.is_none()); // No structured response configured
        assert_eq!(request.stream, Some(false)); // Maintains streaming preference
    }
}
