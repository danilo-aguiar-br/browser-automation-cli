// SPDX-License-Identifier: MIT OR Apache-2.0
//! Injectable clock for deterministic tests (rules: dependency inversion).
//!
//! Production paths use [`SystemClock`](crate::clock::SystemClock).
//! Tests may inject [`FixedClock`](crate::clock::FixedClock).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Wall-clock abstraction for pure domain logic and tests.
pub trait Clock: Send + Sync {
    /// Current time as Unix epoch milliseconds.
    fn now_unix_ms(&self) -> u64;

    /// Current [`SystemTime`].
    fn now(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(self.now_unix_ms())
    }
}

/// Real wall clock (production default).
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_unix_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

/// Fixed clock for deterministic unit tests.
#[derive(Debug, Clone, Copy)]
pub struct FixedClock {
    /// Unix epoch milliseconds returned by every call.
    pub unix_ms: u64,
}

impl Clock for FixedClock {
    fn now_unix_ms(&self) -> u64 {
        self.unix_ms
    }
}

/// Process-wide convenience: system clock milliseconds (no global mutability).
pub fn system_unix_ms() -> u64 {
    SystemClock.now_unix_ms()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_clock_is_stable() {
        let c = FixedClock { unix_ms: 1_700_000_000_000 };
        assert_eq!(c.now_unix_ms(), 1_700_000_000_000);
        assert_eq!(c.now_unix_ms(), c.now_unix_ms());
    }

    #[test]
    fn system_clock_is_nonzero() {
        assert!(SystemClock.now_unix_ms() > 0);
    }
}
