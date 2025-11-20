use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use warp::Filter;

use crate::mock_klipper::MockKlipperClient;

#[derive(Serialize)]
struct StatusResponse {
    state: String,
    is_printing: bool,
    duration_secs: f64,
}

#[derive(Deserialize)]
struct StartPrintRequest {
    #[serde(default)]
    duration_secs: Option<u64>,
}

/// Start the mock API server for controlling the mock printer
pub async fn start_mock_api_server(
    port: u16,
    client: Arc<MockKlipperClient>,
) -> Result<()> {
    let client_filter = warp::any().map(move || client.clone());

    // GET /status
    let status = warp::path("status")
        .and(warp::get())
        .and(client_filter.clone())
        .and_then(get_status);

    // POST /start
    let start = warp::path("start")
        .and(warp::post())
        .and(client_filter.clone())
        .and_then(start_print);

    // POST /complete
    let complete = warp::path("complete")
        .and(warp::post())
        .and(client_filter.clone())
        .and_then(complete_print);

    let routes = status.or(start).or(complete);

    info!("🎮 Mock API server listening on http://0.0.0.0:{}", port);
    info!("   Endpoints:");
    info!("   - GET  /status    - Get printer status");
    info!("   - POST /start     - Start mock print");
    info!("   - POST /complete  - Complete mock print");

    warp::serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;

    Ok(())
}

async fn get_status(
    client: Arc<MockKlipperClient>,
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

    Ok(warp::reply::json(&StatusResponse {
        state: printer_state,
        is_printing,
        duration_secs: duration,
    }))
}

async fn start_print(
    client: Arc<MockKlipperClient>,
) -> Result<impl warp::Reply, warp::Rejection> {
    info!("🖨️  API: Starting mock print");
    
    client
        .start_print()
        .await
        .map_err(|_| warp::reject::reject())?;

    Ok(warp::reply::with_status(
        "Print started",
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

    Ok(warp::reply::with_status(
        "Print completed",
        warp::http::StatusCode::OK,
    ))
}
