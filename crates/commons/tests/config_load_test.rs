//! Tests for configuration loading.
//!
//! Covers error paths for missing/malformed files and environment override behavior
//! for `load_robotorq_config`, increasing branch coverage in config loader.
use commons::util::config::{RoboTorqConfig, load_robotorq_config};
use std::env;
use std::fs;
use std::path::PathBuf;

#[test]
fn load_from_file_nonexistent_returns_error() {
    // Nonexistent path should yield an error from `RoboTorqConfig::load_from_file`.
    let path = PathBuf::from("nonexistent_config.toml");
    let result = RoboTorqConfig::load_from_file(&path);
    assert!(result.is_err());
}

#[test]
fn load_from_file_malformed_toml_returns_error() {
    // Malformed TOML content should cause deserialization failure.
    let mut path = std::env::temp_dir();
    path.push("robotorq_invalid_config.toml");
    fs::write(&path, "invalid = [toml").expect("write");
    let result = RoboTorqConfig::load_from_file(&path);
    assert!(result.is_err());
}

#[test]
fn load_robotorq_config_env_override_reads_alternate_path() {
    // When `ROBOTORQ_CONFIG` is set, loader should read from the overridden path.
    let mut pathbuf = std::env::temp_dir();
    pathbuf.push("robotorq_env_config.toml");
    // minimal valid content matching config schema
    fs::write(
        &pathbuf,
        r#"
mode = "Local"

[observability]

[economic]
enabled = true
[economic.ubd]
enabled = true
min_unit_joules = 1
min_unit_tokens = 1
"#,
    )
    .expect("write");
    let path_str = pathbuf.to_string_lossy().to_string();
    unsafe {
        env::set_var("ROBOTORQ_CONFIG", &path_str);
    }
    let cfg = load_robotorq_config(None).expect("load config from env");
    assert!(cfg.observability.metrics.enabled);
    // cleanup env
    unsafe {
        env::remove_var("ROBOTORQ_CONFIG");
    }
}
