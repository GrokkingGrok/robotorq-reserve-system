use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Request for a transaction quote from the vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRequest {
    pub transaction_id: Uuid,
    pub from_wallet_id: String,
    pub to_wallet_id: String,
    pub amount: f64,
    pub timeframe_hours: u32,
    pub requested_at: DateTime<Utc>,
}

/// Quote response from vault with fees and availability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionQuote {
    pub transaction_id: Uuid,
    pub quote_id: Uuid,
    pub from_wallet_id: String,
    pub to_wallet_id: String,
    pub amount: f64,
    pub timeframe_hours: u32,
    pub estimated_fee: f64,
    pub total_cost: f64,
    pub available_balance: f64,
    pub can_fulfill: bool,
    pub reason_if_unavailable: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub quoted_at: DateTime<Utc>,
}

/// Commitment to execute a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionCommitment {
    pub transaction_id: Uuid,
    pub quote_id: Uuid,
    pub from_wallet_id: String,
    pub to_wallet_id: String,
    pub committed_at: DateTime<Utc>,
}

/// Execution status and progress of a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionExecution {
    pub transaction_id: Uuid,
    pub status: TransactionStatus,
    pub progress_percentage: f64,
    pub amount: f64,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub streaming_updates: Vec<StreamingUpdate>,
}

/// Individual streaming update during transaction execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingUpdate {
    pub timestamp: DateTime<Utc>,
    pub amount_transferred: f64,
    pub remaining_amount: f64,
    pub current_fee: f64,
}

/// Overall status of a transaction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Quoted,
    Committed,
    Executing,
    Completed,
    Failed,
    Expired,
    Cancelled,
}

/// Status response for transaction queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStatusResponse {
    pub transaction_id: Uuid,
    pub status: TransactionStatus,
    pub quote: Option<TransactionQuote>,
    pub execution: Option<TransactionExecution>,
    pub last_updated: DateTime<Utc>,
}

impl TransactionRequest {
    pub fn new(from_wallet_id: String, to_wallet_id: String, amount: f64, timeframe_hours: u32) -> Self {
        Self {
            transaction_id: Uuid::new_v4(),
            from_wallet_id,
            to_wallet_id,
            amount,
            timeframe_hours,
            requested_at: Utc::now(),
        }
    }
}

impl TransactionQuote {
    pub fn new(
        transaction_id: Uuid,
        from_wallet_id: String,
        to_wallet_id: String,
        amount: f64,
        timeframe_hours: u32,
        estimated_fee: f64,
        available_balance: f64,
        can_fulfill: bool,
        reason_if_unavailable: Option<String>,
        expires_in_minutes: i64,
    ) -> Self {
        let total_cost = amount + estimated_fee;
        Self {
            transaction_id,
            quote_id: Uuid::new_v4(),
            from_wallet_id,
            to_wallet_id,
            amount,
            timeframe_hours,
            estimated_fee,
            total_cost,
            available_balance,
            can_fulfill,
            reason_if_unavailable,
            expires_at: Utc::now() + chrono::Duration::minutes(expires_in_minutes),
            quoted_at: Utc::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

impl TransactionCommitment {
    pub fn new(transaction_id: Uuid, quote_id: Uuid, from_wallet_id: String, to_wallet_id: String) -> Self {
        Self {
            transaction_id,
            quote_id,
            from_wallet_id,
            to_wallet_id,
            committed_at: Utc::now(),
        }
    }
}

impl TransactionExecution {
    pub fn new(transaction_id: Uuid, amount: f64) -> Self {
        Self {
            transaction_id,
            status: TransactionStatus::Pending,
            progress_percentage: 0.0,
            amount,
            started_at: None,
            completed_at: None,
            error_message: None,
            streaming_updates: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        self.status = TransactionStatus::Executing;
        self.started_at = Some(Utc::now());
    }

    pub fn complete(&mut self) {
        self.status = TransactionStatus::Completed;
        self.progress_percentage = 100.0;
        self.completed_at = Some(Utc::now());
    }

    pub fn fail(&mut self, error: String) {
        self.status = TransactionStatus::Failed;
        self.error_message = Some(error);
        self.completed_at = Some(Utc::now());
    }

    pub fn add_streaming_update(&mut self, amount_transferred: f64, remaining_amount: f64, current_fee: f64) {
        self.streaming_updates.push(StreamingUpdate {
            timestamp: Utc::now(),
            amount_transferred,
            remaining_amount,
            current_fee,
        });
        self.progress_percentage = ((self.amount - remaining_amount) / self.amount * 100.0).min(100.0);
    }
}

impl TransactionStatusResponse {
    pub fn new(
        transaction_id: Uuid,
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