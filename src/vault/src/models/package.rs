use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// UBD Distribution Package - single delivery to wallet
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UBDDistributionPackage {
    pub package_id: String,
    pub user_id: String,
    pub amount_canonical_jouletorq: i64,
    pub distribution_timestamp: DateTime<Utc>,
    pub provenance_cert_ids: Vec<String>,
    pub package_hash: String,
    pub vault_signature: Option<String>, // Placeholder for future crypto
}

/// Demurrage Release Package - single delivery to wallet
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemurrageReleasePackage {
    pub package_id: String,
    pub user_id: String,
    pub amount_canonical_jouletorq: i64,
    pub request_timestamp: DateTime<Utc>,
    pub release_timestamp: DateTime<Utc>,
    pub package_hash: String,
    pub vault_signature: Option<String>, // Placeholder for future crypto
}

/// Package Confirmation - wallet acknowledges receipt
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageConfirmation {
    pub package_id: String,
    pub user_id: String,
    pub package_type: PackageType,
    pub confirmation_timestamp: DateTime<Utc>,
    pub received_amount: i64,
    pub package_hash: String,
    pub wallet_signature: Option<String>, // Placeholder for future crypto
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PackageType {
    UBD,
    Demurrage,
}

impl UBDDistributionPackage {
    pub fn new(
        user_id: String,
        amount: i64,
        provenance_cert_ids: Vec<String>,
    ) -> Self {
        let package_id = format!("ubd-{}-{}", user_id, chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let distribution_timestamp = chrono::Utc::now();

        // Create package data for hashing (without hash field itself)
        let package_data = format!("{}:{}:{}:{:?}:{:?}",
            package_id,
            user_id,
            amount,
            distribution_timestamp,
            provenance_cert_ids
        );

        let package_hash = sha256::digest(package_data);

        Self {
            package_id,
            user_id,
            amount_canonical_jouletorq: amount,
            distribution_timestamp,
            provenance_cert_ids,
            package_hash,
            vault_signature: None, // TODO: Add real signing
        }
    }
}

impl DemurrageReleasePackage {
    pub fn new(
        user_id: String,
        amount: i64,
    ) -> Self {
        let package_id = format!("demurrage-{}-{}", user_id, chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let request_timestamp = chrono::Utc::now();
        let release_timestamp = chrono::Utc::now();

        // Create package data for hashing
        let package_data = format!("{}:{}:{}:{:?}:{:?}",
            package_id,
            user_id,
            amount,
            request_timestamp,
            release_timestamp
        );

        let package_hash = sha256::digest(package_data);

        Self {
            package_id,
            user_id,
            amount_canonical_jouletorq: amount,
            request_timestamp,
            release_timestamp,
            package_hash,
            vault_signature: None, // TODO: Add real signing
        }
    }
}

impl PackageConfirmation {
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
            confirmation_timestamp: chrono::Utc::now(),
            received_amount,
            package_hash,
            wallet_signature: None, // TODO: Add real signing
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ubd_package_creation() {
        let user_id = "user-123".to_string();
        let amount = 1000;
        let provenance = vec!["cert-1".to_string(), "cert-2".to_string()];

        let package = UBDDistributionPackage::new(user_id.clone(), amount, provenance.clone());

        assert_eq!(package.user_id, user_id);
        assert_eq!(package.amount_canonical_jouletorq, amount);
        assert_eq!(package.provenance_cert_ids, provenance);
        assert!(package.package_id.starts_with("ubd-user-123-"));
        assert!(!package.package_hash.is_empty());
        assert!(package.vault_signature.is_none());
    }

    #[test]
    fn test_ubd_package_hash_uniqueness() {
        let user_id = "user-123".to_string();
        let amount = 1000;
        let provenance = vec!["cert-1".to_string()];

        let package1 = UBDDistributionPackage::new(user_id.clone(), amount, provenance.clone());
        let package2 = UBDDistributionPackage::new(user_id, amount, provenance);

        // Different timestamps should produce different hashes
        assert_ne!(package1.package_hash, package2.package_hash);
        // But they should have the same user_id and amount
        assert_eq!(package1.user_id, package2.user_id);
        assert_eq!(package1.amount_canonical_jouletorq, package2.amount_canonical_jouletorq);
    }

    #[test]
    fn test_demurrage_package_creation() {
        let user_id = "user-456".to_string();
        let amount = 500;

        let package = DemurrageReleasePackage::new(user_id.clone(), amount);

        assert_eq!(package.user_id, user_id);
        assert_eq!(package.amount_canonical_jouletorq, amount);
        assert!(package.package_id.starts_with("demurrage-user-456-"));
        assert!(!package.package_hash.is_empty());
        assert!(package.vault_signature.is_none());
        assert!(package.release_timestamp >= package.request_timestamp);
    }

    #[test]
    fn test_demurrage_package_hash_uniqueness() {
        let user_id = "user-456".to_string();
        let amount = 500;

        let package1 = DemurrageReleasePackage::new(user_id.clone(), amount);
        let package2 = DemurrageReleasePackage::new(user_id, amount);

        // Different timestamps should produce different hashes
        assert_ne!(package1.package_hash, package2.package_hash);
        // But they should have the same user_id and amount
        assert_eq!(package1.user_id, package2.user_id);
        assert_eq!(package1.amount_canonical_jouletorq, package2.amount_canonical_jouletorq);
    }

    #[test]
    fn test_package_confirmation_creation() {
        let package_id = "test-package-123".to_string();
        let user_id = "user-789".to_string();
        let package_type = PackageType::UBD;
        let received_amount = 1000;
        let package_hash = "abcd1234".to_string();

        let confirmation = PackageConfirmation::new(
            package_id.clone(),
            user_id.clone(),
            package_type.clone(),
            received_amount,
            package_hash.clone(),
        );

        assert_eq!(confirmation.package_id, package_id);
        assert_eq!(confirmation.user_id, user_id);
        assert_eq!(confirmation.package_type, package_type);
        assert_eq!(confirmation.received_amount, received_amount);
        assert_eq!(confirmation.package_hash, package_hash);
        assert!(confirmation.wallet_signature.is_none());
    }

    #[test]
    fn test_package_types() {
        let ubd = PackageType::UBD;
        let demurrage = PackageType::Demurrage;

        assert_eq!(format!("{:?}", ubd), "UBD");
        assert_eq!(format!("{:?}", demurrage), "Demurrage");
    }

    #[test]
    fn test_ubd_package_serialization() {
        let package = UBDDistributionPackage::new(
            "user-123".to_string(),
            1000,
            vec!["cert-1".to_string()],
        );

        let json = serde_json::to_string(&package).unwrap();
        let deserialized: UBDDistributionPackage = serde_json::from_str(&json).unwrap();

        assert_eq!(package.package_id, deserialized.package_id);
        assert_eq!(package.user_id, deserialized.user_id);
        assert_eq!(package.amount_canonical_jouletorq, deserialized.amount_canonical_jouletorq);
        assert_eq!(package.package_hash, deserialized.package_hash);
    }

    #[test]
    fn test_demurrage_package_serialization() {
        let package = DemurrageReleasePackage::new("user-456".to_string(), 500);

        let json = serde_json::to_string(&package).unwrap();
        let deserialized: DemurrageReleasePackage = serde_json::from_str(&json).unwrap();

        assert_eq!(package.package_id, deserialized.package_id);
        assert_eq!(package.user_id, deserialized.user_id);
        assert_eq!(package.amount_canonical_jouletorq, deserialized.amount_canonical_jouletorq);
        assert_eq!(package.package_hash, deserialized.package_hash);
    }

    #[test]
    fn test_confirmation_serialization() {
        let confirmation = PackageConfirmation::new(
            "pkg-123".to_string(),
            "user-789".to_string(),
            PackageType::Demurrage,
            500,
            "hash123".to_string(),
        );

        let json = serde_json::to_string(&confirmation).unwrap();
        let deserialized: PackageConfirmation = serde_json::from_str(&json).unwrap();

        assert_eq!(confirmation.package_id, deserialized.package_id);
        assert_eq!(confirmation.user_id, deserialized.user_id);
        assert_eq!(confirmation.received_amount, deserialized.received_amount);
        assert_eq!(confirmation.package_hash, deserialized.package_hash);
    }
}