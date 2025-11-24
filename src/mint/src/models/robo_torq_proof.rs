use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// RoboTorqProof: Complete proof chain for a certificate.
/// Contains the full merkle tree and all signatures for verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoboTorqProof {
    /// Unique proof identifier
    pub proof_id: String,

    /// Associated certificate ID
    pub certificate_id: String,

    /// Merkle root of all ingot hashes in this certificate
    pub merkle_root: String,

    /// Full merkle tree for verification
    pub merkle_tree: Vec<String>,

    /// Individual ingot hashes (1000 for one certificate)
    pub ingot_hashes: Vec<String>,

    /// Digital signatures from all contributing parties
    pub signatures: Vec<ProofSignature>,

    /// Creation timestamp
    pub timestamp: SystemTime,

    /// Proof hash for integrity
    pub proof_hash: String,
}

/// Individual signature in the proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofSignature {
    /// Signer identifier (e.g., "refinery-001", "mint-001")
    pub signer_id: String,

    /// Signature algorithm (e.g., "SPHINCS+")
    pub algorithm: String,

    /// Detached signed message blob (algorithm-specific)
    pub signature: Vec<u8>,

    /// SHA-256 hash (hex) of the signed message payload
    pub message_hash: String,

    /// Fingerprint of the public key (SHA-256 hex of key bytes)
    pub key_fingerprint: String,

    /// Public key bytes (needed for verification by downstream services)
    pub public_key: Vec<u8>,

    /// Timestamp of signing
    pub timestamp: SystemTime,
}