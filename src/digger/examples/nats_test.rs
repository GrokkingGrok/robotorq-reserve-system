/// Minimal async-nats test to verify pub/sub works
/// Run with: cargo run --example nats_test

use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔌 Connecting to NATS...");
    let nc = async_nats::connect("nats://localhost:4222").await?;
    println!("✅ Connected to NATS");

    // Subscribe first
    println!("📝 Creating subscription to 'test.subject'");
    let mut sub = nc.subscribe("test.subject").await?;
    println!("✅ Subscription created");

    // Spawn publisher after small delay
    let nc_pub = nc.clone();
    tokio::spawn(async move {
        println!("⏳ Publisher waiting 500ms...");
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        println!("📤 Publishing message...");
        if let Err(e) = nc_pub.publish("test.subject", "Hello NATS!".into()).await {
            eprintln!("❌ Publish error: {}", e);
        } else {
            println!("✅ Message published");
        }
    });

    // Wait for message with timeout
    println!("📨 Waiting for message (10 second timeout)...");
    
    tokio::select! {
        result = sub.next() => {
            match result {
                Some(msg) => {
                    println!("✅ RECEIVED MESSAGE!");
                    println!("   Subject: {}", msg.subject);
                    println!("   Payload: {}", String::from_utf8_lossy(&msg.payload));
                }
                None => {
                    println!("❌ Subscription closed");
                }
            }
        }
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(10)) => {
            println!("❌ TIMEOUT - No message received");
        }
    }

    Ok(())
}
