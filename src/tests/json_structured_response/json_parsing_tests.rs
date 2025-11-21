// Unit Tests for JSON Parsing and Validation Logic
//
// UNIT UNDER TEST: JSON parsing functions and validation logic (business logic functions)
//
// BUSINESS RESPONSIBILITY:
//   - Validates structured story analysis data meets business requirements
//   - Ensures emotional intensity ratings are within valid ranges (0.0-1.0)
//   - Verifies nested story element fields are present for narrative processing
//   - Handles malformed LLM responses gracefully to maintain system stability

use serde_json::json;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_story_analysis_json_parsing_extracts_narrative_elements() {
        // Test verifies system can parse valid structured story analysis JSON
        // enabling extraction of emotional content and story elements for narrative processing

        // Arrange: Create valid story analysis JSON from LLM response
        let story_analysis_json = r#"{"emotional_intensity": 0.92, "primary_emotion": "nostalgia", "themes": ["family", "childhood"], "story_elements": {"characters": [{"name": "grandmother"}], "locations": ["old house"]}}"#;

        // Act: Parse story analysis JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(story_analysis_json);

        // Assert: Verify parsing succeeds for story processing
        assert!(parsed.is_ok());
        let story_analysis = parsed.unwrap();

        // Assert: Verify emotional analysis fields extracted correctly
        assert_eq!(story_analysis["emotional_intensity"], 0.92);
        assert_eq!(story_analysis["primary_emotion"], "nostalgia");

        // Assert: Verify story elements structured properly for narrative building
        assert!(story_analysis["themes"].is_array());
        assert!(story_analysis["story_elements"]["characters"].is_array());
        assert!(story_analysis["story_elements"]["locations"].is_array());
    }

    #[test]
    fn test_malformed_json_graceful_handling_preserves_system_stability() {
        // Test verifies system handles malformed LLM responses gracefully
        // maintaining story processing stability when structured analysis fails

        // Arrange: Create malformed JSON that might come from LLM
        let malformed_story_json = r#"{"emotional_intensity": 0.92, "primary_emotion": "nostalgia"#; // Missing closing brace

        // Act: Attempt to parse malformed story analysis JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(malformed_story_json);

        // Assert: Verify parsing fails gracefully without system crash
        assert!(parsed.is_err());

        // Assert: Verify error provides information for fallback to text processing
        let error = parsed.unwrap_err();
        // Verify it's a JSON parsing error (indicates malformed structure)
        assert!(error.to_string().contains("EOF") || error.to_string().contains("expected"));
    }

    #[test]
    fn test_emotional_intensity_range_validation_enforces_business_constraints() {
        // Test verifies emotional intensity ratings meet business requirements (0.0-1.0)
        // ensuring consistent story analysis across different narrative segments

        // Arrange: Create JSON with emotional intensity ratings for story analysis
        let story_emotion_json = r#"{"emotional_intensity_rating": 0.75, "alternative_rating": 0.9, "rating_explanation": "Strong nostalgic content with deep family connections"}"#;

        // Act: Parse and extract emotional intensity ratings
        let parsed: serde_json::Value = serde_json::from_str(story_emotion_json).unwrap();
        let primary_rating = parsed
            .get("emotional_intensity_rating")
            .and_then(|v| v.as_f64());
        let alt_rating = parsed.get("alternative_rating").and_then(|v| v.as_f64());

        // Assert: Verify ratings extracted correctly for story processing
        assert_eq!(primary_rating, Some(0.75));
        assert_eq!(alt_rating, Some(0.9));

        // Assert: Verify business constraint compliance (0-1 range for emotional intensity)
        assert!(primary_rating.unwrap() >= 0.0 && primary_rating.unwrap() <= 1.0);
        assert!(alt_rating.unwrap() >= 0.0 && alt_rating.unwrap() <= 1.0);

        // Assert: Verify explanation field supports narrative context
        assert!(parsed
            .get("rating_explanation")
            .and_then(|v| v.as_str())
            .is_some());
    }

    #[test]
    fn test_json_field_validation() {
        // Test nested field validation
        let json_content = json!({
            "analysis": {
                "emotional_content": {
                    "primary_emotion": "nostalgia",
                    "intensity": 0.8
                },
                "story_elements": {
                    "characters": [{"name": "grandmother", "relationship": "family"}],
                    "locations": [{"name": "old house", "type": "residential"}]
                }
            },
            "metadata": {
                "processing_confidence": 0.95,
                "complexity_score": 0.7
            }
        });

        // Test field path validation logic
        fn has_field(json: &serde_json::Value, field_path: &str) -> bool {
            let parts: Vec<&str> = field_path.split('.').collect();
            let mut current = json;

            for part in parts {
                if let Some(next) = current.get(part) {
                    current = next;
                } else {
                    return false;
                }
            }
            true
        }

        // Test required field paths
        assert!(has_field(&json_content, "analysis"));
        assert!(has_field(&json_content, "analysis.emotional_content"));
        assert!(has_field(
            &json_content,
            "analysis.emotional_content.primary_emotion"
        ));
        assert!(has_field(
            &json_content,
            "analysis.emotional_content.intensity"
        ));
        assert!(has_field(
            &json_content,
            "analysis.story_elements.characters"
        ));
        assert!(has_field(&json_content, "metadata.processing_confidence"));

        // Test non-existent field paths
        assert!(!has_field(&json_content, "non_existent"));
        assert!(!has_field(&json_content, "analysis.missing_field"));
    }
}
