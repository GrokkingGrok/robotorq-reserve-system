// NATS subjects used by stripped-down MVP
pub const PHASE3_COMPLETED: &str = "vault.phase3.completed"; // Mint -> Vault
pub const CERT_STORED: &str = "vault.cert.stored";           // Vault -> external observers
pub const ROBOSTAKE_RETURNED: &str = "vault.robostake.returned"; // Vault -> observers
pub const STAKE_ALLOCATED: &str = "vault.stake.allocated";    // Allocation events
pub const DISTOSTREAM_AUTHORIZED: &str = "vault.distostream.authorized"; // Vault -> DistoVault
pub const DISTOSTREAM_DISTRIBUTION_TICK: &str = "vault.distostream.distribution"; // DistoVault -> observers (tick)

// ShortVault subjects
pub const SHORTVAULT_CREATED: &str = "vault.shortvault.created"; // Vault -> observers
pub const SHORTVAULT_DEMURRAGE_APPLIED: &str = "vault.shortvault.demurrage.applied"; // Vault -> observers
pub const SHORTVAULT_UBD_CREDITED: &str = "vault.shortvault.ubd.credited"; // Vault -> observers
pub const SHORTVAULT_WALLET_TRANSFER: &str = "vault.shortvault.wallet.transfer"; // Vault -> observers

// Wallet <-> ShortVault communication
pub const WALLET_DEMURRAGE_REQUEST: &str = "wallet.demurrage.request"; // Wallet -> ShortVault
pub const SHORTVAULT_DEMURRAGE_RESPONSE: &str = "vault.shortvault.demurrage.response"; // ShortVault -> Wallet
pub const SHORTVAULT_DRIP_RELEASED: &str = "vault.shortvault.drip.released"; // ShortVault -> Wallet
pub const WALLET_BALANCE_REPLENISH: &str = "wallet.balance.replenish"; // Wallet -> ShortVault

// Single package delivery subjects (new)
pub const VAULT_UBD_PACKAGE: &str = "vault.ubd.package"; // Vault -> Wallet (single UBD package)
pub const VAULT_DEMURRAGE_PACKAGE: &str = "vault.demurrage.package"; // Vault -> Wallet (single demurrage package)
pub const WALLET_PACKAGE_CONFIRMATION: &str = "wallet.package.confirmation"; // Wallet -> Vault (package received/confirmed)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_subjects_defined() {
        // Test that all subjects are non-empty strings
        assert!(!PHASE3_COMPLETED.is_empty());
        assert!(!CERT_STORED.is_empty());
        assert!(!ROBOSTAKE_RETURNED.is_empty());
        assert!(!STAKE_ALLOCATED.is_empty());
        assert!(!DISTOSTREAM_AUTHORIZED.is_empty());
        assert!(!DISTOSTREAM_DISTRIBUTION_TICK.is_empty());

        assert!(!SHORTVAULT_CREATED.is_empty());
        assert!(!SHORTVAULT_DEMURRAGE_APPLIED.is_empty());
        assert!(!SHORTVAULT_UBD_CREDITED.is_empty());
        assert!(!SHORTVAULT_WALLET_TRANSFER.is_empty());

        assert!(!WALLET_DEMURRAGE_REQUEST.is_empty());
        assert!(!SHORTVAULT_DEMURRAGE_RESPONSE.is_empty());
        assert!(!SHORTVAULT_DRIP_RELEASED.is_empty());
        assert!(!WALLET_BALANCE_REPLENISH.is_empty());

        assert!(!VAULT_UBD_PACKAGE.is_empty());
        assert!(!VAULT_DEMURRAGE_PACKAGE.is_empty());
        assert!(!WALLET_PACKAGE_CONFIRMATION.is_empty());
    }

    #[test]
    fn test_subject_naming_conventions() {
        // Test that subjects follow expected naming patterns
        assert!(PHASE3_COMPLETED.contains("vault.") || PHASE3_COMPLETED.contains("mint."));
        assert!(CERT_STORED.contains("vault."));
        assert!(ROBOSTAKE_RETURNED.contains("vault."));
        assert!(STAKE_ALLOCATED.contains("vault."));
        assert!(DISTOSTREAM_AUTHORIZED.contains("vault."));
        assert!(DISTOSTREAM_DISTRIBUTION_TICK.contains("vault."));

        assert!(SHORTVAULT_CREATED.contains("vault."));
        assert!(SHORTVAULT_DEMURRAGE_APPLIED.contains("vault."));
        assert!(SHORTVAULT_UBD_CREDITED.contains("vault."));
        assert!(SHORTVAULT_WALLET_TRANSFER.contains("vault."));

        assert!(WALLET_DEMURRAGE_REQUEST.contains("wallet."));
        assert!(SHORTVAULT_DEMURRAGE_RESPONSE.contains("vault."));
        assert!(SHORTVAULT_DRIP_RELEASED.contains("vault."));
        assert!(WALLET_BALANCE_REPLENISH.contains("wallet."));

        assert!(VAULT_UBD_PACKAGE.contains("vault."));
        assert!(VAULT_DEMURRAGE_PACKAGE.contains("vault."));
        assert!(WALLET_PACKAGE_CONFIRMATION.contains("wallet."));
    }

    #[test]
    fn test_package_delivery_subjects() {
        // Test the new package delivery subjects specifically
        assert_eq!(VAULT_UBD_PACKAGE, "vault.ubd.package");
        assert_eq!(VAULT_DEMURRAGE_PACKAGE, "vault.demurrage.package");
        assert_eq!(WALLET_PACKAGE_CONFIRMATION, "wallet.package.confirmation");
    }

    #[test]
    fn test_no_duplicate_subjects() {
        let all_subjects = vec![
            PHASE3_COMPLETED,
            CERT_STORED,
            ROBOSTAKE_RETURNED,
            STAKE_ALLOCATED,
            DISTOSTREAM_AUTHORIZED,
            DISTOSTREAM_DISTRIBUTION_TICK,
            SHORTVAULT_CREATED,
            SHORTVAULT_DEMURRAGE_APPLIED,
            SHORTVAULT_UBD_CREDITED,
            SHORTVAULT_WALLET_TRANSFER,
            WALLET_DEMURRAGE_REQUEST,
            SHORTVAULT_DEMURRAGE_RESPONSE,
            SHORTVAULT_DRIP_RELEASED,
            WALLET_BALANCE_REPLENISH,
            VAULT_UBD_PACKAGE,
            VAULT_DEMURRAGE_PACKAGE,
            WALLET_PACKAGE_CONFIRMATION,
        ];

        let mut seen = std::collections::HashSet::new();
        for subject in all_subjects {
            assert!(!seen.contains(subject), "Duplicate subject found: {}", subject);
            seen.insert(subject);
        }
    }
}
