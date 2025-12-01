//! Individual work proof tokens representing robotic labor.
//!
//! Tokens carry energetic value in the RoboTorq Reserve System.
//! Each token represents a specific amount of robotic work measured in joules,
//! providing cryptographically verifiable proof of labor performed. Tokens
//! are created by robots during work execution and aggregated into batches
//! for further processing in the economic hierarchy.

use crate::types::ids::TokenId;
use crate::util::error::InvariantError;
use crate::util::error::token_error::TokenError;
use crate::util::hashing::hash_struct;
use crate::util::schema::TOKEN_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};

/// An atomic unit of work proof representing robotic labor.
///
/// Tokens are the fundamental building blocks of value in the RoboTorq system.
/// Each token represents a quantifiable amount of robotic work measured in
/// joules, providing immutable proof that specific labor was performed.
/// Tokens are cryptographically sealed and uniquely identified, ensuring
/// they cannot be duplicated or forged.
///
/// # Economic Role
/// - Carrier of joules in the work proof chain
/// - Foundation for aggregation into TokenTorqIngots and RoboTorqCertificates
/// - Enable precise measurement and verification of robotic labor
/// - Support the mathematical relationship: 1 TokenTorqIngot = 3,600 JouleTorqOre units
///
/// # Work Proof Properties
/// Tokens maintain cryptographic integrity through hashing and are timestamped
/// through their inclusion in UnmappedOreBatches. This ensures temporal ordering
/// and prevents double-spending or manipulation of work proofs.
///
/// # Fields
/// - `id`: Unique identifier for this token
/// - `joule_count`: Number of joules of work this token represents (must be > 0)
/// - `schema_version`: Version of the token schema for compatibility
/// - `hash`: Cryptographic hash of the token contents for integrity verification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Token {
    /// Unique identifier for this token.
    pub id: TokenId,
    /// Number of joules of work this token represents (must be > 0).
    pub joule_count: u32,
    /// Schema version for compatibility and migrations.
    pub schema_version: u32,
    /// Cryptographic hash of the token contents for integrity verification.
    pub hash: [u8; 32],
}

impl Token {
    /// Creates a new token representing the specified joule count.
    ///
    /// This is the canonical constructor for tokens. It validates that the
    /// joule count is positive (tokens must represent actual work performed)
    /// and creates a cryptographically sealed token with a unique ID.
    ///
    /// # Arguments
    /// * `joule_count` - Number of joules of work this token represents (must be > 0)
    ///
    /// # Returns
    /// Returns a `Result` containing the new token or an `InvariantError` if validation fails.
    ///
    /// # Errors
    /// Returns `TokenError::ZeroJoules` if joule_count is 0.
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::token::Token;
    /// let token = Token::new(100).unwrap();
    /// assert_eq!(token.joule_count, 100);
    /// ```
    pub fn new(joule_count: u32) -> Result<Self, InvariantError> {
        Self::map(joule_count)
    }

    /// Creates a token from a joule count with validation.
    ///
    /// This method performs the core token creation logic, validating that
    /// the joule count represents positive work and generating the cryptographic
    /// hash for integrity verification.
    ///
    /// # Arguments
    /// * `joule_count` - Number of joules of work this token represents (must be > 0)
    ///
    /// # Returns
    /// Returns a `Result` containing the new token or an `InvariantError` if validation fails.
    ///
    /// # Errors
    /// Returns `TokenError::ZeroJoules` if joule_count is 0.
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::token::Token;
    /// let token = Token::map(500).unwrap();
    /// assert_eq!(token.joule_count, 500);
    /// assert!(token.joule_count > 0);
    /// ```
    pub fn map(joule_count: u32) -> Result<Self, InvariantError> {
        if joule_count == 0 {
            return Err(InvariantError::from(TokenError::ZeroJoules(joule_count)));
        }
        let provisional = Self {
            id: TokenId::new(),
            joule_count,
            schema_version: TOKEN_SCHEMA_VERSION,
            hash: [0u8; 32],
        };
        let hash = hash_struct(&provisional);
        Ok(Self {
            hash,
            ..provisional
        })
    }
}
