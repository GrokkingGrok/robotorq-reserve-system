//! TripleTorq Validation Error Types
//!
//! This module defines error types related to TripleTorq validation and balance
//! management in the RoboTorq Reserve System. TripleTorq errors handle violations
//! of the hierarchical unit system and balance rollover invariants.
//!
//! # TripleTorq Hierarchy
//!
//! The TripleTorq system maintains a hierarchical balance structure:
//! - **JouleTorq**: Base unit (0-3,599), rolls over into TokenTorq
//! - **TokenTorq**: Mid-level unit (0-999), rolls over into RoboTorq
//! - **RoboTorq**: Top-level unit (unlimited)
//!
//! # Economic Invariants
//!
//! `1 TokenTorq = 3,600 JouleTorq`
//! `1 RoboTorq = 1,000 TokenTorq = 3,600,000 JouleTorq`
//!
//! # Rollover Rules
//!
//! - When JouleTorq reaches 3,600, it resets to 0 and increments TokenTorq
//! - When TokenTorq reaches 1,000, it resets to 0 and increments RoboTorq
//! - Negative balances are not allowed at any level

use thiserror::Error;

/// Errors that occur during TripleTorq validation and balance operations.
///
/// These errors represent violations of TripleTorq balance invariants
/// that would compromise the hierarchical unit system integrity.
#[derive(Debug, Error)]
pub enum TripleTorqError {
    /// A TripleTorq balance component became negative.
    ///
    /// Negative balances are not allowed as they would violate conservation
    /// of value and the work proof system. All balance components must
    /// remain non-negative.
    ///
    /// # Causes
    /// - Invalid arithmetic operations
    /// - Underflow in balance calculations
    /// - Corruption of balance state
    ///
    /// # Economic Impact
    /// Negative balances would allow creation of value from nothing.
    #[error("TripleTorq cannot be negative")]
    NegativeTripleTorqError,

    /// TokenTorq balance exceeded the maximum allowed value.
    ///
    /// TokenTorq balances cannot exceed 999. When reaching 1,000,
    /// the balance must rollover to increment RoboTorq instead.
    ///
    /// # Causes
    /// - Failed rollover logic
    /// - Invalid balance updates
    /// - Arithmetic overflow handling errors
    ///
    /// # Economic Impact
    /// Exceeding 999 TokenTorq violates the 1:1000 RoboTorq ratio.
    #[error("TokenTorq cannot be >= 1000")]
    TokenTorqRolloverError,

    /// JouleTorq balance exceeded the maximum allowed value.
    ///
    /// JouleTorq balances cannot exceed 3,599. When reaching 3,600,
    /// the balance must rollover to increment TokenTorq instead.
    ///
    /// # Causes
    /// - Failed rollover logic
    /// - Invalid balance updates
    /// - Missing carry-over calculations
    ///
    /// # Economic Impact
    /// Exceeding 3,599 JouleTorq violates the 1:3600 TokenTorq ratio.
    #[error("JouleTorq cannot be >= 3600")]
    JouleTorqRolloverError,
}
