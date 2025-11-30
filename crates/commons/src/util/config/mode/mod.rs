//! Operational Mode Configuration
//!
//! This module defines the operational modes available in the RoboTorq Reserve System.
//! The system can operate in either Production or Simulation mode, each with different
//! behavioral characteristics and safety guarantees.
//!
//! # Production Mode
//!
//! In production mode, the system:
//! - Connects to real robotic hardware
//! - Processes actual economic transactions
//! - Manages real token minting and distribution
//! - Requires physical robot connectivity
//!
//! # Simulation Mode
//!
//! In simulation mode, the system:
//! - Uses virtual robots with configurable behavior
//! - Simulates economic activity without real value transfer
//! - Allows testing of system logic and performance
//! - Provides deterministic or randomized simulation parameters

use serde::{Deserialize, Serialize};

/// Operational mode of the RoboTorq Reserve System.
///
/// This enum determines the fundamental behavior of the system, controlling
/// whether it operates with real robotic hardware and economic transactions
/// or runs in a simulated environment for testing and development.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Production mode for live system operation.
    ///
    /// In this mode, the system connects to real robots, processes actual
    /// work proofs, and manages real economic transactions. This mode
    /// should only be used in production environments with proper security
    /// and operational controls in place.
    Production,

    /// Simulation mode for testing and development.
    ///
    /// In this mode, the system uses virtual robots and simulated economic
    /// activity. No real value is transferred, and all operations are
    /// contained within the simulation environment. This mode is safe
    /// for development, testing, and demonstration purposes.
    Simulation,
}