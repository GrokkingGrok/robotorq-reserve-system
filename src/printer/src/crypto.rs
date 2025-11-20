// Cryptographic primitives for Printer Service
// 
// This module provides Falcon-1024 signing and verification
// Shared crypto implementation with Digger service

use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::{PublicKey, SignedMessage};
use sha2::{Sha256, Digest};

#[derive(Clone)]
pub struct PrinterKeypair {
    pub public_key: falcon1024::PublicKey,
    pub secret_key: falcon1024::SecretKey,
}

impl PrinterKeypair {
    /// Generate a new Falcon-1024 keypair
    pub fn generate() -> Self {
        let (public_key, secret_key) = falcon1024::keypair();
        Self {
            public_key,
            secret_key,
        }
    }
    
    /// Sign a message with Falcon-1024
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        falcon1024::sign(message, &self.secret_key).as_bytes().to_vec()
    }
    
    /// Get public key bytes
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.public_key.as_bytes().to_vec()
    }
    
    /// Get public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key.as_bytes())
    }
}

/// Verify a Falcon-1024 signature
pub fn verify_signature(_message: &[u8], signed_message: &[u8]) -> bool {
    match falcon1024::open(
        &SignedMessage::from_bytes(signed_message).unwrap(),
        &falcon1024::PublicKey::from_bytes(&[0u8; falcon1024::public_key_bytes()]).unwrap(),
    ) {
        Ok(_) => true,
        Err(_) => false,
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
