//! Triple balance representation for precise fractional RoboTorq accounting
//!
//! Based on vault's triple system for exact fractional balances without floating point.

use serde::{Deserialize, Serialize};

/// Conversion constants (matching vault)
pub const INGOTS_PER_ROBOTORQ: i64 = 1000; // TokenTorq per RoboTorq
pub const ORE_PER_INGOT: i64 = 3600;       // JouleTorq per TokenTorq ingot
pub const JOULETORQ_PER_ROBOTORQ: i64 = INGOTS_PER_ROBOTORQ * ORE_PER_INGOT; // 3_600_000

/// Triple balance representation for precise fractional accounting
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Triple {
    /// Whole RoboTorq units
    pub robotorq: i64,
    /// Remainder in TokenTorq ingots (0-999)
    pub tokentorq_remainder: i64,
    /// Remainder in JouleTorq ore (0-3599)
    pub jouletorq_remainder: i64,
}

impl Triple {
    /// Create a new triple
    pub fn new(r: i64, t: i64, j: i64) -> Self {
        Self {
            robotorq: r,
            tokentorq_remainder: t,
            jouletorq_remainder: j,
        }
    }

    /// Create zero triple
    pub fn zero() -> Self {
        Self::new(0, 0, 0)
    }
}

impl Default for Triple {
    fn default() -> Self {
        Self::zero()
    }
}

/// Convert canonical jouletorq to normalized triple
pub fn jouletorq_to_triple(total_jouletorq: i64) -> Triple {
    let robotorq = total_jouletorq / JOULETORQ_PER_ROBOTORQ;
    let remainder = total_jouletorq % JOULETORQ_PER_ROBOTORQ;

    let tokentorq_remainder = remainder / ORE_PER_INGOT;
    let jouletorq_remainder = remainder % ORE_PER_INGOT;

    Triple::new(robotorq, tokentorq_remainder, jouletorq_remainder)
}

/// Convert normalized triple to canonical jouletorq
pub fn triple_to_jouletorq(triple: Triple) -> i64 {
    triple.robotorq * JOULETORQ_PER_ROBOTORQ +
    triple.tokentorq_remainder * ORE_PER_INGOT +
    triple.jouletorq_remainder
}

/// Add two triples with normalization
pub fn add_triples(a: Triple, b: Triple) -> Triple {
    jouletorq_to_triple(triple_to_jouletorq(a) + triple_to_jouletorq(b))
}

/// Subtract triples (a - b) with normalization
pub fn subtract_triples(a: Triple, b: Triple) -> Triple {
    jouletorq_to_triple(triple_to_jouletorq(a) - triple_to_jouletorq(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triple_creation() {
        let triple = Triple::new(1, 500, 1800);
        assert_eq!(triple.robotorq, 1);
        assert_eq!(triple.tokentorq_remainder, 500);
        assert_eq!(triple.jouletorq_remainder, 1800);
    }

    #[test]
    fn test_zero_triple() {
        let zero = Triple::zero();
        assert_eq!(zero.robotorq, 0);
        assert_eq!(zero.tokentorq_remainder, 0);
        assert_eq!(zero.jouletorq_remainder, 0);
    }

    #[test]
    fn test_jouletorq_to_triple_conversion() {
        // Test whole RoboTorq
        let triple = jouletorq_to_triple(JOULETORQ_PER_ROBOTORQ);
        assert_eq!(triple.robotorq, 1);
        assert_eq!(triple.tokentorq_remainder, 0);
        assert_eq!(triple.jouletorq_remainder, 0);

        // Test with remainders
        let total = JOULETORQ_PER_ROBOTORQ + ORE_PER_INGOT + 100; // 1R + 1T + 100J
        let triple = jouletorq_to_triple(total);
        assert_eq!(triple.robotorq, 1);
        assert_eq!(triple.tokentorq_remainder, 1);
        assert_eq!(triple.jouletorq_remainder, 100);
    }

    #[test]
    fn test_triple_to_jouletorq_conversion() {
        let triple = Triple::new(2, 3, 4);
        let jouletorq = triple_to_jouletorq(triple);
        let expected = 2 * JOULETORQ_PER_ROBOTORQ + 3 * ORE_PER_INGOT + 4;
        assert_eq!(jouletorq, expected);
    }

    #[test]
    fn test_round_trip_conversion() {
        let original = 123456789; // Some large number
        let triple = jouletorq_to_triple(original);
        let converted_back = triple_to_jouletorq(triple);
        assert_eq!(original, converted_back);
    }

    #[test]
    fn test_add_triples() {
        let a = Triple::new(1, 500, 1800);
        let b = Triple::new(0, 600, 2000);
        let sum = add_triples(a, b);

        // Should normalize: 1800 + 2000 = 3800 jouletorq = 1 ingot + 200 jouletorq
        // So: 1R + (500 + 600 + 1) ingots + 200 jouletorq = 1R + 1101T + 200J
        // Which normalizes to: 2R + 101T + 200J
        assert_eq!(sum.robotorq, 2);
        assert_eq!(sum.tokentorq_remainder, 101);
        assert_eq!(sum.jouletorq_remainder, 200);
    }

    #[test]
    fn test_subtract_triples() {
        let a = Triple::new(2, 500, 1800);
        let b = Triple::new(1, 200, 1000);
        let diff = subtract_triples(a, b);

        assert_eq!(diff.robotorq, 1);
        assert_eq!(diff.tokentorq_remainder, 300);
        assert_eq!(diff.jouletorq_remainder, 800);
    }

    #[test]
    fn test_constants() {
        assert_eq!(INGOTS_PER_ROBOTORQ, 1000);
        assert_eq!(ORE_PER_INGOT, 3600);
        assert_eq!(JOULETORQ_PER_ROBOTORQ, 3_600_000);
    }
}