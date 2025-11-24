use std::sync::Arc;
use crate::models::{robotorq_certificate::RoboTorqCertificate, robo_torq_proof::{RoboTorqProof, ProofSignature}};
use common::crypto::{SignatureAlgorithm, parse_kind, new_algorithm};
use crate::metrics::MintMetrics;
use crate::config::MintConfig;
use std::time::SystemTime;
use sha2::{Sha256, Digest};

/// Helper function for SHA256 hashing
fn sha256_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// ProofEngine: Creates cryptographic proofs for certificates
pub struct ProofEngine {
    crypto: Option<Arc<dyn SignatureAlgorithm>>,
    config: Arc<MintConfig>,
    metrics: Arc<MintMetrics>,
    keypair: Option<(Vec<u8>, Vec<u8>)>, // (public, secret) persistent if configured
}

impl ProofEngine {
    pub fn new(config: Arc<MintConfig>, metrics: Arc<MintMetrics>) -> Self {
        let crypto: Option<Arc<dyn SignatureAlgorithm>> = if config.enable_crypto {
            match parse_kind(&config.signature_algorithm).map(|kind| new_algorithm(kind)) {
                Ok(algo) => Some(Arc::from(algo)),
                Err(e) => {
                    tracing::warn!(
                        "Unsupported signature algorithm '{}': {}. Running without signatures.",
                        config.signature_algorithm, e
                    );
                    None
                }
            }
        } else { None };

        // Load or generate a persistent keypair if crypto enabled
        let keypair = if let Some(algo) = &crypto {
            match Self::load_or_generate_keypair(algo.as_ref(), config.key_storage_path.as_ref()) {
                Ok(kp) => Some(kp),
                Err(e) => {
                    tracing::warn!("Failed to load/generate keypair: {}. Falling back to ephemeral keys per proof.", e);
                    None
                }
            }
        } else { None };

        Self { crypto, config, metrics, keypair }
    }

    fn load_or_generate_keypair(algo: &dyn SignatureAlgorithm, path_opt: Option<&String>) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
        use std::fs;
        use std::path::Path;
        if let Some(base_path) = path_opt {
            let pub_path = format!("{}.pub", base_path);
            let sec_path = format!("{}.sec", base_path);
            let pub_exists = Path::new(&pub_path).exists();
            let sec_exists = Path::new(&sec_path).exists();
            if pub_exists && sec_exists {
                let pub_bytes = fs::read(&pub_path)?;
                let sec_bytes = fs::read(&sec_path)?;
                return Ok((pub_bytes, sec_bytes));
            }
            let (public, secret) = algo.generate_keypair()?;
            // Attempt to create parent directory if missing
            if let Some(parent) = Path::new(base_path).parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            fs::write(&pub_path, &public)?;
            fs::write(&sec_path, &secret)?;
            Ok((public, secret))
        } else {
            // No path configured, generate ephemeral pair (not persisted)
            algo.generate_keypair()
        }
    }

    /// Create a proof for a certificate with cryptographic signing
    pub async fn create_proof(&self, certificate: &RoboTorqCertificate, ingot_hashes: Vec<String>) -> Result<RoboTorqProof, Box<dyn std::error::Error + Send + Sync>> {
        let proof_id = format!("proof-{}", certificate.cert_id);

        // Build merkle tree from ingot hashes
        let merkle_root = if ingot_hashes.len() > 1 {
            crate::engine::merkle::build_merkle_root(&ingot_hashes)?
        } else if ingot_hashes.len() == 1 {
            ingot_hashes[0].clone()
        } else {
            "".to_string()
        };

        // Create initial proof structure
        let mut proof = RoboTorqProof {
            proof_id: proof_id.clone(),
            certificate_id: certificate.cert_id.clone(),
            merkle_root: merkle_root.clone(),
            merkle_tree: vec![], // TODO: Store full merkle tree for verification
            ingot_hashes,
            signatures: vec![],
            timestamp: SystemTime::now(),
            proof_hash: "".to_string(),
        };

        // Add cryptographic signature if enabled
        if self.crypto_enabled() { self.add_signatures_sync(&mut proof, certificate)?; }

        // Create proof hash for integrity
        proof.proof_hash = self.compute_proof_hash(&proof)?;

        // Update metrics
        self.metrics.inc_proofs_created();
        if !proof.signatures.is_empty() {
            self.metrics.inc_signatures_created(proof.signatures.len() as i64);
        }

        Ok(proof)
    }

    /// Create both certificate and proof from ingots (async)
    pub async fn create_certificate_and_proof(&self, ingots: Vec<crate::models::token_torq_ingot::TokenTorqIngot>) -> Result<(RoboTorqCertificate, RoboTorqProof), Box<dyn std::error::Error + Send + Sync>> {
        use std::time::{SystemTime, UNIX_EPOCH};
        use std::collections::HashSet;

        // Generate certificate ID
        let cert_id = format!("cert-{}", SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos());

        // Collect ingot hashes for merkle tree
        let ingot_hashes: Vec<String> = ingots.iter().map(|i| i.ingot_id.clone()).collect();

        // Build merkle tree
        let merkle_root = if ingot_hashes.len() > 1 {
            crate::engine::merkle::build_merkle_root(&ingot_hashes)?
        } else if ingot_hashes.len() == 1 {
            ingot_hashes[0].clone()
        } else {
            "".to_string()
        };

        // Collect contract IDs
        let contract_ids: Vec<String> = ingots.iter()
            .map(|i| i.contract_id.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        // Calculate totals
        let total_joules: f64 = ingots.iter().map(|i| i.joules_total).sum();
        let _total_stake: i64 = ingots.iter().map(|i| i.robostake_total_micro_rt).sum();

        // Create certificate
        let certificate = RoboTorqCertificate {
            cert_id: cert_id.clone(),
            robotorq_proof_id: format!("proof-{}", cert_id),
            merkle_root: merkle_root.clone(),
            contract_ids,
            timestamp_nanos: SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as i64,
            hash: "".to_string(), // Will be computed after creation
            status: crate::models::robotorq_certificate::CertStatus::Digital,
            bearer_bond_id: None,
        };

        // Create proof
        let proof = self.create_proof(&certificate, ingot_hashes).await?;

        // Compute certificate hash
        let cert_hash_data = format!(
            "{}{}{}{}{}",
            certificate.cert_id,
            certificate.merkle_root,
            certificate.contract_ids.len(),
            certificate.timestamp_nanos,
            total_joules
        );
        let cert_hash = sha256_hash(cert_hash_data.as_bytes());

        // Update certificate hash (we need to clone and modify)
        let mut final_certificate = certificate;
        final_certificate.hash = cert_hash;

        Ok((final_certificate, proof))
    }

    /// Create both certificate and proof from ingots (sync)
    pub fn create_certificate_and_proof_sync(&self, ingots: Vec<crate::models::token_torq_ingot::TokenTorqIngot>) -> Result<(RoboTorqCertificate, RoboTorqProof), Box<dyn std::error::Error + Send + Sync>> {
        use std::time::{SystemTime, UNIX_EPOCH};
        use std::collections::HashSet;

        // Generate certificate ID
        let cert_id = format!("cert-{}", SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos());

        // Collect ingot hashes for merkle tree
        let ingot_hashes: Vec<String> = ingots.iter().map(|i| i.ingot_id.clone()).collect();

        // Build merkle tree
        let merkle_root = if ingot_hashes.len() > 1 {
            crate::engine::merkle::build_merkle_root(&ingot_hashes)?
        } else if ingot_hashes.len() == 1 {
            ingot_hashes[0].clone()
        } else {
            "".to_string()
        };

        // Collect contract IDs
        let contract_ids: Vec<String> = ingots.iter()
            .map(|i| i.contract_id.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        // Calculate totals
        let total_joules: f64 = ingots.iter().map(|i| i.joules_total).sum();
        let _total_stake: i64 = ingots.iter().map(|i| i.robostake_total_micro_rt).sum();

        // Create certificate
        let certificate = RoboTorqCertificate {
            cert_id: cert_id.clone(),
            robotorq_proof_id: format!("proof-{}", cert_id),
            merkle_root: merkle_root.clone(),
            contract_ids,
            timestamp_nanos: SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as i64,
            hash: "".to_string(), // Will be computed after creation
            status: crate::models::robotorq_certificate::CertStatus::Digital,
            bearer_bond_id: None,
        };

        // Create proof synchronously
        let mut proof = RoboTorqProof {
            proof_id: format!("proof-{}", cert_id),
            certificate_id: certificate.cert_id.clone(),
            merkle_root: merkle_root.clone(),
            merkle_tree: vec![], // TODO: Store full merkle tree for verification
            ingot_hashes,
            signatures: vec![],
            timestamp: SystemTime::now(),
            proof_hash: "".to_string(),
        };

        // Add cryptographic signature if enabled (sync path)
        if self.crypto_enabled() { self.add_signatures_sync(&mut proof, &certificate)?; }

        // Create proof hash for integrity
        proof.proof_hash = self.compute_proof_hash(&proof)?;

        // Update metrics
        self.metrics.inc_proofs_created();
        self.metrics.inc_certificates_created();
        if !proof.signatures.is_empty() {
            self.metrics.inc_signatures_created(proof.signatures.len() as i64);
        }

        // Compute certificate hash
        let cert_hash_data = format!(
            "{}{}{}{}{}",
            certificate.cert_id,
            certificate.merkle_root,
            certificate.contract_ids.len(),
            certificate.timestamp_nanos,
            total_joules
        );
        let cert_hash = sha256_hash(cert_hash_data.as_bytes());

        // Update certificate hash
        let mut final_certificate = certificate;
        final_certificate.hash = cert_hash;

        Ok((final_certificate, proof))
    }

    /// Add cryptographic signature to the proof (single signer)
    fn add_signatures_sync(&self, proof: &mut RoboTorqProof, certificate: &RoboTorqCertificate) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // For now, we'll sign as the mint service itself
        // In a real implementation, this would involve multiple parties
        let signer_id = "mint-service".to_string();

        // Create signature data (certificate + proof data)
        let signature_data = self.create_signature_data(certificate, proof)?;
        let message_hash = sha256_hash(&signature_data);

        // Use persistent keypair if available, else fall back to generating anew
        let (public_key, secret_key) = if let Some(kp) = &self.keypair {
            (kp.0.clone(), kp.1.clone())
        } else if let Some(crypto) = &self.crypto {
            crypto.generate_keypair()?
        } else { return Ok(()); };

        // Sign the data
        let signature = if let Some(crypto) = &self.crypto { crypto.sign(message_hash.as_bytes(), &secret_key)? } else { return Ok(()); };

        // Add signature to proof
        let key_fingerprint = sha256_hash(&public_key);
        let proof_signature = ProofSignature { signer_id, algorithm: self.config.signature_algorithm.clone(), signature, message_hash, key_fingerprint, public_key, timestamp: SystemTime::now() };

        proof.signatures.push(proof_signature);

        Ok(())
    }

    /// Add cryptographic signatures to the proof (synchronous version) - stub when crypto disabled

    /// Create the data that will be signed
    fn create_signature_data(&self, certificate: &RoboTorqCertificate, proof: &RoboTorqProof) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // Combine certificate and proof data for signing
        let data = format!(
            "{}{}{}{}{}",
            certificate.cert_id,
            certificate.merkle_root,
            proof.proof_id,
            proof.merkle_root,
            certificate.timestamp_nanos
        );

        Ok(data.into_bytes())
    }

    /// Compute a hash of the proof for integrity verification
    fn compute_proof_hash(&self, proof: &RoboTorqProof) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Create a deterministic representation of the proof
        let hash_data = format!(
            "{}{}{}{}{:?}",
            proof.proof_id,
            proof.certificate_id,
            proof.merkle_root,
            proof.signatures.len(),
            proof.timestamp
        );

        Ok(sha256_hash(hash_data.as_bytes()))
    }

    /// Validate that a certificate meets minimum stake requirements
    pub fn validate_stake(&self, certificate: &RoboTorqCertificate, total_stake_micro_rt: i64) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if total_stake_micro_rt < self.config.min_stake_micro_rt {
            tracing::warn!(
                "Certificate {} has insufficient stake: {} < {}",
                certificate.cert_id,
                total_stake_micro_rt,
                self.config.min_stake_micro_rt
            );
            return Ok(false);
        }

        Ok(true)
    }

    /// Check if crypto operations are enabled
    pub fn crypto_enabled(&self) -> bool { self.crypto.is_some() }

    /// Get the signature algorithm name
    pub fn signature_algorithm(&self) -> &str {
        &self.config.signature_algorithm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::robotorq_certificate::CertStatus;

    fn create_test_certificate(id: &str) -> RoboTorqCertificate {
        RoboTorqCertificate {
            cert_id: id.to_string(),
            robotorq_proof_id: format!("proof-{}", id),
            merkle_root: "test_root".to_string(),
            contract_ids: vec!["contract-1".to_string()],
            timestamp_nanos: 1234567890,
            hash: "test_hash".to_string(),
            status: CertStatus::Digital,
            bearer_bond_id: None,
        }
    }

    #[test]
    fn test_proof_engine_creation() {
        let config = Arc::new(MintConfig {
            nats_url: "nats://test".to_string(),
            ingots_per_cert: 1000,
            batch_threshold_count: 10,
            batch_threshold_seconds: 300,
            proof_interval_count: 100,
            proof_interval_seconds: 3600,
            merkle_tree_depth: 16,
            enable_crypto: false,
            signature_algorithm: "dilithium5".to_string(),
            min_stake_micro_rt: 50000,
            key_storage_path: None,
        });

        let metrics = Arc::new(MintMetrics::new());
        let engine = ProofEngine::new(config, Arc::clone(&metrics));

        assert!(!engine.crypto_enabled());
        assert_eq!(engine.signature_algorithm(), "dilithium5");
    }

    #[tokio::test]
    async fn test_proof_creation_without_crypto() {
        let config = Arc::new(MintConfig {
            nats_url: "nats://test".to_string(),
            ingots_per_cert: 1000,
            batch_threshold_count: 10,
            batch_threshold_seconds: 300,
            proof_interval_count: 100,
            proof_interval_seconds: 3600,
            merkle_tree_depth: 16,
            enable_crypto: false,
            signature_algorithm: "dilithium5".to_string(),
            min_stake_micro_rt: 50000,
            key_storage_path: None,
        });

        let metrics = Arc::new(MintMetrics::new());
        let engine = ProofEngine::new(config, Arc::clone(&metrics));

        let certificate = create_test_certificate("cert-1");
        let ingot_hashes = vec!["hash1".to_string(), "hash2".to_string()];

        let proof = engine.create_proof(&certificate, ingot_hashes).await.unwrap();

        assert_eq!(proof.certificate_id, "cert-1");
        assert_eq!(proof.signatures.len(), 0); // No crypto = no signatures
        assert!(!proof.proof_hash.is_empty());
    }

    #[test]
    fn test_stake_validation() {
        let config = Arc::new(MintConfig {
            nats_url: "nats://test".to_string(),
            ingots_per_cert: 1000,
            batch_threshold_count: 10,
            batch_threshold_seconds: 300,
            proof_interval_count: 100,
            proof_interval_seconds: 3600,
            merkle_tree_depth: 16,
            enable_crypto: false,
            signature_algorithm: "dilithium5".to_string(),
            min_stake_micro_rt: 50000,
            key_storage_path: None,
        });

        let metrics = Arc::new(MintMetrics::new());
        let engine = ProofEngine::new(config, Arc::clone(&metrics));

        let certificate = create_test_certificate("cert-1");

        // Test sufficient stake
        assert!(engine.validate_stake(&certificate, 60000).unwrap());

        // Test insufficient stake
        assert!(!engine.validate_stake(&certificate, 40000).unwrap());
    }

    #[tokio::test]
    async fn test_proof_creation_with_crypto_enabled() {
        let config = Arc::new(MintConfig {
            nats_url: "nats://test".to_string(),
            ingots_per_cert: 1000,
            batch_threshold_count: 10,
            batch_threshold_seconds: 300,
            proof_interval_count: 100,
            proof_interval_seconds: 3600,
            merkle_tree_depth: 16,
            enable_crypto: true,
            signature_algorithm: "falcon1024".to_string(),
            min_stake_micro_rt: 50000,
            key_storage_path: None,
        });

        let metrics = Arc::new(MintMetrics::new());
        let engine = ProofEngine::new(config, Arc::clone(&metrics));

        assert!(engine.crypto_enabled());

        let certificate = create_test_certificate("cert-1");
        let ingot_hashes = vec!["hash1".to_string(), "hash2".to_string()];

        let proof = engine.create_proof(&certificate, ingot_hashes).await.unwrap();

        assert_eq!(proof.certificate_id, "cert-1");
        assert_eq!(proof.signatures.len(), 1, "expected one signature when crypto enabled");
        assert!(!proof.proof_hash.is_empty());

        let signature = &proof.signatures[0];
        assert_eq!(signature.algorithm, "falcon1024");
        assert!(!signature.signature.is_empty());
        assert!(!signature.public_key.is_empty());
        assert_eq!(signature.message_hash.len(), 64); // hex sha256
        assert_eq!(signature.key_fingerprint.len(), 64); // sha256 hex of public key
        // Verify by recomputing hash and validating signature
        use common::crypto::{parse_kind, new_algorithm};
        let algo = new_algorithm(parse_kind(&signature.algorithm).unwrap());
        let recomputed = sha256_hash(&engine.create_signature_data(&create_test_certificate("cert-1"), &proof).unwrap());
        assert_eq!(recomputed, signature.message_hash);
        assert!(algo.verify(signature.message_hash.as_bytes(), &signature.signature, &signature.public_key).unwrap());
    }

    #[tokio::test]
    async fn test_persistent_keypair_reuse() {
        // Use a temp file base path
        let base = std::env::temp_dir().join(format!("mint_falcon_key_{}", uuid::Uuid::new_v4()));
        let base_str = base.to_string_lossy().to_string();

        let config1 = Arc::new(MintConfig {
            nats_url: "nats://test".to_string(),
            ingots_per_cert: 1000,
            batch_threshold_count: 10,
            batch_threshold_seconds: 300,
            proof_interval_count: 100,
            proof_interval_seconds: 3600,
            merkle_tree_depth: 16,
            enable_crypto: true,
            signature_algorithm: "falcon1024".to_string(),
            min_stake_micro_rt: 50000,
            key_storage_path: Some(base_str.clone()),
        });
        let metrics = Arc::new(MintMetrics::new());
        let engine1 = ProofEngine::new(Arc::clone(&config1), Arc::clone(&metrics));
        let cert1 = create_test_certificate("cert-a");
        let proof1 = engine1.create_proof(&cert1, vec!["hash1".to_string()]).await.unwrap();
        let sig1 = &proof1.signatures[0];
        let pub1 = sig1.public_key.clone();
        let fp1 = sig1.key_fingerprint.clone();

        // Second engine, same path, should reuse keypair (same public key)
        let config2 = Arc::new(MintConfig { key_storage_path: Some(base_str.clone()), ..(*config1).clone() });
        let engine2 = ProofEngine::new(Arc::clone(&config2), Arc::clone(&metrics));
        let cert2 = create_test_certificate("cert-b");
        let proof2 = engine2.create_proof(&cert2, vec!["hash2".to_string()]).await.unwrap();
        let sig2 = &proof2.signatures[0];
        let pub2 = sig2.public_key.clone();
        let fp2 = sig2.key_fingerprint.clone();

        assert_eq!(pub1, pub2, "expected persistent keypair reuse (public keys differ)");
        assert_eq!(fp1, fp2, "expected fingerprint stability across proofs");
    }

    #[tokio::test]
    async fn test_signature_verification_negative() {
        let config = Arc::new(MintConfig {
            nats_url: "nats://test".to_string(),
            ingots_per_cert: 1000,
            batch_threshold_count: 10,
            batch_threshold_seconds: 300,
            proof_interval_count: 100,
            proof_interval_seconds: 3600,
            merkle_tree_depth: 16,
            enable_crypto: true,
            signature_algorithm: "falcon1024".to_string(),
            min_stake_micro_rt: 50000,
            key_storage_path: None,
        });
        let metrics = Arc::new(MintMetrics::new());
        let engine = ProofEngine::new(config, Arc::clone(&metrics));
        let certificate = create_test_certificate("cert-neg");
        let proof = engine.create_proof(&certificate, vec!["hashX".to_string()]).await.unwrap();
        let sig = &proof.signatures[0];
        use common::crypto::{parse_kind, new_algorithm};
        let algo = new_algorithm(parse_kind(&sig.algorithm).unwrap());
        // Positive verify
        assert!(algo.verify(sig.message_hash.as_bytes(), &sig.signature, &sig.public_key).unwrap());
        // Negative: tamper message hash
        let bad_hash = "0".repeat(64);
        assert!(!algo.verify(bad_hash.as_bytes(), &sig.signature, &sig.public_key).unwrap());
    }
}