//! Token Validation Error Types
//!
//! This module defines error types related to token validation and energy
//! accounting in the `RoboTorq` Reserve System. Token errors handle violations
//! of energy conservation laws and token specification invariants.
//!
//! # Token Economics
//!
//! Tokens represent quantized units of robotic work with strict invariants:
//! - Joule count must be positive (work performed)
//! - Energy values must be positive (conservation of energy)
//! - Token mappings must preserve economic relationships
//!
//! # `JouleTorqOre` Units
//!
//! The fundamental unit is `JouleTorqOre`, representing:
//! - 1 joule of electrical energy consumed
//! - 1 token of robotic work performed
//! - Atomic work proof in the system
//!
//! # Validation Context
//!
//! Token validation occurs during:
//! - Token creation and mapping
//! - Batch processing and aggregation
//! - Economic invariant verification
//! - Work proof validation

use thiserror::Error;

/// Errors that occur during token validation and processing.
///
/// These errors represent violations of token specification invariants
/// that would compromise the economic integrity of the `RoboTorq` system.
#[derive(Debug, Error)]
pub enum TokenError {
    /// The joule count is zero or negative.
    ///
    /// Tokens must represent positive work performed. Zero joules would
    /// represent no work, violating the work proof requirement.
    ///
    /// # Parameters
    /// - `u32`: The invalid joule count (0)
    ///
    /// # Causes
    /// - Invalid token mapping
    /// - Sensor failure during measurement
    /// - Data transmission errors
    ///
    /// # Economic Impact
    /// Zero joules would create valueless tokens, undermining the reserve system.
    ///
    /// # Examples
    /// ```rust
    /// # use commons::util::error::token_error::TokenError;
    /// let error = TokenError::ZeroJoules(0);
    /// if let TokenError::ZeroJoules(value) = error {
    ///     assert_eq!(value, 0);
    /// }
    /// ```
    #[error("Joule count must be > 0: {0}")]
    ZeroJoules(u32),
}
