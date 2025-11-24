#[cfg(feature = "crypto")]
pub mod crypto {
    use common::crypto::{parse_kind, new_algorithm, SignatureAlgorithm};
    use sha2::{Sha256, Digest};

    /// Cryptographic signing and verification operations
    pub struct CryptoOps {
        algo: Box<dyn SignatureAlgorithm + Send + Sync>,
    }

    impl CryptoOps {
        pub fn new(algorithm: &str) -> anyhow::Result<Self> {
            let kind = parse_kind(algorithm)?;
            let algo = new_algorithm(kind)?;
            Ok(Self { algo })
        }

        pub fn public_key(&self) -> &[u8] { self.algo.public_key() }

        pub fn sign_hash(&self, message_hash_hex: &str) -> anyhow::Result<Vec<u8>> {
            let bytes = hex::decode(message_hash_hex)?;
            Ok(self.algo.sign(&bytes)?)
        }

        pub fn verify_hash(&self, message_hash_hex: &str, signature: &[u8]) -> anyhow::Result<bool> {
            let bytes = hex::decode(message_hash_hex)?;
            Ok(self.algo.verify(&bytes, signature))
        }

        pub fn sha256_hash(data: &[u8]) -> String {
            let mut h = Sha256::new();
            h.update(data);
            format!("{:x}", h.finalize())
        }
    }

    /// Package signing operations for vault packages
    pub mod package_signing {
        use super::*;
        use crate::models::{UBDDistributionPackage, DemurrageReleasePackage, PackageConfirmation};

        impl UBDDistributionPackage {
            pub fn sign_with(&mut self, ops: &CryptoOps) {
                let payload = format!("{}:{}:{}", self.user_id, self.amount_canonical_jouletorq, self.certificates.len());
                let hash = CryptoOps::sha256_hash(payload.as_bytes());
                if let Ok(sig) = ops.sign_hash(&hash) {
                    self.vault_signature = Some(hex::encode(sig));
                }
            }
            pub fn verify_with(&self, ops: &CryptoOps) -> bool {
                if let Some(sig_hex) = &self.vault_signature {
                    let payload = format!("{}:{}:{}", self.user_id, self.amount_canonical_jouletorq, self.certificates.len());
                    let hash = CryptoOps::sha256_hash(payload.as_bytes());
                    if let Ok(sig_bytes) = hex::decode(sig_hex) {
                        return ops.verify_hash(&hash, &sig_bytes).unwrap_or(false);
                    }
                }
                false
            }
        }

        impl DemurrageReleasePackage {
            pub fn sign_with(&mut self, ops: &CryptoOps) {
                let payload = format!("{}:{}", self.user_id, self.amount_canonical_jouletorq);
                let hash = CryptoOps::sha256_hash(payload.as_bytes());
                if let Ok(sig) = ops.sign_hash(&hash) {
                    self.vault_signature = Some(hex::encode(sig));
                }
            }
            pub fn verify_with(&self, ops: &CryptoOps) -> bool {
                if let Some(sig_hex) = &self.vault_signature {
                    let payload = format!("{}:{}", self.user_id, self.amount_canonical_jouletorq);
                    let hash = CryptoOps::sha256_hash(payload.as_bytes());
                    if let Ok(sig_bytes) = hex::decode(sig_hex) {
                        return ops.verify_hash(&hash, &sig_bytes).unwrap_or(false);
                    }
                }
                false
            }
        }

        impl PackageConfirmation {
            pub fn sign_with(&mut self, ops: &CryptoOps) {
                let payload = format!("{}:{}:{}:{}", self.package_id, self.user_id, self.received_amount, self.package_hash);
                let hash = CryptoOps::sha256_hash(payload.as_bytes());
                if let Ok(sig) = ops.sign_hash(&hash) {
                    self.wallet_signature = Some(hex::encode(sig));
                }
            }
            pub fn verify_with(&self, ops: &CryptoOps) -> bool {
                if let Some(sig_hex) = &self.wallet_signature {
                    let payload = format!("{}:{}:{}:{}", self.package_id, self.user_id, self.received_amount, self.package_hash);
                    let hash = CryptoOps::sha256_hash(payload.as_bytes());
                    if let Ok(sig_bytes) = hex::decode(sig_hex) {
                        return ops.verify_hash(&hash, &sig_bytes).unwrap_or(false);
                    }
                }
                false
            }
        }
    }
}

#[cfg(not(feature = "crypto"))]
pub mod crypto {
    use crate::models::{UBDDistributionPackage, DemurrageReleasePackage, PackageConfirmation};

    /// No-op crypto operations when crypto is disabled
    pub struct CryptoOps;

    impl CryptoOps {
        /// Generate a mock keypair (returns empty data)
        pub fn generate_falcon_keypair() -> (Vec<u8>, Vec<u8>) {
            (vec![], vec![])
        }

        /// Mock signing (returns empty signature)
        pub fn sign_falcon(_secret_key: &[u8], _data: &[u8]) -> Vec<u8> {
            vec![]
        }

        /// Mock verification (always succeeds)
        pub fn verify_falcon(_public_key: &[u8], _signature: &[u8]) -> Result<(), &'static str> {
            Ok(())
        }

        /// Compute simple hash when crypto is disabled
        pub fn sha256_hash(data: &[u8]) -> String {
            // Simple fallback hash - NOT cryptographically secure
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            data.hash(&mut hasher);
            format!("{:x}", hasher.finish())
        }
    }

    /// Mock package signing when crypto is disabled
    pub mod package_signing {
        use super::*;
        use crate::models::{UBDDistributionPackage, DemurrageReleasePackage, PackageConfirmation};

        impl UBDDistributionPackage {
            /// Mock signing (sets a placeholder signature)
            pub fn sign(&mut self, _secret_key: &[u8]) {
                self.vault_signature = Some("mock-signature-no-crypto".to_string());
            }

            /// Mock verification (always succeeds)
            pub fn verify_signature(&self, _public_key: &[u8]) -> Result<(), &'static str> {
                Ok(())
            }
        }

        impl DemurrageReleasePackage {
            /// Mock signing
            pub fn sign(&mut self, _secret_key: &[u8]) {
                self.vault_signature = Some("mock-signature-no-crypto".to_string());
            }

            /// Mock verification
            pub fn verify_signature(&self, _public_key: &[u8]) -> Result<(), &'static str> {
                Ok(())
            }
        }

        impl PackageConfirmation {
            /// Mock signing
            pub fn sign(&mut self, _secret_key: &[u8]) {
                self.wallet_signature = Some("mock-signature-no-crypto".to_string());
            }

            /// Mock verification
            pub fn verify_signature(&self, _public_key: &[u8]) -> Result<(), &'static str> {
                Ok(())
            }
        }
    }
}