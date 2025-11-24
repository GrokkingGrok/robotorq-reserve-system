//! Package models for wallet service communication with vault

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// UBD Distribution Package - single delivery from vault to wallet
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UBDDistributionPackage {
    pub package_id: String,
    pub user_id: String,
    pub amount_canonical_jouletorq: i64,
    pub distribution_timestamp: DateTime<Utc>,
    pub provenance_cert_ids: Vec<String>,
    pub package_hash: String,
    pub vault_signature: Option<String>,
}

/// Demurrage Release Package - single delivery from vault to wallet
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemurrageReleasePackage {
    pub package_id: String,
    pub user_id: String,
    pub amount_canonical_jouletorq: i64,
    pub request_timestamp: DateTime<Utc>,
    pub release_timestamp: DateTime<Utc>,
    pub package_hash: String,
    pub vault_signature: Option<String>,
}

/// Package Confirmation - wallet acknowledges receipt to vault
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageConfirmation {
    pub package_id: String,
    pub user_id: String,
    pub package_type: PackageType,
    pub confirmation_timestamp: DateTime<Utc>,
    pub received_amount: i64,
    pub package_hash: String,
    pub wallet_signature: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PackageType {
    UBD,
    Demurrage,
}

impl PackageConfirmation {
    /// Create a new package confirmation
    pub fn new(
        package_id: String,
        user_id: String,
        package_type: PackageType,
        received_amount: i64,
        package_hash: String,
    ) -> Self {
        Self {
            package_id,
            user_id,
            package_type,
            confirmation_timestamp: Utc::now(),
            received_amount,
            package_hash,
            wallet_signature: None, // TODO: Add wallet signing
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_confirmation_creation() {
        let confirmation = PackageConfirmation::new(
            "pkg-123".to_string(),
            "wallet-001".to_string(),
            PackageType::UBD,
            1000,
            "hash123".to_string(),
        );

        assert_eq!(confirmation.package_id, "pkg-123");
        assert_eq!(confirmation.user_id, "wallet-001");
        assert_eq!(confirmation.package_type, PackageType::UBD);
        assert_eq!(confirmation.received_amount, 1000);
        assert_eq!(confirmation.package_hash, "hash123");
    }

    #[test]
    fn test_package_types() {
        assert_eq!(PackageType::UBD, PackageType::UBD);
        assert_eq!(PackageType::Demurrage, PackageType::Demurrage);
        assert_ne!(PackageType::UBD, PackageType::Demurrage);
    }
}