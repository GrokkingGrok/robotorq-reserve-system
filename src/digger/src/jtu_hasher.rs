// JTU Hasher - SHA256 hash generation for JouleTorqUnits
// Phase 1: Digger Rewrite - Day 2

use sha2::{Sha256, Digest};

/// Calculate SHA256 hash for a JouleTorqUnit
/// 
/// Hash includes ALL critical fields to ensure integrity:
/// - token_id: Unique identifier (contract-m{milestone}-t{index})
/// - joules: Energy consumed (THIS is why cross product matters!)
/// - robo_stake: Payment for this specific token
/// - timestamp: When the work was done
/// - contract_id: Which contract this belongs to
/// - digger_id: Who did the work
/// 
/// # Example
/// ```
/// let hash = calculate_jtu_hash(
///     "contract-001-m0-t42",
///     4.17,      // joules
///     0.0000139, // robo_stake
///     1731734400,
///     "contract-001",
///     "digger-dev-001"
/// );
/// assert_eq!(hash.len(), 64); // 32 bytes = 64 hex chars
/// ```
pub fn calculate_jtu_hash(
    token_id: &str,
    joules: f64,
    robo_stake: f64,
    timestamp: i64,
    contract_id: &str,
    digger_id: &str,
) -> String {
    // Build deterministic string (order matters!)
    let data = format!(
        "{}|{}|{}|{}|{}|{}",
        token_id, joules, robo_stake, timestamp, contract_id, digger_id
    );
    
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    
    // Return as hex string (64 characters)
    hex::encode(result)
}

/// Create placeholder signature (64 bytes of zeros)
/// 
/// TODO: Replace with real Falcon-1024 signature in Phase 4
/// Falcon-1024 produces ~1280 byte signatures (quantum-resistant!)
pub fn create_placeholder_signature() -> Vec<u8> {
    vec![0u8; 64]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_jtu_hash_deterministic() {
        // Same inputs should produce same hash
        let hash1 = calculate_jtu_hash(
            "test-m0-t1",
            4.17,
            0.0000139,
            1731734400,
            "test-contract",
            "test-digger"
        );
        
        let hash2 = calculate_jtu_hash(
            "test-m0-t1",
            4.17,
            0.0000139,
            1731734400,
            "test-contract",
            "test-digger"
        );
        
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn test_calculate_jtu_hash_different_inputs() {
        // Different inputs should produce different hashes
        let hash1 = calculate_jtu_hash(
            "test-m0-t1",
            4.17,
            0.0000139,
            1731734400,
            "test-contract",
            "test-digger"
        );
        
        let hash2 = calculate_jtu_hash(
            "test-m0-t2", // Different token_id
            4.17,
            0.0000139,
            1731734400,
            "test-contract",
            "test-digger"
        );
        
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_calculate_jtu_hash_joule_sensitivity() {
        // Changing joules should change hash (cross product matters!)
        let hash1 = calculate_jtu_hash(
            "test-m0-t1",
            4.17,    // 100W robot
            0.0000139,
            1731734400,
            "test-contract",
            "test-digger"
        );
        
        let hash2 = calculate_jtu_hash(
            "test-m0-t1",
            83.33,   // 2kW robot (20x more energy!)
            0.0000139,
            1731734400,
            "test-contract",
            "test-digger"
        );
        
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_placeholder_signature_size() {
        let sig = create_placeholder_signature();
        assert_eq!(sig.len(), 64);
        assert_eq!(sig, vec![0u8; 64]); // All zeros
    }
}
