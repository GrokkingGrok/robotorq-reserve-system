//! Unique identifier types for RoboTorq Reserve System entities.
//!
//! This module defines strongly-typed UUID wrappers for all major entities in the
//! RoboTorq system. Using distinct types prevents mixing up IDs of different
//! entity types, providing compile-time safety for the distributed system.
//!
//! All IDs are backed by UUID v4 for global uniqueness and are serializable
//! for network transmission and persistence.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a robot in the RoboTorq network.
///
/// Robots are the primary work-performing entities that generate JouleTorqOre
/// through physical labor. Each robot has a unique identity that persists
/// across contracts and operational states.
///
/// # Examples
/// ```
/// # use commons::types::ids::RobotId;
/// let robot_id = RobotId::new();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RobotId(pub Uuid);

/// Unique identifier for a token representing work performed.
///
/// Tokens are the atomic units of work proof in the RoboTorq system.
/// Each token corresponds to a specific amount of robotic labor measured
/// in joules and is cryptographically linked to its source robot.
///
/// # Examples
/// ```
/// # use commons::types::ids::TokenId;
/// let token_id = TokenId::new();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TokenId(pub Uuid);

/// Unique identifier for an unmapped ore batch.
///
/// Unmapped ore batches contain raw JouleTorqOre tokens that haven't yet
/// been aggregated into higher-level structures. These batches are created
/// by robots and later processed into TokenTorqIngots.
///
/// # Examples
/// ```
/// # use commons::types::ids::UnmappedOreBatchId;
/// let batch_id = UnmappedOreBatchId::new();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UnmappedOreBatchId(pub Uuid);

/// Unique identifier for a TripleTorq account.
///
/// TripleTorq accounts represent the monetary balance of participants in
/// the RoboTorq reserve system. Each account maintains separate balances
/// for JouleTorq, TokenTorq, and RoboTorq units.
///
/// # Examples
/// ```
/// # use commons::types::ids::TripleTorqId;
/// let account_id = TripleTorqId::new();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TripleTorqId(pub Uuid);

/// Unique identifier for a contract between parties.
///
/// Contracts define the terms of engagement between robots and their
/// operators, including work requirements, compensation rates, and
/// operational parameters.
///
/// # Examples
/// ```
/// # use commons::types::ids::ContractId;
/// let contract_id = ContractId::new();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ContractId(pub Uuid);

/// Unique identifier for a party in the RoboTorq network.
///
/// Parties can be individuals, organizations, or automated systems that
/// participate in the RoboTorq ecosystem as robot operators, token holders,
/// or service providers.
///
/// # Examples
/// ```
/// # use commons::types::ids::PartyId;
/// let party_id = PartyId::new();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PartyId(pub Uuid);

impl RobotId {
    /// Creates a new unique RobotId using UUID v4.
    ///
    /// # Examples
    /// ```
    /// # use commons::types::ids::RobotId;
    /// let robot_id = RobotId::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RobotId {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenId {
    /// Creates a new unique TokenId using UUID v4.
    ///
    /// # Examples
    /// ```
    /// # use commons::types::ids::TokenId;
    /// let token_id = TokenId::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TokenId {
    fn default() -> Self {
        Self::new()
    }
}

impl UnmappedOreBatchId {
    /// Creates a new unique UnmappedOreBatchId using UUID v4.
    ///
    /// # Examples
    /// ```
    /// # use commons::types::ids::UnmappedOreBatchId;
    /// let batch_id = UnmappedOreBatchId::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for UnmappedOreBatchId {
    fn default() -> Self {
        Self::new()
    }
}

impl TripleTorqId {
    /// Creates a new unique TripleTorqId using UUID v4.
    ///
    /// # Examples
    /// ```
    /// # use commons::types::ids::TripleTorqId;
    /// let account_id = TripleTorqId::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TripleTorqId {
    fn default() -> Self {
        Self::new()
    }
}

impl ContractId {
    /// Creates a new unique ContractId using UUID v4.
    ///
    /// # Examples
    /// ```
    /// # use commons::types::ids::ContractId;
    /// let contract_id = ContractId::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ContractId {
    fn default() -> Self {
        Self::new()
    }
}

impl PartyId {
    /// Creates a new unique PartyId using UUID v4.
    ///
    /// # Examples
    /// ```
    /// # use commons::types::ids::PartyId;
    /// let party_id = PartyId::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for PartyId {
    fn default() -> Self {
        Self::new()
    }
}
