//! Cryptographic operations for the wallet service
//!
//! Provides optional cryptographic features with conditional compilation.

#[cfg(feature = "crypto")]
use common::crypto::{parse_kind, new_algorithm, SignatureAlgorithm};
#[cfg(feature = "crypto")]
use sha2::{Sha256, Digest};
#[cfg(feature = "crypto")]
use sha256;

/// Cryptographic utilities
#[cfg(feature = "crypto")]
pub struct Crypto {
    algo: Box<dyn SignatureAlgorithm + Send + Sync>,
}

#[cfg(feature = "crypto")]
impl Crypto {
    /// Generate a new Falcon keypair
    pub fn new(algorithm: &str) -> anyhow::Result<Self> {
        let kind = parse_kind(algorithm)?;
        let algo = new_algorithm(kind)?;
        Ok(Self { algo })
    }
    pub fn public_key(&self) -> &[u8] { self.algo.public_key() }
    pub fn hash(data: &[u8]) -> String {
        let mut h = Sha256::new(); h.update(data); format!("{:x}", h.finalize())
    }
    pub fn sign_hash(&self, message_hash_hex: &str) -> anyhow::Result<Vec<u8>> {
        let bytes = hex::decode(message_hash_hex)?; Ok(self.algo.sign(&bytes)?)
    }
    pub fn verify_hash(&self, message_hash_hex: &str, signature: &[u8]) -> anyhow::Result<bool> {
        let bytes = hex::decode(message_hash_hex)?; Ok(self.algo.verify(&bytes, signature))
    }
}

/// Stub implementation when crypto features are disabled
#[cfg(not(feature = "crypto"))]
pub struct Crypto;

#[cfg(not(feature = "crypto"))]
impl Crypto {
    pub fn generate_keypair() -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
        anyhow::bail!("Crypto features not enabled - compile with --features crypto")
    }

    pub fn sign(_data: &[u8], _secret_key: &[u8]) -> anyhow::Result<Vec<u8>> {
        anyhow::bail!("Crypto features not enabled - compile with --features crypto")
    }

    pub fn verify(_data: &[u8], _signature: &[u8], _public_key: &[u8]) -> anyhow::Result<bool> {
        anyhow::bail!("Crypto features not enabled - compile with --features crypto")
    }

    pub fn hash(data: &[u8]) -> String {
        // Fallback to simple hash when crypto not enabled
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_functionality() {
        let data = b"test data";
        let hash1 = Crypto::hash(data);
        let hash2 = Crypto::hash(data);
        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());
    }

    #[cfg(feature = "crypto")]
    #[test]
    fn test_falcon_crypto() {
        let (pk, sk) = Crypto::generate_keypair().unwrap();
        assert!(!pk.is_empty());
        assert!(!sk.is_empty());

        let data = b"test message";
        let signature = Crypto::sign(data, &sk).unwrap();
        assert!(!signature.is_empty());

        let is_valid = Crypto::verify(data, &signature, &pk).unwrap();
        assert!(is_valid);

        // Test with wrong data
        let wrong_data = b"wrong message";
        let is_valid_wrong = Crypto::verify(wrong_data, &signature, &pk).unwrap();
        assert!(!is_valid_wrong);
    }

    #[cfg(not(feature = "crypto"))]
    #[test]
    fn test_crypto_disabled() {
        assert!(Crypto::generate_keypair().is_err());
        assert!(Crypto::sign(b"test", b"key").is_err());
        assert!(Crypto::verify(b"test", b"sig", b"key").is_err());
    }
}