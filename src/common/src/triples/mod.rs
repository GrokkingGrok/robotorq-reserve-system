// Fixed-point decimal monetary arithmetic module moved into common crate.
// See TODO(triples) list below for remaining invariants and enhancements.
// The design ensures deterministic decimal math without floating point.
// Original standalone crate (triples) has been inlined for unified versioning.

use std::fmt;
use sha2::{Digest, Sha256};
use serde::{Serialize, Deserialize};

// TODO(triples): Implement fixed-point math library conforming to monetary standards
// - Priority 1: Use decimal fixed-point (never binary Qm.n)
// - Priority 2: Never use floating-point for final money values
// - Priority 3: Store as signed 64-bit or 128-bit integers in smallest unit
// - Priority 4: Division → always round using "banker's rounding" (round half to even)
// - Priority 5: For interest/VAT/fees: use largest remainder method
// - Priority 6: Pre-compute reciprocals only if exact
// - Priority 7: Use 128-bit arithmetic for true division with exact rounding
// - Priority 8: Never widen with left-shift; multiply by power of 10
// - Priority 9: Periodic exact reconciliation
// - Priority 10: Compile-time and runtime invariant checks
// - Priority 11: Store two parallel representations and cross-validate

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Decimal {
    value: i64,      // stored in smallest unit
    scale: u32,      // decimal places
    string_repr: Option<String>, // for cross-validation (canonical string)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecimalError {
    ScaleTooHigh(u32),
    DivisionByZero,
    Overflow,
    MismatchedScale(u32, u32),
    InvalidStringFormat(String),
    NegativeSplit,
    EmptySplit,
}

const MAX_SCALE: u32 = 18;

impl DecimalError {
    fn msg(&self) -> &'static str {
        match self {
            DecimalError::ScaleTooHigh(_) => "scale too high",
            DecimalError::DivisionByZero => "division by zero",
            DecimalError::Overflow => "overflow",
            DecimalError::MismatchedScale(_, _) => "mismatched scale",
            DecimalError::InvalidStringFormat(_) => "invalid string format",
            DecimalError::NegativeSplit => "cannot split negative value",
            DecimalError::EmptySplit => "cannot split into zero parts",
        }
    }
}

impl Decimal {
    pub fn new(value: i64, scale: u32) -> Self { Self { value, scale, string_repr: None } }

    pub fn new_with_string(value: i64, scale: u32, string: &str) -> Result<Self, DecimalError> {
        let d = Self { value, scale, string_repr: Some(string.to_string()) };
        if !d.validate_string_repr() { return Err(DecimalError::InvalidStringFormat(string.to_string())); }
        Ok(d)
    }

    pub fn new_checked(value: i64, scale: u32) -> Result<Self, DecimalError> {
        if scale > MAX_SCALE { return Err(DecimalError::ScaleTooHigh(scale)); }
        Ok(Self::new(value, scale))
    }

    pub fn zero() -> Self { Self::new(0, 0) }
    pub fn value(&self) -> i64 { self.value }
    pub fn scale(&self) -> u32 { self.scale }

    pub fn normalize_scales(&self, other: &Self) -> (i128, i128, u32) {
        if self.scale == other.scale {
            (self.value as i128, other.value as i128, self.scale)
        } else if self.scale > other.scale {
            let diff = self.scale - other.scale;
            let factor = 10i128.pow(diff);
            (self.value as i128, other.value as i128 * factor, self.scale)
        } else {
            let diff = other.scale - self.scale;
            let factor = 10i128.pow(diff);
            (self.value as i128 * factor, other.value as i128, other.scale)
        }
    }

    pub fn add(&self, other: Self) -> Result<Self, DecimalError> {
        let (a, b, scale) = self.normalize_scales(&other);
        let sum = a + b;
        if sum > i64::MAX as i128 || sum < i64::MIN as i128 { return Err(DecimalError::Overflow); }
        Ok(Self::new(sum as i64, scale))
    }

    pub fn subtract(&self, other: Self) -> Result<Self, DecimalError> {
        let (a, b, scale) = self.normalize_scales(&other);
        let diff = a - b;
        if diff > i64::MAX as i128 || diff < i64::MIN as i128 { return Err(DecimalError::Overflow); }
        Ok(Self::new(diff as i64, scale))
    }

    pub fn div(&self, other: Self) -> Result<Self, DecimalError> {
        if other.value == 0 { return Err(DecimalError::DivisionByZero); }
        // Scale result to max(self.scale, other.scale) for precision
        let target_scale = self.scale.max(other.scale);
        let (a, b, _common_scale) = self.normalize_scales(&other);
        // Multiply numerator by 10^target_scale to expose fractional digits
        let a_scaled = a * 10i128.pow(target_scale);
        let quotient = a_scaled / b;
        let remainder = a_scaled % b;
        let mut rounded = quotient;
        let twice = remainder.abs() * 2;
        if twice > b.abs() {
            rounded += if (a >= 0) == (b >= 0) { 1 } else { -1 };
        } else if twice == b.abs() && (rounded % 2 != 0) {
            rounded += if (a >= 0) == (b >= 0) { 1 } else { -1 };
        }
        if rounded > i64::MAX as i128 || rounded < i64::MIN as i128 { return Err(DecimalError::Overflow); }
        Ok(Self::new(rounded as i64, target_scale))
    }

    pub fn split_pro_rata(&self, n: usize) -> Result<Vec<Self>, DecimalError> {
        if n == 0 { return Err(DecimalError::EmptySplit); }
        if self.value < 0 { return Err(DecimalError::NegativeSplit); }
        let total = self.value;
        let base = total / n as i64;
        let remainder = (total % n as i64) as usize;
        let mut parts = vec![Self::new(base, self.scale); n];
        // Original deterministic distribution: first R recipients get +1
        for i in 0..remainder { parts[i].value += 1; }
        Ok(parts)
    }

    pub fn reciprocal(&self) -> Option<Self> {
        if self.value == 0 { return None; }
        let denominator = 10i128.pow(self.scale);
        let numerator = denominator * denominator;
        let recip = numerator / self.value as i128;
        if recip * self.value as i128 == numerator && recip <= i64::MAX as i128 {
            Some(Self::new(recip as i64, self.scale))
        } else { None }
    }

    pub fn multiply_by_power_of_10(&self, power: u32) -> Result<Self, DecimalError> {
        let factor = 10i128.pow(power);
        let product = self.value as i128 * factor;
        if product > i64::MAX as i128 || product < i64::MIN as i128 { return Err(DecimalError::Overflow); }
        Ok(Self::new(product as i64, self.scale))
    }

    fn validate_string_repr(&self) -> bool {
        if let Some(ref s) = self.string_repr {
            if let Some(dot_pos) = s.find('.') {
                let frac_len = s.len() - dot_pos - 1;
                return frac_len as u32 == self.scale;
            } else {
                return self.scale == 0;
            }
        }
        true
    }

    pub fn validate_cross(&self) -> bool { self.validate_string_repr() }

    pub fn hash_fingerprint(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.value.to_be_bytes());
        hasher.update(self.scale.to_be_bytes());
        if let Some(ref s) = self.string_repr { hasher.update(s.as_bytes()); }
        let out = hasher.finalize();
        hex::encode(out)
    }

    pub fn is_valid(&self) -> bool {
        self.scale <= MAX_SCALE && self.validate_string_repr()
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let divisor = 10i64.pow(self.scale);
        let integer_part = self.value / divisor;
        let fractional_part = (self.value % divisor).abs();
        let sign = if self.value < 0 { "-" } else { "" };
        if self.scale == 0 {
            write!(f, "{}{}", sign, integer_part.abs())
        } else {
            write!(f, "{}{}.{:0width$}", sign, integer_part.abs(), fractional_part, width = self.scale as usize)
        }
    }
}

// ===== Triple representation (decomposed RoboTorq) =====
// Provides a lossless decomposition of a whole balance into RoboTorq, TokenTorq ingots, and JouleTorq ore.
// Arithmetic is delegated to Decimal for safety; conversion preserves invariants.

pub const INGOTS_PER_ROBOTORQ: i64 = 1000; // TokenTorq per RoboTorq
pub const ORE_PER_INGOT: i64 = 3600;       // JouleTorq per TokenTorq ingot
pub const JOULETORQ_PER_ROBOTORQ: i64 = INGOTS_PER_ROBOTORQ * ORE_PER_INGOT; // 3_600_000

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Triple {
    pub robotorq: i64,
    pub tokentorq_remainder: i64, // 0..999
    pub jouletorq_remainder: i64, // 0..3599
}

impl Triple {
    pub fn new(r: i64, t: i64, j: i64) -> Self { Self { robotorq: r, tokentorq_remainder: t, jouletorq_remainder: j } }
    pub fn zero() -> Self { Self::new(0,0,0) }
}

impl Default for Triple { fn default() -> Self { Triple::zero() } }

pub fn jouletorq_to_triple(total_jouletorq: i64) -> Triple {
    let robotorq = total_jouletorq / JOULETORQ_PER_ROBOTORQ;
    let remainder = total_jouletorq % JOULETORQ_PER_ROBOTORQ;
    let tokentorq_remainder = remainder / ORE_PER_INGOT;
    let jouletorq_remainder = remainder % ORE_PER_INGOT;
    Triple::new(robotorq, tokentorq_remainder, jouletorq_remainder)
}

pub fn triple_to_jouletorq(triple: Triple) -> i64 {
    triple.robotorq * JOULETORQ_PER_ROBOTORQ + triple.tokentorq_remainder * ORE_PER_INGOT + triple.jouletorq_remainder
}

pub fn triple_to_decimal(triple: Triple) -> Decimal {
    Decimal::new(triple_to_jouletorq(triple), 0)
}

pub fn decimal_to_triple(d: &Decimal) -> Result<Triple, DecimalError> {
    if d.scale() != 0 { return Err(DecimalError::MismatchedScale(d.scale(), 0)); }
    if d.value() < 0 { return Err(DecimalError::NegativeSplit); }
    Ok(jouletorq_to_triple(d.value()))
}

pub fn add_triples(a: Triple, b: Triple) -> Triple {
    let da = triple_to_decimal(a);
    let db = triple_to_decimal(b);
    let sum = da.add(db).expect("triple addition overflow");
    decimal_to_triple(&sum).expect("decimal to triple conversion")
}

pub fn subtract_triples(a: Triple, b: Triple) -> Triple {
    let da = triple_to_decimal(a);
    let db = triple_to_decimal(b);
    let diff = da.subtract(db).expect("triple subtraction overflow");
    decimal_to_triple(&diff).expect("decimal to triple conversion")
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        let (a, b, _scale) = self.normalize_scales(other);
        a == b
    }
}

#[derive(Debug, Default)]
pub struct Ledger {
    balance: Decimal,
    transactions: Vec<Decimal>,
}

impl Ledger {
    pub fn new() -> Self { Self { balance: Decimal::zero(), transactions: Vec::new() } }
    pub fn balance(&self) -> &Decimal { &self.balance }
    pub fn add_transaction(&mut self, amount: Decimal) -> Result<(), DecimalError> {
        let new_balance = self.balance.add(amount.clone())?;
        self.balance = new_balance;
        self.transactions.push(amount);
        Ok(())
    }
    pub fn reconcile(&self) -> bool {
        let mut running = Decimal::zero();
        for t in &self.transactions {
            running = match running.add(t.clone()) { Ok(d) => d, Err(_) => return false };
        }
        running == self.balance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decimal_representation() {
        let d = Decimal::new(1, 4); // 0.0001
        assert_eq!(d.to_string(), "0.0001");
        let d2 = Decimal::new(1, 1); // 0.1
        assert_eq!(d2.to_string(), "0.1");
    }

    #[test]
    fn test_no_floating_point() {
        let a = Decimal::new(100, 2); // 1.00
        let b = Decimal::new(200, 2); // 2.00
        let sum = a.add(b).unwrap();
        assert_eq!(sum.to_string(), "3.00");
    }

    #[test]
    fn test_storage_as_i64() {
        let d = Decimal::new(123456789, 8); // 1.23456789
        assert_eq!(d.value(), 123456789i64);
        assert_eq!(d.scale(), 8);
    }

    #[test]
    fn test_bankers_rounding() {
        let a = Decimal::new(100, 2); // 1.00
        let b = Decimal::new(3, 0);   // 3
        let result = a.div(b).unwrap();
        assert_eq!(result.to_string(), "0.33");
    }

    #[test]
    fn test_largest_remainder_method() {
        let total = Decimal::new(100, 0);
        let parts = total.split_pro_rata(3).unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].to_string(), "34");
        assert_eq!(parts[1].to_string(), "33");
        assert_eq!(parts[2].to_string(), "33");
        let sum: Decimal = parts.iter().fold(Decimal::zero(), |acc, x| acc.add(x.clone()).unwrap());
        assert_eq!(sum, total);
    }

    #[test]
    fn test_exact_reciprocals() {
        let half = Decimal::new(5, 1); // 0.5
        let reciprocal = half.reciprocal().unwrap();
        assert_eq!(reciprocal.to_string(), "2.0");
        let third = Decimal::new(333, 3); // 0.333
        assert!(third.reciprocal().is_none());
    }

    #[test]
    fn test_128_bit_arithmetic() {
        let big = Decimal::new(i64::MAX, 0);
        let divisor = Decimal::new(2, 0);
        let result = big.div(divisor).unwrap();
        assert!(result.value() > 0);
    }

    #[test]
    fn test_multiply_by_power_of_10() {
        let d = Decimal::new(123, 2); // 1.23
        let scaled = d.multiply_by_power_of_10(2).unwrap(); // 123.00
        assert_eq!(scaled.to_string(), "123.00");
        assert_eq!(scaled.value(), 12300);
    }

    #[test]
    fn test_reconciliation() {
        let mut ledger = Ledger::new();
        ledger.add_transaction(Decimal::new(100, 0)).unwrap();
        ledger.add_transaction(Decimal::new(-50, 0)).unwrap();
        assert!(ledger.reconcile());
    }

    #[test]
    fn test_invariant_checks() {
        assert!(Decimal::new_checked(100, 100).is_err());
        assert!(Decimal::new_checked(100, MAX_SCALE).is_ok());
    }

    #[test]
    fn test_scale_normalization_add() {
        let a = Decimal::new(150, 2); // 1.50
        let b = Decimal::new(2, 0);   // 2
        let sum = a.add(b).unwrap();
        assert_eq!(sum.to_string(), "3.50");
    }

    #[test]
    fn test_subtract() {
        let a = Decimal::new(250, 2); // 2.50
        let b = Decimal::new(75, 2);  // 0.75
        let diff = a.subtract(b).unwrap();
        assert_eq!(diff.to_string(), "1.75");
    }

    #[test]
    fn test_division_round_half_even_negative() {
        let a = Decimal::new(-100, 2); // -1.00
        let b = Decimal::new(3, 0);    // 3
        let result = a.div(b).unwrap();
        assert_eq!(result.to_string(), "-0.33");
    }

    #[test]
    fn test_hash_fingerprint_stable() {
        let a = Decimal::new(100, 2);
        let b = Decimal::new(100, 2);
        assert_eq!(a.hash_fingerprint(), b.hash_fingerprint());
    }

    #[test]
    fn test_reciprocal_exactness() {
        let d = Decimal::new(5, 1); // 0.5
        let r = d.reciprocal().unwrap();
        assert!(r.is_valid());
    }

    #[test]
    fn test_split_fair_distribution() {
        let total = Decimal::new(10, 0);
        let parts = total.split_pro_rata(3).unwrap();
        let values: Vec<i64> = parts.iter().map(|p| p.value()).collect();
        assert_eq!(values.iter().sum::<i64>(), 10);
        assert!(values.contains(&3) && values.contains(&4));
    }
    #[test]
    fn test_parallel_representations() {
        let d = Decimal::new_with_string(100, 2, "1.00").unwrap();
        assert!(d.validate_cross());
        assert!(d.is_valid());
    }

    // Triple tests (migrated from wallet)
    #[test]
    fn test_triple_round_trip() {
        let t = Triple::new(3, 250, 3599);
        let d = triple_to_decimal(t);
        let back = decimal_to_triple(&d).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn test_triple_add_rollover() {
        let a = Triple::new(0, 999, 3599);
        let b = Triple::new(0, 0, 1);
        let legacy = jouletorq_to_triple(triple_to_jouletorq(a) + triple_to_jouletorq(b));
        let sum = add_triples(a, b);
        assert_eq!(legacy, sum);
    }

    #[test]
    fn test_triple_subtract() {
        let a = Triple::new(2, 500, 1800);
        let b = Triple::new(1, 200, 1000);
        let diff = subtract_triples(a,b);
        assert_eq!(diff.robotorq,1);
        assert_eq!(diff.tokentorq_remainder,300);
        assert_eq!(diff.jouletorq_remainder,800);
    }
}
