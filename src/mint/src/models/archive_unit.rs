use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoboTorqProof {
    pub proof_id: String,
    pub merkle_root: String,
    pub tree_height: u32,
    pub contract_ids: Vec<String>,
    pub minted_at_nanos: i64,
    pub signature: Vec<u8>,      // SPHINCS+ signature
    pub public_key: Vec<u8>,     // Mint's SPHINCS+ public key
}