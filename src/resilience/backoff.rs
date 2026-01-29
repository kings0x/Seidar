//! Exponential backoff with jitter.

use std::time::Duration;
use rand::Rng;

/// Calculate exponential backoff delay with jitter.
pub fn calculate_backoff(attempt: u32, base_ms: u64, max_ms: u64) -> Duration {
    if attempt == 0 {
        return Duration::from_millis(0);
    }

    let exponential_base = 2u64.saturating_pow(attempt - 1);
    let delay_ms = base_ms.saturating_mul(exponential_base);
    let capped_delay = delay_ms.min(max_ms);

    // Apply jitter (0 to 10% of the delay)
    let jitter_range = capped_delay / 10;
    let jitter = if jitter_range > 0 {
        rand::thread_rng().gen_range(0..jitter_range)
    } else {
        0
    };

    Duration::from_millis(capped_delay + jitter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_calculation() {
        let b1 = calculate_backoff(1, 100, 2000);
        assert!(b1.as_millis() >= 100);
        
        let b2 = calculate_backoff(2, 100, 2000);
        assert!(b2.as_millis() >= 200);

        let max = calculate_backoff(10, 100, 1000);
        assert!(max.as_millis() >= 1000);
    }
}
