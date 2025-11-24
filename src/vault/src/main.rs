use robotorq_vault::{VaultConfig, ShadowCertVault, ShadowStakeVault, ShadowDistoVault, VaultMetrics, VaultPersistence, ContractApproval, ApprovalConfig, ShortVaultRegistry};
use robotorq_vault::models::{UBDDistributionPackage, DemurrageReleasePackage, PackageConfirmation};
use robotorq_vault::models::{TransactionRequest, TransactionQuote, TransactionCommitment, TransactionExecution, TransactionStatus, TransactionStatusResponse};
use axum::{Router, routing::get, http::StatusCode, Json};
use axum::extract::State;
use axum::routing::post;
use std::net::SocketAddr;
use std::sync::Arc;
use robotorq_vault::nats_client::connect_nats;
use robotorq_vault::events::subjects;
use robotorq_vault::models::RoboTorqBatch;
use anyhow::Result;
use futures_util::stream::StreamExt;
use tracing::{info, error, warn};
use tokio::task;
use serde_json::json;
use chrono::{DateTime, Utc, Duration};
use uuid;

#[derive(Clone)]
struct AppState {
    cert_vault: Arc<ShadowCertVault>,
    stake_vault: Arc<ShadowStakeVault>,
    short_vault_registry: Arc<ShortVaultRegistry>,
    metrics: Arc<VaultMetrics>,
    nats: async_nats::Client,
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let drip_stats = state.short_vault_registry.get_aggregate_drip_stats().await;

    let response = serde_json::json!({
        "certificates": state.cert_vault.total_robotorq(),
        "available_robostake": state.stake_vault.available_robostake(),
        "deployed_robostake": state.stake_vault.deployed_robostake(),
        "short_vault_total_balance": state.short_vault_registry.total_balance(),
        "drip_stats": drip_stats,
        "single_package_delivery": {
            "enabled": std::env::var("VAULT_SINGLE_PACKAGE_DELIVERY").unwrap_or_else(|_| "false".to_string()) == "true",
            "signing_enabled": std::env::var("VAULT_PACKAGE_SIGNING_ENABLED").unwrap_or_else(|_| "false".to_string()) == "true",
            "hash_verification": std::env::var("VAULT_PACKAGE_HASH_VERIFICATION").unwrap_or_else(|_| "false".to_string()) == "true"
        }
    });

    // Add simulation info only if simulation feature is enabled
    #[cfg(feature = "simulation")]
    {
        response["simulation"] = serde_json::json!({
            "mode": std::env::var("SIMULATION_MODE").unwrap_or_else(|_| "false".to_string()) == "true",
            "time_compression": std::env::var("SIMULATION_TIME_COMPRESSION").unwrap_or_else(|_| "1.0".to_string()),
            "scenario_id": std::env::var("SIMULATION_SCENARIO_ID").ok()
        });
    }

    Json(response)
}async fn metrics(State(state): State<AppState>) -> (StatusCode, [(String, String); 1], String) {
    let body = state.metrics.encode();
    (
        StatusCode::OK,
        [("Content-Type".to_string(), "text/plain; version=0.0.4".to_string())],
        body,
    )
}

#[derive(serde::Deserialize)]
struct UBDDistributeRequest {
    wallet_id: String,
    amount_jouletorq: i64,
}

async fn ubd_distribute(
    State(state): State<AppState>,
    Json(request): Json<UBDDistributeRequest>,
) -> Json<serde_json::Value> {
    // For testing: directly create and send UBD package to specified wallet
    let package = UBDDistributionPackage::new(
        request.wallet_id.clone(),
        request.amount_jouletorq,
        vec!["test-cert-001".to_string()], // Mock certificate ID for testing
    );

    // Send package to wallet
    match state.nats.publish(subjects::VAULT_UBD_PACKAGE, serde_json::to_vec(&package).unwrap().into()).await {
        Ok(_) => {
            info!(wallet_id=%request.wallet_id, package_id=%package.package_id, amount=request.amount_jouletorq, "UBD package sent to wallet for testing");
            Json(serde_json::json!({
                "success": true,
                "package_id": package.package_id,
                "wallet_id": request.wallet_id,
                "amount_jouletorq": request.amount_jouletorq
            }))
        }
        Err(e) => {
            error!(wallet_id=%request.wallet_id, amount=request.amount_jouletorq, error=%e, "Failed to send UBD package to wallet");
            Json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            }))
        }
    }
}

#[derive(serde::Deserialize)]
struct TransactionQuoteRequest {
    from_wallet_id: String,
    to_wallet_id: String,
    amount_jouletorq: i64,
    timeframe_seconds: u64,
}

async fn transaction_quote(
    State(state): State<AppState>,
    Json(request): Json<TransactionQuoteRequest>,
) -> Json<serde_json::Value> {
    // Basic validation
    if request.amount_jouletorq <= 0 {
        return Json(json!({
            "possible": false,
            "reason": "Amount must be positive"
        }));
    }

    if request.timeframe_seconds == 0 {
        return Json(json!({
            "possible": false,
            "reason": "Timeframe must be positive"
        }));
    }

    // For now, reject cross-vault transactions
    // TODO: Implement cross-vault communication
    if !request.to_wallet_id.starts_with("wallet-") {
        return Json(json!({
            "possible": false,
            "reason": "Cross-vault transactions not yet supported"
        }));
    }

    // Calculate fee based on amount and timeframe
    // Higher amounts and shorter timeframes = higher fees
    let base_fee = (request.amount_jouletorq as f64 * 0.01).max(1.0) as i64; // 1% base fee
    let urgency_multiplier = if request.timeframe_seconds < 3600 { 2.0 } else { 1.0 }; // Double fee for < 1 hour
    let fee = (base_fee as f64 * urgency_multiplier) as i64;

    // Estimate completion time (add some buffer)
    let estimated_completion = request.timeframe_seconds + 60;

    let quote_id = format!("quote-{}", uuid::Uuid::new_v4());
    let expires_at = Utc::now() + Duration::seconds(300); // 5 minute expiration

    Json(json!({
        "quote_id": quote_id,
        "request": {
            "from_wallet_id": request.from_wallet_id,
            "to_wallet_id": request.to_wallet_id,
            "amount_jouletorq": request.amount_jouletorq,
            "timeframe_seconds": request.timeframe_seconds
        },
        "fee_jouletorq": fee,
        "estimated_completion_seconds": estimated_completion,
        "possible": true,
        "expires_at": expires_at.to_rfc3339()
    }))
}

#[derive(serde::Deserialize)]
struct TransactionCommitRequest {
    quote_id: String,
    accepted: bool,
}

async fn transaction_commit(
    State(state): State<AppState>,
    Json(request): Json<TransactionCommitRequest>,
) -> Json<serde_json::Value> {
    if !request.accepted {
        return Json(json!({
            "transaction_id": null,
            "status": "cancelled",
            "message": "Transaction cancelled by user"
        }));
    }

    // For now, create a mock transaction execution
    // TODO: Implement actual transaction execution with fund locking and streaming
    let transaction_id = format!("tx-{}", uuid::Uuid::new_v4());

    Json(json!({
        "transaction_id": transaction_id,
        "quote_id": request.quote_id,
        "status": "in_progress",
        "progress": 0.0,
        "transferred_jouletorq": 0,
        "started_at": Utc::now().to_rfc3339(),
        "message": "Transaction initiated, funds locked"
    }))
}

#[derive(serde::Deserialize)]
struct TransactionStatusRequest {
    transaction_id: String,
}

async fn transaction_status(
    State(state): State<AppState>,
    Json(request): Json<TransactionStatusRequest>,
) -> Json<serde_json::Value> {
    // For now, return mock status
    // TODO: Implement actual transaction tracking with persistent storage
    let mut mock_execution = TransactionExecution::new(
        uuid::Uuid::parse_str(&request.transaction_id).unwrap_or_else(|_| uuid::Uuid::new_v4()),
        1000.0, // Mock amount
    );
    mock_execution.status = TransactionStatus::Completed;
    mock_execution.progress_percentage = 100.0;
    mock_execution.started_at = Some(Utc::now() - Duration::seconds(30));
    mock_execution.completed_at = Some(Utc::now());

    let response = TransactionStatusResponse::new(
        mock_execution.transaction_id,
        mock_execution.status.clone(),
        None, // No quote in mock
        Some(mock_execution),
    );

    Json(json!({
        "transaction_id": response.transaction_id,
        "status": response.status,
        "execution": response.execution,
        "last_updated": response.last_updated
    }))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let cfg = VaultConfig::from_env();
    info!(config=?cfg, "vault configuration loaded");
    let nats = match connect_nats(&cfg.nats_url).await {
        Ok(c) => c,
        Err(e) => {
            error!(error=%e, nats_url=%cfg.nats_url, "failed to connect to NATS");
            return Err(e);
        }
    };

    let vault_metrics = VaultMetrics::new();
    let persistence = match VaultPersistence::new(&cfg.db_url).await {
        Ok(p) => {
            info!("persistence initialized");
            p
        }
        Err(e) => {
            error!(error=%e, "failed to initialize persistence; running without DB");
            // Fallback: create a dummy in-memory pool-less persistence (not implemented) – for now we abort.
            return Err(e);
        }
    };
    let cert_vault = Arc::new(ShadowCertVault::new(nats.clone()).with_metrics(vault_metrics.clone()));
    let stake_vault = Arc::new(ShadowStakeVault::new(nats.clone()).with_metrics(vault_metrics.clone()));
    let short_vault_registry = Arc::new(ShortVaultRegistry::new(nats.clone()).with_metrics(vault_metrics.clone()));
    let disto_vault_builder = ShadowDistoVault::new(nats.clone(), cfg.distostream_tick_millis)
        .with_metrics(vault_metrics.clone())
        .with_persistence(persistence.clone());

    #[cfg(feature = "simulation")]
    {
        disto_vault_builder = disto_vault_builder.with_time_compression(cfg.time_compression_factor);
    }

    let disto_vault = Arc::new(disto_vault_builder);
    disto_vault.clone().start();

    // Initialize contract approval if enabled
    if cfg.approval_enabled {
        let approval_config = ApprovalConfig {
            min_available_stake_ratio: cfg.min_available_stake_ratio,
            max_concurrent_contracts: cfg.max_concurrent_contracts,
        };
        let contract_approval = Arc::new(ContractApproval::new(
            stake_vault.clone(),
            nats.clone(),
            approval_config,
            vault_metrics.clone(),
        ));
        if let Err(e) = contract_approval.clone().start().await {
            error!(error = %e, "Failed to start contract approval service");
            return Err(anyhow::anyhow!("Failed to start contract approval: {}", e));
        }
        info!("Contract approval service enabled");
    } else {
        info!("Contract approval service disabled (set VAULT_APPROVAL_ENABLED=true to enable)");
    }
    let state = AppState { cert_vault: cert_vault.clone(), stake_vault: stake_vault.clone(), short_vault_registry: short_vault_registry.clone(), metrics: vault_metrics.clone(), nats: nats.clone() };

    // Subscription processing task (detached).
    let sub_state = state.clone();
    let nats_client = nats.clone();
    task::spawn(async move {
        match nats_client.subscribe(subjects::PHASE3_COMPLETED).await {
            Ok(mut sub) => {
                info!(subject = subjects::PHASE3_COMPLETED, "vault subscribed");
                while let Some(msg) = sub.next().await {
                    match serde_json::from_slice::<RoboTorqBatch>(&msg.payload) {
                        Ok(batch) => {
                            info!(batch_id = %batch.batch_id, cert_count = batch.certificates.len(), "processing batch");
                            for cert in batch.certificates.iter() {
                                if let Err(e) = sub_state.cert_vault.store_certificate(cert.clone()).await {
                                    error!(cert_id=%cert.cert_id, error=%e, "failed to store certificate");
                                }
                            }
                            let returned = batch.total_robostake;
                            sub_state.stake_vault.increment_available(returned);
                            if let Err(e) = sub_state.stake_vault.publish_robostake_return(batch.batch_id.clone(), returned).await {
                                error!(batch_id=%batch.batch_id, error=%e, "failed publishing robostake.returned");
                            }

                            // MVP2: Emit minimal distostream authorization for DistoVault
                            let contract_id = batch
                                .certificates
                                .get(0)
                                .and_then(|c| c.contract_ids.get(0))
                                .cloned()
                                .unwrap_or_else(|| "unknown-contract".to_string());
                            let robotorq_total = batch.certificates.len() as i64;
                            let duration_seconds = 60; // default for MVP2
                            let provenance: Vec<String> = batch
                                .certificates
                                .iter()
                                .map(|c| c.cert_id.clone())
                                .collect();

                            let auth_event = serde_json::json!({
                                "event_type": "distostream_authorized",
                                "contract_id": contract_id,
                                "robotorq_total": robotorq_total,
                                "tokentorq_remainder": 0,
                                "jouletorq_remainder": 0,
                                "duration_seconds": duration_seconds,
                                "provenance_cert_ids": provenance,
                            });

                            if let Err(e) = nats_client.publish(subjects::DISTOSTREAM_AUTHORIZED, serde_json::to_vec(&auth_event).unwrap().into()).await {
                                error!(error=%e, "failed to publish vault.distostream.authorized");
                            } else {
                                sub_state.metrics.inc_distostream_authorized();
                            }
                        }
                        Err(e) => {
                            error!(error=%e, "invalid batch payload");
                        }
                    }
                }
            }
            Err(e) => error!(error=%e, "failed to subscribe to phase3 completed subject"),
        }
    });

    // UBD distribution to ShortVaults task (detached)
    let ubd_state = state.clone();
    let ubd_nats = nats.clone();
    let ubd_config = cfg.clone();
    task::spawn(async move {
        match ubd_nats.subscribe(subjects::DISTOSTREAM_DISTRIBUTION_TICK).await {
            Ok(mut sub) => {
                info!(subject = subjects::DISTOSTREAM_DISTRIBUTION_TICK, "UBD distribution subscribed");
                while let Some(msg) = sub.next().await {
                    match serde_json::from_slice::<serde_json::Value>(&msg.payload) {
                        Ok(event) => {
                            if let Some(distributed) = event.get("distributed_robotorq").and_then(|v| v.as_i64()) {
                                if distributed > 0 {
                                    if ubd_config.single_package_delivery {
                                        // Single package delivery: send directly to wallets
                                        // For now, distribute to known wallets (this would come from wallet registry in production)
                                        // TODO: Get actual wallet list from wallet registry service
                                        let mock_wallet_ids = vec!["wallet-001", "wallet-002", "wallet-003"]; // Mock for MVP

                                        if !mock_wallet_ids.is_empty() {
                                            let per_wallet = distributed / mock_wallet_ids.len() as i64;
                                            let remainder = distributed % mock_wallet_ids.len() as i64;

                                            info!(total_ubd=distributed, wallet_count=mock_wallet_ids.len(), per_wallet=per_wallet, remainder=remainder, "Distributing UBD packages directly to wallets");

                                            for (i, wallet_id) in mock_wallet_ids.iter().enumerate() {
                                                let amount = per_wallet + if i < remainder as usize { 1 } else { 0 };

                                                // Create UBD package
                                                let package = UBDDistributionPackage::new(
                                                    wallet_id.to_string(),
                                                    amount,
                                                    vec!["mock-cert-001".to_string()], // TODO: Get real cert IDs
                                                );

                                                // Send package to wallet
                                                if let Err(e) = ubd_nats.publish(subjects::VAULT_UBD_PACKAGE, serde_json::to_vec(&package).unwrap().into()).await {
                                                    error!(wallet_id=%wallet_id, amount=amount, error=%e, "Failed to send UBD package to wallet");
                                                } else {
                                                    info!(wallet_id=%wallet_id, package_id=%package.package_id, amount=amount, "UBD package sent to wallet");
                                                }
                                            }
                                        } else {
                                            info!(total_ubd=distributed, "No wallets registered yet, UBD not distributed");
                                        }
                                    } else {
                                        // Legacy drip-based delivery: distribute to ShortVaults
                                        let vaults = ubd_state.short_vault_registry.get_all_vaults();
                                        if !vaults.is_empty() {
                                            let per_vault = distributed / vaults.len() as i64;
                                            let remainder = distributed % vaults.len() as i64;

                                            info!(total_ubd=distributed, vault_count=vaults.len(), per_vault=per_vault, remainder=remainder, "Distributing UBD to ShortVaults");

                                            for (i, vault) in vaults.iter().enumerate() {
                                                let amount = per_vault + if i < remainder as usize { 1 } else { 0 };
                                                if let Err(e) = vault.credit_ubd(amount).await {
                                                    error!(user_id=%vault.user_id(), amount=amount, error=%e, "Failed to credit UBD to ShortVault");
                                                }
                                            }
                                        } else {
                                            info!(total_ubd=distributed, "No ShortVaults exist yet, UBD not distributed");
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!(error=%e, "Invalid UBD distribution event payload");
                        }
                    }
                }
            }
            Err(e) => error!(error=%e, "failed to subscribe to UBD distribution subject"),
        }
    });

    // Drip processing task (detached) - only available in simulation builds
    #[cfg(feature = "simulation")]
    let drip_state = state.clone();
    #[cfg(feature = "simulation")]
    let time_compression = cfg.time_compression_factor;
    #[cfg(feature = "simulation")]
    task::spawn(async move {
        // Process drips at configured interval (compressed)
        let base_interval = cfg.drip_processing_interval_seconds as u64;
        let compressed_interval = (base_interval as f64 / time_compression) as u64;
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(compressed_interval));
        interval.tick().await; // Skip first immediate tick

        loop {
            interval.tick().await;
            info!("Processing drips for all ShortVaults");
            if let Err(e) = drip_state.short_vault_registry.apply_demurrage_to_all().await {
                error!(error=%e, "Failed to process drips for all vaults");
            }
        }
    });

    // Wallet communication task (detached) - handles wallet requests
    let _wallet_state = state.clone();
    let wallet_nats = nats.clone();
    let wallet_config = cfg.clone();
    task::spawn(async move {
        match wallet_nats.subscribe(subjects::WALLET_DEMURRAGE_REQUEST).await {
            Ok(mut sub) => {
                info!(subject = subjects::WALLET_DEMURRAGE_REQUEST, "Wallet communication subscribed");
                while let Some(msg) = sub.next().await {
                    match serde_json::from_slice::<serde_json::Value>(&msg.payload) {
                        Ok(request) => {
                            if let (Some(user_id), Some(amount)) = (
                                request.get("user_id").and_then(|v| v.as_str()),
                                request.get("demurrage_amount").and_then(|v| v.as_i64())
                            ) {
                                if wallet_config.single_package_delivery {
                                    // Single package delivery: send demurrage as complete package
                                    let package = DemurrageReleasePackage::new(
                                        user_id.to_string(),
                                        amount,
                                    );

                                    // Send package directly to wallet
                                    if let Err(e) = wallet_nats.publish(subjects::VAULT_DEMURRAGE_PACKAGE, serde_json::to_vec(&package).unwrap().into()).await {
                                        error!(user_id=%user_id, amount=amount, error=%e, "Failed to send demurrage package to wallet");
                                    } else {
                                        info!(user_id=%user_id, package_id=%package.package_id, amount=amount, "Demurrage package sent to wallet");
                                    }
                                } else {
                                    // Legacy drip-based delivery
                                    #[cfg(feature = "simulation")]
                                    {
                                        match wallet_state.short_vault_registry.get_or_create_vault(user_id).await {
                                            Ok(vault) => {
                                                if let Err(e) = vault.request_demurrage_release(amount, wallet_config.drip_duration_hours, wallet_config.time_compression_factor).await {
                                                    error!(user_id=%user_id, amount=amount, error=%e, "Failed to process demurrage release request");
                                                }
                                            }
                                            Err(e) => {
                                                error!(user_id=%user_id, error=%e, "Failed to get/create vault for demurrage request");
                                            }
                                        }
                                    }
                                    #[cfg(not(feature = "simulation"))]
                                    {
                                        warn!(user_id=%user_id, "Demurrage requests not supported in production builds (simulation feature disabled)");
                                    }
                                }
                            } else {
                                warn!("Invalid demurrage request format");
                            }
                        }
                        Err(e) => {
                            error!(error=%e, "Invalid wallet request payload");
                        }
                    }
                }
            }
            Err(e) => error!(error=%e, "failed to subscribe to wallet demurrage requests"),
        }
    });

    // Wallet replenishment task (detached)
    let replenish_state = state.clone();
    let replenish_nats = nats.clone();
    task::spawn(async move {
        match replenish_nats.subscribe(subjects::WALLET_BALANCE_REPLENISH).await {
            Ok(mut sub) => {
                info!(subject = subjects::WALLET_BALANCE_REPLENISH, "Wallet replenishment subscribed");
                while let Some(msg) = sub.next().await {
                    match serde_json::from_slice::<serde_json::Value>(&msg.payload) {
                        Ok(request) => {
                            if let (Some(user_id), Some(amount)) = (
                                request.get("user_id").and_then(|v| v.as_str()),
                                request.get("replenish_amount").and_then(|v| v.as_i64())
                            ) {
                                match replenish_state.short_vault_registry.get_or_create_vault(user_id).await {
                                    Ok(vault) => {
                                        if let Err(e) = vault.replenish_balance(amount).await {
                                            error!(user_id=%user_id, amount=amount, error=%e, "Failed to replenish vault balance");
                                        }
                                    }
                                    Err(e) => {
                                        error!(user_id=%user_id, error=%e, "Failed to get/create vault for replenishment");
                                    }
                                }
                            } else {
                                warn!("Invalid replenishment request format");
                            }
                        }
                        Err(e) => {
                            error!(error=%e, "Invalid replenishment request payload");
                        }
                    }
                }
            }
            Err(e) => error!(error=%e, "failed to subscribe to wallet replenishment"),
        }
    });

    // Package confirmation task (detached) - handles wallet confirmations
    let confirm_nats = nats.clone();
    let confirm_config = cfg.clone();
    task::spawn(async move {
        match confirm_nats.subscribe(subjects::WALLET_PACKAGE_CONFIRMATION).await {
            Ok(mut sub) => {
                info!(subject = subjects::WALLET_PACKAGE_CONFIRMATION, "Package confirmation subscribed");
                while let Some(msg) = sub.next().await {
                    match serde_json::from_slice::<PackageConfirmation>(&msg.payload) {
                        Ok(confirmation) => {
                            if confirm_config.package_hash_verification {
                                // TODO: Verify package hash and signature
                                // For now, just log the confirmation
                                info!(
                                    package_id=%confirmation.package_id,
                                    user_id=%confirmation.user_id,
                                    package_type=?confirmation.package_type,
                                    received_amount=confirmation.received_amount,
                                    "Package confirmation received"
                                );
                            } else {
                                info!(
                                    package_id=%confirmation.package_id,
                                    user_id=%confirmation.user_id,
                                    package_type=?confirmation.package_type,
                                    received_amount=confirmation.received_amount,
                                    "Package confirmation received (hash verification disabled)"
                                );
                            }
                        }
                        Err(e) => {
                            error!(error=%e, "Invalid package confirmation payload");
                        }
                    }
                }
            }
            Err(e) => error!(error=%e, "failed to subscribe to package confirmations"),
        }
    });

    // HTTP server for /health and /metrics (async handlers)

    let app = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .route("/ubd/distribute", post(ubd_distribute))
        .route("/transaction/quote", post(transaction_quote))
        .route("/transaction/commit", post(transaction_commit))
        .route("/transaction/status", post(transaction_status))
        .with_state(state);

    let addr: SocketAddr = "0.0.0.0:8088".parse()?;
    tracing::info!(%addr, "starting vault HTTP server");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server = task::spawn(async move {
        axum::serve(listener, app).await?;
        Ok::<(), anyhow::Error>(())
    });

    server.await??;
    Ok(())
}
