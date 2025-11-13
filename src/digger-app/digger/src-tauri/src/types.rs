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