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