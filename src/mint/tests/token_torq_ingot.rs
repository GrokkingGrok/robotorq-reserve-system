//! Tests for `TokenTorqIngot` helper methods exercising Triple decomposition,
//! accumulation, validation, and RoboTorq equivalence.

use std::time::SystemTime;
use mint::models::token_torq_ingot::TokenTorqIngot;
use common::triples::{decimal_to_triple, Decimal};

fn sample_merkle_root() -> String { "a".repeat(64) }
fn sample_unit_hashes(count: usize) -> Vec<String> { vec!["b".repeat(64); count] }

fn build_ingot(joules: i64, stake: i64, units: u32) -> TokenTorqIngot {
    TokenTorqIngot {
        ingot_id: "ingot-test".into(),
        contract_id: "contract-1".into(),
        joules_total: joules,
        robostake_total_jouletorq: stake,
        unit_count: units,
        merkle_root: sample_merkle_root(),
        unit_hashes: sample_unit_hashes(units as usize),
        timestamp: SystemTime::now(),
        signature: vec![],
    }
}

#[test]
fn triple_decomposition_matches_decimal() {
    let ingot = build_ingot(3600, 3600, 3600);
    let dec = Decimal::new(ingot.joules_total, 0);
    let expected = decimal_to_triple(&dec).expect("valid triple");
    let derived = ingot.triple();
    assert_eq!(expected.robotorq, derived.robotorq);
    assert_eq!(expected.jouletorq_remainder, derived.jouletorq_remainder);
}

#[test]
fn accumulation_adds_joules_and_stake() {
    let mut a = build_ingot(1000, 1000, 1000);
    let b = build_ingot(2600, 2600, 2600); // note: unit_count differs from validate invariant purposely
    a.accumulate(&b);
    assert_eq!(a.joules_total, 3600);
    assert_eq!(a.robostake_total_jouletorq, 3600);
}

#[test]
fn validate_passes_for_expected_unit_count() {
    let ingot = build_ingot(3600, 3600, 3600);
    assert!(ingot.validate().is_ok());
}

#[test]
fn robotorq_equivalent_is_fractional() {
    let ingot = build_ingot(1800, 1800, 1800);
    let eq = ingot.robotorq_equivalent();
    assert!(eq > 0.0);
    assert!(eq < 1.0);
}
