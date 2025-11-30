//! TripleTorq monetary accounting system.
//!
//! TripleTorq represents the hierarchical monetary balances in the RoboTorq Reserve System.
//! It maintains three levels of currency units (RoboTorq, TokenTorq, JouleTorq) with
//! automatic normalization to prevent overflow and maintain mathematical invariants.
//!
//! The system enforces strict economic relationships:
//! - 1 RoboTorq = 1,000 TokenTorq
//! - 1 TokenTorq = 3,600 JouleTorq
//! - 1 RoboTorq = 3,600,000 JouleTorq (derived)
//!
//! Balances are kept in canonical form where sub-level balances never exceed their
//! maximum values, automatically rolling over excess to higher-level units.

use serde::{Serialize, Deserialize};
use core::cmp::Ordering;
use crate::types::ids::TripleTorqId;
use crate::util::error::triple_torq_error::TripleTorqError;
use crate::util::error::InvariantError;
use crate::util::error::config_error::ConfigError;
use crate::util::hashing::hash_struct;
use crate::util::schema::TRIPLE_TORQ_SCHEMA_VERSION;

// TripleTorq accounting math (inlined from former `triple_torq_math` module).
// These helpers operate on monetary circulation balances (not raw ore measurement).
// Invariants:
// - 1 RoboTorq = 1_000 TokenTorq
// - 1 TokenTorq = 3_600 JouleTorq
// - tokentorq_balance < 1_000
// - jouletorq_balance < 3_600
// The canonical form keeps sub-balances within bounds rolling upward.

/// Number of TokenTorq units that equal one RoboTorq unit.
///
/// This fundamental economic constant defines the exchange rate between
/// TokenTorq and RoboTorq in the TripleTorq monetary hierarchy.
pub const TOKEN_TORQS_PER_ROBOTORQ: u128 = 1_000;

/// Number of JouleTorq units that equal one TokenTorq unit.
///
/// This fundamental economic constant defines the exchange rate between
/// JouleTorq and TokenTorq in the TripleTorq monetary hierarchy.
pub const JOULE_TORQS_PER_TOKEN_TORQ: u128 = 3_600;

/// Number of JouleTorq units that equal one RoboTorq unit (derived constant).
///
/// This is calculated as TOKEN_TORQS_PER_ROBOTORQ * JOULE_TORQS_PER_TOKEN_TORQ.
/// It represents the total JouleTorq units in one RoboTorq unit.
pub const JOULE_TORQS_PER_ROBOTORQ: u128 = TOKEN_TORQS_PER_ROBOTORQ * JOULE_TORQS_PER_TOKEN_TORQ; // 3_600_000

/// Normalize raw (robot, token, joule) balances into canonical bounded representation.
/// 
/// This function enforces the TripleTorq invariants by rolling over excess
/// sub-level balances to higher-level units:
/// - Excess JouleTorq (> 3,600) rolls into TokenTorq
/// - Excess TokenTorq (> 1,000) rolls into RoboTorq
/// 
/// # Arguments
/// * `robotorq` - Raw RoboTorq balance
/// * `tokentorq` - Raw TokenTorq balance (may exceed 1,000)
/// * `jouletorq` - Raw JouleTorq balance (may exceed 3,600)
/// 
/// # Returns
/// Returns a tuple `(robotorq, tokentorq, jouletorq)` in canonical form where
/// tokentorq < 1,000 and jouletorq < 3,600.
fn normalize(mut robotorq: u128, mut tokentorq: u128, mut jouletorq: u128) -> (u128, u16, u16) {
    // Roll excess JouleTorq into TokenTorq.
    if jouletorq >= JOULE_TORQS_PER_TOKEN_TORQ {
        let extra_tokens = jouletorq / JOULE_TORQS_PER_TOKEN_TORQ;
        tokentorq += extra_tokens;
        jouletorq %= JOULE_TORQS_PER_TOKEN_TORQ;
    }
    // Roll excess TokenTorq into RoboTorq.
    if tokentorq >= TOKEN_TORQS_PER_ROBOTORQ {
        let extra_robotorq = tokentorq / TOKEN_TORQS_PER_ROBOTORQ;
        robotorq += extra_robotorq;
        tokentorq %= TOKEN_TORQS_PER_ROBOTORQ;
    }
    // Safe casts (post-normalization bounds guaranteed).
    (robotorq, tokentorq as u16, jouletorq as u16)
}

/// Convert a canonical triple to total JouleTorq units.
///
/// This function converts normalized TripleTorq balances back to the total
/// number of JouleTorq units they represent.
///
/// # Arguments
/// * `robotorq` - RoboTorq balance (canonical form)
/// * `tokentorq` - TokenTorq balance (must be < 1,000)
/// * `jouletorq` - JouleTorq balance (must be < 3,600)
///
/// # Returns
/// Returns the total JouleTorq units represented by the triple.
fn to_smallest_units(robotorq: u128, tokentorq: u16, jouletorq: u16) -> u128 {
    robotorq * JOULE_TORQS_PER_ROBOTORQ + (tokentorq as u128) * JOULE_TORQS_PER_TOKEN_TORQ + (jouletorq as u128)
}

/// Decompose total JouleTorq units into canonical triple balances.
///
/// This function converts a total JouleTorq amount into the normalized
/// TripleTorq representation with proper rollover.
///
/// # Arguments
/// * `total` - Total JouleTorq units to decompose
///
/// # Returns
/// Returns a tuple `(robotorq, tokentorq, jouletorq)` in canonical form.
fn from_smallest_units(total: u128) -> (u128, u16, u16) {
    let robotorq = total / JOULE_TORQS_PER_ROBOTORQ;
    let rem_after_robot = total % JOULE_TORQS_PER_ROBOTORQ;
    let tokentorq = rem_after_robot / JOULE_TORQS_PER_TOKEN_TORQ;
    let jouletorq = rem_after_robot % JOULE_TORQS_PER_TOKEN_TORQ;
    (robotorq, tokentorq as u16, jouletorq as u16)
}

/// TripleTorq monetary account representing hierarchical currency balances.
///
/// TripleTorq is the core monetary data structure in the RoboTorq Reserve System.
/// It maintains three levels of currency units with automatic normalization to
/// prevent overflow and maintain economic invariants.
///
/// # Economic Invariants
/// - tokentorq_balance < 1,000 (must rollover to robotorq_balance)
/// - jouletorq_balance < 3,600 (must rollover to tokentorq_balance)
/// - All balances are kept in canonical normalized form
///
/// # Fields
/// - `id`: Unique identifier for this TripleTorq account
/// - `robotorq_balance`: Balance in RoboTorq units (unlimited)
/// - `tokentorq_balance`: Balance in TokenTorq units (0-999)
/// - `jouletorq_balance`: Balance in JouleTorq units (0-3599)
/// - `schema_version`: Version of the TripleTorq schema for compatibility
/// - `hash`: Cryptographic hash of the account state for integrity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripleTorq {
    pub id: TripleTorqId,
    pub robotorq_balance: u128,
    pub tokentorq_balance: u16,         // can never be larer than 1000, must rollover into robotorq
    pub jouletorq_balance: u16,         // can never be larger than 3600, must rollover into tokentorq
    pub schema_version: u32,
    pub hash: [u8; 32],
}

impl TripleTorq {
    /// Creates a new TripleTorq with canonical balances.
    ///
    /// This constructor validates that the provided balances are already in
    /// canonical form (no rollover needed). For balances that may need
    /// normalization, use `from_components_unchecked` instead.
    ///
    /// # Arguments
    /// * `robotorq_balance` - RoboTorq balance
    /// * `tokentorq_balance` - TokenTorq balance (must be < 1,000)
    /// * `jouletorq_balance` - JouleTorq balance (must be < 3,600)
    ///
    /// # Returns
    /// Returns a `Result` containing the new TripleTorq or an `InvariantError` if validation fails.
    ///
    /// # Errors
    /// - `TripleTorqError::TokenTorqRolloverError` if tokentorq_balance >= 1,000
    /// - `TripleTorqError::JouleTorqRolloverError` if jouletorq_balance >= 3,600
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::triple_torq::TripleTorq;
    /// let account = TripleTorq::new(10, 500, 1800).unwrap();
    /// assert_eq!(account.robotorq_balance, 10);
    /// assert_eq!(account.tokentorq_balance, 500);
    /// assert_eq!(account.jouletorq_balance, 1800);
    /// ```
    pub fn new(robotorq_balance: u128, tokentorq_balance: u16, jouletorq_balance: u16) -> Result<Self, InvariantError> {
        if tokentorq_balance >= 1000 { return Err(InvariantError::from(TripleTorqError::TokenTorqRolloverError)); }
        if jouletorq_balance >= 3600 { return Err(InvariantError::from(TripleTorqError::JouleTorqRolloverError)); }
        let provisional = Self { id: TripleTorqId::new(), robotorq_balance, tokentorq_balance, jouletorq_balance, schema_version: TRIPLE_TORQ_SCHEMA_VERSION, hash: [0u8;32] };
        let hash = hash_struct(&provisional);
        Ok(Self { hash, ..provisional })
    }

    /// Creates a TripleTorq from raw components with automatic normalization.
    ///
    /// This constructor accepts potentially overflowing balances and normalizes
    /// them into canonical form by rolling over excess sub-level balances.
    ///
    /// # Arguments
    /// * `robot` - Raw RoboTorq balance
    /// * `token` - Raw TokenTorq balance (may exceed 1,000)
    /// * `joule` - Raw JouleTorq balance (may exceed 3,600)
    ///
    /// # Returns
    /// Returns a normalized TripleTorq in canonical form.
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::triple_torq::TripleTorq;
    /// // 1,500 tokens + 7,500 joules will normalize to 1 robot + 502 tokens + 300 joules
    /// let account = TripleTorq::from_components_unchecked(0, 1500, 7500);
    /// assert_eq!(account.robotorq_balance, 1);
    /// assert_eq!(account.tokentorq_balance, 502);
    /// assert_eq!(account.jouletorq_balance, 300);
    /// ```
    pub fn from_components_unchecked(robot: u128, token: u128, joule: u128) -> Self {
        let (r,t,j) = normalize(robot, token, joule);
        let provisional = Self { id: TripleTorqId::new(), robotorq_balance: r, tokentorq_balance: t, jouletorq_balance: j, schema_version: TRIPLE_TORQ_SCHEMA_VERSION, hash: [0u8;32] };
        let hash = hash_struct(&provisional);
        Self { hash, ..provisional }
    }

    /// Creates a TripleTorq from total JouleTorq units.
    ///
    /// This constructor converts a total JouleTorq amount into the normalized
    /// TripleTorq representation with proper rollover to higher-level units.
    ///
    /// # Arguments
    /// * `total` - Total JouleTorq units to represent
    ///
    /// # Returns
    /// Returns a normalized TripleTorq equivalent to the total JouleTorq units.
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::triple_torq::TripleTorq;
    /// // 3,600,000 JouleTorq = 1 RoboTorq
    /// let account = TripleTorq::from_smallest_units(3_600_000);
    /// assert_eq!(account.robotorq_balance, 1);
    /// assert_eq!(account.tokentorq_balance, 0);
    /// assert_eq!(account.jouletorq_balance, 0);
    /// ```
    pub fn from_smallest_units(total: u128) -> Self {
        let (r,t,j) = from_smallest_units(total);
        let provisional = Self { id: TripleTorqId::new(), robotorq_balance: r, tokentorq_balance: t, jouletorq_balance: j, schema_version: TRIPLE_TORQ_SCHEMA_VERSION, hash: [0u8;32] };
        let hash = hash_struct(&provisional);
        Self { hash, ..provisional }
    }

    /// Returns the total JouleTorq units represented by this TripleTorq.
    ///
    /// This method converts the hierarchical balances back to the total
    /// number of JouleTorq units they represent.
    ///
    /// # Returns
    /// The total JouleTorq units in this account.
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::triple_torq::TripleTorq;
    /// let account = TripleTorq::new(1, 500, 1800).unwrap();
    /// let total = account.total_smallest_units();
    /// // 1 RoboTorq + 500 TokenTorq + 1800 JouleTorq
    /// assert_eq!(total, 3_600_000 + 500 * 3_600 + 1800);
    /// ```
    pub fn total_smallest_units(&self) -> u128 {
        to_smallest_units(self.robotorq_balance, self.tokentorq_balance, self.jouletorq_balance)
    }

    /// Adds another TripleTorq balance, returning the normalized sum.
    ///
    /// This method adds two TripleTorq balances and automatically normalizes
    /// the result to maintain canonical form.
    ///
    /// # Arguments
    /// * `other` - The TripleTorq balance to add
    ///
    /// # Returns
    /// A new normalized TripleTorq representing the sum.
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::triple_torq::TripleTorq;
    /// let a = TripleTorq::new(1, 500, 1800).unwrap();
    /// let b = TripleTorq::new(0, 600, 2000).unwrap();
    /// let sum = a.add(&b);
    /// // Result will be normalized
    /// assert_eq!(sum.total_smallest_units(), a.total_smallest_units() + b.total_smallest_units());
    /// ```
    pub fn add(&self, other: &TripleTorq) -> TripleTorq {
        TripleTorq::from_components_unchecked(
            self.robotorq_balance + other.robotorq_balance,
            self.tokentorq_balance as u128 + other.tokentorq_balance as u128,
            self.jouletorq_balance as u128 + other.jouletorq_balance as u128,
        )
    }

    /// Adds JouleTorq units directly, returning the normalized result.
    ///
    /// This method adds a raw JouleTorq amount and automatically normalizes
    /// the result across all balance levels.
    ///
    /// # Arguments
    /// * `add_units` - JouleTorq units to add
    ///
    /// # Returns
    /// A new normalized TripleTorq with the added units.
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::triple_torq::TripleTorq;
    /// let account = TripleTorq::new(0, 0, 3599).unwrap();
    /// let result = account.add_smallest_units(1);
    /// // Should rollover: 3599 + 1 = 3600 JouleTorq = 1 TokenTorq
    /// assert_eq!(result.tokentorq_balance, 1);
    /// assert_eq!(result.jouletorq_balance, 0);
    /// ```
    pub fn add_smallest_units(&self, add_units: u128) -> TripleTorq {
        TripleTorq::from_smallest_units(self.total_smallest_units() + add_units)
    }

    /// Subtracts another TripleTorq balance with borrowing across levels.
    ///
    /// This method subtracts one TripleTorq from another, borrowing from
    /// higher-level balances when needed. Returns an error if the result
    /// would be negative.
    ///
    /// # Arguments
    /// * `other` - The TripleTorq balance to subtract
    ///
    /// # Returns
    /// Returns a `Result` containing the difference or an `InvariantError` if the result would be negative.
    ///
    /// # Errors
    /// Returns `TripleTorqError::NegativeTripleTorqError` if other > self.
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::triple_torq::TripleTorq;
    /// let a = TripleTorq::new(1, 0, 0).unwrap();
    /// let b = TripleTorq::new(0, 500, 0).unwrap();
    /// let result = a.subtract(&b).unwrap();
    /// assert_eq!(result.total_smallest_units(), a.total_smallest_units() - b.total_smallest_units());
    /// ```
    pub fn subtract(&self, other: &TripleTorq) -> Result<TripleTorq, InvariantError> {
        let self_total = self.total_smallest_units();
        let other_total = other.total_smallest_units();
        if other_total > self_total { return Err(InvariantError::from(TripleTorqError::NegativeTripleTorqError)); }
        Ok(TripleTorq::from_smallest_units(self_total - other_total))
    }

    /// Multiplies the balance by a scalar factor with normalization.
    ///
    /// This method multiplies all balance levels by the factor and normalizes
    /// the result to maintain canonical form.
    ///
    /// # Arguments
    /// * `factor` - The scalar multiplication factor
    ///
    /// # Returns
    /// A new normalized TripleTorq representing the product.
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::triple_torq::TripleTorq;
    /// let account = TripleTorq::new(0, 1, 0).unwrap();
    /// let doubled = account.mul(2);
    /// assert_eq!(doubled.tokentorq_balance, 2);
    /// ```
    pub fn mul(&self, factor: u128) -> TripleTorq {
        // Multiplication in smallest units then decompose ensures normalization.
        TripleTorq::from_smallest_units(self.total_smallest_units() * factor)
    }

    /// Divides the balance by a scalar divisor using floor division.
    ///
    /// This method divides all balance levels by the divisor using floor
    /// division on the total JouleTorq units.
    ///
    /// # Arguments
    /// * `divisor` - The scalar division divisor (must not be zero)
    ///
    /// # Returns
    /// Returns a `Result` containing the quotient or an `InvariantError` on division by zero.
    ///
    /// # Errors
    /// Returns `ConfigError::Invalid` if divisor is zero.
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::triple_torq::TripleTorq;
    /// let account = TripleTorq::new(0, 2, 0).unwrap();
    /// let halved = account.div_floor(2).unwrap();
    /// assert_eq!(halved.tokentorq_balance, 1);
    /// ```
    pub fn div_floor(&self, divisor: u128) -> Result<TripleTorq, InvariantError> {
        if divisor == 0 {
            return Err(InvariantError::from(ConfigError::Invalid("division by zero".to_string())));
        }
        Ok(TripleTorq::from_smallest_units(self.total_smallest_units() / divisor))
    }

    /// Returns the absolute difference between two TripleTorq balances.
    ///
    /// This method calculates the absolute difference (always non-negative)
    /// between two balances by comparing their total JouleTorq units.
    ///
    /// # Arguments
    /// * `other` - The TripleTorq balance to compare against
    ///
    /// # Returns
    /// A new TripleTorq representing the absolute difference.
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::triple_torq::TripleTorq;
    /// let a = TripleTorq::new(0, 10, 0).unwrap();
    /// let b = TripleTorq::new(0, 5, 0).unwrap();
    /// let diff = a.abs_diff(&b);
    /// assert_eq!(diff.tokentorq_balance, 5);
    /// ```
    pub fn abs_diff(&self, other: &TripleTorq) -> TripleTorq {
        let a = self.total_smallest_units();
        let b = other.total_smallest_units();
        let diff = if a >= b { a - b } else { b - a };
        TripleTorq::from_smallest_units(diff)
    }

    /// Compares two TripleTorq balances by their total value.
    ///
    /// This method compares balances by converting both to total JouleTorq
    /// units and comparing those values.
    ///
    /// # Arguments
    /// * `other` - The TripleTorq balance to compare against
    ///
    /// # Returns
    /// An `Ordering` indicating the relative values of the balances.
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::triple_torq::TripleTorq;
    /// # use core::cmp::Ordering;
    /// let a = TripleTorq::new(0, 1, 0).unwrap();
    /// let b = TripleTorq::new(0, 2, 0).unwrap();
    /// assert_eq!(a.compare(&b), Ordering::Less);
    /// ```
    pub fn compare(&self, other: &TripleTorq) -> Ordering {
        self.total_smallest_units().cmp(&other.total_smallest_units())
    }

    /// Returns the minimum of two TripleTorq balances by total value.
    ///
    /// # Arguments
    /// * `other` - The TripleTorq balance to compare against
    ///
    /// # Returns
    /// A clone of the smaller balance.
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::triple_torq::TripleTorq;
    /// let a = TripleTorq::new(0, 1, 0).unwrap();
    /// let b = TripleTorq::new(0, 2, 0).unwrap();
    /// let min = a.min(&b);
    /// assert_eq!(min.tokentorq_balance, 1);
    /// ```
    pub fn min(&self, other: &TripleTorq) -> TripleTorq {
        if self.total_smallest_units() <= other.total_smallest_units() { self.clone() } else { other.clone() }
    }

    /// Returns the maximum of two TripleTorq balances by total value.
    ///
    /// # Arguments
    /// * `other` - The TripleTorq balance to compare against
    ///
    /// # Returns
    /// A clone of the larger balance.
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::triple_torq::TripleTorq;
    /// let a = TripleTorq::new(0, 1, 0).unwrap();
    /// let b = TripleTorq::new(0, 2, 0).unwrap();
    /// let max = a.max(&b);
    /// assert_eq!(max.tokentorq_balance, 2);
    /// ```
    pub fn max(&self, other: &TripleTorq) -> TripleTorq {
        if self.total_smallest_units() >= other.total_smallest_units() { self.clone() } else { other.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Tests that the normalize function correctly rolls up excess balances across levels.
    /// 
    /// Verifies the economic invariant that excess JouleTorq (> 3,600) rolls into TokenTorq,
    /// and excess TokenTorq (> 1,000) rolls into RoboTorq.
    fn normalize_rolls_up() {
        let (r,t,j) = normalize(0, 1_500, 7_500); // 1500 tokens, 7500 joules
        // 1500 tokens -> 1 robot + 500 tokens; 7500 joules -> 2 tokens + 300 joules
        // Combined before final token normalization: robot=0+1=1, token=500+2=502, joule=300
        assert_eq!((r,t,j), (1, 502, 300));
    }

    #[test]
    /// Tests round-trip conversion between hierarchical balances and total JouleTorq units.
    /// 
    /// Ensures that `to_smallest_units` and `from_smallest_units` are inverses of each other,
    /// maintaining data integrity across the normalization process.
    fn round_trip_smallest_units() {
        let (r,t,j) = normalize(2, 1234, 9999);
        let total = to_smallest_units(r, t, j);
        let (r2,t2,j2) = from_smallest_units(total);
        assert_eq!((r,t,j), (r2,t2,j2));
    }

    #[test]
    /// Tests successful creation of TripleTorq with valid canonical balances.
    /// 
    /// Verifies that `TripleTorq::new` accepts balances within the economic bounds
    /// (tokentorq < 1,000, jouletorq < 3,600) and creates the expected account.
    fn triple_torq_ok() {
        let t = TripleTorq::new(10, 999, 3599).unwrap();
        assert_eq!(t.robotorq_balance, 10);
        assert_eq!(t.tokentorq_balance, 999);
        assert_eq!(t.jouletorq_balance, 3599);
    }

    #[test]
    /// Tests error handling when TokenTorq balance exceeds the rollover threshold.
    /// 
    /// Ensures that `TripleTorq::new` rejects balances where tokentorq_balance >= 1,000,
    /// enforcing the economic invariant that TokenTorq must rollover to RoboTorq.
    fn triple_torq_token_rollover_error() {
        let err = TripleTorq::new(0, 1000, 0).unwrap_err();
        matches!(err, InvariantError::TripleTorq(TripleTorqError::TokenTorqRolloverError));
    }

    #[test]
    /// Tests error handling when JouleTorq balance exceeds the rollover threshold.
    /// 
    /// Ensures that `TripleTorq::new` rejects balances where jouletorq_balance >= 3,600,
    /// enforcing the economic invariant that JouleTorq must rollover to TokenTorq.
    fn triple_torq_joule_rollover_error() {
        let err = TripleTorq::new(0, 0, 3600).unwrap_err();
        matches!(err, InvariantError::TripleTorq(TripleTorqError::JouleTorqRolloverError));
    }

    #[test]
    /// Tests addition and subtraction operations with automatic normalization.
    /// 
    /// Verifies that `add` correctly sums balances with rollover, and `subtract`
    /// correctly computes differences with borrowing across balance levels.
    fn triple_torq_add_and_subtract() {
        let a = TripleTorq::new(1, 500, 100).unwrap();
        let b = TripleTorq::new(0, 750, 3500).unwrap();
        let sum = a.add(&b); // 1 robot + (500+750=1250 tokens) + (100+3500=3600 joules)
        // 1250 tokens -> +1 robot + 250 tokens; 3600 joules -> +1 token + 0 joules
        // Final: robot=1+1=2, tokens=250+1=251, joules=0
        assert_eq!(sum.robotorq_balance, 2);
        assert_eq!(sum.tokentorq_balance, 251);
        assert_eq!(sum.jouletorq_balance, 0);
        let diff = sum.subtract(&a).unwrap();
        assert_eq!(diff.robotorq_balance, 0);
        assert_eq!(diff.tokentorq_balance, 750);
        assert_eq!(diff.jouletorq_balance, 3500);
    }

    #[test]
    /// Tests error handling for negative results in subtraction.
    /// 
    /// Ensures that `subtract` returns an error when attempting to subtract
    /// a larger balance from a smaller one, preventing negative monetary values.
    fn triple_torq_subtract_negative_error() {
        let a = TripleTorq::new(0,10,0).unwrap();
        let b = TripleTorq::new(0,11,0).unwrap();
        let err = a.subtract(&b).unwrap_err();
        matches!(err, InvariantError::TripleTorq(TripleTorqError::NegativeTripleTorqError));
    }

    #[test]
    /// Tests scalar multiplication, division, and absolute difference operations.
    /// 
    /// Verifies that `mul` scales balances correctly, `div_floor` performs floor division,
    /// and `abs_diff` computes the absolute difference between balances.
    fn triple_torq_mul_div_absdiff() {
        let t = TripleTorq::new(1, 250, 900).unwrap();
        let twice = t.mul(2);
        assert_eq!(twice.total_smallest_units(), t.total_smallest_units() * 2);

        let half = twice.div_floor(2).unwrap();
        assert_eq!(half.robotorq_balance, t.robotorq_balance);
        assert_eq!(half.tokentorq_balance, t.tokentorq_balance);
        assert_eq!(half.jouletorq_balance, t.jouletorq_balance);

        let zero = t.div_floor(u128::MAX).unwrap();
        assert_eq!(zero.total_smallest_units(), t.total_smallest_units() / u128::MAX);

        let _err = t.div_floor(0).unwrap_err();

        let a = TripleTorq::new(0, 10, 0).unwrap();
        let b = TripleTorq::new(0, 12, 0).unwrap();
        let d1 = a.abs_diff(&b);
        let d2 = b.abs_diff(&a);
        assert_eq!(d1.total_smallest_units(), d2.total_smallest_units());
    }

    #[test]
    /// Tests comparison operations and min/max functions.
    /// 
    /// Verifies that `compare`, `min`, and `max` work correctly based on total
    /// JouleTorq value, and that round-trip conversions maintain equality.
    fn triple_torq_comparisons() {
        use core::cmp::Ordering as Ord2;
        let a = TripleTorq::new(0, 999, 3599).unwrap();
        let b = TripleTorq::new(1, 0, 0).unwrap();
        assert_eq!(a.compare(&b), Ord2::Less);
        let m = a.max(&b);
        assert_eq!(m.total_smallest_units(), b.total_smallest_units());
        let n = a.min(&b);
        assert_eq!(n.total_smallest_units(), a.total_smallest_units());
        let eq1 = TripleTorq::new(0, 0, 3600 - 1).unwrap();
        let eq2 = TripleTorq::from_smallest_units(eq1.total_smallest_units());
        assert_eq!(eq1.total_smallest_units(), eq2.total_smallest_units());
    }
}