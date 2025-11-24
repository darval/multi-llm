//! Provider trait and types for LLM abstraction
//!
//! Defines the `LlmProvider` trait that all providers implement, along with
//! request/response types, tool definitions, and configuration.

use crate::core_types::errors::UserErrorCategory;
#[cfg(feature = "events")]
use crate::core_types::events::{BusinessEvent, EventScope};
use crate::core_types::messages::{UnifiedLLMRequest, UnifiedMessage};
use crate::core_types::Result;
use serde::{Deserialize, Serialize};

/// Tool definition for LLM operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tool {
    /// Tool name - must be unique within a request
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON Schema defining the tool's input parameters
    pub parameters: serde_json::Value,
}

/// Tool call from LLM response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the tool to call
    pub name: String,
    /// Arguments to pass to the tool (as JSON)
    pub arguments: serde_json::Value,
}

/// Tool execution result to send back to LLM
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResult {
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
pub struct RequestConfig {
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
    pub response_format: Option<ResponseFormat>,

    // Tool-specific configuration
    /// Tools available for this request
    pub tools: Vec<Tool>,
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
pub struct ResponseFormat {
    /// Name of the JSON schema
    pub name: String,
    /// JSON schema specification
    pub schema: serde_json::Value,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenUsage {
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,
    /// Number of tokens in the completion
    pub completion_tokens: u32,
    /// Total tokens used (prompt + completion)
    pub total_tokens: u32,
}

/// Response from LLM operations
#[derive(Debug, Clone)]
pub struct Response {
    /// Primary text content of the response
    pub content: String,
    /// Structured JSON response (if response_format was specified)
    pub structured_response: Option<serde_json::Value>,
    /// Tool calls made by the LLM (if any)
    pub tool_calls: Vec<ToolCall>,
    /// Token usage information
    pub usage: Option<TokenUsage>,
    /// Model that generated the response
    pub model: Option<String>,
    /// Raw response body for debugging
    pub raw_body: Option<String>,
}

/// Business event generated during LLM operations
///
/// Only available with the `events` feature enabled.
#[cfg(feature = "events")]
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
    pub tool_results: Vec<ToolResult>,
}

/// Trait for LLM providers to implement
///
/// This trait defines the contract between multi-llm and LLM providers.
/// Phase 2 should consider if this trait belongs in multi-llm rather than being extracted.
///
/// # Return Types
/// - With `events` feature: Returns `(Response, Vec<LLMBusinessEvent>)`
/// - Without `events` feature: Returns `Response`
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    /// Execute LLM operation with unified context
    ///
    /// # Returns
    /// - With `events` feature: `Result<(Response, Vec<LLMBusinessEvent>)>`
    /// - Without `events` feature: `Result<Response>`
    #[cfg(feature = "events")]
    async fn execute_llm(
        &self,
        request: UnifiedLLMRequest,
        current_tool_round: Option<ToolCallingRound>,
        config: Option<RequestConfig>,
    ) -> Result<(Response, Vec<LLMBusinessEvent>)>;

    /// Execute LLM operation with unified context
    ///
    /// # Returns
    /// - With `events` feature: `Result<(Response, Vec<LLMBusinessEvent>)>`
    /// - Without `events` feature: `Result<Response>`
    #[cfg(not(feature = "events"))]
    async fn execute_llm(
        &self,
        request: UnifiedLLMRequest,
        current_tool_round: Option<ToolCallingRound>,
        config: Option<RequestConfig>,
    ) -> Result<Response>;

    /// Execute structured LLM operation with unified context
    ///
    /// Returns Response with structured_response field populated.
    ///
    /// # Returns
    /// - With `events` feature: `Result<(Response, Vec<LLMBusinessEvent>)>`
    /// - Without `events` feature: `Result<Response>`
    #[cfg(feature = "events")]
    async fn execute_structured_llm(
        &self,
        request: UnifiedLLMRequest,
        current_tool_round: Option<ToolCallingRound>,
        schema: serde_json::Value,
        config: Option<RequestConfig>,
    ) -> Result<(Response, Vec<LLMBusinessEvent>)>;

    /// Execute structured LLM operation with unified context
    ///
    /// Returns Response with structured_response field populated.
    ///
    /// # Returns
    /// - With `events` feature: `Result<(Response, Vec<LLMBusinessEvent>)>`
    /// - Without `events` feature: `Result<Response>`
    #[cfg(not(feature = "events"))]
    async fn execute_structured_llm(
        &self,
        request: UnifiedLLMRequest,
        current_tool_round: Option<ToolCallingRound>,
        schema: serde_json::Value,
        config: Option<RequestConfig>,
    ) -> Result<Response>;

    /// Get provider name for logging and debugging
    fn provider_name(&self) -> &'static str;
}

/// Type aliases for backward compatibility
pub type LLMRequestConfig = RequestConfig;
pub type LLMResponseFormat = ResponseFormat;
pub type LLMTokenUsage = TokenUsage;
