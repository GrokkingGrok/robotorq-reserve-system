use std::process::{Command, Child};
use std::time::Duration;
use async_nats::Client;
use tokio::time::sleep;
use rust_trust::store::contract_store::ContractStore; // crate name may differ; adjust if needed
use rust_trust::handlers::nats_handler::NatsHandler;
use rust_trust::config::TrustConfig;
use std::sync::Arc;

// Helper to start a local nats-server if available.
fn start_nats() -> Option<Child> {
    if Command::new("nats-server").arg("--version").output().is_err() {
        eprintln!("Skipping e2e test: nats-server not found in PATH");
        return None;
    }
    let child = Command::new("nats-server")
        .arg("-p").arg("4223")
        .arg("-a").arg("127.0.0.1")
        .spawn()
        .ok()?;
    Some(child)
}

async fn wait_for_nats(url: &str) -> bool {
    for _ in 0..50 { // ~5s max
        match async_nats::connect(url).await {
            Ok(_) => return true,
            Err(_) => sleep(Duration::from_millis(100)).await,
        }
    }
    false
}

#[tokio::test]
async fn e2e_genesis_contract_request() {
    // Start NATS or skip
    let maybe = start_nats();
    if maybe.is_none() { return; }
    let mut nats_child = maybe.unwrap();
    let url = "nats://127.0.0.1:4223";
    assert!(wait_for_nats(url).await, "nats-server did not become ready");

    // Config
    let mut cfg = TrustConfig::default();
    cfg.nats_url = url.to_string();

    let store = Arc::new(ContractStore::new());
    store.load_genesis(&cfg.genesis_contract_path).expect("load genesis");
    let handler = NatsHandler::new(Arc::new(cfg), store.clone());

    let (tx, rx) = tokio::sync::watch::channel(false);
    let h_task = tokio::spawn(async move { handler.start(rx).await.unwrap() });

    // Client request
    let client: Client = async_nats::connect(url).await.expect("connect client");
    let req_payload = serde_json::json!({"contract_id":"genesis-0001"});
    let resp = client.request("trust.contract.request", serde_json::to_vec(&req_payload).unwrap().into()).await.expect("request");
    let v: serde_json::Value = serde_json::from_slice(&resp.payload).expect("deserialize reply");
    assert_eq!(v.get("contract_id").and_then(|c| c.as_str()), Some("genesis-0001"));

    // Shutdown
    let _ = tx.send(true);
    sleep(Duration::from_millis(200)).await;
    let _ = h_task.await;
    let _ = nats_child.kill();
}