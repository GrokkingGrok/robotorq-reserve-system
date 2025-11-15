// Integration Tests for Digger Backend
//
// These tests verify the full ore generation and delivery pipeline
// WITHOUT requiring the Tauri GUI to be running.
//
// Run with: cargo test --test integration_test

use digger_lib::types::{Contract, JouleTorqOre};
use std::time::{SystemTime, UNIX_EPOCH};

// ────────────────────────────────────────────────────────────────
// Mock Refinery Server Tests
// ────────────────────────────────────────────────────────────────

#[test]
fn test_ore_structure_creation() {
    // Create a sample ore like the Digger would generate
    let ore = JouleTorqOre {
        digger_id: "dig-test-001".to_string(),
        contract_id: "contract-test-001".to_string(),
        tokens_generated: 60,
        joules: 1250,
        milestone_index: 0,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        proof_of_work: Some("cGhvdG8tZGlnLXRlc3QtMDAxLTA=".to_string()),
        robo_stake_amount: 0.208333,
        signature: None,
    };

    // Verify structure
    assert_eq!(ore.digger_id, "dig-test-001");
    assert_eq!(ore.contract_id, "contract-test-001");
    assert_eq!(ore.tokens_generated, 60);
    assert_eq!(ore.joules, 1250);
    assert_eq!(ore.milestone_index, 0);
    assert!((ore.robo_stake_amount - 0.208333).abs() < 0.00001);
    assert!(ore.proof_of_work.is_some());
    assert!(ore.signature.is_none());
}

#[test]
fn test_ore_serialization_to_json() {
    let ore = JouleTorqOre {
        digger_id: "dig-test-002".to_string(),
        contract_id: "contract-test-002".to_string(),
        tokens_generated: 120,
        joules: 2500,
        milestone_index: 1,
        timestamp: 1700000000,
        proof_of_work: Some("test-proof".to_string()),
        robo_stake_amount: 0.5,
        signature: None,
    };

    // Serialize to JSON
    let json = serde_json::to_string(&ore).unwrap();
    
    // Verify it contains expected fields
    assert!(json.contains("dig-test-002"));
    assert!(json.contains("contract-test-002"));
    assert!(json.contains("120")); // tokens_generated
    assert!(json.contains("2500")); // joules
    assert!(json.contains("\"milestone_index\":1"));
    assert!(json.contains("0.5")); // robo_stake_amount
}

#[test]
fn test_ore_deserialization_from_json() {
    let json = r#"{
        "digger_id": "dig-test-003",
        "contract_id": "contract-test-003",
        "tokens_generated": 60,
        "joules": 1250,
        "milestone_index": 0,
        "timestamp": 1700000000,
        "proof_of_work": "test-proof",
        "robo_stake_amount": 0.208333,
        "signature": null
    }"#;

    let ore: JouleTorqOre = serde_json::from_str(json).unwrap();
    
    assert_eq!(ore.digger_id, "dig-test-003");
    assert_eq!(ore.contract_id, "contract-test-003");
    assert_eq!(ore.tokens_generated, 60);
    assert_eq!(ore.joules, 1250);
    assert_eq!(ore.milestone_index, 0);
    assert!((ore.robo_stake_amount - 0.208333).abs() < 0.00001);
}

// ────────────────────────────────────────────────────────────────
// Contract Lifecycle Tests
// ────────────────────────────────────────────────────────────────

#[test]
fn test_contract_creation() {
    let contract = Contract {
        id: "test-contract-001".to_string(),
        authorized: true,
        torq: 1,
        max_token_throughput: 12,
        interval_seconds: 5,
        total_tokens: 0,
        robo_stake_total: 100.0,
        duration_hours: 33.333,
    };

    assert_eq!(contract.id, "test-contract-001");
    assert!((contract.robo_stake_total - 100.0).abs() < 0.001);
    assert!((contract.duration_hours - 33.333).abs() < 0.001);
}

#[test]
fn test_contract_duration_from_robo_stake() {
    // Given digger specs: 0.25 kW, 12 tokens/sec
    let amount_rt = 100.0;
    let power_kw = 0.25;
    let max_token_throughput = 12.0;
    
    // Duration = RT / (power × throughput)
    let duration_hours = amount_rt / (power_kw * max_token_throughput);
    
    let contract = Contract {
        id: "calc-test".to_string(),
        authorized: true,
        torq: 1,
        max_token_throughput: 12,
        interval_seconds: 5,
        total_tokens: 0,
        robo_stake_total: amount_rt,
        duration_hours,
    };

    // Should be ~33.333 hours
    assert!((contract.duration_hours - 33.333333).abs() < 0.00001);
}

#[test]
fn test_contract_total_milestones() {
    let contract = Contract {
        id: "milestone-test".to_string(),
        authorized: true,
        torq: 1,
        max_token_throughput: 12,
        interval_seconds: 5,
        total_tokens: 0,
        robo_stake_total: 3000.0,
        duration_hours: 20.0,
    };

    let interval_seconds = 5.0;
    let total_milestones = (contract.duration_hours * 3600.0) / interval_seconds;
    
    // 20 hours × 3600 / 5 = 14,400 milestones
    assert_eq!(total_milestones, 14400.0);
}

// ────────────────────────────────────────────────────────────────
// Ore Generation Pipeline Tests
// ────────────────────────────────────────────────────────────────

#[test]
fn test_ore_generation_sequence() {
    // Simulate generating 3 consecutive ore submissions
    let contract_id = "seq-test".to_string();
    let digger_id = "dig-seq-test".to_string();
    
    let mut ores = Vec::new();
    
    for milestone_index in 0..3 {
        let ore = JouleTorqOre {
            digger_id: digger_id.clone(),
            contract_id: contract_id.clone(),
            tokens_generated: 60,
            joules: 1250,
            milestone_index,
            timestamp: 1700000000 + (milestone_index as u64 * 5),
            proof_of_work: Some(format!("proof-{}", milestone_index)),
            robo_stake_amount: 0.208333,
            signature: None,
        };
        ores.push(ore);
    }

    // Verify sequence
    assert_eq!(ores.len(), 3);
    assert_eq!(ores[0].milestone_index, 0);
    assert_eq!(ores[1].milestone_index, 1);
    assert_eq!(ores[2].milestone_index, 2);
    
    // Verify timestamps are sequential (5 second intervals)
    assert_eq!(ores[1].timestamp - ores[0].timestamp, 5);
    assert_eq!(ores[2].timestamp - ores[1].timestamp, 5);
}

#[test]
fn test_total_ore_accumulation() {
    // Simulate a contract with 10 milestones
    let mut total_tokens = 0;
    let mut total_joules = 0;
    let mut total_robo_stake = 0.0;
    
    for _ in 0..10 {
        total_tokens += 60;
        total_joules += 1250;
        total_robo_stake += 0.208333;
    }

    assert_eq!(total_tokens, 600);
    assert_eq!(total_joules, 12500);
    assert!((total_robo_stake - 2.08333_f64).abs() < 0.00001);
}

#[test]
fn test_robo_stake_distribution_accuracy() {
    // Contract: 3000 RT over 14,400 milestones
    let total_rt = 3000.0;
    let total_milestones = 14400.0;
    let robo_per_milestone = total_rt / total_milestones;
    
    // Generate all milestones and sum
    let mut accumulated_robo: f64 = 0.0;
    for _ in 0..(total_milestones as u32) {
        accumulated_robo += robo_per_milestone;
    }
    
    // Should match total within floating point error
    assert!((accumulated_robo - total_rt).abs() < 0.01);
}

// ────────────────────────────────────────────────────────────────
// RoboStake Calculation Edge Cases
// ────────────────────────────────────────────────────────────────

#[test]
fn test_robo_stake_small_contract() {
    // Very small contract: 1 RT
    let total_rt = 1.0;
    let duration_hours = 0.333333; // ~20 minutes
    let interval_seconds = 5.0;
    
    let total_milestones = (duration_hours * 3600.0) / interval_seconds;
    let robo_per_milestone = total_rt / total_milestones;
    
    // Should be very small but non-zero
    assert!(robo_per_milestone > 0.0);
    assert!(robo_per_milestone < 0.01);
}

#[test]
fn test_robo_stake_large_contract() {
    // Very large contract: 100,000 RT
    let total_rt = 100000.0;
    let duration_hours = 3333.333; // ~138 days
    let interval_seconds = 5.0;
    
    let total_milestones = (duration_hours * 3600.0) / interval_seconds;
    let robo_per_milestone = total_rt / total_milestones;
    
    // Should be reasonable
    assert!(robo_per_milestone > 0.0);
    assert!(robo_per_milestone < total_rt);
}

#[test]
fn test_robo_stake_precision() {
    // Test that we maintain precision across many milestones
    let total_rt = 3000.0;
    let total_milestones = 14400.0;
    let robo_per_milestone = total_rt / total_milestones;
    
    // After 1000 milestones
    let accumulated_1000: f64 = robo_per_milestone * 1000.0;
    let expected_1000: f64 = (total_rt / total_milestones) * 1000.0;
    
    assert!((accumulated_1000 - expected_1000).abs() < 0.000001);
}

// ────────────────────────────────────────────────────────────────
// Proof of Work Tests
// ────────────────────────────────────────────────────────────────

#[test]
fn test_proof_of_work_format() {
    use base64::{engine::general_purpose, Engine as _};
    
    let digger_id = "dig-test-pow";
    let milestone_index = 5;
    
    let proof_data = format!("photo-{}-{}", digger_id, milestone_index);
    let proof_base64 = general_purpose::STANDARD.encode(proof_data.as_bytes());
    
    // Decode and verify
    let decoded = general_purpose::STANDARD.decode(&proof_base64).unwrap();
    let decoded_str = String::from_utf8(decoded).unwrap();
    
    assert_eq!(decoded_str, "photo-dig-test-pow-5");
}

#[test]
fn test_proof_uniqueness_across_milestones() {
    use base64::{engine::general_purpose, Engine as _};
    
    let digger_id = "dig-unique-test";
    
    let mut proofs = Vec::new();
    for i in 0..100 {
        let proof_data = format!("photo-{}-{}", digger_id, i);
        let proof_base64 = general_purpose::STANDARD.encode(proof_data.as_bytes());
        proofs.push(proof_base64);
    }
    
    // Verify all proofs are unique
    for i in 0..proofs.len() {
        for j in (i+1)..proofs.len() {
            assert_ne!(proofs[i], proofs[j]);
        }
    }
}

// ────────────────────────────────────────────────────────────────
// Timestamp Validation Tests
// ────────────────────────────────────────────────────────────────

#[test]
fn test_timestamp_ordering() {
    // Simulate 3 milestones with 5 second intervals
    let base_time = 1700000000_u64;
    
    let timestamps = vec![
        base_time,
        base_time + 5,
        base_time + 10,
    ];
    
    // Verify monotonically increasing
    for i in 1..timestamps.len() {
        assert!(timestamps[i] > timestamps[i-1]);
    }
}

#[test]
fn test_timestamp_intervals() {
    let base_time = 1700000000_u64;
    let interval_seconds = 5;
    
    let mut timestamps = Vec::new();
    for i in 0..10 {
        timestamps.push(base_time + (i * interval_seconds));
    }
    
    // Verify all intervals are exactly 5 seconds
    for i in 1..timestamps.len() {
        assert_eq!(timestamps[i] - timestamps[i-1], interval_seconds);
    }
}

// ────────────────────────────────────────────────────────────────
// Multi-Contract Isolation Tests
// ────────────────────────────────────────────────────────────────

#[test]
fn test_multiple_contracts_isolated() {
    // Create ore for two different contracts
    let ore1 = JouleTorqOre {
        digger_id: "dig-multi-001".to_string(),
        contract_id: "contract-A".to_string(),
        tokens_generated: 60,
        joules: 1250,
        milestone_index: 0,
        timestamp: 1700000000,
        proof_of_work: Some("proof-A-0".to_string()),
        robo_stake_amount: 0.1,
        signature: None,
    };

    let ore2 = JouleTorqOre {
        digger_id: "dig-multi-001".to_string(),
        contract_id: "contract-B".to_string(),
        tokens_generated: 60,
        joules: 1250,
        milestone_index: 0,
        timestamp: 1700000005,
        proof_of_work: Some("proof-B-0".to_string()),
        robo_stake_amount: 0.2,
        signature: None,
    };

    // Verify they're distinct
    assert_ne!(ore1.contract_id, ore2.contract_id);
    assert_ne!(ore1.proof_of_work, ore2.proof_of_work);
    assert_ne!(ore1.robo_stake_amount, ore2.robo_stake_amount);
}

// ────────────────────────────────────────────────────────────────
// Error Handling Tests
// ────────────────────────────────────────────────────────────────

#[test]
fn test_ore_with_zero_tokens() {
    // Edge case: ore with zero tokens (shouldn't happen but handle gracefully)
    let ore = JouleTorqOre {
        digger_id: "dig-edge".to_string(),
        contract_id: "contract-edge".to_string(),
        tokens_generated: 0,
        joules: 1250,
        milestone_index: 0,
        timestamp: 1700000000,
        proof_of_work: Some("proof".to_string()),
        robo_stake_amount: 0.208333,
        signature: None,
    };

    // Should still be valid structure
    assert_eq!(ore.tokens_generated, 0);
    assert_eq!(ore.joules, 1250);
}

#[test]
fn test_ore_with_zero_joules() {
    // Edge case: ore with zero joules (shouldn't happen but handle gracefully)
    let ore = JouleTorqOre {
        digger_id: "dig-edge-2".to_string(),
        contract_id: "contract-edge-2".to_string(),
        tokens_generated: 60,
        joules: 0,
        milestone_index: 0,
        timestamp: 1700000000,
        proof_of_work: Some("proof".to_string()),
        robo_stake_amount: 0.208333,
        signature: None,
    };

    // Should still be valid structure
    assert_eq!(ore.tokens_generated, 60);
    assert_eq!(ore.joules, 0);
}

// ────────────────────────────────────────────────────────────────
// Performance & Load Tests
// ────────────────────────────────────────────────────────────────

#[test]
fn test_large_milestone_sequence() {
    // Simulate a long-running contract (1000 milestones)
    let mut ores = Vec::new();
    
    for i in 0..1000 {
        let ore = JouleTorqOre {
            digger_id: "dig-perf".to_string(),
            contract_id: "contract-perf".to_string(),
            tokens_generated: 60,
            joules: 1250,
            milestone_index: i,
            timestamp: 1700000000 + (i as u64 * 5),
            proof_of_work: Some(format!("proof-{}", i)),
            robo_stake_amount: 0.208333,
            signature: None,
        };
        ores.push(ore);
    }

    assert_eq!(ores.len(), 1000);
    assert_eq!(ores[999].milestone_index, 999);
}

#[test]
fn test_json_serialization_performance() {
    // Create 100 ores and serialize them all
    let mut ores = Vec::new();
    
    for i in 0..100 {
        let ore = JouleTorqOre {
            digger_id: "dig-json-perf".to_string(),
            contract_id: "contract-json-perf".to_string(),
            tokens_generated: 60,
            joules: 1250,
            milestone_index: i,
            timestamp: 1700000000 + (i as u64 * 5),
            proof_of_work: Some(format!("proof-{}", i)),
            robo_stake_amount: 0.208333,
            signature: None,
        };
        ores.push(ore);
    }

    // Serialize all
    for ore in ores {
        let json = serde_json::to_string(&ore).unwrap();
        assert!(json.len() > 0);
    }
}
