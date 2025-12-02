//! Timekeeping utilities.
//!
//! Provides an abstraction over `SystemTime::now()` to make time access
//! testable and consistent across the codebase.
use std::time::{Duration, SystemTime};

/// Abstraction over time source and sleeping.
///
/// Provides testable indirection for time-related operations. Production code
/// uses `SystemTimeProvider`; simulation builds can use `SimulatedTimeProvider`.
/// Implementations should be cheap to clone and thread-safe.
#[allow(async_fn_in_trait)]
pub trait TimeProvider: Send + Sync {
    /// Current logical time.
    fn now(&self) -> SystemTime;
    /// Sleep for the given logical duration.
    async fn sleep(&self, duration: Duration);
}

/// Production time provider backed by the system clock.
#[derive(Debug, Default, Clone)]
pub struct SystemTimeProvider;

impl SystemTimeProvider {
    /// Construct a new `SystemTimeProvider`.
    ///
    /// This is a trivial zero‑cost constructor returning a stateless provider
    /// that delegates to the standard library for `now()` and `sleep()`.
    ///
    /// # Examples
    /// ```
    /// use commons::util::timekeeping::SystemTimeProvider;
    /// use commons::util::timekeeping::TimeProvider;
    /// let tp = SystemTimeProvider::new();
    /// let t1 = tp.now();
    /// tp.sleep(std::time::Duration::from_millis(1));
    /// let t2 = tp.now();
    /// assert!(t2 >= t1);
    /// ```
    pub fn new() -> Self {
        Self
    }
}

impl TimeProvider for SystemTimeProvider {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await
    }
}

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

#[cfg(feature = "sim")]
/// Deterministic time source for simulation mode.
///
/// Provides a controllable time source that can be sped up, slowed down, or paused
/// for deterministic testing and simulation scenarios.
#[derive(Debug, Clone)]
pub struct DeterministicTime {
    start_time: SystemTime,
    speedup: f64,
    paused: bool,
    pause_time: Option<SystemTime>,
}

#[cfg(feature = "sim")]
impl DeterministicTime {
    /// Create a new deterministic time source with the given speedup factor.
    ///
    /// # Arguments
    ///
    /// * `speedup` - Time dilation factor (e.g., 2.0 for double speed, 0.5 for half speed)
    ///
    /// # Panics
    ///
    /// Panics if speedup is negative or zero.
    pub fn new(speedup: f64) -> Self {
        assert!(speedup > 0.0, "Speedup factor must be positive");
        Self {
            start_time: SystemTime::now(),
            speedup,
            paused: false,
            pause_time: None,
        }
    }

    /// Get the current simulated time.
    pub fn now(&self) -> SystemTime {
        if self.paused {
            self.pause_time.unwrap_or(self.start_time)
        } else {
            let elapsed_real = SystemTime::now()
                .duration_since(self.start_time)
                .unwrap_or_default();
            let elapsed_sim = elapsed_real.mul_f64(self.speedup);
            self.start_time + elapsed_sim
        }
    }

    /// Pause the simulation time.
    pub fn pause(&mut self) {
        if !self.paused {
            self.pause_time = Some(self.now());
            self.paused = true;
        }
    }

    /// Resume the simulation time.
    pub fn resume(&mut self) {
        if self.paused {
            self.paused = false;
            // Adjust start_time to account for pause
            if let Some(pause_time) = self.pause_time.take() {
                let now_real = SystemTime::now();
                let elapsed_during_pause = now_real.duration_since(pause_time).unwrap_or_default();
                self.start_time = now_real - elapsed_during_pause.mul_f64(self.speedup);
            }
        }
    }

    /// Set a new speedup factor.
    ///
    /// # Arguments
    ///
    /// * `speedup` - New time dilation factor
    ///
    /// # Panics
    ///
    /// Panics if speedup is negative or zero.
    pub fn set_speedup(&mut self, speedup: f64) {
        assert!(speedup > 0.0, "Speedup factor must be positive");
        if !self.paused {
            // Adjust start_time to maintain continuity
            let current_sim = self.now();
            self.speedup = speedup;
            self.start_time = SystemTime::now()
                - (current_sim
                    .duration_since(self.start_time)
                    .unwrap_or_default())
                .div_f64(speedup);
        } else {
            self.speedup = speedup;
        }
    }

    /// Sleep for the given duration in simulated time.
    ///
    /// Adjusts the sleep duration based on the speedup factor to maintain
    /// logical timing in simulation mode.
    ///
    /// # Arguments
    ///
    /// * `duration` - The logical duration to sleep in simulated time
    pub async fn sim_sleep(&self, duration: std::time::Duration) {
        let adjusted_duration = duration.div_f64(self.speedup);
        tokio::time::sleep(adjusted_duration).await;
    }
}

#[cfg(feature = "sim")]
/// Simulation provider wrapping a `DeterministicTime` for logical time and speedup-aware sleeping.
#[derive(Clone, Debug)]
pub struct SimulatedTimeProvider {
    inner: DeterministicTime,
}

#[cfg(feature = "sim")]
impl SimulatedTimeProvider {
    pub fn new(speedup: f64) -> Self {
        Self {
            inner: DeterministicTime::new(speedup),
        }
    }
    pub fn inner(&self) -> &DeterministicTime {
        &self.inner
    }
}

#[cfg(feature = "sim")]
impl TimeProvider for SimulatedTimeProvider {
    fn now(&self) -> SystemTime {
        self.inner.now()
    }
    async fn sleep(&self, duration: Duration) {
        self.inner.sim_sleep(duration).await
    }
}

#[cfg(feature = "sim")]
impl SimulatedTimeProvider {
    /// Construct from simulation config convenience.
    pub fn from_config(cfg: &crate::util::config::simulation::Simulation) -> Self {
        let speedup = cfg.speedup.unwrap_or(1.0);
        Self::new(speedup.max(0.0001))
    }
}

/// Sleep with time dilation for simulation mode.
///
/// This function adjusts sleep duration based on the simulation speedup factor.
/// In simulation mode with speedup enabled, the actual sleep time is reduced
/// to accelerate testing while maintaining logical timing.
///
/// # Arguments
///
/// * `duration` - The logical duration to sleep (in simulated time)
/// * `config` - Simulation configuration to determine speedup
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use commons::util::config::Simulation;
/// use commons::util::timekeeping::sleep;
///
/// # let rt = tokio::runtime::Runtime::new().unwrap();
/// # rt.block_on(async {
/// let config = Simulation {
///     enabled: true,
///     speedup: Some(10.0),
///     ..Default::default()
/// };
///
/// // Sleep for 1 second of simulated time (actually 100ms real time)
/// sleep(Duration::from_secs(1), &config).await;
/// # });
/// ```
#[cfg(feature = "sim")]
pub async fn sleep(
    duration: std::time::Duration,
    config: &crate::util::config::simulation::Simulation,
) {
    let speedup = config.speedup.unwrap_or(1.0);
    let adjusted_duration = duration.div_f64(speedup);
    tokio::time::sleep(adjusted_duration).await;
}

#[cfg(not(feature = "sim"))]
/// Sleep function for production builds.
///
/// In production builds without simulation support, this function
/// simply sleeps for the specified duration, ignoring any simulation config.
pub async fn sleep(
    _duration: std::time::Duration,
    _config: &crate::util::config::simulation::Simulation,
) {
    // In production builds, ignore simulation config and just sleep normally
    tokio::time::sleep(_duration).await;
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
        assert!(
            elapsed.as_millis() < 10,
            "now() should be very close to SystemTime::now()"
        );
    }

    #[cfg(feature = "sim")]
    mod sim_tests {
        use super::*;
        use crate::util::config::simulation::Simulation;
        use std::time::Duration;

        #[tokio::test]
        async fn test_deterministic_time_basic() {
            let dt = DeterministicTime::new(1.0);
            let start = dt.now();
            tokio::time::sleep(Duration::from_millis(10)).await;
            let end = dt.now();
            let elapsed = end.duration_since(start).unwrap();
            assert!(elapsed.as_millis() >= 10, "Time should advance");
        }

        #[tokio::test]
        async fn test_deterministic_time_speedup() {
            let dt = DeterministicTime::new(2.0); // 2x speed
            let start = dt.now();
            tokio::time::sleep(Duration::from_millis(100)).await; // Real time: 100ms
            let end = dt.now();
            let elapsed = end.duration_since(start).unwrap();
            // Simulated time should be ~200ms (allow for timing variance)
            assert!(
                elapsed.as_millis() >= 180 && elapsed.as_millis() <= 250,
                "Time should be sped up, got {}ms",
                elapsed.as_millis()
            );
        }

        #[tokio::test]
        async fn test_deterministic_time_pause_resume() {
            let mut dt = DeterministicTime::new(1.0);
            let start = dt.now();
            tokio::time::sleep(Duration::from_millis(10)).await;
            dt.pause();
            let paused_time = dt.now();
            tokio::time::sleep(Duration::from_millis(50)).await; // Should not advance
            let after_pause = dt.now();
            assert_eq!(
                paused_time, after_pause,
                "Time should not advance while paused"
            );
            dt.resume();
            tokio::time::sleep(Duration::from_millis(10)).await;
            let resumed = dt.now();
            let total_elapsed = resumed.duration_since(start).unwrap();
            assert!(
                total_elapsed.as_millis() >= 20,
                "Time should advance after resume"
            );
        }

        #[tokio::test]
        async fn test_sim_sleep_no_simulation() {
            let config = Simulation::default();
            let start = std::time::Instant::now();
            sleep(Duration::from_millis(50), &config).await;
            let elapsed = start.elapsed();
            assert!(
                elapsed.as_millis() >= 40 && elapsed.as_millis() <= 70,
                "Should sleep for full duration, got {}ms",
                elapsed.as_millis()
            );
        }

        #[tokio::test]
        async fn test_sim_sleep_with_speedup() {
            let config = Simulation {
                enabled: true,
                speedup: Some(10.0),
                ..Default::default()
            };
            let start = std::time::Instant::now();
            sleep(Duration::from_millis(100), &config).await; // Logical 100ms, real ~10ms
            let elapsed = start.elapsed();
            assert!(
                elapsed.as_millis() >= 5 && elapsed.as_millis() <= 25,
                "Should sleep for adjusted duration, got {}ms",
                elapsed.as_millis()
            );
        }
    }
}
