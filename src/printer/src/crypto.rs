// Cryptographic primitives for Printer Service
// 
// This module provides Falcon-1024 signing and verification
// Shared crypto implementation with Digger service

use anyhow::Result;
use oqs::{sig::Sig, sig::Algorithm};
use sha2::{Sha256, Digest};

pub struct PrinterKeypair {
    secret_key: Vec<u8>,
    public_key: Vec<u8>,
}

impl PrinterKeypair {
    /// Generate a new Falcon-1024 keypair
    pub fn generate() -> Result<Self> {
        let sig = Sig::new(Algorithm::Falcon1024)?;
        let (public_key, secret_key) = sig.keypair()?;
        
        Ok(Self {
            secret_key: secret_key.into_vec(),
            public_key: public_key.into_vec(),
        })
    }
    
    /// Load keypair from bytes
    pub fn from_bytes(public_key: Vec<u8>, secret_key: Vec<u8>) -> Self {
        Self {
            secret_key,
            public_key,
        }
    }
    
    /// Sign a message with Falcon-1024
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        let sig = Sig::new(Algorithm::Falcon1024)?;
        let signature = sig.sign(message, &self.secret_key)?;
        Ok(signature.into_vec())
    }
    
    /// Get public key bytes
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }
    
    /// Get public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(&self.public_key)
    }
}

/// Verify a Falcon-1024 signature
pub fn verify_signature(message: &[u8], signature: &[u8], public_key: &[u8]) -> Result<bool> {
    let sig = Sig::new(Algorithm::Falcon1024)?;
    match sig.verify(message, signature, public_key) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Hash data with SHA-256
pub fn hash_sha256(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Hash data with SHA-256 and return hex string
pub fn hash_sha256_hex(data: &[u8]) -> String {
    hex::encode(hash_sha256(data))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_keypair_generation() {
        let keypair = PrinterKeypair::generate().unwrap();
        assert!(!keypair.public_key.is_empty());
        assert!(!keypair.secret_key.is_empty());
    }
    
    #[test]
    fn test_sign_and_verify() {
        let keypair = PrinterKeypair::generate().unwrap();
        let message = b"test message";
        
        let signature = keypair.sign(message).unwrap();
        let valid = verify_signature(message, &signature, keypair.public_key()).unwrap();
        
        assert!(valid);
    }
    
    #[test]
    fn test_sha256_hash() {
        let data = b"test data";
        let hash = hash_sha256(data);
        assert_eq!(hash.len(), 32); // SHA-256 produces 32 bytes
    }
}
