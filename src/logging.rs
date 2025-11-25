//! Logging utilities for multi-llm
//!
//! Re-exports tracing macros with log_* naming convention for consistency.

// Re-export tracing macros with log_* naming
// Allow unused - these are available for internal use as needed
#[allow(unused_imports)]
pub use tracing::{
    debug as log_debug, error as log_error, info as log_info, trace as log_trace, warn as log_warn,
};
