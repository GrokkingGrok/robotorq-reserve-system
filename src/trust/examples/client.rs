use async_nats::ConnectOptions;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let nats_url = env::var("TRUST_NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".to_string());
    println!("connecting to NATS at {}", nats_url);

    let nc = ConnectOptions::new()
        .connect(&nats_url)
        .await?;

    let requested = env::var("TRUST_REQUEST_CONTRACT_ID").unwrap_or_else(|_| "genesis-0001".to_string());
    let req = serde_json::json!({ "contract_id": requested });
    let payload = serde_json::to_vec(&req)?;

    let msg = nc.request("trust.contract.request", payload.into()).await?;
    let body = String::from_utf8_lossy(&msg.payload);
    println!("reply: {}", body);

    Ok(())
}
