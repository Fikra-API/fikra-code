//! Shared exponential retry delays with jitter.

use std::time::Duration;

use rand::Rng;

const INITIAL_DELAY_MS: u64 = 200;
const BACKOFF_FACTOR: f64 = 2.0;
const MAX_DELAY_MS: u64 = 15_000;

/// Return a retry delay starting at 200 ms and doubling on each subsequent attempt,
/// with up to 10% jitter. Attempts zero and one both use the initial delay.
pub fn backoff(attempt: u64) -> Duration {
    let exp = BACKOFF_FACTOR.powi(attempt.saturating_sub(1) as i32);
    // Cap each wait so longer retry budgets (Fikra provider: 12 stream
    // retries) ride out a ~100 s outage instead of waiting minutes per try.
    let base = ((INITIAL_DELAY_MS as f64 * exp) as u64).min(MAX_DELAY_MS);
    let jitter = rand::rng().random_range(0.9..1.1);
    Duration::from_millis((base as f64 * jitter) as u64)
}

#[cfg(test)]
mod fikra_backoff_cap_tests {
    use super::*;

    #[test]
    fn backoff_is_capped_at_fifteen_seconds() {
        assert!(backoff(30) <= Duration::from_millis(16_500));
        assert!(backoff(3) < Duration::from_secs(1));
    }
}
