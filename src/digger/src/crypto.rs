/*!
 * Post-Quantum Cryptography Module
 * 
 * Implements Falcon-1024 signatures for JouleTorqOre validation
 * 
 * Why Falcon-1024?
 * - Fast signing (~0.5ms) - critical for high-throughput Diggers
 * - Compact signatures (~1.3KB) - reasonable bandwidth overhead
 * - NIST Round 3 finalist - proven security
 * - Perfect for ephemeral proofs (ore → ingot pipeline)
 */

use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::{PublicKey, SecretKey, SignedMessage};
use sha2::{Digest, Sha256};
use std::fmt;

/// Falcon-1024 keypair for signing ore
#[derive(Clone)]
pub struct DiggerKeypair {
    pub public_key: falcon1024::PublicKey,
    pub secret_key: falcon1024::SecretKey,
}

impl DiggerKeypair {
    /// Generate a new Falcon-1024 keypair
    /// 
    /// Should be called once per Digger on startup and stored securely.
    /// The public key is shared with Refinery for verification.
    /// 
    /// # Performance
    /// - Keygen: ~10ms (one-time cost)
    pub fn generate() -> Self {
        let (public_key, secret_key) = falcon1024::keypair();
        Self {
            public_key,
            secret_key,
        }
    }

    /// Sign a message with Falcon-1024
    /// 
    /// # Arguments
    /// * `message` - Raw bytes to sign (typically SHA256 hash of ore data)
    /// 
    /// # Returns
    /// Signed message (includes signature + original message)
    /// 
    /// # Performance
    /// - Signing: ~0.5ms (fast!)
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let signed = falcon1024::sign(message, &self.secret_key);
        signed.as_bytes().to_vec()
    }

    /// Get public key bytes for transmission
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.public_key.as_bytes().to_vec()
    }

    /// Get secret key bytes for storage
    /// 
    /// ⚠️ WARNING: Store this securely! 
    /// Compromise of secret key allows forging ore proofs.
    pub fn secret_key_bytes(&self) -> Vec<u8> {
        self.secret_key.as_bytes().to_vec()
    }

    /// Reconstruct keypair from stored bytes
    /// 
    /// # Arguments
    /// * `public_key_bytes` - Serialized public key
    /// * `secret_key_bytes` - Serialized secret key
    /// 
    /// # Returns
    /// Result with reconstructed keypair or error
    pub fn from_bytes(
        public_key_bytes: &[u8],
        secret_key_bytes: &[u8],
    ) -> Result<Self, CryptoError> {
        let public_key = falcon1024::PublicKey::from_bytes(public_key_bytes)
            .map_err(|_| CryptoError::InvalidPublicKey)?;
        
        let secret_key = falcon1024::SecretKey::from_bytes(secret_key_bytes)
            .map_err(|_| CryptoError::InvalidSecretKey)?;

        Ok(Self {
            public_key,
            secret_key,
        })
    }
}

impl fmt::Debug for DiggerKeypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DiggerKeypair")
            .field("public_key_len", &self.public_key.as_bytes().len())
            .field("secret_key", &"<redacted>")
            .finish()
    }
}

/// Verify a Falcon-1024 signature
/// 
/// Used by Refinery to validate ore signatures from Diggers.
/// 
/// # Arguments
/// * `signed_message` - Signed message bytes (from Digger.sign())
/// * `public_key_bytes` - Digger's public key
/// 
/// # Returns
/// Result with original message if valid, error if invalid
/// 
/// # Performance
/// - Verification: ~1ms (still fast!)
pub fn verify_signature(
    signed_message: &[u8],
    public_key_bytes: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let public_key = falcon1024::PublicKey::from_bytes(public_key_bytes)
        .map_err(|_| CryptoError::InvalidPublicKey)?;

    let signed = falcon1024::SignedMessage::from_bytes(signed_message)
        .map_err(|_| CryptoError::InvalidSignature)?;

    let message = falcon1024::open(&signed, &public_key)
        .map_err(|_| CryptoError::SignatureVerificationFailed)?;

    Ok(message.to_vec())
}

/// Hash JouleTorqOre data for signing
/// 
/// Creates deterministic SHA256 hash of ore fields.
/// This hash is what gets signed by Falcon-1024.
/// 
/// # Arguments
/// * `contract_id` - Contract ID
/// * `digger_id` - Digger ID
/// * `milestone_index` - Milestone index
/// * `joules_consumed` - Total joules
/// * `robo_stake_paid` - RoboStake amount
/// * `unit_hashes` - Array of JTU hashes (already hashed units)
/// * `timestamp` - ISO8601 timestamp
/// 
/// # Returns
/// 32-byte SHA256 hash
pub fn hash_ore_for_signing(
    contract_id: &str,
    digger_id: &str,
    milestone_index: u32,
    joules_consumed: f64,
    robo_stake_paid: f64,
    unit_hashes: &[String],
    timestamp: &str,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    
    // Hash in deterministic order
    hasher.update(contract_id.as_bytes());
    hasher.update(digger_id.as_bytes());
    hasher.update(&milestone_index.to_le_bytes());
    hasher.update(&joules_consumed.to_le_bytes());
    hasher.update(&robo_stake_paid.to_le_bytes());
    
    // Hash all unit hashes
    for unit_hash in unit_hashes {
        hasher.update(unit_hash.as_bytes());
    }
    
    hasher.update(timestamp.as_bytes());
    
    hasher.finalize().into()
}

/// Crypto error types
#[derive(Debug, Clone, PartialEq)]
pub enum CryptoError {
    InvalidPublicKey,
    InvalidSecretKey,
    InvalidSignature,
    SignatureVerificationFailed,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoError::InvalidPublicKey => write!(f, "Invalid Falcon-1024 public key"),
            CryptoError::InvalidSecretKey => write!(f, "Invalid Falcon-1024 secret key"),
            CryptoError::InvalidSignature => write!(f, "Invalid Falcon-1024 signature format"),
            CryptoError::SignatureVerificationFailed => write!(f, "Falcon-1024 signature verification failed"),
        }
    }
}

impl std::error::Error for CryptoError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = DiggerKeypair::generate();
        
        // Verify key sizes
        assert_eq!(keypair.public_key_bytes().len(), 1793); // Falcon-1024 public key
        assert_eq!(keypair.secret_key_bytes().len(), 2305); // Falcon-1024 secret key
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = DiggerKeypair::generate();
        let message = b"test ore data";
        
        // Sign
        let signed = keypair.sign(message);
        
        // Verify
        let verified = verify_signature(&signed, &keypair.public_key_bytes())
            .expect("Signature should verify");
        
        assert_eq!(verified, message);
    }

    #[test]
    fn test_invalid_signature() {
        let keypair1 = DiggerKeypair::generate();
        let keypair2 = DiggerKeypair::generate();
        
        let message = b"test ore data";
        let signed = keypair1.sign(message);
        
        // Try to verify with wrong public key
        let result = verify_signature(&signed, &keypair2.public_key_bytes());
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CryptoError::SignatureVerificationFailed);
    }

    #[test]
    fn test_keypair_serialization() {
        let keypair = DiggerKeypair::generate();
        
        let public_bytes = keypair.public_key_bytes();
        let secret_bytes = keypair.secret_key_bytes();
        
        // Reconstruct
        let restored = DiggerKeypair::from_bytes(&public_bytes, &secret_bytes)
            .expect("Should reconstruct keypair");
        
        // Verify same keys
        assert_eq!(restored.public_key_bytes(), public_bytes);
        assert_eq!(restored.secret_key_bytes(), secret_bytes);
    }

    #[test]
    fn test_hash_ore_deterministic() {
        let hash1 = hash_ore_for_signing(
            "contract-001",
            "digger-001",
            0,
            5000.0,
            0.05,
            &["unit-hash-1".to_string(), "unit-hash-2".to_string()],
            "2025-11-16T12:00:00Z",
        );

        let hash2 = hash_ore_for_signing(
            "contract-001",
            "digger-001",
            0,
            5000.0,
            0.05,
            &["unit-hash-1".to_string(), "unit-hash-2".to_string()],
            "2025-11-16T12:00:00Z",
        );

        // Same input → same hash
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_ore_different_data() {
        let hash1 = hash_ore_for_signing(
            "contract-001",
            "digger-001",
            0,
            5000.0,
            0.05,
            &["unit-hash-1".to_string()],
            "2025-11-16T12:00:00Z",
        );

        let hash2 = hash_ore_for_signing(
            "contract-001",
            "digger-001",
            0,
            5001.0, // Different joules
            0.05,
            &["unit-hash-1".to_string()],
            "2025-11-16T12:00:00Z",
        );

        // Different input → different hash
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_sign_verify_full_ore_hash() {
        let keypair = DiggerKeypair::generate();
        
        let ore_hash = hash_ore_for_signing(
            "contract-001",
            "digger-001",
            0,
            5000.0,
            0.05,
            &["unit-1".to_string(), "unit-2".to_string()],
            "2025-11-16T12:00:00Z",
        );
        
        // Sign the hash
        let signed = keypair.sign(&ore_hash);
        
        // Verify
        let verified = verify_signature(&signed, &keypair.public_key_bytes())
            .expect("Signature should verify");
        
        assert_eq!(verified, ore_hash);
    }
}
