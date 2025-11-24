#[cfg(feature = "crypto")]
pub mod crypto {
    use pqcrypto_falcon::falcon1024;
    use sha256::{digest, Sha256};

    /// Cryptographic signing and verification operations
    pub struct CryptoOps;

    impl CryptoOps {
        /// Generate a new Falcon-1024 keypair for signing
        pub fn generate_falcon_keypair() -> (falcon1024::PublicKey, falcon1024::SecretKey) {
            falcon1024::keypair()
        }

        /// Sign data with Falcon-1024
        pub fn sign_falcon(secret_key: &falcon1024::SecretKey, data: &[u8]) -> falcon1024::SignedMessage {
            falcon1024::sign(data, secret_key)
        }

        /// Verify Falcon-1024 signature
        pub fn verify_falcon(public_key: &falcon1024::PublicKey, signed_message: &falcon1024::SignedMessage) -> Result<(), &'static str> {
            falcon1024::verify(signed_message, public_key)
                .map_err(|_| "Falcon signature verification failed")
        }

        /// Compute SHA256 hash of data
        pub fn sha256_hash(data: &[u8]) -> String {
            format!("{:x}", Sha256::digest(data))
        }
    }

    /// Package signing operations for vault packages
    pub mod package_signing {
        use super::*;
        use crate::models::{UBDDistributionPackage, DemurrageReleasePackage, PackageConfirmation};

        impl UBDDistributionPackage {
            /// Sign a UBD package with Falcon-1024
            pub fn sign(&mut self, secret_key: &falcon1024::SecretKey) {
                let data_to_sign = format!("{}:{}:{}", self.user_id, self.amount_canonical_jouletorq, self.certificates.len());
                let signature = CryptoOps::sign_falcon(secret_key, data_to_sign.as_bytes());
                self.vault_signature = Some(hex::encode(signature.as_bytes()));
            }

            /// Verify a UBD package signature
            pub fn verify_signature(&self, public_key: &falcon1024::PublicKey) -> Result<(), &'static str> {
                if let Some(ref sig_hex) = self.vault_signature {
                    let data_to_verify = format!("{}:{}:{}", self.user_id, self.amount_canonical_jouletorq, self.certificates.len());
                    let signature_bytes = hex::decode(sig_hex)
                        .map_err(|_| "Invalid signature hex encoding")?;
                    let signed_message = falcon1024::SignedMessage::from_bytes(&signature_bytes)
                        .map_err(|_| "Invalid signature format")?;

                    CryptoOps::verify_falcon(public_key, &signed_message)
                } else {
                    Err("No signature present")
                }
            }
        }

        impl DemurrageReleasePackage {
            /// Sign a demurrage package
            pub fn sign(&mut self, secret_key: &falcon1024::SecretKey) {
                let data_to_sign = format!("{}:{}", self.user_id, self.amount_canonical_jouletorq);
                let signature = CryptoOps::sign_falcon(secret_key, data_to_sign.as_bytes());
                self.vault_signature = Some(hex::encode(signature.as_bytes()));
            }

            /// Verify a demurrage package signature
            pub fn verify_signature(&self, public_key: &falcon1024::PublicKey) -> Result<(), &'static str> {
                if let Some(ref sig_hex) = self.vault_signature {
                    let data_to_verify = format!("{}:{}", self.user_id, self.amount_canonical_jouletorq);
                    let signature_bytes = hex::decode(sig_hex)
                        .map_err(|_| "Invalid signature hex encoding")?;
                    let signed_message = falcon1024::SignedMessage::from_bytes(&signature_bytes)
                        .map_err(|_| "Invalid signature format")?;

                    CryptoOps::verify_falcon(public_key, &signed_message)
                } else {
                    Err("No signature present")
                }
            }
        }

        impl PackageConfirmation {
            /// Sign a package confirmation
            pub fn sign(&mut self, secret_key: &falcon1024::SecretKey) {
                let data_to_sign = format!("{}:{}:{}:{}", self.package_id, self.user_id, self.received_amount, self.package_hash);
                let signature = CryptoOps::sign_falcon(secret_key, data_to_sign.as_bytes());
                self.wallet_signature = Some(hex::encode(signature.as_bytes()));
            }

            /// Verify a package confirmation signature
            pub fn verify_signature(&self, public_key: &falcon1024::PublicKey) -> Result<(), &'static str> {
                if let Some(ref sig_hex) = self.wallet_signature {
                    let data_to_verify = format!("{}:{}:{}:{}", self.package_id, self.user_id, self.received_amount, self.package_hash);
                    let signature_bytes = hex::decode(sig_hex)
                        .map_err(|_| "Invalid signature hex encoding")?;
                    let signed_message = falcon1024::SignedMessage::from_bytes(&signature_bytes)
                        .map_err(|_| "Invalid signature format")?;

                    CryptoOps::verify_falcon(public_key, &signed_message)
                } else {
                    Err("No signature present")
                }
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