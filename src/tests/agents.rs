// Unit Tests for Agent Context Types
//
// UNIT UNDER TEST: AgentContext
//
// BUSINESS RESPONSIBILITY:
//   - Provides agent-aware context tracking for LLM requests
//   - Enables request correlation via session identifiers
//   - Supports agent-specific metadata for logging and monitoring
//   - Allows per-request retry policy customization
//   - Facilitates debugging and observability in multi-agent systems
//
// TEST COVERAGE:
//   - Factory methods for each agent type (ConversationAgent, StoryMemoryAgent, ContextManagerAgent)
//   - Proper default metadata for agent-specific priorities and patterns
//   - Session ID propagation and tracking
//   - Custom metadata addition via builder pattern
//   - Custom retry policy configuration
//   - Agent name consistency and uniqueness

use crate::agents::AgentContext;
use crate::tests::helpers::create_test_retry_policy;

#[cfg(test)]
mod agent_context_factory_tests {
    use super::*;

    #[test]
    fn test_conversation_agent_context_creation() {
        // Test verifies ConversationAgent context has proper defaults
        // Ensures real-time priority and ReAct pattern metadata

        // Arrange
        let session_id = "test-session-123";

        // Act
        let context = AgentContext::conversation(session_id);

        // Assert
        assert_eq!(context.agent_name, "ConversationAgent");
        assert_eq!(context.session_id, session_id);
        assert_eq!(
            context.metadata.get("pattern"),
            Some(&serde_json::json!("ReAct"))
        );
        assert_eq!(
            context.metadata.get("priority"),
            Some(&serde_json::json!("real-time"))
        );
        assert!(
            context.retry_policy.is_none(),
            "Should not have custom retry policy by default"
        );
    }

    #[test]
    fn test_story_memory_agent_context_creation() {
        // Test verifies StoryMemoryAgent context has proper defaults
        // Ensures accuracy priority and extraction function metadata

        // Arrange
        let session_id = "test-session-456";

        // Act
        let context = AgentContext::story_memory(session_id);

        // Assert
        assert_eq!(context.agent_name, "StoryMemoryAgent");
        assert_eq!(context.session_id, session_id);
        assert_eq!(
            context.metadata.get("function"),
            Some(&serde_json::json!("extraction"))
        );
        assert_eq!(
            context.metadata.get("priority"),
            Some(&serde_json::json!("accuracy"))
        );
        assert!(
            context.retry_policy.is_none(),
            "Should not have custom retry policy by default"
        );
    }

    #[test]
    fn test_context_manager_agent_context_creation() {
        // Test verifies ContextManagerAgent context has proper defaults
        // Ensures efficiency priority and context-selection function metadata

        // Arrange
        let session_id = "test-session-789";

        // Act
        let context = AgentContext::context_manager(session_id);

        // Assert
        assert_eq!(context.agent_name, "ContextManagerAgent");
        assert_eq!(context.session_id, session_id);
        assert_eq!(
            context.metadata.get("function"),
            Some(&serde_json::json!("context-selection"))
        );
        assert_eq!(
            context.metadata.get("priority"),
            Some(&serde_json::json!("efficiency"))
        );
        assert!(
            context.retry_policy.is_none(),
            "Should not have custom retry policy by default"
        );
    }
}

#[cfg(test)]
mod agent_context_builder_tests {
    use super::*;

    #[test]
    fn test_with_metadata_adds_custom_entry() {
        // Test verifies custom metadata can be added to context
        // Enables agent-specific logging and monitoring enrichment

        // Arrange
        let session_id = "test-session";
        let custom_key = "request_type";
        let custom_value = serde_json::json!("initial");

        // Act
        let context =
            AgentContext::conversation(session_id).with_metadata(custom_key, custom_value.clone());

        // Assert
        assert_eq!(
            context.metadata.get(custom_key),
            Some(&custom_value),
            "Custom metadata should be present"
        );
        // Original metadata should still be present
        assert_eq!(
            context.metadata.get("pattern"),
            Some(&serde_json::json!("ReAct"))
        );
    }

    #[test]
    fn test_with_metadata_allows_chaining() {
        // Test verifies multiple metadata entries can be added via builder pattern
        // Ensures fluent API for context construction

        // Arrange
        let session_id = "test-session";

        // Act
        let context = AgentContext::story_memory(session_id)
            .with_metadata("extraction_type", serde_json::json!("characters"))
            .with_metadata("confidence_threshold", serde_json::json!(0.8))
            .with_metadata("batch_id", serde_json::json!(42));

        // Assert
        assert_eq!(context.metadata.len(), 5); // 2 defaults + 3 custom
        assert_eq!(
            context.metadata.get("extraction_type"),
            Some(&serde_json::json!("characters"))
        );
        assert_eq!(
            context.metadata.get("confidence_threshold"),
            Some(&serde_json::json!(0.8))
        );
        assert_eq!(
            context.metadata.get("batch_id"),
            Some(&serde_json::json!(42))
        );
    }

    #[test]
    fn test_with_retry_policy_sets_custom_policy() {
        // Test verifies custom retry policy can be configured per request
        // Enables per-agent retry behavior customization

        // Arrange
        let session_id = "test-session";
        let custom_policy = create_test_retry_policy();

        // Act
        let context =
            AgentContext::context_manager(session_id).with_retry_policy(custom_policy.clone());

        // Assert
        assert!(
            context.retry_policy.is_some(),
            "Should have custom retry policy"
        );
        let policy = context.retry_policy.unwrap();
        assert_eq!(policy.max_attempts, custom_policy.max_attempts);
        assert_eq!(policy.initial_delay, custom_policy.initial_delay);
    }

    #[test]
    fn test_builder_pattern_combines_metadata_and_retry_policy() {
        // Test verifies builder pattern supports both metadata and retry policy
        // Ensures comprehensive context customization

        // Arrange
        let session_id = "test-session";
        let custom_policy = create_test_retry_policy();

        // Act
        let context = AgentContext::conversation(session_id)
            .with_metadata("timeout_override", serde_json::json!(30))
            .with_retry_policy(custom_policy.clone())
            .with_metadata("priority_boost", serde_json::json!(true));

        // Assert
        assert!(context.retry_policy.is_some());
        assert_eq!(
            context.metadata.get("timeout_override"),
            Some(&serde_json::json!(30))
        );
        assert_eq!(
            context.metadata.get("priority_boost"),
            Some(&serde_json::json!(true))
        );
    }
}

#[cfg(test)]
mod agent_context_session_tracking_tests {
    use super::*;

    #[test]
    fn test_session_id_propagates_correctly() {
        // Test verifies session IDs are preserved across context creation
        // Ensures request correlation and tracing functionality

        // Arrange
        let session_ids = vec!["session-abc-123", "session-xyz-789", "session-test-000"];

        // Act & Assert
        for session_id in session_ids {
            let conv_context = AgentContext::conversation(session_id);
            let memory_context = AgentContext::story_memory(session_id);
            let ctx_mgr_context = AgentContext::context_manager(session_id);

            assert_eq!(conv_context.session_id, session_id);
            assert_eq!(memory_context.session_id, session_id);
            assert_eq!(ctx_mgr_context.session_id, session_id);
        }
    }

    #[test]
    fn test_session_id_accepts_string_types() {
        // Test verifies session ID accepts both String and &str
        // Ensures flexible API usage

        // Arrange
        let owned_string = String::from("owned-session-id");
        let string_ref = "borrowed-session-id";

        // Act
        let context1 = AgentContext::conversation(owned_string.clone());
        let context2 = AgentContext::conversation(string_ref);

        // Assert
        assert_eq!(context1.session_id, owned_string);
        assert_eq!(context2.session_id, string_ref);
    }
}

#[cfg(test)]
mod agent_context_serialization_tests {
    use super::*;

    #[test]
    fn test_context_serialization_roundtrip() {
        // Test verifies AgentContext can be serialized and deserialized
        // Ensures context can be transmitted across service boundaries

        // Arrange
        let original = AgentContext::conversation("test-session")
            .with_metadata("custom_field", serde_json::json!("custom_value"));

        // Act
        let serialized = serde_json::to_string(&original).expect("Should serialize successfully");
        let deserialized: AgentContext =
            serde_json::from_str(&serialized).expect("Should deserialize successfully");

        // Assert
        assert_eq!(deserialized.agent_name, original.agent_name);
        assert_eq!(deserialized.session_id, original.session_id);
        assert_eq!(deserialized.metadata.len(), original.metadata.len());
        assert_eq!(
            deserialized.metadata.get("custom_field"),
            original.metadata.get("custom_field")
        );
    }

    #[test]
    fn test_context_with_retry_policy_serialization() {
        // Test verifies context with retry policy serializes correctly
        // Ensures complete context state can be persisted

        // Arrange
        let policy = create_test_retry_policy();
        let original = AgentContext::story_memory("test-session").with_retry_policy(policy);

        // Act
        let serialized = serde_json::to_string(&original).expect("Should serialize successfully");
        let deserialized: AgentContext =
            serde_json::from_str(&serialized).expect("Should deserialize successfully");

        // Assert
        assert!(deserialized.retry_policy.is_some());
        let original_policy = original.retry_policy.unwrap();
        let deserialized_policy = deserialized.retry_policy.unwrap();
        assert_eq!(
            deserialized_policy.max_attempts,
            original_policy.max_attempts
        );
        assert_eq!(
            deserialized_policy.initial_delay,
            original_policy.initial_delay
        );
    }
}

#[cfg(test)]
mod agent_context_clone_tests {
    use super::*;

    #[test]
    fn test_context_clone_creates_independent_copy() {
        // Test verifies cloning creates independent context copies
        // Ensures context can be safely shared across operations

        // Arrange
        let original = AgentContext::conversation("test-session")
            .with_metadata("original", serde_json::json!(true));

        // Act
        let mut cloned = original.clone();
        cloned
            .metadata
            .insert("cloned".to_string(), serde_json::json!(true));

        // Assert
        assert!(
            original.metadata.get("cloned").is_none(),
            "Original should not have cloned metadata"
        );
        assert!(
            cloned.metadata.get("original").is_some(),
            "Clone should have original metadata"
        );
        assert_eq!(original.session_id, cloned.session_id);
        assert_eq!(original.agent_name, cloned.agent_name);
    }
}
