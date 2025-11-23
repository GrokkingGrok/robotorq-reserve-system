use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoboTorqCertificate {
    pub cert_id: String,
    pub merkle_root: String,
    pub tree_height: Option<u32>,
    pub contract_ids: Vec<String>,
    pub minted_at: i64,
}
