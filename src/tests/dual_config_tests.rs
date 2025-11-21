//! Unit Tests for Dual LLM Configuration Parsing
//!
//! UNIT UNDER TEST: DualLLMConfig
//!
//! BUSINESS RESPONSIBILITY:
//!   - Parses dual LLM configuration from parsed TOML sections supporting [user_llm] and [nlp_llm] sections
//!   - Validates configuration completeness and provider settings
//!   - Enables cost optimization through different providers/models per path
//!
//! TEST COVERAGE:
//!   - Complete dual configuration parsing ([user_llm] + [nlp_llm])
//!   - Error handling for incomplete configurations
//!   - Cross-provider configuration (OpenAI + LMStudio)
//!   - Parameter customization (temperature, max_tokens, etc.)
//!   - Configuration validation and error reporting

use crate::config::{DualLLMConfig, LLMPath};
use std::collections::HashMap;

/// Check if line should be skipped (empty or comment)
fn should_skip_line(line: &str) -> bool {
    line.is_empty() || line.starts_with('#')
}

/// Extract section name from header line [section_name]
fn extract_section_name(line: &str) -> Option<String> {
    if line.starts_with('[') && line.ends_with(']') {
        Some(line[1..line.len() - 1].to_string())
    } else {
        None
    }
}

/// Parse key=value pair from line
fn parse_key_value(line: &str) -> Option<(String, String)> {
    if !line.contains('=') {
        return None;
    }

    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() == 2 {
        let key = parts[0].trim().to_string();
        let value = parts[1].trim().trim_matches('"').to_string();
        Some((key, value))
    } else {
        None
    }
}

/// Process section header line and update state
fn process_section_header(
    line: &str,
    sections: &mut HashMap<String, HashMap<String, String>>,
) -> Option<String> {
    extract_section_name(line).map(|section_name| {
        sections.entry(section_name.clone()).or_insert_with(HashMap::new);
        section_name
    })
}

/// Process key-value line within a section
fn process_key_value(
    line: &str,
    section_name: &str,
    sections: &mut HashMap<String, HashMap<String, String>>,
) {
    if let Some((key, value)) = parse_key_value(line) {
        if let Some(section) = sections.get_mut(section_name) {
            section.insert(key, value);
        }
    }
}

/// Helper function to parse TOML content into sections (mimics backend parsing)
fn parse_toml_to_sections(toml_content: &str) -> HashMap<String, HashMap<String, String>> {
    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_section: Option<String> = None;

    for line in toml_content.lines() {
        let line = line.trim();

        if should_skip_line(line) {
            continue;
        }

        // Try to parse as section header
        if let Some(section_name) = process_section_header(line, &mut sections) {
            current_section = Some(section_name);
            continue;
        }

        // Otherwise, try to parse as key=value
        if let Some(ref section_name) = current_section {
            process_key_value(line, section_name, &mut sections);
        }
    }

    sections
}

/// Helper function to create test configurations
fn create_dual_config_toml() -> &'static str {
    r#"
# User-facing conversation path
[user_llm]
provider = "lmstudio"
model = "openai/gpt-oss-20b"
temperature = 0.6
max_tokens = 1000

# Background NLP analysis path  
[nlp_llm]
provider = "lmstudio"
model = "openai/gpt-oss-20b"
temperature = 0.1
max_tokens = 500
"#
}

fn create_mixed_providers_toml() -> &'static str {
    r#"
[user_llm]
provider = "openai"
model = "gpt-4"
api_key = "test-key"
temperature = 0.8

[nlp_llm]
provider = "lmstudio"
model = "llama-3-8b"
temperature = 0.0
"#
}

#[cfg(test)]
mod dual_configuration_tests {
    use super::*;

    #[test]
    fn test_complete_dual_configuration_parsing() {
        // Test verifies dual configuration parsing with separate user and NLP sections

        // Arrange: Complete dual configuration
        let test_config = create_dual_config_toml();
        let sections = parse_toml_to_sections(test_config);

        // Act: Parse dual configuration from sections
        let dual_config = DualLLMConfig::from_sections(
            sections
                .get("user_llm")
                .expect("user_llm section should exist"),
            sections
                .get("nlp_llm")
                .expect("nlp_llm section should exist"),
        )
        .expect("Should successfully parse complete dual configuration");

        let user_config = dual_config.get_config(LLMPath::User);
        let nlp_config = dual_config.get_config(LLMPath::Nlp);

        // Assert: User configuration matches specification
        assert_eq!(
            user_config.provider.provider_name(),
            "lmstudio",
            "User LLM should use specified provider"
        );
        assert_eq!(
            user_config.provider.default_model(),
            "openai/gpt-oss-20b",
            "User LLM should use specified model"
        );
        assert_eq!(
            user_config.default_params.temperature, 0.6,
            "User LLM should use creative temperature"
        );
        assert_eq!(
            user_config.default_params.max_tokens, 1000,
            "User LLM should allow longer responses"
        );

        // Assert: NLP configuration matches specification
        assert_eq!(
            nlp_config.provider.provider_name(),
            "lmstudio",
            "NLP LLM should use specified provider"
        );
        assert_eq!(
            nlp_config.provider.default_model(),
            "openai/gpt-oss-20b",
            "NLP LLM should use specified model"
        );
        assert_eq!(
            nlp_config.default_params.temperature, 0.1,
            "NLP LLM should use analytical temperature"
        );
        assert_eq!(
            nlp_config.default_params.max_tokens, 500,
            "NLP LLM should use shorter responses for analysis"
        );

        // Assert: Validation passes for complete configuration
        dual_config
            .validate()
            .expect("Complete dual configuration should validate successfully");
    }

    #[test]
    fn test_cross_provider_configuration() {
        // Test verifies different providers can be configured for different paths

        // Arrange: Mixed provider configuration
        let mixed_providers_config = create_mixed_providers_toml();
        let sections = parse_toml_to_sections(mixed_providers_config);

        // Act: Parse cross-provider configuration
        let dual_config = DualLLMConfig::from_sections(
            sections
                .get("user_llm")
                .expect("user_llm section should exist"),
            sections
                .get("nlp_llm")
                .expect("nlp_llm section should exist"),
        )
        .expect("Should parse mixed provider configuration");

        let user_config = dual_config.get_config(LLMPath::User);
        let nlp_config = dual_config.get_config(LLMPath::Nlp);

        // Assert: User path configured for premium provider
        assert_eq!(
            user_config.provider.provider_name(),
            "openai",
            "User path should use premium OpenAI provider"
        );
        assert_eq!(
            user_config.provider.default_model(),
            "gpt-4",
            "User path should use premium GPT-4 model"
        );

        // Assert: NLP path configured for local provider
        assert_eq!(
            nlp_config.provider.provider_name(),
            "lmstudio",
            "NLP path should use local LMStudio provider for cost optimization"
        );
        assert_eq!(
            nlp_config.provider.default_model(),
            "llama-3-8b",
            "NLP path should use efficient local model"
        );
    }

    #[test]
    fn test_incomplete_configuration_error_handling() {
        // Test verifies proper error handling when nlp_llm section is missing

        // Arrange: Configuration with only user_llm section
        let incomplete_config = r#"
[user_llm]
provider = "lmstudio"
model = "openai/gpt-oss-20b"
"#;
        let sections = parse_toml_to_sections(incomplete_config);

        // Act: Attempt to parse without nlp_llm section
        let user_section = sections
            .get("user_llm")
            .expect("user_llm section should exist");
        let empty_section = HashMap::new();

        let result = DualLLMConfig::from_sections(user_section, &empty_section);

        // Assert: Should fail with descriptive error
        assert!(
            result.is_err(),
            "Incomplete dual configuration should fail parsing"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing 'provider' field"),
            "Error message should specify missing provider field in nlp section"
        );
    }

    #[test]
    fn test_empty_configuration_error_handling() {
        // Test verifies error handling when both sections are empty

        // Arrange: Empty configuration sections
        let empty_section = HashMap::new();

        // Act: Attempt to parse empty configuration
        let result = DualLLMConfig::from_sections(&empty_section, &empty_section);

        // Assert: Should fail with descriptive error
        assert!(
            result.is_err(),
            "Empty LLM configuration should fail parsing"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing 'provider' field"),
            "Error message should indicate missing provider field"
        );
    }
}

#[cfg(test)]
mod configuration_parameter_tests {
    use super::*;

    #[test]
    fn test_parameter_customization_parsing() {
        // Test verifies all supported parameters are correctly parsed

        // Arrange: Configuration with all supported parameters
        let full_config = r#"
[user_llm]
provider = "openai"
model = "gpt-4"
api_key = "user-key"
temperature = 0.9
max_tokens = 1500
top_p = 0.95
presence_penalty = 0.2

[nlp_llm]
provider = "lmstudio"
model = "llama-3-8b"
temperature = 0.0
max_tokens = 300
top_p = 0.5
presence_penalty = 0.0
"#;

        // Act: Parse full parameter configuration
        let sections = parse_toml_to_sections(full_config);
        let dual_config = DualLLMConfig::from_sections(
            sections
                .get("user_llm")
                .expect("user_llm section should exist"),
            sections
                .get("nlp_llm")
                .expect("nlp_llm section should exist"),
        )
        .expect("Should parse configuration with all parameters");

        let user_config = dual_config.get_config(LLMPath::User);
        let nlp_config = dual_config.get_config(LLMPath::Nlp);

        // Assert: User configuration parameters
        assert_eq!(user_config.default_params.temperature, 0.9);
        assert_eq!(user_config.default_params.max_tokens, 1500);
        assert_eq!(user_config.default_params.top_p, 0.95);
        assert_eq!(user_config.default_params.presence_penalty, 0.2);

        // Assert: NLP configuration parameters
        assert_eq!(nlp_config.default_params.temperature, 0.0);
        assert_eq!(nlp_config.default_params.max_tokens, 300);
        assert_eq!(nlp_config.default_params.top_p, 0.5);
        assert_eq!(nlp_config.default_params.presence_penalty, 0.0);
    }

    #[test]
    fn test_missing_provider_error() {
        // Test verifies error when required provider field is missing

        // Arrange: Configuration missing provider field
        let invalid_config = r#"
[user_llm]
model = "gpt-4"
temperature = 0.7

[nlp_llm]  
model = "llama-3-8b"
temperature = 0.1
"#;

        // Act: Attempt to parse configuration missing provider
        let sections = parse_toml_to_sections(invalid_config);
        let result = DualLLMConfig::from_sections(
            sections.get("user_llm").unwrap_or(&HashMap::new()),
            sections.get("nlp_llm").unwrap_or(&HashMap::new()),
        );

        // Assert: Should fail with provider error
        assert!(
            result.is_err(),
            "Configuration missing provider should fail"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing 'provider' field"),
            "Error should indicate missing provider field"
        );
    }
}
