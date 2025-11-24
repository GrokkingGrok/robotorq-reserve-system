//! Transaction models for audit trail

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Transaction types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionType {
    /// UBD inflow from DistoDam
    UbdInflow,
    /// Investment in BidNet contract
    Investment,
    /// Return from successful investment
    InvestmentReturn,
    /// Subscription fee payment
    Subscription,
    /// Transfer received from another wallet
    TransferIn,
    /// Transfer sent to another wallet
    TransferOut,
    /// Physical RoboTorq activation deposit
    Activation,
}

impl TransactionType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionType::UbdInflow => "ubd_inflow",
            TransactionType::Investment => "investment",
            TransactionType::InvestmentReturn => "investment_return",
            TransactionType::Subscription => "subscription",
            TransactionType::TransferIn => "transfer_in",
            TransactionType::TransferOut => "transfer_out",
            TransactionType::Activation => "activation",
        }
    }

    /// Create from string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ubd_inflow" => Some(TransactionType::UbdInflow),
            "investment" => Some(TransactionType::Investment),
            "investment_return" => Some(TransactionType::InvestmentReturn),
            "subscription" => Some(TransactionType::Subscription),
            "transfer_in" => Some(TransactionType::TransferIn),
            "transfer_out" => Some(TransactionType::TransferOut),
            "activation" => Some(TransactionType::Activation),
            _ => None,
        }
    }
}

/// Transaction request from wallet to vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRequest {
    /// Source wallet ID
    pub from_wallet_id: String,
    /// Destination wallet ID
    pub to_wallet_id: String,
    /// Amount to transfer in jouletorq
    pub amount_jouletorq: i64,
    /// Desired completion timeframe in seconds
    pub timeframe_seconds: u64,
}

/// Transaction quote from vault to wallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionQuote {
    /// Unique quote ID
    pub quote_id: String,
    /// Transaction request this quote is for
    pub request: TransactionRequest,
    /// Calculated transaction fee in jouletorq
    pub fee_jouletorq: i64,
    /// Estimated completion time in seconds
    pub estimated_completion_seconds: u64,
    /// Whether the transaction is possible
    pub possible: bool,
    /// Reason if not possible
    pub reason: Option<String>,
    /// Quote expiration timestamp
    pub expires_at: DateTime<Utc>,
}

/// Transaction commitment from wallet to vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionCommitment {
    /// Quote ID being committed to
    pub quote_id: String,
    /// Whether the wallet accepts the quote
    pub accepted: bool,
}

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    /// Transaction is being quoted
    Quoted,
    /// Transaction committed and in progress
    InProgress,
    /// Transaction completed successfully
    Completed,
    /// Transaction failed
    Failed,
    /// Transaction cancelled
    Cancelled,
}

impl TransactionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionStatus::Quoted => "quoted",
            TransactionStatus::InProgress => "in_progress",
            TransactionStatus::Completed => "completed",
            TransactionStatus::Failed => "failed",
            TransactionStatus::Cancelled => "cancelled",
        }
    }
}

/// Transaction execution details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionExecution {
    /// Unique transaction ID
    pub transaction_id: String,
    /// Quote this execution is based on
    pub quote_id: String,
    /// Current status
    pub status: TransactionStatus,
    /// Progress (0.0 to 1.0)
    pub progress: f64,
    /// Amount transferred so far
    pub transferred_jouletorq: i64,
    /// Start timestamp
    pub started_at: DateTime<Utc>,
    /// Completion timestamp (if completed)
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message (if failed)
    pub error_message: Option<String>,
}

/// Status response for transaction queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStatusResponse {
    pub transaction_id: String,
    pub status: TransactionStatus,
    pub quote: Option<TransactionQuote>,
    pub execution: Option<TransactionExecution>,
    pub last_updated: DateTime<Utc>,
}

impl TransactionStatusResponse {
    pub fn new(
        transaction_id: String,
        status: TransactionStatus,
        quote: Option<TransactionQuote>,
        execution: Option<TransactionExecution>,
    ) -> Self {
        Self {
            transaction_id,
            status,
            quote,
            execution,
            last_updated: Utc::now(),
        }
    }
}

/// Transaction record for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Unique transaction ID
    pub id: String,
    /// Wallet ID this transaction belongs to
    pub wallet_id: String,
    /// Type of transaction
    pub transaction_type: TransactionType,
    /// Amount (positive for inflow, negative for outflow)
    pub amount: f64,
    /// Balance after this transaction
    pub balance_after: f64,
    /// Related entity (contract ID, request ID, etc.)
    pub related_to: Option<String>,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(
        wallet_id: String,
        transaction_type: TransactionType,
        amount: f64,
        balance_after: f64,
        related_to: Option<String>,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        Self {
            id: format!("tx-{}", uuid::Uuid::new_v4()),
            wallet_id,
            transaction_type,
            amount,
            balance_after,
            related_to,
            metadata,
            created_at: Utc::now(),
        }
    }

    /// Create UBD inflow transaction
    pub fn ubd_inflow(wallet_id: String, amount: f64, balance_after: f64, request_id: String) -> Self {
        Self::new(
            wallet_id,
            TransactionType::UbdInflow,
            amount,
            balance_after,
            Some(request_id),
            None,
        )
    }

    /// Create investment transaction
    pub fn investment(wallet_id: String, amount: f64, balance_after: f64, contract_id: String) -> Self {
        Self::new(
            wallet_id,
            TransactionType::Investment,
            -amount, // Negative for outflow
            balance_after,
            Some(contract_id),
            None,
        )
    }

    /// Create transfer out transaction
    pub fn transfer_out(wallet_id: String, amount: f64, balance_after: f64, transaction_id: String) -> Self {
        Self::new(
            wallet_id,
            TransactionType::TransferOut,
            -amount, // Negative for outflow
            balance_after,
            Some(transaction_id),
            None,
        )
    }

    /// Create transfer in transaction
    pub fn transfer_in(wallet_id: String, amount: f64, balance_after: f64, transaction_id: String) -> Self {
        Self::new(
            wallet_id,
            TransactionType::TransferIn,
            amount, // Positive for inflow
            balance_after,
            Some(transaction_id),
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_type_conversion() {
        assert_eq!(TransactionType::UbdInflow.as_str(), "ubd_inflow");
        assert_eq!(TransactionType::from_str("ubd_inflow"), Some(TransactionType::UbdInflow));
        assert_eq!(TransactionType::from_str("invalid"), None);
    }

    #[test]
    fn test_transaction_status_as_str() {
        assert_eq!(TransactionStatus::Quoted.as_str(), "quoted");
        assert_eq!(TransactionStatus::InProgress.as_str(), "in_progress");
        assert_eq!(TransactionStatus::Completed.as_str(), "completed");
        assert_eq!(TransactionStatus::Failed.as_str(), "failed");
        assert_eq!(TransactionStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn test_transaction_request_creation() {
        let request = TransactionRequest {
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
    fn test_transaction_quote_creation() {
        let request = TransactionRequest {
            from_wallet_id: "wallet-001".to_string(),
            to_wallet_id: "wallet-002".to_string(),
            amount_jouletorq: 1000,
            timeframe_seconds: 3600,
        };

        let quote = TransactionQuote {
            quote_id: "quote-123".to_string(),
            request: request.clone(),
            fee_jouletorq: 10,
            estimated_completion_seconds: 3660,
            possible: true,
            reason: None,
            expires_at: Utc::now() + chrono::Duration::seconds(300),
        };

        assert_eq!(quote.quote_id, "quote-123");
        assert_eq!(quote.request.from_wallet_id, "wallet-001");
        assert_eq!(quote.fee_jouletorq, 10);
        assert_eq!(quote.estimated_completion_seconds, 3660);
        assert!(quote.possible);
        assert!(quote.reason.is_none());
    }

    #[test]
    fn test_transaction_commitment_creation() {
        let commitment = TransactionCommitment {
            quote_id: "quote-123".to_string(),
            accepted: true,
        };

        assert_eq!(commitment.quote_id, "quote-123");
        assert!(commitment.accepted);
    }

    #[test]
    fn test_transaction_execution_creation() {
        let execution = TransactionExecution {
            transaction_id: "tx-123".to_string(),
            quote_id: "quote-123".to_string(),
            status: TransactionStatus::InProgress,
            progress: 0.5,
            transferred_jouletorq: 500,
            started_at: Utc::now(),
            completed_at: None,
            error_message: None,
        };

        assert_eq!(execution.transaction_id, "tx-123");
        assert_eq!(execution.quote_id, "quote-123");
        assert_eq!(execution.status, TransactionStatus::InProgress);
        assert_eq!(execution.progress, 0.5);
        assert_eq!(execution.transferred_jouletorq, 500);
        assert!(execution.completed_at.is_none());
        assert!(execution.error_message.is_none());
    }

    #[test]
    fn test_transfer_transactions() {
        let tx_out = Transaction::transfer_out(
            "wallet-001".to_string(),
            100.0,
            900.0,
            "tx-123".to_string(),
        );

        assert_eq!(tx_out.amount, -100.0); // Negative for outflow
        assert_eq!(tx_out.balance_after, 900.0);
        assert!(matches!(tx_out.transaction_type, TransactionType::TransferOut));

        let tx_in = Transaction::transfer_in(
            "wallet-002".to_string(),
            100.0,
            1100.0,
            "tx-123".to_string(),
        );

        assert_eq!(tx_in.amount, 100.0); // Positive for inflow
        assert_eq!(tx_in.balance_after, 1100.0);
        assert!(matches!(tx_in.transaction_type, TransactionType::TransferIn));
    }

    #[test]
    fn test_transaction_serialization() {
        let tx = Transaction::ubd_inflow(
            "wallet-123".to_string(),
            10.0,
            25.0,
            "request-456".to_string(),
        );

        // Test JSON serialization
        let json = serde_json::to_string(&tx).unwrap();
        let deserialized: Transaction = serde_json::from_str(&json).unwrap();

        assert_eq!(tx.id, deserialized.id);
        assert_eq!(tx.wallet_id, deserialized.wallet_id);
        assert_eq!(tx.amount, deserialized.amount);
        assert_eq!(tx.balance_after, deserialized.balance_after);
    }
}