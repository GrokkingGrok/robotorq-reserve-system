// HTTP API for Digger - allows Trust to query status and send stake
//
// Endpoints:
// GET  /robot/status → Check if robot is available
// POST /stake        → Receive RoboStake for a contract
// GET  /health       → Health check

use crate::digger::DiggerManager;
use serde::{Deserialize, Serialize};
use tiny_http::{Response, Server};

/// Response for /robot/status
#[derive(Serialize)]
struct RobotStatusResponse {
    available: bool,
    digger_id: String,
    current_contract: Option<String>,
}

/// Request body for /stake
#[derive(Deserialize)]
struct StakeRequest {
    contract_id: String,
    amount_rt: f64,
    builder: String,
}

/// Response for /stake
#[derive(Serialize)]
struct StakeResponse {
    status: String,
    contract_id: String,
    message: String,
}

/// Start the HTTP server on port 9000
pub fn start_http_server() {
    std::thread::spawn(|| {
        let server = Server::http("0.0.0.0:9000").unwrap();
        println!("🌐 Digger HTTP API listening on :9000");

        for request in server.incoming_requests() {
            let url = request.url().to_string();
            
            match (request.method().as_str(), url.as_str()) {
                ("GET", "/robot/status") => handle_robot_status(request),
                ("POST", "/stake") => handle_stake(request),
                ("GET", "/health") => {
                    let response = Response::from_string("ok");
                    let _ = request.respond(response);
                }
                _ => {
                    let response = Response::from_string("Not Found").with_status_code(404);
                    let _ = request.respond(response);
                }
            }
        }
    });
}

fn handle_robot_status(request: tiny_http::Request) {
    let manager = DiggerManager::global();
    let lock = manager.lock().unwrap();
    
    // For now, we only have one digger: "dig-jon-ai-001"
    let digger = lock.get_digger("dig-jon-ai-001");
    
    let status = match digger {
        Some(d) => RobotStatusResponse {
            available: d.current_contract.is_none(),
            digger_id: d.id.clone(),
            current_contract: d.current_contract.as_ref().map(|c| c.id.clone()),
        },
        None => RobotStatusResponse {
            available: false,
            digger_id: "none".to_string(),
            current_contract: None,
        },
    };

    let json = serde_json::to_string(&status).unwrap();
    let response = Response::from_string(json)
        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
    
    let _ = request.respond(response);
}

fn handle_stake(mut request: tiny_http::Request) {
    // Read request body
    let mut body = String::new();
    if let Err(e) = request.as_reader().read_to_string(&mut body) {
        let response = Response::from_string(format!("Failed to read body: {}", e))
            .with_status_code(400);
        let _ = request.respond(response);
        return;
    }

    // Parse JSON
    let stake_req: StakeRequest = match serde_json::from_str(&body) {
        Ok(req) => req,
        Err(e) => {
            let response = Response::from_string(format!("Invalid JSON: {}", e))
                .with_status_code(400);
            let _ = request.respond(response);
            return;
        }
    };

    println!("💰 Stake received: {} RT for contract {}", stake_req.amount_rt, stake_req.contract_id);

    // TODO: Actually process the stake (store it, trigger work, etc.)
    // For now, just acknowledge receipt

    let resp = StakeResponse {
        status: "accepted".to_string(),
        contract_id: stake_req.contract_id.clone(),
        message: format!("Received {} RT for contract", stake_req.amount_rt),
    };

    let json = serde_json::to_string(&resp).unwrap();
    let response = Response::from_string(json)
        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
    
    let _ = request.respond(response);
}
