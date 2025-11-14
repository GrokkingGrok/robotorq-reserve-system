use serde::{Serialize, Deserialize};

// ────────────────────────────────────────────────────────────────
// DATA STRUCTURES — Explained Like You're in High School
// ────────────────────────────────────────────────────────────────

/// This is like a **report card** from the robot (Digger).
/// It tells the system: "Here's what I made this time!"
#[derive(Clone, Serialize, Deserialize)]
pub struct JouleTorqOre {
    /// The name of the robot that did the work
    /// Example: "dig-jon-ai-001"
    pub digger_id: String,

    /// The job number this work belongs to
    /// Example: "contract-001"
    pub contract_id: String,

    /// How many **digital coins** (tokens) the robot earned this time
    /// Example: 60 tokens
    pub tokens_generated: u64,

    /// How much **electricity** (in joules) the robot used
    /// Example: 216000 joules
    pub joules: u64,

    /// Which step of the job this is (like "Step 1 of 5")
    /// Starts at 0
    pub milestone_index: u32,

    /// When this report was made (in seconds since 1970)
    /// Like a timestamp on a photo
    pub timestamp: u64,

    /// Optional: A photo to prove the robot really did the work
    /// Stored as a long text string (base64)
    /// Can be empty (None) if no photo
    pub proof_of_work: Option<String>, // base64 photo

    /// The portion of RoboStake (RT) allocated to this milestone
    /// Example: 0.04166 RT (from 3000 RT / 72000 milestones)
    pub robo_stake_amount: f64,

    /// Post-quantum cryptographic signature (Dilithium)
    /// Used to prove authenticity and prevent tampering
    /// None = unsigned (stub mode), Some(bytes) = signed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Vec<u8>>,
}

impl JouleTorqOre {
    /// Serialize ore data for signing (excludes signature field)
    /// 
    /// This creates the message that will be signed by Dilithium.
    /// The signature is computed over all fields EXCEPT the signature itself.
    /// 
    /// # Returns
    /// * `Vec<u8>` - Serialized bytes ready for signing
    /// 
    /// # Example
    /// ```rust,ignore
    /// let ore = JouleTorqOre { /* ... */ signature: None };
    /// let message = ore.unsigned_bytes();
    /// let signature = crypto::sign_ore(&ore, &private_key);
    /// ore.signature = Some(signature);
    /// ```
    pub fn unsigned_bytes(&self) -> Vec<u8> {
        // Create a temporary struct without the signature
        let unsigned = UnsignedOre {
            digger_id: &self.digger_id,
            contract_id: &self.contract_id,
            tokens_generated: self.tokens_generated,
            joules: self.joules,
            milestone_index: self.milestone_index,
            timestamp: self.timestamp,
            proof_of_work: &self.proof_of_work,
            robo_stake_amount: self.robo_stake_amount,
        };
        
        // Serialize to JSON bytes (deterministic for crypto)
        serde_json::to_vec(&unsigned).unwrap_or_default()
    }
}

/// Helper struct for serializing ore without signature
#[derive(Serialize)]
struct UnsignedOre<'a> {
    digger_id: &'a str,
    contract_id: &'a str,
    tokens_generated: u64,
    joules: u64,
    milestone_index: u32,
    timestamp: u64,
    proof_of_work: &'a Option<String>,
    robo_stake_amount: f64,
}

/// This is the **job contract** — like a work agreement.
#[derive(Clone, Serialize, Deserialize)]
pub struct Contract {
    /// The job number (same as in JouleTorqOre)
    /// Example: "contract-001"
    pub id: String,

    /// Is the job allowed to start?
    /// true = yes, false = no
    pub authorized: bool,

    /// How valuable the work is (higher = more pay)
    /// Example: 5 means 5× normal pay
    pub torq: u16,

    /// The **fastest** the robot can make tokens per second
    /// Example: 60 tokens/sec
    pub max_token_throughput: u64,

    /// How many seconds to wait between each report
    /// Example: 1 = one update per second
    pub interval_seconds: u64,

    /// Total tokens earned so far in the whole job
    /// Starts at 0, goes up
    pub total_tokens: u64,

    /// Total RoboStake (RT) received from Trust for this contract
    /// Example: 3000.0 RT
    pub robo_stake_total: f64,

    /// How long the contract will run (in hours)
    /// Calculated as: robo_stake_total / (power_kw × max_token_throughput)
    /// Example: 20.0 hours
    pub duration_hours: f64,
}

/// This is the **robot's ID card** — who it is and what it can do.
#[derive(Clone, Serialize, Deserialize)]
pub struct DiggerConfig {
    /// The robot's name
    /// Example: "dig-jon-ai-001"
    pub id: String,

    /// How much electricity it uses (in kilowatts)
    /// Example: 2.5 kW
    pub power_kw: f64,

    /// Its **maximum speed** in making tokens per second
    /// Example: 60 tokens/sec
    pub max_token_throughput: u64,
}