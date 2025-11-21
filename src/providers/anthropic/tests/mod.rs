//! Tests for Anthropic Provider Implementation
//!
//! This module contains unit tests for Anthropic-specific functionality.
//! Trait compliance tests are in `providers/tests/trait_compliance.rs`.

mod caching;
mod conversion;

// NOTE: Provider HTTP tests are in tests/anthropic_provider_integration_tests.rs
// These tests use MockServer and are slow, so they don't belong in unit tests
