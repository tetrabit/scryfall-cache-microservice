use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitState {
    pub fn as_metric_value(&self) -> i64 {
        match self {
            CircuitState::Closed => 0,
            CircuitState::Open => 1,
            CircuitState::HalfOpen => 2,
        }
    }
}

#[derive(Debug)]
pub struct CircuitStateData {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_time: Option<Instant>,
    pub half_open_attempts: u32,
}

impl CircuitStateData {
    pub fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            half_open_attempts: 0,
        }
    }

    pub fn reset(&mut self) {
        self.failure_count = 0;
        self.success_count = 0;
        self.half_open_attempts = 0;
    }

    pub fn should_attempt_reset(&self, timeout: Duration) -> bool {
        if self.state != CircuitState::Open {
            return false;
        }

        if let Some(last_failure) = self.last_failure_time {
            last_failure.elapsed() >= timeout
        } else {
            false
        }
    }
}

impl Default for CircuitStateData {
    fn default() -> Self {
        Self::new()
    }
}
