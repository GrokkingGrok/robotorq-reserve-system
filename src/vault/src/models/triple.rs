use serde::{Serialize, Deserialize};

pub const INGOTS_PER_ROBOTORQ: i64 = 1000; // TokenTorq per RoboTorq
pub const ORE_PER_INGOT: i64 = 3600;       // JouleTorq per TokenTorq ingot
pub const JOULETORQ_PER_ROBOTORQ: i64 = INGOTS_PER_ROBOTORQ * ORE_PER_INGOT; // 3_600_000

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Triple {
    pub robotorq: i64,
    pub tokentorq_remainder: i64,
    pub jouletorq_remainder: i64,
}

impl Triple {
    pub fn new(r: i64, t: i64, j: i64) -> Self { Self { robotorq: r, tokentorq_remainder: t, jouletorq_remainder: j } }
}

pub fn normalize_triple(mut r: i64, mut t: i64, mut j: i64) -> Triple {
    if j >= ORE_PER_INGOT { let carry = j / ORE_PER_INGOT; t += carry; j %= ORE_PER_INGOT; }
    if t >= INGOTS_PER_ROBOTORQ { let carry = t / INGOTS_PER_ROBOTORQ; r += carry; t %= INGOTS_PER_ROBOTORQ; }
    if j < 0 || t < 0 || r < 0 { panic!("Negative values not allowed in normalization"); }
    Triple { robotorq: r, tokentorq_remainder: t, jouletorq_remainder: j }
}

pub fn canonical_jouletorq(triple: Triple) -> i64 {
    triple.robotorq * JOULETORQ_PER_ROBOTORQ + triple.tokentorq_remainder * ORE_PER_INGOT + triple.jouletorq_remainder
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalize_basic() {
        let t = normalize_triple(0, 999, 3599);
        assert_eq!(t.robotorq, 0);
        assert_eq!(t.tokentorq_remainder, 999);
        assert_eq!(t.jouletorq_remainder, 3599);
    }
    #[test]
    fn normalize_carry() {
        let t = normalize_triple(1, 1001, 7201);
        assert_eq!(t.robotorq, 2); // 1001 ingots -> +1 R (carry), remainder 1 ingot
        assert_eq!(t.tokentorq_remainder, 3);
        assert_eq!(t.jouletorq_remainder, 1); // 7201 -> 2 ingots carry -> remainder 1 ore
    }
}
