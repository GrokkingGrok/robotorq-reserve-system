//! Robot Gateway Error Types
//!
//! This module defines error types specific to the Robot Gateway service operations.
//! These errors handle various failure conditions that can occur during robot registration,
//! batch processing, and gateway management in the RoboTorq Reserve System.
//!
//! # Error Categories
//!
//! - **Registration Errors**: Issues with robot registration and management
//! - **Configuration Errors**: Problems with gateway configuration and metrics setup
//! - **Operational Errors**: Runtime issues during gateway operations

use thiserror::Error;

/// Errors that can occur during Robot Gateway operations.
///
/// This enum represents all possible error conditions that may arise when
/// interacting with the Robot Gateway service, including robot registration,
/// batch processing, and configuration issues.
#[derive(Debug, Error)]
pub enum RobotGatewayError {
    /// The specified robot ID is not recognized or the robot is not registered.
    ///
    /// This error occurs when attempting to perform operations on a robot that
    /// has not been properly registered with the gateway or when an invalid
    /// robot ID is provided.
    ///
    /// # Causes
    /// - Robot ID does not exist in the registry
    /// - Robot was previously registered but has been removed
    /// - Typographical error in robot ID
    #[error("Unknown robot id or robot not registered")]
    UnknownRobotId,

    /// Attempted to register a robot that is already registered.
    ///
    /// This error prevents duplicate registrations of the same robot,
    /// ensuring each robot has a unique identity within the system.
    ///
    /// # Causes
    /// - Robot with the same ID already exists
    /// - Attempting to re-register an active robot
    #[error("Robot already registered")]
    AlreadyRegistered,

    /// The gateway's metrics system has not been properly configured.
    ///
    /// This error occurs when attempting to record metrics or access
    /// metrics functionality without proper initialization.
    ///
    /// # Causes
    /// - Metrics registry not initialized
    /// - Prometheus configuration missing
    /// - Gateway started without metrics enabled
    #[error("metrics not configured for gateway")]
    MetricsNotConfigured,

    /// The gateway configuration is invalid or incomplete.
    ///
    /// This error occurs when the gateway is initialized with invalid
    /// or inconsistent configuration parameters.
    ///
    /// # Causes
    /// - Invalid port numbers (e.g., port 0)
    /// - Missing required configuration fields
    /// - Inconsistent configuration values
    #[error("Invalid gateway configuration: {0}")]
    InvalidConfiguration(String),
}