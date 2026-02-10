mod state;

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{info, warn};

pub use state::{CircuitState, CircuitStateData};

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
    pub half_open_max_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
            half_open_max_requests: 3,
        }
    }
}

impl CircuitBreakerConfig {
    pub fn from_env() -> Self {
        Self {
            failure_threshold: std::env::var("CIRCUIT_BREAKER_FAILURE_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            success_threshold: std::env::var("CIRCUIT_BREAKER_SUCCESS_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
            timeout: Duration::from_secs(
                std::env::var("CIRCUIT_BREAKER_TIMEOUT_SECONDS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60),
            ),
            half_open_max_requests: std::env::var("CIRCUIT_BREAKER_HALF_OPEN_REQUESTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    name: String,
    config: CircuitBreakerConfig,
    state: Arc<Mutex<CircuitStateData>>,
}

impl CircuitBreaker {
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        let name_str = name.into();
        info!(
            name = %name_str,
            failure_threshold = config.failure_threshold,
            success_threshold = config.success_threshold,
            timeout_secs = config.timeout.as_secs(),
            "Initialized circuit breaker"
        );

        Self {
            name: name_str,
            config,
            state: Arc::new(Mutex::new(CircuitStateData::new())),
        }
    }

    pub async fn state(&self) -> CircuitState {
        let mut state = self.state.lock().await;

        // Check if we should transition from Open to HalfOpen
        if state.should_attempt_reset(self.config.timeout) {
            info!(name = %self.name, "Circuit breaker transitioning to half-open");
            state.state = CircuitState::HalfOpen;
            state.reset();
        }

        state.state
    }

    pub async fn call<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: Future<Output = Result<T, E>>,
    {
        let current_state = self.state().await;

        match current_state {
            CircuitState::Open => {
                warn!(name = %self.name, "Circuit breaker open, rejecting request");
                Err(CircuitBreakerError::Open)
            }
            CircuitState::HalfOpen => {
                let mut state = self.state.lock().await;
                if state.half_open_attempts >= self.config.half_open_max_requests {
                    warn!(name = %self.name, "Circuit breaker half-open limit reached");
                    return Err(CircuitBreakerError::Open);
                }
                state.half_open_attempts += 1;
                drop(state);

                self.execute(f).await
            }
            CircuitState::Closed => self.execute(f).await,
        }
    }

    async fn execute<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: Future<Output = Result<T, E>>,
    {
        match f.await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(error) => {
                self.on_failure().await;
                Err(CircuitBreakerError::Inner(error))
            }
        }
    }

    async fn on_success(&self) {
        let mut state = self.state.lock().await;

        match state.state {
            CircuitState::Closed => {
                // Reset failure count on success
                state.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                state.success_count += 1;
                if state.success_count >= self.config.success_threshold {
                    info!(name = %self.name, "Circuit breaker closing after successful recovery");
                    state.state = CircuitState::Closed;
                    state.reset();
                }
            }
            CircuitState::Open => {
                // Should not happen, but reset if it does
                state.reset();
            }
        }
    }

    async fn on_failure(&self) {
        let mut state = self.state.lock().await;
        state.failure_count += 1;
        state.last_failure_time = Some(std::time::Instant::now());

        match state.state {
            CircuitState::Closed => {
                if state.failure_count >= self.config.failure_threshold {
                    warn!(
                        name = %self.name,
                        failures = state.failure_count,
                        "Circuit breaker opening due to failures"
                    );
                    state.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                warn!(name = %self.name, "Circuit breaker reopening after failed recovery attempt");
                state.state = CircuitState::Open;
                state.reset();
            }
            CircuitState::Open => {
                // Already open, just track the failure
            }
        }
    }

    pub async fn metrics(&self) -> CircuitBreakerMetrics {
        let state = self.state.lock().await;
        CircuitBreakerMetrics {
            name: self.name.clone(),
            state: state.state,
            failure_count: state.failure_count,
            success_count: state.success_count,
        }
    }
}

#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    Open,
    Inner(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::Open => write!(f, "Circuit breaker is open"),
            CircuitBreakerError::Inner(e) => write!(f, "{}", e),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CircuitBreakerError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CircuitBreakerError::Open => None,
            CircuitBreakerError::Inner(e) => Some(e),
        }
    }
}

#[derive(Debug)]
pub struct CircuitBreakerMetrics {
    pub name: String,
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_secs(1),
            half_open_max_requests: 2,
        };

        let cb = CircuitBreaker::new("test", config);

        // Simulate 3 failures
        for _ in 0..3 {
            let result = cb.call(async { Err::<(), _>("failure") }).await;
            assert!(result.is_err());
        }

        // Circuit should be open now
        assert_eq!(cb.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_rejects_when_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_secs(1),
            half_open_max_requests: 2,
        };

        let cb = CircuitBreaker::new("test", config);

        // Trigger failures to open circuit
        for _ in 0..2 {
            let _ = cb.call(async { Err::<(), _>("failure") }).await;
        }

        // Next request should be rejected
        let result = cb.call(async { Ok::<_, &str>(()) }).await;
        assert!(matches!(result, Err(CircuitBreakerError::Open)));
    }

    #[tokio::test]
    async fn test_circuit_breaker_transitions_to_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
            half_open_max_requests: 2,
        };

        let cb = CircuitBreaker::new("test", config);

        // Open the circuit
        for _ in 0..2 {
            let _ = cb.call(async { Err::<(), _>("failure") }).await;
        }

        assert_eq!(cb.state().await, CircuitState::Open);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should transition to half-open
        assert_eq!(cb.state().await, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_breaker_closes_after_recovery() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
            half_open_max_requests: 3,
        };

        let cb = CircuitBreaker::new("test", config);

        // Open the circuit
        for _ in 0..2 {
            let _ = cb.call(async { Err::<(), _>("failure") }).await;
        }

        // Wait for half-open
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(cb.state().await, CircuitState::HalfOpen);

        // Successful requests should close it
        for _ in 0..2 {
            let result = cb.call(async { Ok::<_, &str>(()) }).await;
            assert!(result.is_ok());
        }

        assert_eq!(cb.state().await, CircuitState::Closed);
    }
}
