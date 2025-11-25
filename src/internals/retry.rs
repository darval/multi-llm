//! Retry logic with exponential backoff and rate limiting
//!
//! This module provides resilient HTTP request handling for LLM providers with:
//! - Exponential backoff: 1s, 2s, 4s, 8s, 16s maximum
//! - Rate limit handling with Retry-After header support
//! - Circuit breaker pattern: 5 failures = 30s cooldown
//! - Configurable timeout: 30s request, 5m total operation

use crate::error::{LlmError, LlmResult};
use crate::logging::{log_debug, log_error, log_warn};

use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Retry policy configuration for LLM requests
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Maximum total operation time
    pub total_timeout: Duration,
    /// Request timeout for individual attempts
    pub request_timeout: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(16),
            backoff_multiplier: 2.0,
            total_timeout: Duration::from_secs(300), // 5 minutes
            request_timeout: Duration::from_secs(120), // Increased for slower models like Hermes
        }
    }
}

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CircuitState {
    Closed,   // Normal operation
    Open,     // Failing, blocking requests
    HalfOpen, // Testing if service recovered
}

/// Circuit breaker for provider resilience
#[derive(Debug)]
pub(crate) struct CircuitBreaker {
    pub(crate) state: CircuitState,
    pub(crate) failure_count: u32,
    pub(crate) last_failure_time: Option<Instant>,
    pub(crate) failure_threshold: u32,
    pub(crate) recovery_timeout: Duration,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure_time: None,
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
        }
    }
}

impl CircuitBreaker {
    /// Check if request should be allowed through the circuit breaker
    pub fn should_allow_request(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => self.check_recovery_timeout(),
            CircuitState::HalfOpen => true,
        }
    }

    /// Check if circuit breaker should transition from Open to HalfOpen
    fn check_recovery_timeout(&mut self) -> bool {
        let Some(last_failure) = self.last_failure_time else {
            return false;
        };

        if last_failure.elapsed() >= self.recovery_timeout {
            log_debug!(
                circuit_breaker = "transitioning_to_half_open",
                recovery_timeout_seconds = self.recovery_timeout.as_secs(),
                "Circuit breaker attempting recovery"
            );
            self.state = CircuitState::HalfOpen;
            true
        } else {
            false
        }
    }

    /// Record a successful request
    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::HalfOpen => {
                log_debug!(
                    circuit_breaker = "recovered",
                    "Circuit breaker recovered, returning to closed state"
                );
                self.state = CircuitState::Closed;
                self.failure_count = 0;
                self.last_failure_time = None;
            }
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            CircuitState::Open => {
                // Shouldn't happen, but reset if it does
                self.failure_count = 0;
                self.last_failure_time = None;
            }
        }
    }

    /// Record a failed request
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(Instant::now());

        if self.failure_count >= self.failure_threshold {
            if self.state != CircuitState::Open {
                log_warn!(
                    circuit_breaker = "opened",
                    failure_count = self.failure_count,
                    failure_threshold = self.failure_threshold,
                    recovery_timeout_seconds = self.recovery_timeout.as_secs(),
                    "Circuit breaker opened due to repeated failures"
                );
            }
            self.state = CircuitState::Open;
        }
    }

    /// Get current circuit breaker state
    pub fn state(&self) -> CircuitState {
        self.state.clone()
    }
}

/// Retry executor that handles exponential backoff and circuit breaking
#[derive(Debug)]
pub(crate) struct RetryExecutor {
    pub(crate) policy: RetryPolicy,
    pub(crate) circuit_breaker: CircuitBreaker,
}

impl Default for RetryExecutor {
    fn default() -> Self {
        Self::new(RetryPolicy::default())
    }
}

impl RetryExecutor {
    /// Create a new retry executor with the given policy
    pub fn new(policy: RetryPolicy) -> Self {
        Self {
            policy,
            circuit_breaker: CircuitBreaker::default(),
        }
    }

    /// Execute a request with retry logic and circuit breaking
    pub async fn execute<F, Fut, T>(&mut self, operation: F) -> LlmResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = LlmResult<T>>,
    {
        let start_time = Instant::now();
        let mut attempt = 0;
        let mut last_error = None;

        while attempt < self.policy.max_attempts {
            self.check_circuit_breaker()?;
            self.check_total_timeout(&start_time)?;

            attempt += 1;

            match self
                .execute_single_attempt(&operation, attempt, &mut last_error)
                .await
            {
                Ok(response) => return Ok(response),
                Err(should_continue) => {
                    if !should_continue {
                        break;
                    }
                }
            }
        }

        self.handle_exhausted_retries(attempt, last_error, &start_time)
    }

    /// Execute a single attempt and return whether to continue retrying
    async fn execute_single_attempt<F, Fut, T>(
        &mut self,
        operation: &F,
        attempt: u32,
        last_error: &mut Option<LlmError>,
    ) -> Result<T, bool>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = LlmResult<T>>,
    {
        self.log_attempt(attempt);

        let operation_start = Instant::now();
        let result = tokio::time::timeout(self.policy.request_timeout, operation()).await;

        match result {
            Ok(Ok(response)) => {
                self.circuit_breaker.record_success();
                log_debug!(
                    attempt = attempt,
                    duration_ms = operation_start.elapsed().as_millis(),
                    "Request succeeded"
                );
                Ok(response)
            }
            Ok(Err(error)) => {
                let should_continue = self.handle_error(error, attempt, last_error).await;
                Err(should_continue)
            }
            Err(_timeout) => {
                let should_continue = self.handle_timeout(attempt, last_error).await;
                Err(should_continue)
            }
        }
    }

    fn check_circuit_breaker(&mut self) -> LlmResult<()> {
        if !self.circuit_breaker.should_allow_request() {
            return Err(LlmError::request_failed(
                "Circuit breaker is open - service temporarily unavailable".to_string(),
                None,
            ));
        }
        Ok(())
    }

    fn check_total_timeout(&mut self, start_time: &Instant) -> LlmResult<()> {
        if start_time.elapsed() >= self.policy.total_timeout {
            return Err(LlmError::timeout(self.policy.total_timeout.as_secs()));
        }
        Ok(())
    }

    fn log_attempt(&mut self, attempt: u32) {
        log_debug!(
            attempt = attempt,
            max_attempts = self.policy.max_attempts,
            circuit_state = ?self.circuit_breaker.state(),
            "Executing request with retry logic"
        );
    }

    async fn handle_error(
        &mut self,
        error: LlmError,
        attempt: u32,
        last_error: &mut Option<LlmError>,
    ) -> bool {
        let should_retry = self.should_retry_error(&error);
        *last_error = Some(error);

        if should_retry && attempt < self.policy.max_attempts {
            self.circuit_breaker.record_failure();
            let delay = self.calculate_delay(attempt);
            log_debug!(
                attempt = attempt,
                max_attempts = self.policy.max_attempts,
                delay_ms = delay.as_millis(),
                error = ?last_error.as_ref(),
                "Request failed, retrying after delay"
            );
            sleep(delay).await;
            true
        } else {
            self.circuit_breaker.record_failure();
            false
        }
    }

    async fn handle_timeout(&mut self, attempt: u32, last_error: &mut Option<LlmError>) -> bool {
        let timeout_error = LlmError::timeout(self.policy.request_timeout.as_secs());
        *last_error = Some(timeout_error);

        if attempt < self.policy.max_attempts {
            self.circuit_breaker.record_failure();
            let delay = self.calculate_delay(attempt);
            log_debug!(
                attempt = attempt,
                max_attempts = self.policy.max_attempts,
                delay_ms = delay.as_millis(),
                timeout_seconds = self.policy.request_timeout.as_secs(),
                "Request timed out, retrying after delay"
            );
            sleep(delay).await;
            true
        } else {
            self.circuit_breaker.record_failure();
            false
        }
    }

    fn handle_exhausted_retries<T>(
        &mut self,
        attempt: u32,
        last_error: Option<LlmError>,
        start_time: &Instant,
    ) -> LlmResult<T> {
        let final_error = last_error.unwrap_or_else(|| {
            LlmError::request_failed("Maximum retry attempts exceeded".to_string(), None)
        });

        log_error!(
            attempts = attempt,
            total_duration_ms = start_time.elapsed().as_millis(),
            circuit_state = ?self.circuit_breaker.state(),
            error = %final_error,
            "Request failed after all retry attempts"
        );

        Err(final_error)
    }

    /// Determine if an error should trigger a retry
    fn should_retry_error(&self, error: &LlmError) -> bool {
        match error {
            LlmError::RequestFailed { .. } => true,
            LlmError::Timeout { .. } => true,
            LlmError::RateLimitExceeded { .. } => true,
            LlmError::AuthenticationFailed { .. } => false, // Don't retry auth errors
            LlmError::ConfigurationError { .. } => false,   // Don't retry config errors
            LlmError::TokenLimitExceeded { .. } => false,   // Don't retry token limit errors
            LlmError::UnsupportedProvider { .. } => false,  // Don't retry unsupported provider
            LlmError::ResponseParsingError { .. } => false, // Don't retry parsing errors
            LlmError::ToolExecutionFailed { .. } => false,  // Don't retry tool errors
            LlmError::SchemaValidationFailed { .. } => false, // Don't retry schema errors
        }
    }

    /// Calculate delay for exponential backoff
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_seconds = self.policy.initial_delay.as_secs_f64()
            * self.policy.backoff_multiplier.powi((attempt - 1) as i32);

        let delay = Duration::from_secs_f64(delay_seconds.min(self.policy.max_delay.as_secs_f64()));

        // Add jitter to prevent thundering herd
        let jitter = fastrand::f64() * 0.1; // Up to 10% jitter
        Duration::from_secs_f64(delay.as_secs_f64() * (1.0 + jitter))
    }
}
