/// Test printer registration by publishing from Rust
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔌 Connecting to NATS...");
    let nc = async_nats::connect("nats://localhost:4222").await?;
    println!("✅ Connected");

    // Wait a moment for Digger's subscription to be ready
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Build registration message
    let registration = json!({
        "printer_id": "rust-test-001",
        "model": "Ender3V3KE",
        "manufacturer": "Creality",
        "serial_number": "RUST123456",
        "rated_watts": 350,
        "public_key": "deadbeef1234567890abcdef"
    });

    println!("📤 Publishing registration to 'printer.register'...");
    nc.publish(
        "printer.register",
        serde_json::to_vec(&registration)?.into()
    ).await?;
    
    println!("✅ Published!");
    
    // Wait for Digger to process
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    println!("✅ Test complete - check Digger logs for '📨 RECEIVED MESSAGE'");

    Ok(())
}
