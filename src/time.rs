//! Provide a unified interface for different interval crates
#![warn(missing_docs)]
#![warn(unsafe_code)]

#[cfg(feature = "async_tokio")]
use tokio::time::{
    interval as time_interval, Duration as time_Duration, Instant as time_Instant,
    Interval as time_Interval,
};

/// A Duration that represents a span of time
pub struct Duration {
    duration: time_Duration,
}

/// Interval between times
pub struct Interval {
    interval: time_Interval,
}

/// A measurement of a monotonically increasing clock
pub struct Instant {
    instant: time_Instant,
}

impl Duration {
    /// Create a Duration from a nanosecond count
    pub fn from_nanos(period: u64) -> Duration {
        Duration {
            duration: time_Duration::from_nanos(period),
        }
    }

    /// Create a Duration from a millisecond count
    pub fn from_millis(millis: u64) -> Duration {
        Duration {
            duration: time_Duration::from_millis(millis),
        }
    }
}

impl From<Duration> for std::time::Duration {
    fn from(duration: Duration) -> time_Duration {
        duration.duration
    }
}

impl From<time_Duration> for Duration {
    fn from(duration: time_Duration) -> Duration {
        Duration { duration }
    }
}

#[cfg(feature = "async_tokio")]
impl Interval {
    /// An asynchronous function that completes after the next instant
    /// in the interval is completed.
    pub async fn tick(&mut self) -> Instant {
        Instant {
            instant: self.interval.tick().await,
        }
    }
}

impl From<Interval> for time_Interval {
    fn from(interval: Interval) -> time_Interval {
        interval.interval
    }
}

impl From<Instant> for time_Instant {
    fn from(instant: Instant) -> time_Instant {
        instant.instant
    }
}

/// Create a new Interval that yields in periods of length duration.
pub fn interval(duration: Duration) -> Interval {
    Interval {
        interval: time_interval(duration.duration),
    }
}
