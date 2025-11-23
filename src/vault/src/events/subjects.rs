// NATS subjects used by stripped-down MVP
pub const PHASE3_COMPLETED: &str = "vault.phase3.completed"; // Mint -> Vault
pub const CERT_STORED: &str = "vault.cert.stored";           // Vault -> external observers
pub const ROBOSTAKE_RETURNED: &str = "vault.robostake.returned"; // Vault -> observers
pub const STAKE_ALLOCATED: &str = "vault.stake.allocated";    // Allocation events
