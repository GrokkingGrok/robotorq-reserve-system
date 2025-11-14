// ════════════════════════════════════════════════════════════════
// TODO #4: Refinery HTTP Client
// ════════════════════════════════════════════════════════════════
//
// GOAL: Send signed JouleTorqOre batches to the Refinery service
//
// CURRENT STATE: Module exists but not implemented
// NEEDED: HTTP client using reqwest to call Refinery API
//
// ────────────────────────────────────────────────────────────────
// IMPLEMENTATION STEPS
// ────────────────────────────────────────────────────────────────
//
// STEP 1: Add reqwest dependency to Cargo.toml
// [dependencies]
// reqwest = { version = "0.11", features = ["json"] }
//
// STEP 2: Implement send_ore_to_refinery()
// - Serialize JouleTorqOre to JSON
// - POST to http://<refinery_host>:8100/receive-ore
// - Handle HTTP response (200 OK, 400 Bad Request, 503 Service Unavailable)
// - Return Result<(), RefineryError>
//
// STEP 3: Error Handling
// - Network errors (connection refused, timeout)
// - HTTP errors (4xx, 5xx)
// - JSON serialization errors
// - Define RefineryError enum with all cases
//
// STEP 4: Configuration
// - Refinery URL from environment variable: REFINERY_URL
// - Default: http://localhost:8100
// - Timeout: 5 seconds (configurable)
//
// STEP 5: Retry Logic (Future Enhancement)
// - Retry transient errors (503, network timeout)
// - Exponential backoff: 1s, 2s, 4s, 8s
// - Max retries: 3
// - Dead-letter queue for permanent failures
//
// ────────────────────────────────────────────────────────────────
// INTEGRATION WITH DIGGER.RS
// ────────────────────────────────────────────────────────────────
//
// When digger.rs completes a milestone:
// 1. Generate JouleTorqOre (joules + robo_stake_amount)
// 2. Sign ore using crypto::sign_ore()
// 3. Call send_ore_to_refinery(&ore)
// 4. Update milestone status based on result (TODO #8)
// 5. Emit status event to UI (TODO #9)
//
// ────────────────────────────────────────────────────────────────
// REFINERY ENDPOINT (TODO #5 - Separate Refinery Refactor)
// ────────────────────────────────────────────────────────────────
//
// POST /receive-ore
// Content-Type: application/json
//
// Request Body: JouleTorqOre JSON
// {
//   "digger_id": "dig-jon-ai-001",
//   "contract_id": "contract-001",
//   "tokens_generated": 60,
//   "joules": 216000,
//   "milestone_index": 0,
//   "timestamp": 1699999999,
//   "proof_of_work": "base64string",
//   "robo_stake_amount": 150.0,    // NEW: TODO #1
//   "signature": [1,2,3,...]        // NEW: TODO #2
// }
//
// Response 200 OK:
// {
//   "status": "accepted",
//   "ingot_id": "ingot-12345"
// }
//
// Response 400 Bad Request:
// {
//   "error": "invalid_signature",
//   "message": "Dilithium signature verification failed"
// }
//
// Response 503 Service Unavailable:
// {
//   "error": "queue_full",
//   "message": "Refinery queue at capacity, retry later"
// }
//
// ════════════════════════════════════════════════════════════════

use crate::types::JouleTorqOre;
use std::error::Error;
use std::fmt;
use std::time::Duration;
use serde::Deserialize;

/// Errors that can occur when communicating with Refinery
#[derive(Debug)]
pub enum RefineryError {
    NetworkError(String),
    HttpError { status: u16, message: String },
    SerializationError(String),
    InvalidSignature,
    QueueFull,
}

impl fmt::Display for RefineryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RefineryError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            RefineryError::HttpError { status, message } => {
                write!(f, "HTTP {} error: {}", status, message)
            }
            RefineryError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            RefineryError::InvalidSignature => write!(f, "Refinery rejected invalid signature"),
            RefineryError::QueueFull => write!(f, "Refinery queue full, retry later"),
        }
    }
}

impl Error for RefineryError {}

/// Response from Refinery when ore is accepted
#[derive(Deserialize)]
struct RefineryResponse {
    status: String,
    #[allow(dead_code)]
    ingot_id: Option<String>,
}

/// Error response from Refinery
#[derive(Deserialize)]
struct RefineryErrorResponse {
    error: String,
    message: String,
}

/// Send a JouleTorqOre batch to the Refinery service
///
/// This function uses reqwest to POST the ore batch to the Refinery's
/// /receive-ore endpoint. The Refinery will validate the signature,
/// process the ore, and store it in the ingot queue.
///
/// # Arguments
/// * `ore` - The signed ore batch to send
///
/// # Returns
/// * `Result<(), RefineryError>` - Ok if accepted, Err if failed
///
/// # Environment Variables
/// * `REFINERY_URL` - Base URL for Refinery (default: http://localhost:8100)
///
/// # Example
/// ```rust,ignore
/// let ore = JouleTorqOre { /* ... */ };
/// match send_ore_to_refinery(&ore) {
///     Ok(()) => println!("Ore accepted by Refinery!"),
///     Err(RefineryError::QueueFull) => println!("Refinery busy, retry later"),
///     Err(e) => println!("Failed to send ore: {}", e),
/// }
/// ```
pub fn send_ore_to_refinery(ore: &JouleTorqOre) -> Result<(), RefineryError> {
    // Get Refinery URL from environment or use default
    let refinery_url = std::env::var("REFINERY_URL")
        .unwrap_or_else(|_| "http://localhost:8100".to_string());
    
    println!(
        "📤 Sending ore to Refinery: contract={}, milestone={}, robo_stake={}, url={}",
        ore.contract_id,
        ore.milestone_index,
        ore.robo_stake_amount,
        refinery_url
    );

    // Create HTTP client with timeout
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| RefineryError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

    // Send POST request to Refinery
    let response = client
        .post(format!("{}/receive-ore", refinery_url))
        .json(&ore)
        .send()
        .map_err(|e| {
            if e.is_timeout() {
                RefineryError::NetworkError("Request timeout".to_string())
            } else if e.is_connect() {
                RefineryError::NetworkError(format!("Connection refused: {}", e))
            } else {
                RefineryError::NetworkError(e.to_string())
            }
        })?;

    // Handle response based on status code
    match response.status().as_u16() {
        200 => {
            // Success - ore accepted
            let refinery_resp: RefineryResponse = response
                .json()
                .map_err(|e| RefineryError::SerializationError(e.to_string()))?;
            
            println!("✅ Refinery accepted ore: status={}", refinery_resp.status);
            Ok(())
        }
        400 => {
            // Bad request - likely invalid signature
            match response.json::<RefineryErrorResponse>() {
                Ok(err_resp) if err_resp.error == "invalid_signature" => {
                    println!("❌ Refinery rejected ore: invalid signature");
                    Err(RefineryError::InvalidSignature)
                }
                Ok(err_resp) => {
                    println!("❌ Refinery rejected ore: {}", err_resp.message);
                    Err(RefineryError::HttpError {
                        status: 400,
                        message: err_resp.message,
                    })
                }
                Err(e) => {
                    println!("❌ Refinery error (failed to parse): {}", e);
                    Err(RefineryError::HttpError {
                        status: 400,
                        message: "Bad request".to_string(),
                    })
                }
            }
        }
        503 => {
            // Service unavailable - queue full
            println!("⚠️  Refinery queue full, should retry later");
            Err(RefineryError::QueueFull)
        }
        status => {
            // Other HTTP error
            let message = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            println!("❌ Refinery HTTP error {}: {}", status, message);
            Err(RefineryError::HttpError { status, message })
        }
    }
}
