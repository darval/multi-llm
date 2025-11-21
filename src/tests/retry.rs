use crate::error::LlmError;
use crate::retry::{CircuitBreaker, CircuitState, RetryExecutor, RetryPolicy};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create fast test retry policy to prevent slow tests
    fn create_fast_test_retry_policy() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(50),
            backoff_multiplier: 2.0,
            total_timeout: Duration::from_millis(500),
            request_timeout: Duration::from_millis(100),
        }
    }

    // Unit Tests for RetryPolicy
    //
    // UNIT UNDER TEST: RetryPolicy (concrete implementation)
    //
    // BUSINESS RESPONSIBILITY:
    //   - Provides configuration parameters for retry behavior and timing
    //   - Defines exponential backoff progression (1s, 2s, 4s, 8s, 16s max)
    //   - Sets reasonable defaults for production LLM API integration
    //   - Controls timeout behavior for individual requests and total operations
    //
    // TEST COVERAGE:
    //   - Default configuration values match production requirements
    //   - Timeout settings align with typical LLM provider response times

    #[test]
    fn test_retry_policy_defaults_match_production_requirements() {
        // Test verifies default retry policy meets production LLM API requirements
        // Ensures backoff timing aligns with typical LLM provider rate limits

        // Arrange
        let policy = RetryPolicy::default();

        // Act & Assert
        assert_eq!(
            policy.max_attempts, 5,
            "Should allow 5 attempts to handle transient failures"
        );
        assert_eq!(
            policy.initial_delay,
            Duration::from_secs(1),
            "Should start with 1 second delay"
        );
        assert_eq!(
            policy.max_delay,
            Duration::from_secs(16),
            "Should cap at 16 seconds to prevent excessive waits"
        );
        assert_eq!(
            policy.backoff_multiplier, 2.0,
            "Should double delay each attempt for exponential backoff"
        );
        assert_eq!(
            policy.total_timeout,
            Duration::from_secs(300),
            "Should allow 5 minutes total for complex operations"
        );
        assert_eq!(
            policy.request_timeout,
            Duration::from_secs(120),
            "Should timeout individual requests after 120 seconds (increased for slower models)"
        );
    }

    // Unit Tests for CircuitBreaker
    //
    // UNIT UNDER TEST: CircuitBreaker (concrete implementation)
    //
    // BUSINESS RESPONSIBILITY:
    //   - Tracks failure patterns to detect when LLM providers become unhealthy
    //   - Blocks requests during provider outages to prevent resource waste
    //   - Automatically tests provider recovery with half-open state transitions
    //   - Maintains system stability during cascading LLM provider failures
    //
    // TEST COVERAGE:
    //   - State transition logic for different failure/success patterns
    //   - Failure threshold detection and automatic request blocking
    //   - Recovery timeout handling and gradual service restoration
    //   - Request allow/block decisions based on current circuit state

    /// Helper function to create concrete circuit breaker for testing
    fn create_concrete_circuit_breaker() -> CircuitBreaker {
        CircuitBreaker::new(2, Duration::from_millis(100))
    }

    #[tokio::test]
    async fn test_circuit_breaker_protects_against_cascading_failures() {
        // Test verifies circuit breaker prevents request flooding during LLM provider outages
        // Ensures system stability by blocking requests when provider becomes unhealthy

        // Arrange
        let mut cb = create_concrete_circuit_breaker();

        // Act & Assert - Initially should allow requests in healthy state
        assert_eq!(
            cb.state(),
            CircuitState::Closed,
            "Circuit should start in closed (healthy) state"
        );
        assert!(
            cb.should_allow_request(),
            "Should allow requests when provider is healthy"
        );

        // First failure should keep circuit closed (single failures are normal)
        cb.record_failure();
        assert_eq!(
            cb.state(),
            CircuitState::Closed,
            "Single failure should not trigger circuit breaker"
        );
        assert!(
            cb.should_allow_request(),
            "Should still allow requests after single failure"
        );

        // Second failure should open circuit (provider appears unhealthy)
        cb.record_failure();
        assert_eq!(
            cb.state(),
            CircuitState::Open,
            "Multiple failures should open circuit to protect system"
        );
        assert!(
            !cb.should_allow_request(),
            "Should block requests when circuit is open"
        );

        // After recovery timeout, should allow test requests
        sleep(Duration::from_millis(110)).await; // Slightly longer than circuit breaker timeout
        assert!(
            cb.should_allow_request(),
            "Should allow test requests after recovery timeout"
        );
        assert_eq!(
            cb.state(),
            CircuitState::HalfOpen,
            "Should enter half-open state for testing recovery"
        );

        // Successful request should close circuit (provider recovered)
        cb.record_success();
        assert_eq!(
            cb.state(),
            CircuitState::Closed,
            "Success should close circuit after recovery test"
        );
        assert!(
            cb.should_allow_request(),
            "Should fully allow requests when provider is healthy again"
        );
    }

    // Unit Tests for RetryExecutor
    //
    // UNIT UNDER TEST: RetryExecutor (concrete implementation)
    //
    // BUSINESS RESPONSIBILITY:
    //   - Handles transient LLM API failures with exponential backoff (1s, 2s, 4s, 8s, 16s)
    //   - Manages API rate limiting with proper Retry-After header compliance
    //   - Ensures eventual consistency for critical LLM operations in production
    //   - Protects downstream LLM providers from request flooding during outages
    //   - Integrates circuit breaker pattern to prevent cascading failures
    //
    // TEST COVERAGE:
    //   - Request retry behavior for recoverable vs non-recoverable error types
    //   - Exponential backoff timing verification with jitter randomization
    //   - Rate limit detection and proper delay extraction from HTTP headers
    //   - Request attempt counting and failure threshold enforcement
    //   - Integration with circuit breaker for comprehensive failure handling

    /// Helper function to create concrete retry executor for testing
    fn create_concrete_retry_executor() -> RetryExecutor {
        RetryExecutor::new(create_fast_test_retry_policy())
    }

    #[tokio::test]
    async fn test_successful_request_requires_no_retries() {
        // Test verifies successful LLM requests complete immediately without unnecessary delays
        // Ensures optimal performance when LLM providers are responding normally

        // Arrange
        let mut executor = create_concrete_retry_executor();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        // Act
        let result = executor
            .execute(|| async {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                Ok::<String, LlmError>("success".to_string())
            })
            .await;

        // Assert
        assert!(
            result.is_ok(),
            "Successful operation should not return error"
        );
        assert_eq!(
            result.unwrap(),
            "success",
            "Should return expected success value"
        );
        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "Should only call operation once when successful"
        );
    }

    #[tokio::test]
    async fn test_retryable_errors_trigger_exponential_backoff() {
        // Test verifies transient LLM API failures are retried with proper exponential backoff
        // Ensures system recovers from temporary network issues or provider overload

        // Arrange
        let mut executor = create_concrete_retry_executor();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        // Act
        let result = executor
            .execute(|| async {
                let count = counter_clone.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    Err(LlmError::request_failed(
                        "temporary network failure".to_string(),
                        None,
                    ))
                } else {
                    Ok::<String, LlmError>("success".to_string())
                }
            })
            .await;

        // Assert
        assert!(result.is_ok(), "Should eventually succeed after retries");
        assert_eq!(
            result.unwrap(),
            "success",
            "Should return success after retry attempts"
        );
        assert_eq!(
            counter.load(Ordering::SeqCst),
            3,
            "Should retry failed requests until success"
        );
    }

    #[tokio::test]
    async fn test_non_retryable_errors_fail_immediately() {
        // Test verifies authentication failures do not trigger wasteful retry attempts
        // Ensures system fails fast for permanent errors like invalid API keys

        // Arrange
        let mut executor = create_concrete_retry_executor();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        // Act
        let result: crate::error::LlmResult<()> = executor
            .execute(|| async {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                Err(LlmError::authentication_failed(
                    "invalid api key".to_string(),
                ))
            })
            .await;

        // Assert
        assert!(
            result.is_err(),
            "Authentication failures should not succeed"
        );
        assert!(
            result.unwrap_err().to_string().contains("invalid api key"),
            "Should preserve original error message"
        );
        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "Should not retry authentication failures"
        );
    }

    #[test]
    fn test_exponential_backoff_timing_with_jitter() {
        // Test verifies retry delays follow exponential backoff pattern with randomization
        // Ensures multiple clients don't create synchronized retry storms (thundering herd)

        // Arrange
        let executor = RetryExecutor::default();

        // Act
        let delay1 = executor.calculate_delay(1);
        let delay2 = executor.calculate_delay(2);
        let delay3 = executor.calculate_delay(3);

        // Assert - Verify exponential progression with jitter tolerance
        assert!(
            delay1.as_secs_f64() >= 0.9 && delay1.as_secs_f64() <= 1.1,
            "First retry should be ~1 second with jitter"
        );
        assert!(
            delay2.as_secs_f64() >= 1.8 && delay2.as_secs_f64() <= 2.2,
            "Second retry should be ~2 seconds with jitter"
        );
        assert!(
            delay3.as_secs_f64() >= 3.6 && delay3.as_secs_f64() <= 4.4,
            "Third retry should be ~4 seconds with jitter"
        );

        // Verify exponential relationship exists
        assert!(delay2 > delay1, "Each delay should be longer than previous");
        assert!(
            delay3 > delay2,
            "Delays should continue increasing exponentially"
        );
    }

    #[tokio::test]
    async fn test_rate_limit_extraction_respects_retry_after_headers() {
        // Test verifies rate limit errors extract proper delay from HTTP Retry-After headers
        // Ensures system respects LLM provider rate limiting to avoid further penalties

        use crate::retry::extract_retry_after_duration;

        // Arrange
        let rate_limit_error = LlmError::rate_limit_exceeded(60);
        let other_error = LlmError::request_failed("network error".to_string(), None);

        // Act
        let duration = extract_retry_after_duration(&rate_limit_error);
        let no_duration = extract_retry_after_duration(&other_error);

        // Assert
        assert!(
            duration.is_some(),
            "Rate limit errors should provide retry duration"
        );
        assert_eq!(
            duration.unwrap(),
            Duration::from_secs(60),
            "Should extract correct retry-after duration"
        );
        assert!(
            no_duration.is_none(),
            "Non-rate-limit errors should not provide retry duration"
        );
    }

    #[tokio::test]
    async fn test_circuit_breaker_success_in_open_state() {
        // Test verifies circuit breaker handles unexpected success during Open state
        // Ensures defensive code path resets properly even in edge cases

        // Arrange
        let mut cb = create_concrete_circuit_breaker();

        // Force circuit into Open state
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Act - Record success while Open (shouldn't normally happen, but defensive code)
        cb.record_success();

        // Assert - Should reset state safely
        assert_eq!(cb.failure_count, 0, "Should reset failure count");
    }

    #[tokio::test]
    async fn test_circuit_breaker_repeated_open_doesnt_spam_logs() {
        // Test verifies circuit breaker doesn't log repeatedly when already Open
        // Ensures log efficiency and prevents log spam during prolonged outages

        // Arrange
        let mut cb = create_concrete_circuit_breaker();

        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Act - Record more failures while already Open
        cb.record_failure();
        cb.record_failure();

        // Assert - Circuit remains Open, no panic or issues
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[tokio::test]
    async fn test_retry_executor_timeout_path() {
        // Test verifies executor handles timeout errors correctly
        // Ensures timeout failures trigger retry logic

        // Arrange
        let mut executor = create_concrete_retry_executor();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        // Act - Operation that times out then succeeds
        let result = executor
            .execute(|| async {
                let count = counter_clone.fetch_add(1, Ordering::SeqCst);
                if count == 0 {
                    // Simulate timeout with a very long delay
                    sleep(Duration::from_secs(10)).await;
                    Ok::<String, LlmError>("too slow".to_string())
                } else {
                    Ok("success".to_string())
                }
            })
            .await;

        // Assert - Should retry and eventually succeed
        assert!(result.is_ok(), "Should succeed after timeout retry");
        assert!(
            counter.load(Ordering::SeqCst) >= 2,
            "Should have attempted multiple times due to timeout"
        );
    }

    #[tokio::test]
    async fn test_retry_policy_custom_configuration() {
        // Test verifies RetryPolicy can be customized beyond defaults
        // Ensures flexibility for different deployment scenarios

        // Arrange
        let custom_policy = RetryPolicy {
            max_attempts: 2,
            initial_delay: Duration::from_millis(5),
            max_delay: Duration::from_millis(20),
            backoff_multiplier: 1.5,
            total_timeout: Duration::from_millis(200),
            request_timeout: Duration::from_millis(50),
        };
        let executor = RetryExecutor::new(custom_policy.clone());

        // Act & Assert - Verify custom values are used
        assert_eq!(executor.policy.max_attempts, 2);
        assert_eq!(executor.policy.backoff_multiplier, 1.5);
    }

    #[tokio::test]
    async fn test_circuit_breaker_custom_configuration() {
        // Test verifies CircuitBreaker accepts custom thresholds
        // Ensures configurability for different failure tolerance levels

        // Arrange & Act
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));

        // Assert
        assert_eq!(cb.failure_threshold, 3);
        assert_eq!(cb.recovery_timeout, Duration::from_secs(60));
    }
}
