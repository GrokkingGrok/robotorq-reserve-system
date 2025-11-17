// HTTP API for Digger contract management
//
// Provides REST endpoints for:
// - Contract creation (with torq + robo_stake)
// - Stake payment
// - Contract execution simulation
// - Status queries
// - JTU queries

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::{info, warn, error};

use crate::config::DiggerConfig;
use crate::contract_state::{ContractStateManager, ApprovalStatus};
use crate::crypto::DiggerKeypair;
use crate::jtu_storage::JtuStorageManager;

// ============================================================================
// API State (shared across handlers)
// ============================================================================

#[derive(Clone)]
pub struct ApiState {
    pub config: Arc<DiggerConfig>,
    pub contract_manager: Arc<Mutex<ContractStateManager>>,
    pub storage_manager: Arc<Mutex<JtuStorageManager>>,
    pub nats_client: async_nats::Client,  // NATS client for hash transmission
    pub keypair: Arc<DiggerKeypair>,      // Falcon-1024 keypair for signing
}

impl ApiState {
    pub fn new(
        config: DiggerConfig,
        contract_manager: ContractStateManager,
        storage_manager: JtuStorageManager,
        nats_client: async_nats::Client,
        keypair: DiggerKeypair,
    ) -> Self {
        Self {
            config: Arc::new(config),
            contract_manager: Arc::new(Mutex::new(contract_manager)),
            storage_manager: Arc::new(Mutex::new(storage_manager)),
            nats_client,
            keypair: Arc::new(keypair),
        }
    }
}

// ============================================================================
// Request/Response DTOs
// ============================================================================

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateContractRequest {
    pub contract_id: String,
    pub torq: f64,              // Scalar ratio (e.g., 100 = 100:1 output/input)
    pub robo_stake: f64,        // RT amount (e.g., 5.0 RT)
    pub milestones: i64,        // Number of milestones
    pub power_watts: f64,       // Robot power (for cross product)
}

#[derive(Debug, Serialize)]
pub struct CreateContractResponse {
    pub contract_id: String,
    pub ore_target: f64,        // Calculated: torq × robo_stake
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PayStakeRequest {
    pub contract_id: String,
}

#[derive(Debug, Serialize)]
pub struct PayStakeResponse {
    pub contract_id: String,
    pub approval_status: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExecuteContractRequest {
    pub contract_id: String,
    pub duration_seconds: u64,  // How long to simulate work
}

#[derive(Debug, Serialize)]
pub struct ExecuteContractResponse {
    pub contract_id: String,
    pub jtus_generated: i64,
    pub ore_generated: f64,
    pub ore_target: f64,
    pub target_reached: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ContractStatusResponse {
    pub contract_id: String,
    pub torq: f64,
    pub robo_stake: f64,
    pub ore_target: f64,
    pub ore_generated: f64,
    pub target_reached: bool,
    pub approval_status: String,
    pub jtu_count: i64,
    pub milestones_total: i64,
    pub milestones_completed: i64,
    pub progress: f64,
    pub is_complete: bool,
}

#[derive(Debug, Serialize)]
pub struct JtuQueryResponse {
    pub contract_id: String,
    pub jtu_count: i64,
    pub hash_count: i64,
    pub total_joules: f64,
    pub total_stake: f64,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

// ============================================================================
// HTTP Handlers
// ============================================================================

/// POST /contracts/create - Create new contract
///
/// Creates a contract with specified torq (ratio) and robo_stake (RT).
/// Calculates ore_target = torq × robo_stake automatically.
///
/// Example:
///   POST /contracts/create
///   {
///     "contract_id": "contract-001",
///     "torq": 100.0,           // 100:1 ratio
///     "robo_stake": 5.0,       // 5 RT
///     "milestones": 10,
///     "power_watts": 2000.0
///   }
///
/// Response:
///   {
///     "contract_id": "contract-001",
///     "ore_target": 500.0,     // 100 × 5 = 500 RT
///     "message": "Contract created successfully"
///   }
pub async fn create_contract(
    State(state): State<ApiState>,
    Json(req): Json<CreateContractRequest>,
) -> Result<Json<CreateContractResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Creating contract: {} (torq: {}, stake: {}, power: {}W)",
        req.contract_id, req.torq, req.robo_stake, req.power_watts
    );

    // Validation
    if req.torq <= 0.0 {
        warn!("Invalid torq: {}", req.torq);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Torq must be positive".to_string(),
            }),
        ));
    }

    if req.robo_stake <= 0.0 {
        warn!("Invalid robo_stake: {}", req.robo_stake);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "RoboStake must be positive".to_string(),
            }),
        ));
    }

    if req.milestones <= 0 {
        warn!("Invalid milestones: {}", req.milestones);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Milestones must be positive".to_string(),
            }),
        ));
    }

    if req.power_watts <= 0.0 {
        warn!("Invalid power: {}", req.power_watts);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Power must be positive".to_string(),
            }),
        ));
    }

    // Create contract
    let mut manager = state.contract_manager.lock().unwrap();
    
    match manager.create_contract(
        req.contract_id.clone(),
        req.torq,
        req.robo_stake,
        req.milestones,
        req.power_watts,
    ) {
        Ok(_) => {
            let ore_target = req.torq * req.robo_stake;
            
            info!(
                "Contract {} created: ore_target = {} RT",
                req.contract_id, ore_target
            );

            Ok(Json(CreateContractResponse {
                contract_id: req.contract_id,
                ore_target,
                message: "Contract created successfully. Pay stake to approve.".to_string(),
            }))
        }
        Err(e) => {
            error!("Failed to create contract: {}", e);
            Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse { error: e }),
            ))
        }
    }
}

/// POST /contracts/stake - Pay RoboStake and approve contract
///
/// Transitions contract from PendingStake → StakeApproved.
///
/// Example:
///   POST /contracts/stake
///   { "contract_id": "contract-001" }
pub async fn pay_stake(
    State(state): State<ApiState>,
    Json(req): Json<PayStakeRequest>,
) -> Result<Json<PayStakeResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Paying stake for contract: {}", req.contract_id);

    let mut manager = state.contract_manager.lock().unwrap();
    
    let contract = match manager.get_mut(&req.contract_id) {
        Some(c) => c,
        None => {
            warn!("Contract not found: {}", req.contract_id);
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Contract {} not found", req.contract_id),
                }),
            ));
        }
    };

    match contract.pay_stake() {
        Ok(_) => {
            info!("Stake paid for contract: {}", req.contract_id);
            
            Ok(Json(PayStakeResponse {
                contract_id: req.contract_id,
                approval_status: "StakeApproved".to_string(),
                message: "Stake paid, contract approved for execution".to_string(),
            }))
        }
        Err(e) => {
            error!("Failed to pay stake: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse { error: e }),
            ))
        }
    }
}

/// POST /contracts/execute - Simulate contract execution
///
/// Generates JTUs for the specified duration.
/// Uses cross product: JTU/sec = (tokens/sec) × (watts)
///
/// TODO(phase-4-crypto): Add robot identity to JTU generation!
/// - Currently NOT passing digger_id to JTUs (they exist but unused!)
/// - Need to track which robot(s) generated which JTUs
/// - Must support multi-robot contracts
/// - Each JTU must be cryptographically signed by generating robot
///
/// Example:
///   POST /contracts/execute
///   {
///     "contract_id": "contract-001",
///     "duration_seconds": 10
///   }
pub async fn execute_contract(
    State(state): State<ApiState>,
    Json(req): Json<ExecuteContractRequest>,
) -> Result<Json<ExecuteContractResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Executing contract: {} for {} seconds",
        req.contract_id, req.duration_seconds
    );

    let mut manager = state.contract_manager.lock().unwrap();
    
    let contract = match manager.get_mut(&req.contract_id) {
        Some(c) => c,
        None => {
            warn!("Contract not found: {}", req.contract_id);
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Contract {} not found", req.contract_id),
                }),
            ));
        }
    };

    // Must be approved
    if contract.approval_status != ApprovalStatus::StakeApproved
        && contract.approval_status != ApprovalStatus::ExecutionComplete
    {
        warn!(
            "Contract {} not approved (status: {:?})",
            req.contract_id, contract.approval_status
        );
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Contract must be approved before execution".to_string(),
            }),
        ));
    }

    // Generate JTUs
    // Cross product: JTU/sec = (tokens/sec) × (watts)
    // For simplicity, assume 1 token/sec
    let tokens_per_sec = 1.0;
    let jtus_per_sec = (tokens_per_sec * contract.power_watts) as i64;
    let jtus_generated = jtus_per_sec * req.duration_seconds as i64;
    
    // Calculate ore value per JTU
    let ore_value_per_jtu = contract.robo_stake / contract.ore_target;
    let ore_generated = jtus_generated as f64 * ore_value_per_jtu;
    
    // Calculate joules and robo_stake per JTU (distribute evenly)
    let total_joules = contract.power_watts * req.duration_seconds as f64;
    let joules_per_jtu = total_joules / jtus_generated as f64;
    let robo_stake_per_jtu = ore_value_per_jtu;
    
    // Get current JTU count for token indexing
    let starting_index = contract.jtu_count;
    
    // Create actual JTUs
    let mut jtus = Vec::with_capacity(jtus_generated as usize);
    let now = chrono::Utc::now().timestamp();
    
    for i in 0..jtus_generated {
        let token_index = starting_index + i;
        let token_id = format!("{}-t{}", req.contract_id, token_index);
        
        // Create JTU
        let jtu = crate::jtu_storage::JouleTorqUnit {
            hash: crate::jtu_hasher::calculate_jtu_hash(
                &token_id,
                joules_per_jtu,
                robo_stake_per_jtu,
                now,
                &req.contract_id,
                &state.config.digger_id,
            ),
            signature: crate::jtu_hasher::create_placeholder_signature(), // TODO(phase-4): Falcon-1024
            digger_id: state.config.digger_id.clone(),
            contract_id: req.contract_id.clone(),
            token_index,
            milestone_index: 0, // Simplified for now
            timestamp: now,
            joules_consumed: joules_per_jtu,
            robo_stake_paid: robo_stake_per_jtu,
        };
        
        jtus.push(jtu);
    }
    
    // Store JTUs in database
    {
        let storage = state.storage_manager.lock().unwrap();
        if let Err(e) = storage.insert_batch(&jtus) {
            error!("Failed to store JTUs: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to store JTUs: {}", e),
                }),
            ));
        }
    }
    
    info!("Stored {} JTUs in database for contract {}", jtus_generated, req.contract_id);

    // Update contract state
    contract.add_jtus(jtus_generated, ore_generated);

    let ore_target = contract.ore_target;
    let total_ore = contract.ore_generated;
    let target_reached = contract.is_ore_target_reached();

    info!(
        "Contract {} executed: {} JTUs generated, {} RT ore ({:.1}% of target)",
        req.contract_id,
        jtus_generated,
        total_ore,
        (total_ore / ore_target) * 100.0
    );

    Ok(Json(ExecuteContractResponse {
        contract_id: req.contract_id,
        jtus_generated,
        ore_generated: total_ore,
        ore_target,
        target_reached,
        message: if target_reached {
            "Ore target reached! Contract ready for minting.".to_string()
        } else {
            format!(
                "Generated {} RT of {} RT target ({:.1}% complete)",
                total_ore,
                ore_target,
                (total_ore / ore_target) * 100.0
            )
        },
    }))
}

/// GET /contracts/:id - Get contract status
///
/// Example:
///   GET /contracts/contract-001
pub async fn get_contract_status(
    State(state): State<ApiState>,
    Path(contract_id): Path<String>,
) -> Result<Json<ContractStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    let manager = state.contract_manager.lock().unwrap();
    
    let contract = match manager.get(&contract_id) {
        Some(c) => c,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Contract {} not found", contract_id),
                }),
            ));
        }
    };

    Ok(Json(ContractStatusResponse {
        contract_id: contract.contract_id.clone(),
        torq: contract.torq,
        robo_stake: contract.robo_stake,
        ore_target: contract.ore_target,
        ore_generated: contract.ore_generated,
        target_reached: contract.is_ore_target_reached(),
        approval_status: format!("{:?}", contract.approval_status),
        jtu_count: contract.jtu_count,
        milestones_total: contract.milestones_total,
        milestones_completed: contract.milestones_completed,
        progress: contract.progress(),
        is_complete: contract.is_complete(),
    }))
}

/// GET /jtus/:contract_id - Query JTU stats for contract
///
/// Example:
///   GET /jtus/contract-001
pub async fn get_jtu_stats(
    State(state): State<ApiState>,
    Path(contract_id): Path<String>,
) -> Result<Json<JtuQueryResponse>, (StatusCode, Json<ErrorResponse>)> {
    let storage = state.storage_manager.lock().unwrap();
    
    // Get stats from storage
    let stats = storage.get_stats(&contract_id).map_err(|e| {
        error!("Failed to get JTU stats: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to query JTUs: {}", e),
            }),
        )
    })?;

    // Get hash count
    let hashes = storage.get_all_hashes(&contract_id).map_err(|e| {
        error!("Failed to get hashes: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to query hashes: {}", e),
            }),
        )
    })?;

    Ok(Json(JtuQueryResponse {
        contract_id,
        jtu_count: stats.jtu_count,
        hash_count: hashes.len() as i64,
        total_joules: stats.total_joules,
        total_stake: stats.total_robo_stake,
    }))
}

/// GET /health - Health check
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "digger",
        "version": "0.2.0"
    }))
}

/// GET / - Root endpoint
pub async fn root() -> impl IntoResponse {
    "Digger v0.2.0 - Contract Management API\n\nEndpoints:\n\
     POST   /contracts/create - Create contract\n\
     POST   /contracts/stake  - Pay stake\n\
     POST   /contracts/execute - Execute contract\n\
     GET    /contracts/:id    - Get contract status\n\
     GET    /jtus/:id         - Get JTU stats\n\
     GET    /health           - Health check\n"
}

// ============================================================================
// Router Setup
// ============================================================================

pub fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/contracts/create", post(create_contract))
        .route("/contracts/stake", post(pay_stake))
        .route("/contracts/execute", post(execute_contract))
        .route("/contracts/{id}", get(get_contract_status))
        .route("/jtus/{id}", get(get_jtu_stats))
        .with_state(state)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DiggerConfig;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn create_test_state() -> Option<ApiState> {
        let config = DiggerConfig::default_test();
        let contract_manager = ContractStateManager::new();
        let storage_manager = JtuStorageManager::new(
            std::env::temp_dir().join("test_digger_api")
        ).expect("Failed to create test storage");
        
        // Try to connect to NATS - if not available, return None
        let nats_client = match async_nats::connect("nats://localhost:4222").await {
            Ok(client) => client,
            Err(_) => {
                eprintln!("⚠️  NATS not available - skipping test (this is OK in CI)");
                return None;
            }
        };

        // Generate test keypair for signing
        let keypair = DiggerKeypair::generate();

        Some(ApiState::new(config, contract_manager, storage_manager, nats_client, keypair))
    }

    #[tokio::test]
    async fn test_root_endpoint() {
        let Some(state) = create_test_state().await else {
            eprintln!("Skipping test_root_endpoint - NATS not available");
            return;
        };
        let app = create_router(state);

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_health_check() {
        let Some(state) = create_test_state().await else {
            eprintln!("Skipping test_health_check - NATS not available");
            return;
        };
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_contract() {
        let Some(state) = create_test_state().await else {
            eprintln!("Skipping test_create_contract - NATS not available");
            return;
        };
        let app = create_router(state);

        let body = serde_json::json!({
            "contract_id": "test-001",
            "torq": 100.0,
            "robo_stake": 5.0,
            "milestones": 10,
            "power_watts": 2000.0
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/contracts/create")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
