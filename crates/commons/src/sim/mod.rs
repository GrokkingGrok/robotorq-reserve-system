#[cfg(feature = "sim")]
pub mod api {
    use std::thread::sleep;
    use std::time::{Duration, Instant};
    use crate::util::config::simulation::{Simulation, load_simuation_config_from_default};

    pub fn load_sim() -> Simulation {
        load_simuation_config_from_default()
    }

    pub fn effective_sleep(seconds: u64, speedup: Option<f64>) -> Duration {
        let sp = speedup.unwrap_or(1.0).max(1e-6);
        Duration::from_secs_f64((seconds as f64) / sp)
    }

    pub fn sleep_simulated(seconds: u64, sim: &Simulation) {
        sleep(effective_sleep(seconds, sim.speedup));
    }

    pub fn time_block<F, R>(sim: &Simulation, seconds: u64, f: F) -> (R, Duration)
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        sleep_simulated(seconds, sim);
        (result, start.elapsed())
    }
}
