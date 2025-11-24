// Re-export shared Triple decomposition & Decimal bridging from common crate to eliminate duplication.
pub use common::triples::{
    Triple,
    INGOTS_PER_ROBOTORQ,
    ORE_PER_INGOT,
    JOULETORQ_PER_ROBOTORQ,
    jouletorq_to_triple as normalize_triple,
    triple_to_jouletorq as canonical_jouletorq,
    add_triples,
    subtract_triples,
    triple_to_decimal,
    decimal_to_triple,
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_decimal_bridge() {
        let t = Triple::new(2, 500, 1800);
        let d = triple_to_decimal(t);
        let back = decimal_to_triple(&d).unwrap();
        assert_eq!(t, back);
    }
    #[test]
    fn addition_matches_canonical() {
        let a = Triple::new(0, 999, 3599);
        let b = Triple::new(0, 0, 1);
        let legacy = normalize_triple(canonical_jouletorq(a) + canonical_jouletorq(b));
        let sum = add_triples(a,b);
        assert_eq!(legacy, sum);
    }
}
