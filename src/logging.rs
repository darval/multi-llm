//! Logging utilities for multi-llm
//!
//! This module abstracts the logging implementation by re-exporting tracing macros
//! with `log_*` naming convention. This approach provides several benefits:
//!
//! - **Implementation Independence**: Library code uses `log_debug!`, `log_info!`, etc.
//!   instead of directly calling `tracing::debug!`. If we ever need to switch logging
//!   backends, we only change this module.
//!
//! - **Consistency**: All logging uses the same macros throughout the codebase,
//!   making it easy to audit and maintain.
//!
//! - **Library Best Practice**: Avoids `println!`/`eprintln!` in library code, letting
//!   consumers decide how to handle log output.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::logging::{log_debug, log_info, log_warn, log_error};
//!
//! log_debug!(provider = "openai", "Processing request");
//! log_warn!(error = %e, "Recoverable error occurred");
//! log_error!("Critical failure: {}", message);
//! ```
//!
//! # Guidelines
//!
//! - Use `log_debug!` for detailed diagnostic information
//! - Use `log_info!` for notable events during normal operation
//! - Use `log_warn!` for recoverable problems or unexpected conditions
//! - Use `log_error!` for failures that affect functionality
//! - Never use `println!`/`eprintln!` in library code (examples and tests excepted)

// Re-export tracing macros with log_* naming
// Allow unused - these are available for internal use as needed
#[allow(unused_imports)]
pub use tracing::{
    debug as log_debug, error as log_error, info as log_info, trace as log_trace, warn as log_warn,
};
