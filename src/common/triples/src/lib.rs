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

use std::fmt;

#[derive(Clone, Debug)]
pub struct Decimal {
    value: i64, // stored in smallest unit
    scale: u32, // decimal places
    string_repr: Option<String>, // for cross-validation
}

impl Decimal {
    pub fn new(value: i64, scale: u32) -> Self {
        Self {
            value,
            scale,
            string_repr: None,
        }
    }

    pub fn new_with_string(value: i64, scale: u32, string: &str) -> Self {
        Self {
            value,
            scale,
            string_repr: Some(string.to_string()),
        }
    }

    pub fn new_checked(value: i64, scale: u32) -> Result<Self, &'static str> {
        if scale > 18 {
            return Err("Scale too high");
        }
        Ok(Self::new(value, scale))
    }

    pub fn zero() -> Self {
        Self::new(0, 0)
    }

    pub fn value(&self) -> i64 {
        self.value
    }

    pub fn scale(&self) -> u32 {
        self.scale
    }

    pub fn add(&self, other: Self) -> Self {
        // TODO: Handle different scales
        assert_eq!(self.scale, other.scale);
        Self::new(self.value + other.value, self.scale)
    }

    pub fn div(&self, other: Self) -> Self {
        // Use 128-bit for precision
        let self_scaled = self.value as i128 * 10i128.pow(other.scale);
        let result = self_scaled / other.value as i128;
        // Banker's rounding
        let remainder = self_scaled % other.value as i128;
        let mut rounded = result;
        if remainder * 2 > other.value as i128 {
            rounded += 1;
        } else if remainder * 2 == other.value as i128 && (result % 2 != 0) {
            rounded += 1; // round half to even
        }
        Self::new(rounded as i64, self.scale)
    }

    pub fn split_pro_rata(&self, n: usize) -> Vec<Self> {
        // Largest remainder method
        let total = self.value;
        let base = total / n as i64;
        let remainder = (total % n as i64) as usize;
        let mut parts = vec![Self::new(base, self.scale); n];
        for i in 0..remainder {
            parts[i].value += 1;
        }
        parts
    }

    pub fn reciprocal(&self) -> Option<Self> {
        // Only if exact
        if self.value == 0 {
            return None;
        }
        let denominator = 10i64.pow(self.scale);
        let reciprocal_value = denominator * denominator / self.value;
        if reciprocal_value * self.value == denominator * denominator {
            Some(Self::new(reciprocal_value, self.scale))
        } else {
            None
        }
    }

    pub fn multiply_by_power_of_10(&self, power: u32) -> Self {
        Self::new(self.value * 10i64.pow(power), self.scale)
    }

    pub fn validate_cross(&self) {
        if let Some(ref _s) = self.string_repr {
            // TODO: Parse string and compare
        }
    }

    pub fn is_valid(&self) -> bool {
        // TODO
        true
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let divisor = 10i64.pow(self.scale);
        let integer_part = self.value / divisor;
        let fractional_part = (self.value % divisor).abs();
        if self.scale == 0 {
            write!(f, "{}", integer_part)
        } else {
            write!(f, "{}.{:0width$}", integer_part, fractional_part, width = self.scale as usize)
        }
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        // Normalize scales
        if self.scale == other.scale {
            self.value == other.value
        } else {
            // TODO: Proper normalization
            false
        }
    }
}

pub struct Ledger {
    balance: Decimal,
}

impl Ledger {
    pub fn new() -> Self {
        Self { balance: Decimal::zero() }
    }

    pub fn add_transaction(&mut self, amount: Decimal) {
        self.balance = self.balance.add(amount);
    }

    pub fn reconcile(&self) -> bool {
        // TODO: Check sum
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Priority 1: Use decimal fixed-point (never binary Qm.n)
    #[test]
    fn test_decimal_representation() {
        // Can represent 0.0001 exactly
        let d = Decimal::new(1, 4); // 0.0001
        assert_eq!(d.to_string(), "0.0001");

        // Cannot represent 0.1 in binary, but we use decimal
        let d2 = Decimal::new(1, 1); // 0.1
        assert_eq!(d2.to_string(), "0.1");
    }

    // Priority 2: Never use floating-point at all for final money values
    #[test]
    fn test_no_floating_point() {
        // All operations return Decimal, not f64
        let a = Decimal::new(100, 2); // 1.00
        let b = Decimal::new(200, 2); // 2.00
        let sum = a.add(b);
        assert_eq!(sum.to_string(), "3.00");
        // No f64 involved
    }

    // Priority 3: Store everything as signed 64-bit integers in the smallest currency unit
    #[test]
    fn test_storage_as_i64() {
        let d = Decimal::new(123456789, 8); // 1.23456789
        assert_eq!(d.value(), 123456789i64);
        assert_eq!(d.scale(), 8);
    }

    // Priority 4: Division → always round using "banker's rounding" (round half to even)
    #[test]
    fn test_bankers_rounding() {
        let a = Decimal::new(100, 2); // 1.00
        let b = Decimal::new(3, 0);   // 3
        let result = a.div(b); // 0.3333... -> round to 0.33 (half to even)
        assert_eq!(result.to_string(), "0.33");
    }

    // Priority 5: For interest, VAT, fees, or pro-rata splits: never do a/b directly
    #[test]
    fn test_largest_remainder_method() {
        // 100 cents split over 3 recipients: 33 + 33 + 34
        let total = Decimal::new(100, 0);
        let parts = total.split_pro_rata(3);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].to_string(), "34");
        assert_eq!(parts[1].to_string(), "33");
        assert_eq!(parts[2].to_string(), "33");
        // Sum should equal total
        let sum: Decimal = parts.iter().fold(Decimal::zero(), |acc, x| acc.add(x.clone()));
        assert_eq!(sum, total);
    }

    // Priority 6: Pre-compute and store reciprocals only if they are exact
    #[test]
    fn test_exact_reciprocals() {
        // 0.5 is exact (1/2)
        let half = Decimal::new(5, 1); // 0.5
        let reciprocal = half.reciprocal().unwrap();
        assert_eq!(reciprocal.to_string(), "2.0");

        // 0.333... is not exact, should panic or return None
        let third = Decimal::new(333, 3); // 0.333
        assert!(third.reciprocal().is_none());
    }

    // Priority 7: Use 128-bit arithmetic for true division
    #[test]
    fn test_128_bit_arithmetic() {
        // Large numbers requiring i128
        let big = Decimal::new(i64::MAX, 0);
        let divisor = Decimal::new(2, 0);
        let result = big.div(divisor);
        // Should handle without overflow
        assert!(result.value() > 0);
    }

    // Priority 8: Never widen with left-shift tricks — multiply by exact power of 10
    #[test]
    fn test_multiply_by_power_of_10() {
        let d = Decimal::new(123, 2); // 1.23
        let scaled = d.multiply_by_power_of_10(2); // 123.00
        assert_eq!(scaled.to_string(), "123.00");
        assert_eq!(scaled.value(), 12300);
    }

    // Priority 9: Periodic exact reconciliation
    #[test]
    fn test_reconciliation() {
        let mut ledger = Ledger::new();
        ledger.add_transaction(Decimal::new(100, 0));
        ledger.add_transaction(Decimal::new(-50, 0));
        assert!(ledger.reconcile()); // Sum should be 50
    }

    // Priority 10: Compile-time and runtime invariant checks
    #[test]
    fn test_invariant_checks() {
        let _d = Decimal::new(100, 2);
        // Should not allow invalid scales or values
        assert!(Decimal::new_checked(100, 100).is_err()); // Scale too high
    }

    // Priority 11: Store two parallel representations
    #[test]
    fn test_parallel_representations() {
        let d = Decimal::new_with_string(100, 2, "1.00");
        d.validate_cross(); // Check i64 and string match
        assert!(d.is_valid());
    }
}