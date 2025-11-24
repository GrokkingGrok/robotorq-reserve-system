use std::sync::Arc;
use crate::models::{robotorq_certificate::RoboTorqCertificate, robo_torq_proof::{RoboTorqProof, ProofSignature}};
use common::crypto::{SignatureAlgorithm, parse_kind, new_algorithm};
use common::merkle::build_merkle_root;
use crate::metrics::MintMetrics;
use crate::config::MintConfig;
use std::time::SystemTime;
use sha2::{Sha256, Digest};
use common::triples::jouletorq_to_triple;

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
    pub async fn create_proof(&self, certificate: &RoboTorqCertificate, ingot_hashes: Vec<String>, total_joules: i64) -> Result<RoboTorqProof, Box<dyn std::error::Error + Send + Sync>> {
        let proof_id = format!("proof-{}", certificate.cert_id);

        // Build full merkle tree levels from ingot hashes for retention
        let (merkle_root, merkle_levels) = Self::build_full_merkle(&ingot_hashes);

        // Create initial proof structure
        let mut proof = RoboTorqProof {
            proof_id: proof_id.clone(),
            certificate_id: certificate.cert_id.clone(),
            merkle_root: merkle_root.clone(),
            merkle_tree: merkle_levels,
            ingot_hashes,
            signatures: vec![],
            timestamp: SystemTime::now(),
            proof_hash: "".to_string(),
            total_jouletorq: 0,
            total_triple: common::triples::Triple::zero(),
        };

        // Set total joule-torq & triple decomposition
        proof.set_totals(total_joules);

        // Add cryptographic signature if enabled
        if self.crypto_enabled() { self.add_signatures_sync(&mut proof, certificate)?; } else { self.add_placeholder_signature(&mut proof); }

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

        // Validate ingot structural invariants before proceeding
        // Lightweight structural validation (strict unit hash count deferred to upstream)
        for ingot in &ingots {
            if ingot.unit_count != 3600 { return Err(format!("invalid ingot {}: unit_count {} != 3600", ingot.ingot_id, ingot.unit_count).into()); }
            if ingot.joules_total != ingot.unit_count as i64 { return Err(format!("invalid ingot {}: joules_total {} != unit_count {}", ingot.ingot_id, ingot.joules_total, ingot.unit_count).into()); }
            if ingot.merkle_root.len() != 64 { return Err(format!("invalid ingot {}: merkle_root length {} != 64", ingot.ingot_id, ingot.merkle_root.len()).into()); }
        }
        // Collect ingot hashes for merkle tree (using ingot_id as leaf proxy now)
        let ingot_hashes: Vec<String> = ingots.iter().map(|i| i.ingot_id.clone()).collect();

        // Build merkle tree
        let merkle_root = if ingot_hashes.len() > 1 {
            build_merkle_root(&ingot_hashes)
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

        // Calculate totals with overflow-safe intermediate for stake
        let total_joules_i64: i64 = ingots.iter().map(|i| i.joules_total).sum();
        let stake_sum_i128: i128 = ingots.iter().map(|i| i.robostake_total_jouletorq as i128).sum();
        if stake_sum_i128 < 0 { return Err("negative stake total".into()); }
        if stake_sum_i128 > i64::MAX as i128 { return Err("stake overflow".into()); }
        let total_stake_jouletorq: i64 = stake_sum_i128 as i64;

        // Create certificate
        let mut certificate = RoboTorqCertificate {
            cert_id: cert_id.clone(),
            robotorq_proof_id: format!("proof-{}", cert_id),
            merkle_root: merkle_root.clone(),
            contract_ids,
            timestamp_nanos: SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as i64,
            hash: "".to_string(), // Will be computed after creation
            status: crate::models::robotorq_certificate::CertStatus::Digital,
            bearer_bond_id: None,
            total_jouletorq: total_joules_i64,
            total_stake_jouletorq: total_stake_jouletorq,
            total_triple: common::triples::Triple::zero(),
        };

        certificate.derive_triple();
        // Invariant check (Option A): triple must reconstruct total_jouletorq exactly.
        // We intentionally DO NOT include jouletorq_remainder in the hash because
        // total_jouletorq + (robotorq, tokentorq_remainder) already fully determine it.
        // This keeps hash payload lean while preventing silent drift via explicit check.
        let expected_triple = jouletorq_to_triple(certificate.total_jouletorq);
        if expected_triple.robotorq != certificate.total_triple.robotorq
            || expected_triple.tokentorq_remainder != certificate.total_triple.tokentorq_remainder
            || expected_triple.jouletorq_remainder != certificate.total_triple.jouletorq_remainder {
            return Err("triple invariant mismatch (async path)".into());
        }

        // Create proof
        let proof = self.create_proof(&certificate, ingot_hashes, total_joules_i64).await?;

        // Compute certificate hash
        // Hash payload (Option A): omit jouletorq_remainder as it is implied by total_jouletorq & higher-order buckets.
        let cert_hash_data = format!(
            "{}{}{}{}{}{}{}",
            certificate.cert_id,
            certificate.merkle_root,
            certificate.contract_ids.len(),
            certificate.timestamp_nanos,
            certificate.total_jouletorq,
            certificate.total_triple.robotorq,
            certificate.total_triple.tokentorq_remainder
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

        for ingot in &ingots {
            if ingot.unit_count != 3600 { return Err(format!("invalid ingot {}: unit_count {} != 3600", ingot.ingot_id, ingot.unit_count).into()); }
            if ingot.joules_total != ingot.unit_count as i64 { return Err(format!("invalid ingot {}: joules_total {} != unit_count {}", ingot.ingot_id, ingot.joules_total, ingot.unit_count).into()); }
            if ingot.merkle_root.len() != 64 { return Err(format!("invalid ingot {}: merkle_root length {} != 64", ingot.ingot_id, ingot.merkle_root.len()).into()); }
        }
        let ingot_hashes: Vec<String> = ingots.iter().map(|i| i.ingot_id.clone()).collect();

        // Build merkle tree
        let (merkle_root, merkle_levels) = Self::build_full_merkle(&ingot_hashes);

        // Collect contract IDs
        let contract_ids: Vec<String> = ingots.iter()
            .map(|i| i.contract_id.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        // Calculate totals with overflow-safe intermediate for stake
        let total_joules_i64: i64 = ingots.iter().map(|i| i.joules_total).sum();
        let stake_sum_i128: i128 = ingots.iter().map(|i| i.robostake_total_jouletorq as i128).sum();
        if stake_sum_i128 < 0 { return Err("negative stake total".into()); }
        if stake_sum_i128 > i64::MAX as i128 { return Err("stake overflow".into()); }
        let total_stake_jouletorq: i64 = stake_sum_i128 as i64;

        // Create certificate
        let mut certificate = RoboTorqCertificate {
            cert_id: cert_id.clone(),
            robotorq_proof_id: format!("proof-{}", cert_id),
            merkle_root: merkle_root.clone(),
            contract_ids,
            timestamp_nanos: SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as i64,
            hash: "".to_string(), // Will be computed after creation
            status: crate::models::robotorq_certificate::CertStatus::Digital,
            bearer_bond_id: None,
            total_jouletorq: total_joules_i64,
            total_stake_jouletorq: total_stake_jouletorq,
            total_triple: common::triples::Triple::zero(),
        };

        certificate.derive_triple();
        // Invariant check (sync path)
        let expected_triple = jouletorq_to_triple(certificate.total_jouletorq);
        if expected_triple.robotorq != certificate.total_triple.robotorq
            || expected_triple.tokentorq_remainder != certificate.total_triple.tokentorq_remainder
            || expected_triple.jouletorq_remainder != certificate.total_triple.jouletorq_remainder {
            return Err("triple invariant mismatch (sync path)".into());
        }

        // Create proof synchronously
        let mut proof = RoboTorqProof {
            proof_id: format!("proof-{}", cert_id),
            certificate_id: certificate.cert_id.clone(),
            merkle_root: merkle_root.clone(),
            merkle_tree: merkle_levels,
            ingot_hashes,
            signatures: vec![],
            timestamp: SystemTime::now(),
            proof_hash: "".to_string(),
            total_jouletorq: 0,
            total_triple: common::triples::Triple::zero(),
        };

        proof.set_totals(total_joules_i64);

        // Add cryptographic signature if enabled (sync path)
        if self.crypto_enabled() { self.add_signatures_sync(&mut proof, &certificate)?; } else { self.add_placeholder_signature(&mut proof); }

        // Create proof hash for integrity
        proof.proof_hash = self.compute_proof_hash(&proof)?;

        // Update metrics
        self.metrics.inc_proofs_created();
        self.metrics.inc_certificates_created();
        if !proof.signatures.is_empty() {
            self.metrics.inc_signatures_created(proof.signatures.len() as i64);
        }

        // Compute certificate hash
        // Hash payload (Option A) - see async path comment.
        let cert_hash_data = format!(
            "{}{}{}{}{}{}{}",
            certificate.cert_id,
            certificate.merkle_root,
            certificate.contract_ids.len(),
            certificate.timestamp_nanos,
            certificate.total_jouletorq,
            certificate.total_triple.robotorq,
            certificate.total_triple.tokentorq_remainder
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

    /// Add deterministic placeholder signature metadata when crypto disabled, enabling pipeline consumers
    /// to rely on uniform structure (algorithm="none", fingerprint=sha256(cert_id||proof_id)).
    fn add_placeholder_signature(&self, proof: &mut RoboTorqProof) {
        use sha2::{Sha256, Digest};
        let payload = format!("{}{}", proof.certificate_id, proof.proof_id);
        let mut h = Sha256::new();
        h.update(payload.as_bytes());
        let fp = format!("{:x}", h.finalize());
        proof.signatures.push(ProofSignature {
            signer_id: "mint-service".into(),
            algorithm: "none".into(),
            signature: vec![],
            message_hash: fp.clone(),
            key_fingerprint: fp,
            public_key: vec![],
            timestamp: SystemTime::now(),
        });
    }

    /// Build full merkle tree levels (Vec<Vec<String>>) from leaves; returns (root, levels).
    fn build_full_merkle(leaves: &[String]) -> (String, Vec<Vec<String>>) {
        if leaves.is_empty() { return (String::new(), vec![]); }
        let mut levels: Vec<Vec<String>> = Vec::new();
        levels.push(leaves.to_vec());
        let mut current = leaves.to_vec();
        while current.len() > 1 {
            let mut next: Vec<String> = Vec::with_capacity((current.len()+1)/2);
            for i in (0..current.len()).step_by(2) {
                let left = &current[i];
                let right = if i+1 < current.len() { &current[i+1] } else { left }; // duplicate last if odd
                let combined = format!("{}{}", left, right);
                next.push(sha256_hash(combined.as_bytes()));
            }
            levels.push(next.clone());
            current = next;
        }
        let root = current[0].clone();
        (root, levels)
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
        let mut c = RoboTorqCertificate {
            cert_id: id.to_string(),
            robotorq_proof_id: format!("proof-{}", id),
            merkle_root: "test_root".to_string(),
            contract_ids: vec!["contract-1".to_string()],
            timestamp_nanos: 1234567890,
            hash: "test_hash".to_string(),
            status: CertStatus::Digital,
            bearer_bond_id: None,
            total_jouletorq: 3600 * 1000, // simulate one certificate worth of joules
            total_stake_jouletorq: 3600 * 500, // simulate stake amount
            total_triple: common::triples::Triple::zero(),
        };
        c.derive_triple();
        c
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
            enable_archive: true,
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
            enable_archive: true,
        });

        let metrics = Arc::new(MintMetrics::new());
        let engine = ProofEngine::new(config, Arc::clone(&metrics));

        let certificate = create_test_certificate("cert-1");
        let ingot_hashes = vec!["hash1".to_string(), "hash2".to_string()];

        let proof = engine.create_proof(&certificate, ingot_hashes, certificate.total_jouletorq).await.unwrap();
        assert_eq!(proof.certificate_id, "cert-1");
        // Placeholder signature present when crypto disabled
        assert_eq!(proof.signatures.len(), 1, "expected placeholder signature when crypto disabled");
        assert_eq!(proof.signatures[0].algorithm, "none");
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
            enable_archive: true,
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
            enable_archive: true,
        });

        let metrics = Arc::new(MintMetrics::new());
        let engine = ProofEngine::new(config, Arc::clone(&metrics));

        assert!(engine.crypto_enabled());

        let certificate = create_test_certificate("cert-1");
        let ingot_hashes = vec!["hash1".to_string(), "hash2".to_string()];

        let proof = engine.create_proof(&certificate, ingot_hashes, certificate.total_jouletorq).await.unwrap();

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
            enable_archive: true,
        });
        let metrics = Arc::new(MintMetrics::new());
        let engine1 = ProofEngine::new(Arc::clone(&config1), Arc::clone(&metrics));
        let cert1 = create_test_certificate("cert-a");
        let proof1 = engine1.create_proof(&cert1, vec!["hash1".to_string()], cert1.total_jouletorq).await.unwrap();
        let sig1 = &proof1.signatures[0];
        let pub1 = sig1.public_key.clone();
        let fp1 = sig1.key_fingerprint.clone();

        // Second engine, same path, should reuse keypair (same public key)
        let config2 = Arc::new(MintConfig { key_storage_path: Some(base_str.clone()), ..(*config1).clone() });
        let engine2 = ProofEngine::new(Arc::clone(&config2), Arc::clone(&metrics));
        let cert2 = create_test_certificate("cert-b");
        let proof2 = engine2.create_proof(&cert2, vec!["hash2".to_string()], cert2.total_jouletorq).await.unwrap();
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
            enable_archive: true,
        });
        let metrics = Arc::new(MintMetrics::new());
        let engine = ProofEngine::new(config, Arc::clone(&metrics));
        let certificate = create_test_certificate("cert-neg");
        let proof = engine.create_proof(&certificate, vec!["hashX".to_string()], certificate.total_jouletorq).await.unwrap();
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