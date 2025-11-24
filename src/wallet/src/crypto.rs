//! Cryptographic operations for the wallet service
//!
//! Provides optional cryptographic features with conditional compilation.

#[cfg(feature = "crypto")]
use anyhow::Result;
#[cfg(feature = "crypto")]
use pqcrypto_falcon::falcon512;
#[cfg(feature = "crypto")]
use sha256;

/// Cryptographic utilities
#[cfg(feature = "crypto")]
pub struct Crypto;

#[cfg(feature = "crypto")]
impl Crypto {
    /// Generate a new Falcon keypair
    pub fn generate_keypair() -> Result<(Vec<u8>, Vec<u8>)> {
        let (pk, sk) = falcon512::keypair();
        Ok((pk, sk))
    }

    /// Sign data with Falcon private key
    pub fn sign(data: &[u8], secret_key: &[u8]) -> Result<Vec<u8>> {
        let sk = falcon512::SecretKey::from_bytes(secret_key)?;
        let signature = falcon512::sign(data, &sk);
        Ok(signature)
    }

    /// Verify Falcon signature
    pub fn verify(data: &[u8], signature: &[u8], public_key: &[u8]) -> Result<bool> {
        let pk = falcon512::PublicKey::from_bytes(public_key)?;
        let is_valid = falcon512::verify(data, signature, &pk).is_ok();
        Ok(is_valid)
    }

    /// Hash data with SHA256
    pub fn hash(data: &[u8]) -> String {
        sha256::digest(data)
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