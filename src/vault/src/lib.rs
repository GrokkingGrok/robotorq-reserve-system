pub mod config;
pub mod nats_client;
pub mod models;
pub mod shadow_vaults;
pub mod events;
pub mod metrics;
pub mod persistence;
pub mod contract_approval;
pub mod crypto;
#[cfg(test)]
mod config_simulation_tests;

pub use config::VaultConfig;
pub use shadow_vaults::{cert_vault::ShadowCertVault, stake_vault::ShadowStakeVault, distostream_vault::ShadowDistoVault, short_vault::{ShortVault, ShortVaultRegistry}};
pub use metrics::VaultMetrics;
pub use persistence::VaultPersistence;
pub use contract_approval::{ContractApproval, ApprovalConfig, PendingContract};
pub use models::{TransactionRequest, TransactionQuote, TransactionCommitment, TransactionExecution, TransactionStatus, TransactionStatusResponse};
