use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// TokenTorqIngot: Aggregate of 3600 JouleTorqUnits from Refinery.
/// Represents one "ingot" of value ready for minting into certificates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTorqIngot {
    /// Unique ingot identifier (e.g., "ingot-20251115-210012.547794")
    pub ingot_id: String,

    /// Contract that produced this ingot
    pub contract_id: String,

    /// Total joules consumed across all 3600 units
    pub joules_total: f64,

    /// Total robo-stake paid (in micro-RT units)
    pub robostake_total_micro_rt: i64,

    /// Number of units in this ingot (should be 3600)
    pub unit_count: u32,

    /// Merkle root hash of all unit hashes
    pub merkle_root: String,

    /// Individual unit hashes for proof construction
    pub unit_hashes: Vec<String>,

    /// Timestamp when ingot was created
    pub timestamp: SystemTime,

    /// Digital signature from Refinery
    pub signature: Vec<u8>,
}