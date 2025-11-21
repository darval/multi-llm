//! Helper functions for JSON structured response tests
//!
//! Common test utilities and data builders for structured response testing.
//! These helpers support story analysis schema creation and test response generation.

use crate::providers::openai_shared::OpenAIRequest;
use crate::core_types::executor::ExecutorResponseFormat;
use crate::core_types::executor::{ExecutorLLMResponse, ExecutorTokenUsage};
use serde_json::json;

/// Create a test schema for story emotional analysis
pub fn create_test_story_emotional_analysis_schema() -> ExecutorResponseFormat {
    ExecutorResponseFormat {
        name: "story_emotional_analysis".to_string(),
        schema: json!({
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
                    "enum": ["joy", "sadness", "nostalgia", "fear", "anger", "love", "excitement"],
                    "description": "Dominant emotion in narrative segment"
                },
                "themes": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Key themes extracted from story"
                }
            },
            "required": ["emotional_intensity", "primary_emotion"]
        }),
    }
}

/// Create a test LLM response with structured story analysis data
pub fn create_test_story_analysis_response() -> ExecutorLLMResponse {
    let story_analysis_data = json!({
        "emotional_intensity": 0.85,
        "primary_emotion": "nostalgia",
        "themes": ["family", "childhood", "home"],
        "story_elements": {
            "characters": [{"name": "grandmother", "relationship": "family"}],
            "locations": [{"name": "old house", "type": "residential"}]
        }
    });

    ExecutorLLMResponse {
        content: "The story shows strong nostalgic themes...".to_string(),
        structured_response: Some(story_analysis_data),
        tool_calls: vec![],
        usage: Some(ExecutorTokenUsage {
            prompt_tokens: 50,
            completion_tokens: 30,
            total_tokens: 80,
        }),
        model: Some("test-model".to_string()),
        raw_body: None,
    }
}

/// Create a complex nested schema for comprehensive story analysis
pub fn create_test_complex_story_schema() -> ExecutorResponseFormat {
    ExecutorResponseFormat {
        name: "complex_story_analysis".to_string(),
        schema: json!({
            "type": "object",
            "properties": {
                "analysis": {
                    "type": "object",
                    "properties": {
                        "emotional_content": {
                            "type": "object",
                            "properties": {
                                "primary_emotion": {"type": "string"},
                                "intensity": {"type": "number", "minimum": 0, "maximum": 1}
                            },
                            "required": ["primary_emotion", "intensity"]
                        },
                        "story_elements": {
                            "type": "object",
                            "properties": {
                                "characters": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "name": {"type": "string"},
                                            "relationship": {"type": "string"}
                                        },
                                        "required": ["name", "relationship"]
                                    }
                                },
                                "locations": {
                                    "type": "array",
                                    "items": {"type": "object"}
                                }
                            },
                            "required": ["characters", "locations"]
                        }
                    },
                    "required": ["emotional_content", "story_elements"]
                }
            },
            "required": ["analysis"]
        }),
    }
}

/// Create a basic OpenAI request for testing
pub fn create_openai_request_for_testing() -> OpenAIRequest {
    OpenAIRequest {
        model: "test-model".to_string(),
        messages: vec![],
        temperature: None,
        max_tokens: None,
        top_p: None,
        presence_penalty: None,
        stream: Some(false),
        tools: Some(vec![]),
        tool_choice: None,
        response_format: None,
    }
}
