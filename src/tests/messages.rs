//! Unit Tests for Messages Module
//!
//! UNIT UNDER TEST: CacheType enum and UnifiedMessage cache builder methods
//!
//! BUSINESS RESPONSIBILITY:
//!   - Provides cache type enum for Anthropic prompt caching (Ephemeral/Extended)
//!   - Provides builder methods to mark messages for specific cache types
//!   - Ensures messages can be marked for 5-minute (ephemeral) or 1-hour (extended) caching
//!
//! TEST COVERAGE:
//!   - CacheType enum variants and default behavior
//!   - Builder methods set correct cache_type and cacheable flags
//!   - Builder methods are chainable and preserve message data
//!   - Edge cases: builder methods on messages with different roles and content types

use crate::messages::{CacheType, MessageRole, UnifiedMessage};

// ============================================================================
// CacheType Enum Tests
// ============================================================================

#[test]
fn test_cache_type_default_is_ephemeral() {
    // Test verifies CacheType::default() returns Ephemeral variant
    //
    // Business rule: Ephemeral (5-minute) cache is the default for safety

    // Act
    let cache_type = CacheType::default();

    // Assert
    assert_eq!(cache_type, CacheType::Ephemeral);
}

#[test]
fn test_cache_type_variants_exist() {
    // Test verifies both cache type variants are available
    //
    // Business rule: Support both Anthropic cache TTL options

    // Act & Assert - verify both variants compile and are distinct
    let ephemeral = CacheType::Ephemeral;
    let extended = CacheType::Extended;

    assert_ne!(ephemeral, extended, "Cache types should be distinct");
}

// ============================================================================
// Builder Method Tests - with_ephemeral_cache()
// ============================================================================

#[test]
fn test_with_ephemeral_cache_sets_cache_type() {
    // Test verifies with_ephemeral_cache() sets cache_type to Ephemeral
    //
    // Business rule: Ephemeral cache marks content for 5-minute caching

    // Arrange
    let message = UnifiedMessage::user("Test content");

    // Act
    let cached_message = message.with_ephemeral_cache();

    // Assert
    assert_eq!(
        cached_message.attributes.cache_type,
        Some(CacheType::Ephemeral),
        "cache_type should be set to Ephemeral"
    );
}

#[test]
fn test_with_ephemeral_cache_sets_cacheable_true() {
    // Test verifies with_ephemeral_cache() enables caching
    //
    // Business rule: Cache type implies message is cacheable

    // Arrange
    let message = UnifiedMessage::user("Test content");
    assert!(!message.attributes.cacheable, "Should start non-cacheable");

    // Act
    let cached_message = message.with_ephemeral_cache();

    // Assert
    assert!(
        cached_message.attributes.cacheable,
        "cacheable flag should be set to true"
    );
}

#[test]
fn test_with_ephemeral_cache_preserves_message_content() {
    // Test verifies builder method doesn't alter message content
    //
    // Business rule: Cache control is metadata, not content modification

    // Arrange
    let original_content = "Important message content";
    let message = UnifiedMessage::user(original_content);

    // Act
    let cached_message = message.with_ephemeral_cache();

    // Assert
    match cached_message.content {
        crate::messages::MessageContent::Text(ref text) => {
            assert_eq!(text, original_content, "Content should be preserved");
        }
        _ => panic!("Expected text content"),
    }
}

#[test]
fn test_with_ephemeral_cache_preserves_role() {
    // Test verifies builder method doesn't alter message role
    //
    // Business rule: Cache control doesn't change message semantics

    // Arrange
    let message = UnifiedMessage::system("System prompt");

    // Act
    let cached_message = message.with_ephemeral_cache();

    // Assert
    assert_eq!(
        cached_message.role,
        MessageRole::System,
        "Role should be preserved"
    );
}

// ============================================================================
// Builder Method Tests - with_extended_cache()
// ============================================================================

#[test]
fn test_with_extended_cache_sets_cache_type() {
    // Test verifies with_extended_cache() sets cache_type to Extended
    //
    // Business rule: Extended cache marks content for 1-hour caching

    // Arrange
    let message = UnifiedMessage::user("Test content");

    // Act
    let cached_message = message.with_extended_cache();

    // Assert
    assert_eq!(
        cached_message.attributes.cache_type,
        Some(CacheType::Extended),
        "cache_type should be set to Extended"
    );
}

#[test]
fn test_with_extended_cache_sets_cacheable_true() {
    // Test verifies with_extended_cache() enables caching
    //
    // Business rule: Cache type implies message is cacheable

    // Arrange
    let message = UnifiedMessage::user("Test content");
    assert!(!message.attributes.cacheable, "Should start non-cacheable");

    // Act
    let cached_message = message.with_extended_cache();

    // Assert
    assert!(
        cached_message.attributes.cacheable,
        "cacheable flag should be set to true"
    );
}

#[test]
fn test_with_extended_cache_preserves_message_content() {
    // Test verifies builder method doesn't alter message content
    //
    // Business rule: Cache control is metadata, not content modification

    // Arrange
    let original_content = "Important message content";
    let message = UnifiedMessage::user(original_content);

    // Act
    let cached_message = message.with_extended_cache();

    // Assert
    match cached_message.content {
        crate::messages::MessageContent::Text(ref text) => {
            assert_eq!(text, original_content, "Content should be preserved");
        }
        _ => panic!("Expected text content"),
    }
}

// ============================================================================
// Builder Method Tests - Chaining and Edge Cases
// ============================================================================

#[test]
fn test_cache_builders_are_distinct() {
    // Test verifies ephemeral and extended builders produce different results
    //
    // Business rule: Two cache types must be distinguishable

    // Arrange
    let message = UnifiedMessage::user("Test content");

    // Act
    let ephemeral_cached = message.clone().with_ephemeral_cache();
    let extended_cached = message.with_extended_cache();

    // Assert
    assert_ne!(
        ephemeral_cached.attributes.cache_type, extended_cached.attributes.cache_type,
        "Different builders should produce different cache types"
    );
}

#[test]
fn test_cache_builder_on_system_message() {
    // Test verifies cache builders work with system messages
    //
    // Business rule: System prompts are prime candidates for caching

    // Arrange
    let message = UnifiedMessage::system("You are a helpful assistant");

    // Act
    let cached_message = message.with_extended_cache();

    // Assert
    assert_eq!(cached_message.role, MessageRole::System);
    assert_eq!(
        cached_message.attributes.cache_type,
        Some(CacheType::Extended)
    );
    assert!(cached_message.attributes.cacheable);
}

#[test]
fn test_cache_builder_on_assistant_message() {
    // Test verifies cache builders work with assistant messages
    //
    // Business rule: All message roles support caching

    // Arrange
    let message = UnifiedMessage::assistant("Previous response");

    // Act
    let cached_message = message.with_ephemeral_cache();

    // Assert
    assert_eq!(cached_message.role, MessageRole::Assistant);
    assert_eq!(
        cached_message.attributes.cache_type,
        Some(CacheType::Ephemeral)
    );
    assert!(cached_message.attributes.cacheable);
}

#[test]
fn test_overwriting_cache_type() {
    // Test verifies calling a second builder overwrites the first
    //
    // Business rule: Last cache type specification wins

    // Arrange
    let message = UnifiedMessage::user("Test content");

    // Act - apply ephemeral then extended
    let cached_message = message.with_ephemeral_cache().with_extended_cache();

    // Assert - should be extended (last applied)
    assert_eq!(
        cached_message.attributes.cache_type,
        Some(CacheType::Extended),
        "Second builder should overwrite first"
    );
}
