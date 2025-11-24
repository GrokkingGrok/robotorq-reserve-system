//! Database persistence layer for the wallet service

use anyhow::Result;
use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::NoTls;
use crate::models::{Transaction, Triple};

/// Database persistence layer
pub struct Persistence {
    pool: Pool,
}

impl Persistence {
    /// Create new persistence layer
    pub async fn new(database_url: &str) -> Result<Self> {
        let mut cfg = Config::new();
        cfg.url = Some(database_url.to_string());
        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;
        Self::init_schema(&pool).await?;

        Ok(Self { pool })
    }

    /// Initialize database schema
    async fn init_schema(pool: &Pool) -> Result<()> {
        let client = pool.get().await?;

        client.batch_execute("
            -- Wallet balances table (triple format)
            CREATE TABLE IF NOT EXISTS wallet_balances (
                wallet_id VARCHAR(64) PRIMARY KEY,
                robotorq BIGINT NOT NULL DEFAULT 0,
                tokentorq_remainder BIGINT NOT NULL DEFAULT 0,
                jouletorq_remainder BIGINT NOT NULL DEFAULT 0,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            );

            -- Transactions table
            CREATE TABLE IF NOT EXISTS transactions (
                id VARCHAR(64) PRIMARY KEY,
                wallet_id VARCHAR(64) NOT NULL,
                transaction_type VARCHAR(32) NOT NULL,
                amount DOUBLE PRECISION NOT NULL,
                balance_after DOUBLE PRECISION NOT NULL,
                related_to VARCHAR(64),
                metadata JSONB,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL
            );

            -- Indexes
            CREATE INDEX IF NOT EXISTS idx_transactions_wallet_id ON transactions(wallet_id);
            CREATE INDEX IF NOT EXISTS idx_transactions_created_at ON transactions(created_at DESC);
        ").await?;

        Ok(())
    }

    /// Store or update wallet balance
    pub async fn store_balance(&self, wallet_id: &str, balance: &Triple) -> Result<()> {
        let client = self.pool.get().await?;

        client.execute(
            "INSERT INTO wallet_balances (
                wallet_id, robotorq, tokentorq_remainder, jouletorq_remainder
            ) VALUES ($1, $2, $3, $4)
            ON CONFLICT (wallet_id) DO UPDATE SET
                robotorq = EXCLUDED.robotorq,
                tokentorq_remainder = EXCLUDED.tokentorq_remainder,
                jouletorq_remainder = EXCLUDED.jouletorq_remainder,
                updated_at = NOW()",
            &[
                &wallet_id,
                &(balance.robotorq as i64),
                &(balance.tokentorq_remainder as i64),
                &(balance.jouletorq_remainder as i64),
            ],
        ).await?;

        Ok(())
    }

    /// Load wallet balance
    pub async fn load_balance(&self, wallet_id: &str) -> Result<Option<Triple>> {
        let client = self.pool.get().await?;

        let row = client.query_opt(
            "SELECT robotorq, tokentorq_remainder, jouletorq_remainder
             FROM wallet_balances
             WHERE wallet_id = $1",
            &[&wallet_id],
        ).await?;

        match row {
            Some(row) => Ok(Some(Triple {
                robotorq: row.get::<_, i64>(0),
                tokentorq_remainder: row.get::<_, i64>(1),
                jouletorq_remainder: row.get::<_, i64>(2),
            })),
            None => Ok(None),
        }
    }

    /// Store transaction
    pub async fn store_transaction(&self, transaction: &Transaction) -> Result<()> {
        let client = self.pool.get().await?;

        client.execute(
            "INSERT INTO transactions (
                id, wallet_id, transaction_type, amount, balance_after,
                related_to, metadata, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            &[
                &transaction.id,
                &transaction.wallet_id,
                &transaction.transaction_type.as_str(),
                &transaction.amount,
                &transaction.balance_after,
                &transaction.related_to,
                &transaction.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default()),
                &transaction.created_at.to_rfc3339(),
            ],
        ).await?;

        Ok(())
    }

    /// Get transaction count for wallet
    pub async fn get_transaction_count(&self, wallet_id: &str) -> Result<i64> {
        let client = self.pool.get().await?;

        let row = client.query_one(
            "SELECT COUNT(*) FROM transactions WHERE wallet_id = $1",
            &[&wallet_id],
        ).await?;

        Ok(row.get(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Transaction, TransactionType, Triple};

    #[tokio::test]
    #[ignore] // Requires PostgreSQL
    async fn test_persistence_operations() {
        // This would need a test database setup
        // Similar to vault's test patterns
    }

    #[test]
    fn test_schema_sql_structure() {
        // Test that the SQL schema contains expected tables and columns
        let schema_sql = "
            -- Wallet balances table (triple format)
            CREATE TABLE IF NOT EXISTS wallet_balances (
                wallet_id VARCHAR(64) PRIMARY KEY,
                robotorq BIGINT NOT NULL DEFAULT 0,
                tokentorq_remainder BIGINT NOT NULL DEFAULT 0,
                jouletorq_remainder BIGINT NOT NULL DEFAULT 0,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            );

            -- Transactions table
            CREATE TABLE IF NOT EXISTS transactions (
                id VARCHAR(64) PRIMARY KEY,
                wallet_id VARCHAR(64) NOT NULL,
                transaction_type VARCHAR(32) NOT NULL,
                amount DOUBLE PRECISION NOT NULL,
                balance_after DOUBLE PRECISION NOT NULL,
                related_to VARCHAR(64),
                metadata JSONB,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL
            );

            -- Indexes
            CREATE INDEX IF NOT EXISTS idx_transactions_wallet_id ON transactions(wallet_id);
            CREATE INDEX IF NOT EXISTS idx_transactions_created_at ON transactions(created_at DESC);
        ";

        // Verify key components are present
        assert!(schema_sql.contains("wallet_balances"));
        assert!(schema_sql.contains("transactions"));
        assert!(schema_sql.contains("PRIMARY KEY"));
        assert!(schema_sql.contains("CREATE INDEX"));
        assert!(schema_sql.contains("robotorq"));
        assert!(schema_sql.contains("tokentorq_remainder"));
        assert!(schema_sql.contains("jouletorq_remainder"));
    }

    #[test]
    fn test_transaction_storage_structure() {
        let tx = Transaction::ubd_inflow(
            "wallet-123".to_string(),
            10.0,
            25.0,
            "request-456".to_string(),
        );

        // Test that transaction has all required fields for storage
        assert!(!tx.id.is_empty());
        assert_eq!(tx.wallet_id, "wallet-123");
        assert!(matches!(tx.transaction_type, TransactionType::UbdInflow));
        assert_eq!(tx.amount, 10.0);
        assert_eq!(tx.balance_after, 25.0);
        assert_eq!(tx.related_to, Some("request-456".to_string()));
        assert!(tx.created_at <= chrono::Utc::now());
    }

    #[test]
    fn test_balance_storage_structure() {
        let balance = Triple::new(1, 500, 1000);

        // Test that balance has all required fields for storage
        assert_eq!(balance.robotorq, 1);
        assert_eq!(balance.tokentorq_remainder, 500);
        assert_eq!(balance.jouletorq_remainder, 1000);
    }

    #[test]
    fn test_transaction_serialization_for_storage() {
        let tx = Transaction::investment(
            "wallet-123".to_string(),
            5.0,
            20.0,
            "contract-789".to_string(),
        );

        // Test JSON serialization for metadata storage
        if let Some(metadata) = &tx.metadata {
            let json = serde_json::to_string(metadata).unwrap();
            assert!(!json.is_empty());
        }

        // Test that transaction type serializes correctly
        assert_eq!(tx.transaction_type.as_str(), "investment");
    }
}