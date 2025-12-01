//! Timekeeping utilities.
//!
//! Provides an abstraction over `SystemTime::now()` to make time access
//! testable and consistent across the codebase.
use std::time::SystemTime;

/// Get the current system time.
///
/// This function provides a centralized way to access the current system time,
/// which can be useful for testing and abstraction purposes.
///
/// # Returns
///
/// A `SystemTime` representing the current system time.
///
/// # Example
/// ```
/// use commons::util::timekeeping::now;
///
/// let current_time = now();
/// // Use the current time for timestamping operations
/// ```
pub fn now() -> SystemTime {
    SystemTime::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_returns_current_time() {
        let time1 = now();
        let time2 = now();

        // Ensure that now() returns a valid SystemTime
        assert!(time1 <= time2, "Time should not go backwards");

        // Optionally, check that it's close to SystemTime::now()
        let direct_now = SystemTime::now();
        let elapsed = direct_now.duration_since(time1).unwrap_or_default();
        assert!(elapsed.as_millis() < 10, "now() should be very close to SystemTime::now()");
    }
}
