use serde::{Deserialize, Serialize};
use common::triples::{Triple, jouletorq_to_triple};

// Legacy constant retained for historical reference; mark unused to silence warnings.
#[allow(dead_code)]
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
    /// Total joule-torq across all ingots (smallest unit, integer)
    pub total_jouletorq: i64,
    /// Total robo-stake aggregated from ingots (integer smallest unit). Not included in hash payload yet.
    pub total_stake_jouletorq: i64,
    /// Triple decomposition of total_jouletorq (robotorq, ingot remainder, joule remainder)
    pub total_triple: Triple,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CertStatus {
    Digital,
    OffGrid,
}

impl RoboTorqCertificate {
    /// Construct Triple from total joule-torq; caller must ensure non-negative.
    pub fn derive_triple(&mut self) {
        self.total_triple = jouletorq_to_triple(self.total_jouletorq);
    }
}