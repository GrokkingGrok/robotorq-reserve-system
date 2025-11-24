use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use common::triples::{Triple, decimal_to_triple, JOULETORQ_PER_ROBOTORQ, ORE_PER_INGOT};

/// TokenTorqIngot: Aggregate of 3600 JouleTorqUnits from Refinery.
/// Represents one "ingot" of value ready for minting into certificates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTorqIngot {
    /// Unique ingot identifier (e.g., "ingot-20251115-210012.547794")
    pub ingot_id: String,

    /// Contract that produced this ingot
    pub contract_id: String,

    /// Total joules consumed across all 3600 units (integer joule units)
    pub joules_total: i64,

    /// Total robo-stake paid (in joule-torq units, integer smallest unit)
    pub robostake_total_jouletorq: i64,

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

impl TokenTorqIngot {
    /// Derive a Triple decomposition of the joule total.
    pub fn triple(&self) -> Triple { decimal_to_triple(&common::triples::Decimal::new(self.joules_total, 0)).expect("valid triple conversion") }

    /// Add another ingot's economic value in-place (mutating joules & robostake).
    pub fn accumulate(&mut self, other: &TokenTorqIngot) {
        self.joules_total += other.joules_total;
        self.robostake_total_jouletorq += other.robostake_total_jouletorq;
    }

    /// Validate structural and economic invariants.
    pub fn validate(&self) -> Result<(), String> {
        if self.unit_count != ORE_PER_INGOT as u32 { return Err(format!("unit_count {} != {}", self.unit_count, ORE_PER_INGOT)); }
        if self.joules_total != self.unit_count as i64 { return Err(format!("joules_total {} should equal unit_count {} (1J/unit)", self.joules_total, self.unit_count)); }
        if self.joules_total < 0 || self.robostake_total_jouletorq < 0 { return Err("negative totals not allowed".into()); }
        if self.unit_hashes.len() != self.unit_count as usize { return Err(format!("unit_hashes length {} != unit_count {}", self.unit_hashes.len(), self.unit_count)); }
        if self.merkle_root.len() != 64 { return Err(format!("merkle_root length {} != 64", self.merkle_root.len())); }
        Ok(())
    }

    /// Total RoboTorq equivalent as fractional (in whole RoboTorq units) for reporting.
    pub fn robotorq_equivalent(&self) -> f64 {
        self.robostake_total_jouletorq as f64 / JOULETORQ_PER_ROBOTORQ as f64
    }
}