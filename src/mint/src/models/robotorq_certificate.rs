use serde::{Deserialize, Serialize};

pub const INGOTS_PER_CERT: usize = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoboTorqCertificate {
    pub cert_id: String,
    /// Link to Mint-side proof (RoboTorqProof) retaining full merkle + signature
    pub robotorq_proof_id: String,
    /// Cached merkle root (duplicated from proof for Vault index/lookups)
    pub merkle_root: String,
    /// All contracts that contributed work (can be large; kept as vector)
    pub contract_ids: Vec<String>,
    /// Minting timestamp (unix nanos for precision)
    pub timestamp_nanos: i64,
    /// Deterministic SHA-256 over serialized certificate minus this field
    pub hash: String,
    pub status: CertStatus,
    pub bearer_bond_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CertStatus {
    Digital,
    OffGrid,
}