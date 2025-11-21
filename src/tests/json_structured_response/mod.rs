//! JSON Structured Response Test Suite
//!
//! Split into modules to maintain manageable file sizes
//! (following the <600 LOC requirement from module-refactoring-template.md).
//!
//! - `helpers` - Common test utilities and data builders (108 LOC)
//! - `structured_response_tests` - LLMResponse and schema validation (105 LOC)
//! - `openai_response_format_tests` - OpenAI format conversion (87 LOC)
//! - `config_application_tests` - Configuration application logic (112 LOC)
//! - `json_parsing_tests` - JSON parsing and validation (96 LOC)
//! - `edge_case_tests` - Edge cases and error conditions (59 LOC)
//! - `structured_response_trait_compliance_tests` - Cross-provider compliance (108 LOC)

pub mod config_application_tests;
pub mod edge_case_tests;
pub mod helpers;
pub mod json_parsing_tests;
pub mod openai_response_format_tests;
pub mod structured_response_tests;
pub mod structured_response_trait_compliance_tests;
