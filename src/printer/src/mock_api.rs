use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use warp::Filter;

use crate::mock_klipper::MockKlipperClient;

/// Shared state for contract tracking between mock API and printer service
#[derive(Debug, Clone, Default)]
pub struct MockContractState {
    pub assigned_contract_id: Option<String>,
}

#[derive(Serialize)]
struct StatusResponse {
    state: String,
    is_printing: bool,
    duration_secs: f64,
    assigned_contract: Option<String>,
}

#[derive(Deserialize)]
struct StartPrintRequest {
    #[serde(default)]
    pub contract_id: Option<String>,
}

#[derive(Deserialize)]
struct AssignContractRequest {
    pub contract_id: String,
}

/// Start the mock API server for controlling the mock printer
pub async fn start_mock_api_server(
    port: u16,
    client: Arc<MockKlipperClient>,
    contract_state: Arc<RwLock<MockContractState>>,
) -> Result<()> {
    let client_filter = warp::any().map(move || client.clone());
    let state_filter = warp::any().map(move || contract_state.clone());

    // GET /status
    let status = warp::path("status")
        .and(warp::get())
        .and(client_filter.clone())
        .and(state_filter.clone())
        .and_then(get_status);
    
    // POST /assign
    let assign = warp::path("assign")
        .and(warp::post())
        .and(warp::body::json())
        .and(state_filter.clone())
        .and_then(assign_contract);

    // POST /start
    let start = warp::path("start")
        .and(warp::post())
        .and(warp::body::json())
        .and(client_filter.clone())
        .and(state_filter.clone())
        .and_then(start_print);

    // POST /complete
    let complete = warp::path("complete")
        .and(warp::post())
        .and(client_filter.clone())
        .and_then(complete_print);

    let routes = status.or(assign).or(start).or(complete);

    info!("🎮 Mock API server listening on http://0.0.0.0:{}", port);
    info!("   Endpoints:");
    info!("   - GET  /status          - Get printer status");
    info!("   - POST /assign          - Assign contract (JSON: {{\"contract_id\": \"...\"}})");
    info!("   - POST /start           - Start mock print (JSON: {{\"contract_id\": \"...\"}})");
    info!("   - POST /complete        - Complete mock print");

    warp::serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;

    Ok(())
}

async fn get_status(
    client: Arc<MockKlipperClient>,
    state: Arc<RwLock<MockContractState>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let printer_state = client
        .get_printer_state()
        .await
        .map_err(|_| warp::reject::reject())?;
    
    let is_printing = client
        .is_printing()
        .await
        .map_err(|_| warp::reject::reject())?;
    
    let duration = client
        .get_print_duration()
        .await
        .map_err(|_| warp::reject::reject())?;
    
    let assigned_contract = state.read().await.assigned_contract_id.clone();

    Ok(warp::reply::json(&StatusResponse {
        state: printer_state,
        is_printing,
        duration_secs: duration,
        assigned_contract,
    }))
}

async fn assign_contract(
    req: AssignContractRequest,
    state: Arc<RwLock<MockContractState>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    info!("📋 API: Assigning contract: {}", req.contract_id);
    
    let mut state_guard = state.write().await;
    state_guard.assigned_contract_id = Some(req.contract_id.clone());
    drop(state_guard);
    
    Ok(warp::reply::with_status(
        format!("Contract {} assigned", req.contract_id),
        warp::http::StatusCode::OK,
    ))
}

async fn start_print(
    req: StartPrintRequest,
    client: Arc<MockKlipperClient>,
    state: Arc<RwLock<MockContractState>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Use provided contract_id or fallback to assigned contract
    let contract_id = if let Some(cid) = req.contract_id {
        cid
    } else {
        state.read().await.assigned_contract_id.clone()
            .unwrap_or_else(|| "no-contract".to_string())
    };
    
    info!("🖨️  API: Starting mock print for contract: {}", contract_id);
    
    client
        .start_print()
        .await
        .map_err(|_| warp::reject::reject())?;
    
    info!("✅ API: Mock print started, is_printing should now be true");

    Ok(warp::reply::with_status(
        format!("Print started for contract {}", contract_id),
        warp::http::StatusCode::OK,
    ))
}

async fn complete_print(
    client: Arc<MockKlipperClient>,
) -> Result<impl warp::Reply, warp::Rejection> {
    info!("✅ API: Completing mock print");
    
    client
        .complete_print()
        .await
        .map_err(|_| warp::reject::reject())?;
    
    info!("✅ API: Mock print completed, is_printing should now be false");

    Ok(warp::reply::with_status(
        "Print completed",
        warp::http::StatusCode::OK,
    ))
}
