//! Shared OpenAI-compatible API structures and utilities
//!
//! This module contains the common request/response structures used by
//! OpenAI-compatible providers like OpenAI itself and LM Studio.
//!
//! Split into modules to maintain manageable file sizes:
//! - `types` - Core data structures and types (85 LOC)
//! - `utils` - Configuration and conversion utilities (remaining functions)

pub mod types;
pub mod utils;

#[cfg(test)]
mod tests;

// Re-export main types for backward compatibility
pub use types::*;
pub use utils::*;
