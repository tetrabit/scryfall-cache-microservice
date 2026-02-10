use governor::{
    clock::{Clock, DefaultClock},
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;
use tokio::time::sleep;
use tracing::debug;

use crate::metrics::registry::SCRYFALL_RATE_LIMIT_WAITS_TOTAL;

/// Rate limiter for Scryfall API requests
#[derive(Clone)]
pub struct RateLimiter {
    limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    requests_per_second: u32,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(requests_per_second: u32) -> Self {
        let quota = Quota::per_second(
            NonZeroU32::new(requests_per_second).expect("requests_per_second must be > 0"),
        );

        let limiter = GovernorRateLimiter::direct(quota);

        Self {
            limiter: Arc::new(limiter),
            requests_per_second,
        }
    }

    /// Wait until we're allowed to make a request
    pub async fn acquire(&self) {
        loop {
            match self.limiter.check() {
                Ok(_) => {
                    debug!("Rate limit check passed");
                    return;
                }
                Err(not_until) => {
                    let wait_time = not_until.wait_time_from(DefaultClock::default().now());
                    debug!("Rate limit exceeded, waiting {:?}", wait_time);
                    SCRYFALL_RATE_LIMIT_WAITS_TOTAL
                        .with_label_values(&[])
                        .inc();
                    sleep(wait_time).await;
                }
            }
        }
    }

    /// Try to acquire without waiting
    pub fn try_acquire(&self) -> bool {
        self.limiter.check().is_ok()
    }

    /// Get the configured requests per second
    pub fn requests_per_second(&self) -> u32 {
        self.requests_per_second
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::Instant;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(10);

        // Make 10 requests rapidly - should all succeed within a second
        let start = Instant::now();
        for _ in 0..10 {
            limiter.acquire().await;
        }
        let elapsed = start.elapsed();

        // Should complete in roughly 1 second
        assert!(elapsed < Duration::from_secs(2));
    }

    #[tokio::test]
    async fn test_rate_limiter_throttling() {
        let limiter = RateLimiter::new(5);

        // Make 10 requests - second batch should be throttled
        let start = Instant::now();
        for _ in 0..10 {
            limiter.acquire().await;
        }
        let elapsed = start.elapsed();

        // Governor allows a burst, so we only assert it wasn't instantaneous.
        // 10 requests at 5 req/sec should take roughly >= 1s in practice.
        assert!(elapsed >= Duration::from_millis(900));
    }
}
