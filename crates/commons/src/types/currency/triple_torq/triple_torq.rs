use serde::{Serialize, Deserialize};
use core::cmp::Ordering;
use crate::{TripleTorqId, TripleTorqError, InvariantError};
use crate::hashing::hash_struct;
use crate::triple_torq_math::{normalize, to_smallest_units, from_smallest_units};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripleTorq {
    pub id: TripleTorqId,
    pub robotorq_balance: u128,
    pub tokentorq_balance: u16,         // can never be larer than 1000, must rollover into robotorq
    pub jouletorq_balance: u16,         // can never be larger than 3600, must rollover into tokentorq
    pub hash: [u8; 32],
}

impl TripleTorq {
    /// Canonical constructor. Fails if already out of bounds (prefer `from_components_unchecked` + `normalize_canonical` if needed).
    pub fn new(robotorq_balance: u128, tokentorq_balance: u16, jouletorq_balance: u16) -> Result<Self, InvariantError> {
        if tokentorq_balance >= 1000 { return Err(InvariantError::from(TripleTorqError::TokenTorqRolloverError)); }
        if jouletorq_balance >= 3600 { return Err(InvariantError::from(TripleTorqError::JouleTorqRolloverError)); }
        let provisional = Self { id: TripleTorqId::new(), robotorq_balance, tokentorq_balance, jouletorq_balance, hash: [0u8;32] };
        let hash = hash_struct(&provisional);
        Ok(Self { hash, ..provisional })
    }

    /// Create from raw possibly overflowed components, normalizing into canonical form.
    pub fn from_components_unchecked(robot: u128, token: u128, joule: u128) -> Self {
        let (r,t,j) = normalize(robot, token, joule);
        let provisional = Self { id: TripleTorqId::new(), robotorq_balance: r, tokentorq_balance: t, jouletorq_balance: j, hash: [0u8;32] };
        let hash = hash_struct(&provisional);
        Self { hash, ..provisional }
    }

    /// Create from total smallest units (JouleTorq) directly.
    pub fn from_smallest_units(total: u128) -> Self {
        let (r,t,j) = from_smallest_units(total);
        let provisional = Self { id: TripleTorqId::new(), robotorq_balance: r, tokentorq_balance: t, jouletorq_balance: j, hash: [0u8;32] };
        let hash = hash_struct(&provisional);
        Self { hash, ..provisional }
    }

    /// Total smallest units (JouleTorq) represented by this TripleTorq.
    pub fn total_smallest_units(&self) -> u128 {
        to_smallest_units(self.robotorq_balance, self.tokentorq_balance, self.jouletorq_balance)
    }

    /// Add another TripleTorq, returning normalized sum.
    pub fn add(&self, other: &TripleTorq) -> TripleTorq {
        TripleTorq::from_components_unchecked(
            self.robotorq_balance + other.robotorq_balance,
            self.tokentorq_balance as u128 + other.tokentorq_balance as u128,
            self.jouletorq_balance as u128 + other.jouletorq_balance as u128,
        )
    }

    /// Add smallest units (JouleTorq) directly.
    pub fn add_smallest_units(&self, add_units: u128) -> TripleTorq {
        TripleTorq::from_smallest_units(self.total_smallest_units() + add_units)
    }

    /// Subtract another TripleTorq (borrow across levels) returning error if result would be negative.
    pub fn subtract(&self, other: &TripleTorq) -> Result<TripleTorq, InvariantError> {
        let self_total = self.total_smallest_units();
        let other_total = other.total_smallest_units();
        if other_total > self_total { return Err(InvariantError::from(TripleTorqError::NegativeTripleTorqError)); }
        Ok(TripleTorq::from_smallest_units(self_total - other_total))
    }

    /// Multiply by a scalar factor, returning normalized product.
    pub fn mul(&self, factor: u128) -> TripleTorq {
        // Multiplication in smallest units then decompose ensures normalization.
        TripleTorq::from_smallest_units(self.total_smallest_units() * factor)
    }

    /// Divide by a scalar divisor using floor division on smallest units.
    /// Returns error on division by zero.
    pub fn div_floor(&self, divisor: u128) -> Result<TripleTorq, InvariantError> {
        if divisor == 0 {
            return Err(InvariantError::from(crate::ConfigError::Invalid("division by zero".to_string())));
        }
        Ok(TripleTorq::from_smallest_units(self.total_smallest_units() / divisor))
    }

    /// Absolute difference (non-negative) between two TripleTorq balances.
    pub fn abs_diff(&self, other: &TripleTorq) -> TripleTorq {
        let a = self.total_smallest_units();
        let b = other.total_smallest_units();
        let diff = if a >= b { a - b } else { b - a };
        TripleTorq::from_smallest_units(diff)
    }

    /// Compare two balances; returns `Ordering` by smallest units.
    pub fn compare(&self, other: &TripleTorq) -> Ordering {
        self.total_smallest_units().cmp(&other.total_smallest_units())
    }

    /// Minimum of two balances (by smallest units).
    pub fn min(&self, other: &TripleTorq) -> TripleTorq {
        if self.total_smallest_units() <= other.total_smallest_units() { self.clone() } else { other.clone() }
    }

    /// Maximum of two balances (by smallest units).
    pub fn max(&self, other: &TripleTorq) -> TripleTorq {
        if self.total_smallest_units() >= other.total_smallest_units() { self.clone() } else { other.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triple_torq_ok() {
        let t = TripleTorq::new(10, 999, 3599).unwrap();
        assert_eq!(t.robotorq_balance, 10);
        assert_eq!(t.tokentorq_balance, 999);
        assert_eq!(t.jouletorq_balance, 3599);
    }

    #[test]
    fn triple_torq_token_rollover_error() {
        let err = TripleTorq::new(0, 1000, 0).unwrap_err();
        matches!(err, InvariantError::TripleTorq(TripleTorqError::TokenTorqRolloverError));
    }

    #[test]
    fn triple_torq_joule_rollover_error() {
        let err = TripleTorq::new(0, 0, 3600).unwrap_err();
        matches!(err, InvariantError::TripleTorq(TripleTorqError::JouleTorqRolloverError));
    }

    #[test]
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
    fn triple_torq_subtract_negative_error() {
        let a = TripleTorq::new(0,10,0).unwrap();
        let b = TripleTorq::new(0,11,0).unwrap();
        let err = a.subtract(&b).unwrap_err();
        matches!(err, InvariantError::TripleTorq(TripleTorqError::NegativeTripleTorqError));
    }

    #[test]
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