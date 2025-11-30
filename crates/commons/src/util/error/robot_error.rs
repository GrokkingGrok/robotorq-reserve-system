//! Robot Validation Error Types
//!
//! This module defines error types related to robot validation and configuration
//! in the RoboTorq Reserve System. Robot errors handle violations of robot
//! specification invariants and operational constraints.
//!
//! # Robot Invariants
//!
//! Robots in the system must satisfy several invariants:
//! - Names must be non-empty and unique
//! - Throughput ratings must be positive (token/sec and joule/sec)
//! - Configuration must be valid for economic calculations
//!
//! # Validation Context
//!
//! Robot validation occurs during:
//! - Robot registration with the gateway
//! - Configuration loading and parsing
//! - Runtime parameter updates
//! - Economic invariant verification

use thiserror::Error;

/// Errors that occur during robot validation and configuration.
///
/// These errors represent violations of robot specification invariants
/// that would compromise system integrity or economic calculations.
#[derive(Debug, Error)]
pub enum RobotError {
    /// The robot name is empty or invalid.
    ///
    /// Robot names must be non-empty strings to ensure proper identification
    /// and logging throughout the system.
    ///
    /// # Causes
    /// - Empty string provided as robot name
    /// - Whitespace-only names
    /// - Null or invalid string values
    #[error("Robot name must be non-empty")]
    InvalidRobotName,

    /// The token throughput rating is zero or negative.
    ///
    /// Robots must have a positive token throughput rating (tokens per second)
    /// to participate in the economic system and perform useful work.
    ///
    /// # Causes
    /// - Zero throughput specified
    /// - Negative values (should be caught by type system)
    /// - Invalid configuration parsing
    ///
    /// # Economic Impact
    /// Zero throughput would result in division by zero in economic calculations.
    #[error("Token throughput per second must be > 0")]
    ZeroTokenThroughput,

    /// The joule throughput rating (power consumption) is zero or negative.
    ///
    /// Robots must have a positive joule throughput rating (watts) to ensure
    /// proper energy accounting and work verification.
    ///
    /// # Causes
    /// - Zero power rating specified
    /// - Negative values (should be caught by type system)
    /// - Invalid configuration parsing
    ///
    /// # Economic Impact
    /// Zero joule throughput violates the JouleTorqOre work proof invariant.
    #[error("Joule throughput (watts) must be > 0")]
    ZeroJouleThroughput,
}
