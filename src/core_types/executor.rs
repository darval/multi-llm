//! Executor types for LLM operations (extracted from mystory-core)
//!
//! These types define the contract between the executor and LLM providers.
//! Phase 2 will review ownership - this trait should probably live in multi-llm.

use crate::core_types::errors::UserErrorCategory;
use crate::core_types::events::{BusinessEvent, EventScope};
use crate::core_types::messages::{UnifiedLLMRequest, UnifiedMessage};
use crate::core_types::Result;
use serde::{Deserialize, Serialize};

/// Tool definition for LLM operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutorTool {
    /// Tool name - must be unique within a request
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON Schema defining the tool's input parameters
    pub parameters: serde_json::Value,
}

/// Tool call from LLM response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutorToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the tool to call
    pub name: String,
    /// Arguments to pass to the tool (as JSON)
    pub arguments: serde_json::Value,
}

/// Tool execution result to send back to LLM
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutorToolResult {
    /// ID of the tool call this is responding to
    pub tool_call_id: String,
    /// Result content from the tool execution
    pub content: String,
    /// Whether this result represents an error
    pub is_error: bool,
    /// Error category for user-facing error handling (if is_error is true)
    pub error_category: Option<UserErrorCategory>,
}

/// Tool choice strategy for LLM operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ToolChoice {
    /// Let the model decide whether and which tools to use
    #[default]
    Auto,
    /// Don't use any tools
    None,
    /// Must use at least one tool
    Required,
    /// Use a specific tool by name
    Specific(String),
}

/// Configuration for LLM operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ExecutorLLMConfig {
    // Standard LLM parameters
    /// Temperature setting for response randomness (0.0 to 1.0)
    pub temperature: Option<f64>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Top-p sampling parameter (0.0 to 1.0)
    pub top_p: Option<f64>,
    /// Top-k sampling parameter
    pub top_k: Option<u32>,
    /// Min-p sampling parameter (0.0 to 1.0)
    pub min_p: Option<f64>,
    /// Presence penalty to discourage repetition
    pub presence_penalty: Option<f64>,
    /// Response format specification (for structured output)
    pub response_format: Option<ExecutorResponseFormat>,

    // Tool-specific configuration
    /// Tools available for this request
    pub tools: Vec<ExecutorTool>,
    /// How the model should handle tool selection
    pub tool_choice: Option<ToolChoice>,

    // Context metadata for logging and analytics
    /// User ID for cache hit analysis
    pub user_id: Option<String>,
    /// Session ID for session-level cache analysis
    pub session_id: Option<String>,
    /// LLM path context for distinguishing call types
    pub llm_path: Option<String>,
}

/// Response format specification for structured output
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutorResponseFormat {
    /// Name of the JSON schema
    pub name: String,
    /// JSON schema specification
    pub schema: serde_json::Value,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutorTokenUsage {
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,
    /// Number of tokens in the completion
    pub completion_tokens: u32,
    /// Total tokens used (prompt + completion)
    pub total_tokens: u32,
}

/// Response from LLM operations
#[derive(Debug, Clone)]
pub struct ExecutorLLMResponse {
    /// Primary text content of the response
    pub content: String,
    /// Structured JSON response (if response_format was specified)
    pub structured_response: Option<serde_json::Value>,
    /// Tool calls made by the LLM (if any)
    pub tool_calls: Vec<ExecutorToolCall>,
    /// Token usage information
    pub usage: Option<ExecutorTokenUsage>,
    /// Model that generated the response
    pub model: Option<String>,
    /// Raw response body for debugging
    pub raw_body: Option<String>,
}

/// Business event generated during LLM operations
#[derive(Debug, Clone)]
pub struct LLMBusinessEvent {
    /// The business event to log
    pub event: BusinessEvent,
    /// Event scope (user or system level)
    pub scope: EventScope,
}

/// Complete tool calling round state for provider message building
#[derive(Debug, Clone)]
pub struct ToolCallingRound {
    /// The complete assistant message that initiated tool calls
    pub assistant_message: UnifiedMessage,
    /// Tool execution results from the current round
    pub tool_results: Vec<ExecutorToolResult>,
}

/// Trait for LLM providers to implement
///
/// This trait defines the contract between multi-llm and LLM providers.
/// Phase 2 should consider if this trait belongs in multi-llm rather than being extracted.
#[async_trait::async_trait]
pub trait ExecutorLLMProvider: Send + Sync {
    /// Execute LLM operation with unified context
    ///
    /// Returns a tuple of (response, events) where events are business events
    /// generated during the LLM operation.
    async fn execute_llm(
        &self,
        request: UnifiedLLMRequest,
        current_tool_round: Option<ToolCallingRound>,
        config: Option<ExecutorLLMConfig>,
    ) -> Result<(ExecutorLLMResponse, Vec<LLMBusinessEvent>)>;

    /// Execute structured LLM operation with unified context
    ///
    /// Returns ExecutorLLMResponse with structured_response field populated.
    async fn execute_structured_llm(
        &self,
        request: UnifiedLLMRequest,
        current_tool_round: Option<ToolCallingRound>,
        schema: serde_json::Value,
        config: Option<ExecutorLLMConfig>,
    ) -> Result<(ExecutorLLMResponse, Vec<LLMBusinessEvent>)>;

    /// Get provider name for logging and debugging
    fn provider_name(&self) -> &'static str;
}

/// Type aliases for backward compatibility
pub type LLMRequestConfig = ExecutorLLMConfig;
pub type LLMResponseFormat = ExecutorResponseFormat;
pub type TokenUsage = ExecutorTokenUsage;
