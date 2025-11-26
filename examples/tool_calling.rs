//! Complete tool calling example demonstrating function calling with LLMs.
//!
//! This example shows how to:
//! - Define tools with JSON Schema parameters
//! - Send requests with tools attached
//! - Handle tool calls from the LLM response
//! - Execute tools and return results
//! - Continue the conversation with tool results
//!
//! # Running
//!
//! ```bash
//! # With Anthropic (default)
//! export ANTHROPIC_API_KEY="sk-ant-..."
//! cargo run --example tool_calling
//!
//! # With OpenAI
//! export AI_PROVIDER=openai
//! export OPENAI_API_KEY="sk-..."
//! cargo run --example tool_calling
//! ```
//!
//! # Tool Calling Flow
//!
//! 1. User asks a question that requires tool use
//! 2. LLM returns tool calls instead of a direct answer
//! 3. Your application executes the requested tools
//! 4. Tool results are sent back to the LLM
//! 5. LLM uses the results to form a final response

use multi_llm::{
    unwrap_response, AnthropicConfig, DefaultLLMParams, LLMConfig, LlmProvider, OpenAIConfig,
    RequestConfig, Tool, ToolCallingRound, ToolChoice, ToolResult, UnifiedLLMClient,
    UnifiedLLMRequest, UnifiedMessage,
};

/// Simulated weather data for our mock weather service
fn get_weather(city: &str, units: &str) -> String {
    // In a real application, this would call an actual weather API
    let temp = match city.to_lowercase().as_str() {
        "london" => 18,
        "paris" => 22,
        "tokyo" => 28,
        "new york" => 25,
        _ => 20,
    };

    let temp_str = if units == "fahrenheit" {
        format!("{}°F", temp * 9 / 5 + 32)
    } else {
        format!("{}°C", temp)
    };

    format!(
        "Weather in {}: Partly cloudy, {}. Humidity: 65%",
        city, temp_str
    )
}

/// Define our weather tool with JSON Schema parameters
fn create_weather_tool() -> Tool {
    Tool {
        name: "get_weather".to_string(),
        description: "Get the current weather for a city. Use this when the user asks about weather conditions.".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "city": {
                    "type": "string",
                    "description": "The city name to get weather for (e.g., 'London', 'Paris', 'Tokyo')"
                },
                "units": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "Temperature units (default: celsius)"
                }
            },
            "required": ["city"]
        }),
    }
}

/// Execute a tool call and return the result
fn execute_tool(name: &str, arguments: &serde_json::Value) -> Result<String, String> {
    match name {
        "get_weather" => {
            let city = arguments["city"]
                .as_str()
                .ok_or("Missing 'city' argument")?;
            let units = arguments["units"].as_str().unwrap_or("celsius");
            Ok(get_weather(city, units))
        }
        _ => Err(format!("Unknown tool: {}", name)),
    }
}

fn create_config() -> anyhow::Result<(LLMConfig, &'static str)> {
    let provider = std::env::var("AI_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());

    match provider.as_str() {
        "openai" => {
            let api_key = std::env::var("OPENAI_API_KEY")
                .expect("OPENAI_API_KEY environment variable must be set");
            Ok((
                LLMConfig {
                    provider: Box::new(OpenAIConfig {
                        api_key: Some(api_key),
                        base_url: "https://api.openai.com".to_string(),
                        default_model: "gpt-4o-mini".to_string(),
                        max_context_tokens: 128_000,
                        retry_policy: Default::default(),
                    }),
                    default_params: DefaultLLMParams::default(),
                },
                "OpenAI",
            ))
        }
        _ => {
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .expect("ANTHROPIC_API_KEY environment variable must be set");
            Ok((
                LLMConfig {
                    provider: Box::new(AnthropicConfig {
                        api_key: Some(api_key),
                        base_url: "https://api.anthropic.com".to_string(),
                        default_model: "claude-sonnet-4-20250514".to_string(),
                        max_context_tokens: 200_000,
                        retry_policy: Default::default(),
                        enable_prompt_caching: false,
                        cache_ttl: "5m".to_string(),
                    }),
                    default_params: DefaultLLMParams::default(),
                },
                "Anthropic",
            ))
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create configuration based on AI_PROVIDER env var (default: anthropic)
    let (config, provider_name) = create_config()?;

    // Create the client
    let client = UnifiedLLMClient::from_config(config)?;

    // Define our tools
    let weather_tool = create_weather_tool();

    // Build request configuration with tools
    let request_config = RequestConfig {
        tools: vec![weather_tool],
        tool_choice: Some(ToolChoice::Auto), // Let the model decide when to use tools
        ..Default::default()
    };

    // Create initial conversation
    let messages = vec![
        UnifiedMessage::system("You are a helpful weather assistant. Use the get_weather tool to answer questions about weather."),
        UnifiedMessage::user("What's the weather like in Paris and Tokyo?"),
    ];

    let request = UnifiedLLMRequest::new(messages.clone());

    println!("User: What's the weather like in Paris and Tokyo?\n");
    println!("Sending request to {} with tools...\n", provider_name);

    // First request - LLM may return tool calls
    // The unwrap_response! macro handles both with/without "events" feature
    let response = unwrap_response!(
        client
            .execute_llm(request, None, Some(request_config.clone()))
            .await?
    );

    // Check if the model wants to call tools
    if !response.tool_calls.is_empty() {
        println!(
            "LLM requested {} tool call(s):\n",
            response.tool_calls.len()
        );

        // Execute each tool call and collect results
        let mut tool_results = Vec::new();

        for tool_call in &response.tool_calls {
            println!("  Tool: {}", tool_call.name);
            println!("  Arguments: {}", tool_call.arguments);

            // Execute the tool
            let result = match execute_tool(&tool_call.name, &tool_call.arguments) {
                Ok(content) => {
                    println!("  Result: {}\n", content);
                    ToolResult {
                        tool_call_id: tool_call.id.clone(),
                        content,
                        is_error: false,
                        error_category: None,
                    }
                }
                Err(error) => {
                    println!("  Error: {}\n", error);
                    ToolResult {
                        tool_call_id: tool_call.id.clone(),
                        content: error,
                        is_error: true,
                        error_category: None,
                    }
                }
            };

            tool_results.push(result);
        }

        // Create the assistant message with tool calls for the next round
        // We need to create a message that represents what the assistant said
        let assistant_message = if response.tool_calls.len() == 1 {
            UnifiedMessage::tool_call(
                response.tool_calls[0].id.clone(),
                response.tool_calls[0].name.clone(),
                response.tool_calls[0].arguments.clone(),
            )
        } else {
            // For multiple tool calls, we use the first one (simplified)
            // In a real app, you'd handle multiple tool calls properly
            UnifiedMessage::tool_call(
                response.tool_calls[0].id.clone(),
                response.tool_calls[0].name.clone(),
                response.tool_calls[0].arguments.clone(),
            )
        };

        // Create tool calling round with the assistant's tool requests and our results
        let tool_round = ToolCallingRound {
            assistant_message,
            tool_results,
        };

        // Send follow-up request with tool results
        let follow_up_request = UnifiedLLMRequest::new(messages);

        println!("Sending tool results back to LLM...\n");

        let final_response = unwrap_response!(
            client
                .execute_llm(follow_up_request, Some(tool_round), Some(request_config))
                .await?
        );

        println!("Assistant: {}", final_response.content);

        // Print token usage if available
        if let Some(usage) = &final_response.usage {
            println!(
                "\nToken usage: {} input + {} output = {} total",
                usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
            );
        }
    } else {
        // Model responded directly without tool calls
        println!("Assistant: {}", response.content);
    }

    Ok(())
}
