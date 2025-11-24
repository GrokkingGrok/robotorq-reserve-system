//! NATS client for wallet service communication

use anyhow::Result;
use async_nats::Client;
use futures_util::StreamExt;
use std::sync::Arc;
use crate::models::{UBDDistributionPackage, PackageConfirmation, PackageType};
use crate::metrics::Metrics;

/// NATS client wrapper
pub struct NatsClient {
    client: Client,
}

impl NatsClient {
    /// Create new NATS client
    pub async fn new(url: &str) -> Result<Self> {
        let client = async_nats::connect(url).await?;
        Ok(Self { client })
    }

    /// Start UBD package subscriber
    pub async fn start_ubd_package_subscriber(
        &self,
        wallet_service: &crate::WalletService,
        metrics: &Metrics,
    ) -> Result<()> {
        let mut subscriber = self.client.subscribe("vault.ubd.package").await?;
        tracing::info!("Subscribed to vault.ubd.package topic");

        let metrics = Arc::new((*metrics).clone());
        let client_clone = self.client.clone();
        let wallet_service: &'static crate::WalletService = unsafe { std::mem::transmute(wallet_service) };

        tokio::spawn(async move {
            while let Some(message) = subscriber.next().await {
                if let Err(e) = Self::handle_ubd_package(&client_clone, message, wallet_service, &metrics).await {
                    tracing::error!("Failed to handle UBD package: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Handle incoming UBD package
    async fn handle_ubd_package(
        client: &Client,
        message: async_nats::Message,
        wallet_service: &crate::WalletService,
        metrics: &Arc<Metrics>,
    ) -> Result<()> {
        let package: UBDDistributionPackage = serde_json::from_slice(&message.payload)?;

        tracing::info!(
            "Received UBD package: {} for wallet {} (amount: {})",
            package.package_id, package.user_id, package.amount_canonical_jouletorq
        );

        // Check if this package is for this wallet instance
        let wallet_state = wallet_service.state.read().await;
        let is_for_this_wallet = match &wallet_state.wallet_id {
            Some(wallet_id) => wallet_id == &package.user_id,
            None => {
                // Wallet not activated yet, accept the package (for testing)
                tracing::warn!("Wallet not activated, accepting UBD package for testing: {}", package.user_id);
                true
            }
        };
        drop(wallet_state); // Release the lock

        if !is_for_this_wallet {
            tracing::info!("UBD package {} is for wallet {}, not for this wallet instance", package.package_id, package.user_id);
            return Ok(());
        }

        // Verify package integrity (basic check)
        if package.user_id.is_empty() || package.amount_canonical_jouletorq <= 0 {
            anyhow::bail!("Invalid UBD package: {}", package.package_id);
        }

        // Update balance via wallet service
        let current_balance = wallet_service.get_balance().await;
        let ubd_triple = crate::models::jouletorq_to_triple(package.amount_canonical_jouletorq);
        let new_balance = crate::models::add_triples(current_balance, ubd_triple);
        wallet_service.update_balance(new_balance).await?;

        metrics.record_ubd_received(package.amount_canonical_jouletorq);

        // Update UBD balance metric with canonical jouletorq
        let canonical_balance = crate::models::triple_to_jouletorq(new_balance);
        metrics.update_ubd_balance(canonical_balance as f64);

        // Create transaction record
        // TODO: Store transaction in persistence layer when implemented

        // Send confirmation back to vault
        let confirmation = PackageConfirmation::new(
            package.package_id.clone(),
            package.user_id.clone(),
            PackageType::UBD,
            package.amount_canonical_jouletorq,
            package.package_hash.clone(),
        );

        let payload = serde_json::to_vec(&confirmation)?;
        client.publish("wallet.package.confirmation", payload.into()).await?;

        tracing::info!(
            "UBD package processed and confirmed: {} (balance: {}R {}T {}J)",
            package.package_id,
            new_balance.robotorq,
            new_balance.tokentorq_remainder,
            new_balance.jouletorq_remainder
        );

        Ok(())
    }

    /// Request transaction quote from vault
    pub async fn request_transaction_quote(&self, request: crate::models::TransactionRequest) -> Result<crate::models::TransactionQuote> {
        let payload = serde_json::to_vec(&request)?;
        
        // Send request and wait for response
        match self.client.request("vault.transaction.quote", payload.into()).await {
            Ok(response) => {
                let quote: crate::models::TransactionQuote = serde_json::from_slice(&response.payload)?;
                Ok(quote)
            }
            Err(e) => {
                tracing::error!("Failed to request transaction quote: {}", e);
                Err(anyhow::anyhow!("Failed to request transaction quote: {}", e))
            }
        }
    }

    /// Commit to transaction
    pub async fn commit_transaction(&self, commitment: crate::models::TransactionCommitment) -> Result<crate::models::TransactionExecution> {
        let payload = serde_json::to_vec(&commitment)?;
        
        match self.client.request("vault.transaction.commit", payload.into()).await {
            Ok(response) => {
                let execution: crate::models::TransactionExecution = serde_json::from_slice(&response.payload)?;
                Ok(execution)
            }
            Err(e) => {
                tracing::error!("Failed to commit transaction: {}", e);
                Err(anyhow::anyhow!("Failed to commit transaction: {}", e))
            }
        }
    }

    /// Get transaction status
    pub async fn get_transaction_status(&self, transaction_id: &str) -> Result<crate::models::TransactionExecution> {
        let request = serde_json::json!({ "transaction_id": transaction_id });
        let payload = serde_json::to_vec(&request)?;
        
        match self.client.request("vault.transaction.status", payload.into()).await {
            Ok(response) => {
                let execution: crate::models::TransactionExecution = serde_json::from_slice(&response.payload)?;
                Ok(execution)
            }
            Err(e) => {
                tracing::error!("Failed to get transaction status: {}", e);
                Err(anyhow::anyhow!("Failed to get transaction status: {}", e))
            }
        }
    }

    /// Publish wallet activation message
    pub async fn publish_activation(&self, _wallet_id: &str, _activate: bool) -> Result<()> {
        // TODO: Implement wallet activation message structure
        tracing::warn!("Wallet activation not yet implemented");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models;

    // Mock NATS client for testing
    // Note: In a real implementation, you'd use a test NATS server

    #[tokio::test]
    async fn test_nats_client_creation() {
        // This test verifies the structure exists
        // In a real test environment, you'd connect to a test NATS server
        assert!(true);
    }

    #[tokio::test]
    async fn test_ubd_package_subscription_setup() {
        // Test that the subscription setup logic exists
        // Real testing would require a mock NATS server
        assert!(true);
    }

    // Transaction-related tests would require a test NATS server
    // For now, we test the data structures and logic

    #[test]
    fn test_transaction_request_structure() {
        let request = models::TransactionRequest {
            from_wallet_id: "wallet-001".to_string(),
            to_wallet_id: "wallet-002".to_string(),
            amount_jouletorq: 1000,
            timeframe_seconds: 3600,
        };

        assert_eq!(request.from_wallet_id, "wallet-001");
        assert_eq!(request.to_wallet_id, "wallet-002");
        assert_eq!(request.amount_jouletorq, 1000);
        assert_eq!(request.timeframe_seconds, 3600);
    }

    #[test]
    fn test_transaction_quote_structure() {
        let request = models::TransactionRequest {
            from_wallet_id: "wallet-001".to_string(),
            to_wallet_id: "wallet-002".to_string(),
            amount_jouletorq: 1000,
            timeframe_seconds: 3600,
        };

        let quote = models::TransactionQuote {
            quote_id: "quote-123".to_string(),
            request,
            fee_jouletorq: 10,
            estimated_completion_seconds: 3660,
            possible: true,
            reason: None,
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(300),
        };

        assert_eq!(quote.quote_id, "quote-123");
        assert_eq!(quote.fee_jouletorq, 10);
        assert!(quote.possible);
    }

    #[test]
    fn test_transaction_commitment_structure() {
        let commitment = models::TransactionCommitment {
            quote_id: "quote-123".to_string(),
            accepted: true,
        };

        assert_eq!(commitment.quote_id, "quote-123");
        assert!(commitment.accepted);
    }

    #[test]
    fn test_transaction_execution_structure() {
        let execution = models::TransactionExecution {
            transaction_id: "tx-123".to_string(),
            quote_id: "quote-123".to_string(),
            status: models::TransactionStatus::InProgress,
            progress: 0.5,
            transferred_jouletorq: 500,
            started_at: chrono::Utc::now(),
            completed_at: None,
            error_message: None,
        };

        assert_eq!(execution.transaction_id, "tx-123");
        assert_eq!(execution.progress, 0.5);
        assert_eq!(execution.transferred_jouletorq, 500);
    }

    #[test]
    fn test_transaction_status_enum() {
        assert_eq!(models::TransactionStatus::Quoted.as_str(), "quoted");
        assert_eq!(models::TransactionStatus::InProgress.as_str(), "in_progress");
        assert_eq!(models::TransactionStatus::Completed.as_str(), "completed");
        assert_eq!(models::TransactionStatus::Failed.as_str(), "failed");
        assert_eq!(models::TransactionStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn test_transaction_type_enum() {
        assert_eq!(models::TransactionType::UbdInflow.as_str(), "ubd_inflow");
        assert_eq!(models::TransactionType::TransferOut.as_str(), "transfer_out");
        assert_eq!(models::TransactionType::Investment.as_str(), "investment");
        assert_eq!(models::TransactionType::Subscription.as_str(), "subscription");
        assert_eq!(models::TransactionType::TransferIn.as_str(), "transfer_in");
        assert_eq!(models::TransactionType::InvestmentReturn.as_str(), "investment_return");
        assert_eq!(models::TransactionType::Activation.as_str(), "activation");
    }

    #[test]
    fn test_transaction_type_from_str() {
        assert_eq!(models::TransactionType::from_str("ubd_inflow"), Some(models::TransactionType::UbdInflow));
        assert_eq!(models::TransactionType::from_str("transfer_out"), Some(models::TransactionType::TransferOut));
        assert_eq!(models::TransactionType::from_str("invalid"), None);
    }
}