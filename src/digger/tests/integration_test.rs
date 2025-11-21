// Integration Test - Full Contract Lifecycle
//
// Tests the complete flow:
// 1. Create contract (with torq × robo_stake economics)
// 2. Pay stake (approve contract)
// 3. Execute contract (generate JTUs, track ore)
// 4. Verify ore target reached
// 5. Query contract status and JTU stats

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

// Import from main crate
use digger::{
    config::DiggerConfig,
    contract_state::ContractStateManager,
    crypto::DiggerKeypair,
    jtu_storage::JtuStorageManager,
    http_api::{ApiState, create_router},
    metrics::DiggerMetrics,
};

#[tokio::test]
async fn test_full_contract_lifecycle() {
    // Setup test environment
    let temp_dir = tempfile::tempdir().unwrap();
    let config = DiggerConfig {
        digger_id: "test-robot-001".to_string(),
        nats_url: "nats://localhost:4222".to_string(),
        http_port: 9999,
        storage_path: temp_dir.path().to_path_buf(),
        batch_interval_sec: 60,
        prune_after_days: 30,
        log_level: "debug".to_string(),
    };

    let contract_manager = ContractStateManager::new();
    let storage_manager = JtuStorageManager::new(config.storage_path.clone())
        .expect("Failed to create storage manager");

    // Try to connect to NATS - skip test if unavailable (CI environment)
    let nats_client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("⚠️  Skipping test_full_contract_lifecycle - NATS not available (this is OK in CI)");
            return;
        }
    };

    let keypair = DiggerKeypair::generate();
    let metrics = DiggerMetrics::default();
    let printer_registry = PrinterRegistry::new();
    let state = ApiState::new(config, contract_manager, storage_manager, nats_client, keypair, metrics, printer_registry);
    let app = create_router(state);

    // ========================================================================
    // Step 1: Create Contract
    // ========================================================================
    println!("\n=== Step 1: Create Contract ===");
    
    let create_body = json!({
        "contract_id": "integration-test-001",
        "torq": 100.0,           // 100:1 ratio (unitless)
        "robo_stake": 5.0,       // 5 RT (currency)
        "milestones": 10,
        "power_watts": 2000.0    // 2kW robot
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/contracts/create")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&create_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let create_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    println!("Create Response: {}", serde_json::to_string_pretty(&create_response).unwrap());
    
    // Verify ore target calculation: torq × robo_stake = 100 × 5 = 500 RT
    assert_eq!(create_response["ore_target"], 500.0);
    assert_eq!(create_response["contract_id"], "integration-test-001");

    // ========================================================================
    // Step 2: Pay Stake (Approve Contract)
    // ========================================================================
    println!("\n=== Step 2: Pay Stake ===");
    
    let stake_body = json!({
        "contract_id": "integration-test-001"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/contracts/stake")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&stake_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let stake_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    println!("Stake Response: {}", serde_json::to_string_pretty(&stake_response).unwrap());
    
    assert_eq!(stake_response["approval_status"], "StakeApproved");

    // ========================================================================
    // Step 3: Execute Contract (Generate Ore - First Batch)
    // ========================================================================
    println!("\n=== Step 3: Execute Contract (First Batch) ===");
    
    // Cross product: 2000W × 1 token/sec = 2000 JTU/sec
    // 10 seconds = 20,000 JTUs
    // ore_value_per_jtu = robo_stake / ore_target = 5 / 500 = 0.01 RT/JTU
    // Expected ore = 20,000 × 0.01 = 200 RT
    
    let execute_body = json!({
        "contract_id": "integration-test-001",
        "duration_seconds": 10
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/contracts/execute")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&execute_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let execute_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    println!("Execute Response (Batch 1): {}", serde_json::to_string_pretty(&execute_response).unwrap());
    
    assert_eq!(execute_response["jtus_generated"], 20000);
    assert_eq!(execute_response["ore_generated"], 200.0);
    assert_eq!(execute_response["ore_target"], 500.0);
    assert_eq!(execute_response["target_reached"], false);

    // ========================================================================
    // Step 4: Execute Again (Second Batch - Reach Target)
    // ========================================================================
    println!("\n=== Step 4: Execute Contract (Second Batch - Reach Target) ===");
    
    // Need 300 more RT to reach 500 RT target
    // 300 RT / 0.01 RT/JTU = 30,000 JTUs
    // 30,000 JTUs / 2000 JTU/sec = 15 seconds
    
    let execute_body = json!({
        "contract_id": "integration-test-001",
        "duration_seconds": 15
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/contracts/execute")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&execute_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let execute_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    println!("Execute Response (Batch 2): {}", serde_json::to_string_pretty(&execute_response).unwrap());
    
    // Total: 200 RT + 300 RT = 500 RT
    assert_eq!(execute_response["ore_generated"], 500.0);
    assert_eq!(execute_response["target_reached"], true);
    assert!(execute_response["message"].as_str().unwrap().contains("target reached"));

    // ========================================================================
    // Step 5: Query Contract Status
    // ========================================================================
    println!("\n=== Step 5: Query Contract Status ===");
    
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/contracts/integration-test-001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let status_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    println!("Status Response: {}", serde_json::to_string_pretty(&status_response).unwrap());
    
    assert_eq!(status_response["contract_id"], "integration-test-001");
    assert_eq!(status_response["torq"], 100.0);
    assert_eq!(status_response["robo_stake"], 5.0);
    assert_eq!(status_response["ore_target"], 500.0);
    assert_eq!(status_response["ore_generated"], 500.0);
    assert_eq!(status_response["target_reached"], true);
    assert_eq!(status_response["approval_status"], "StakeApproved");
    assert_eq!(status_response["jtu_count"], 50000);  // 20,000 + 30,000

    // ========================================================================
    // Step 6: Verify Economics
    // ========================================================================
    println!("\n=== Step 6: Verify Economics ===");
    
    // Torq = 100 (scalar ratio)
    // RoboStake = 5 RT
    // Ore Target = 100 × 5 = 500 RT ✓
    // Ore Generated = 500 RT ✓
    // JTUs = 50,000 ✓
    
    println!("✅ All economics verified:");
    println!("   Torq (ratio): 100");
    println!("   RoboStake: 5 RT");
    println!("   Ore Target: 500 RT (100 × 5)");
    println!("   Ore Generated: 500 RT");
    println!("   JTUs Created: 50,000");
    println!("   Cross Product: 2000W × 1 tok/s = 2000 JTU/s");
    println!("   Target Reached: ✓");
}

#[tokio::test]
async fn test_contract_not_found() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = DiggerConfig {
        digger_id: "test-robot-002".to_string(),
        nats_url: "nats://localhost:4222".to_string(),
        http_port: 9999,
        storage_path: temp_dir.path().to_path_buf(),
        batch_interval_sec: 60,
        prune_after_days: 30,
        log_level: "debug".to_string(),
    };

    let contract_manager = ContractStateManager::new();
    let storage_manager = JtuStorageManager::new(config.storage_path.clone())
        .expect("Failed to create storage manager");

    // Try to connect to NATS - skip test if unavailable (CI environment)
    let nats_client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("⚠️  Skipping test_error_handling - NATS not available (this is OK in CI)");
            return;
        }
    };

    let keypair = DiggerKeypair::generate();
    let metrics = DiggerMetrics::default();
    let printer_registry = PrinterRegistry::new();
    let state = ApiState::new(config, contract_manager, storage_manager, nats_client, keypair, metrics, printer_registry);
    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/contracts/nonexistent-contract")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_execute_before_stake_payment() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = DiggerConfig {
        digger_id: "test-robot-003".to_string(),
        nats_url: "nats://localhost:4222".to_string(),
        http_port: 9999,
        storage_path: temp_dir.path().to_path_buf(),
        batch_interval_sec: 60,
        prune_after_days: 30,
        log_level: "debug".to_string(),
    };

    let contract_manager = ContractStateManager::new();
    let storage_manager = JtuStorageManager::new(config.storage_path.clone())
        .expect("Failed to create storage manager");

    // Try to connect to NATS - skip test if unavailable (CI environment)
    let nats_client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("⚠️  Skipping test_execute_before_stake_payment - NATS not available (this is OK in CI)");
            return;
        }
    };

    let keypair = DiggerKeypair::generate();
    let metrics = DiggerMetrics::default();
    let printer_registry = PrinterRegistry::new();
    let state = ApiState::new(config, contract_manager, storage_manager, nats_client, keypair, metrics, printer_registry);
    let app = create_router(state);

    // Create contract
    let create_body = json!({
        "contract_id": "test-unpaid-contract",
        "torq": 100.0,
        "robo_stake": 5.0,
        "milestones": 10,
        "power_watts": 2000.0
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/contracts/create")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&create_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Try to execute WITHOUT paying stake
    let execute_body = json!({
        "contract_id": "test-unpaid-contract",
        "duration_seconds": 10
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/contracts/execute")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&execute_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should reject (contract not approved)
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let error_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(error_response["error"].as_str().unwrap().contains("approved"));
}

#[tokio::test]
async fn test_invalid_torq_values() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = DiggerConfig {
        digger_id: "test-robot-004".to_string(),
        nats_url: "nats://localhost:4222".to_string(),
        http_port: 9999,
        storage_path: temp_dir.path().to_path_buf(),
        batch_interval_sec: 60,
        prune_after_days: 30,
        log_level: "debug".to_string(),
    };

    let contract_manager = ContractStateManager::new();
    let storage_manager = JtuStorageManager::new(config.storage_path.clone())
        .expect("Failed to create storage manager");

    // Try to connect to NATS - skip test if unavailable (CI environment)
    let nats_client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("⚠️  Skipping test_stake_validation - NATS not available (this is OK in CI)");
            return;
        }
    };

    let keypair = DiggerKeypair::generate();
    let metrics = DiggerMetrics::default();
    let printer_registry = PrinterRegistry::new();
    let state = ApiState::new(config, contract_manager, storage_manager, nats_client, keypair, metrics, printer_registry);
    let app = create_router(state);

    // Test negative torq
    let create_body = json!({
        "contract_id": "invalid-contract-001",
        "torq": -100.0,
        "robo_stake": 5.0,
        "milestones": 10,
        "power_watts": 2000.0
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/contracts/create")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&create_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Test zero robo_stake
    let create_body = json!({
        "contract_id": "invalid-contract-002",
        "torq": 100.0,
        "robo_stake": 0.0,
        "milestones": 10,
        "power_watts": 2000.0
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/contracts/create")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&create_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

