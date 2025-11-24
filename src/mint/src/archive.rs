use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::models::{robotorq_certificate::RoboTorqCertificate, robo_torq_proof::RoboTorqProof};

/// MintArchive: In-memory archival of certificates & proofs for local retrieval / future export.
/// Disabled if enable_archive = false in config to avoid memory use in high-throughput benchmarks.
pub struct MintArchive {
    enabled: bool,
    certificates: RwLock<HashMap<String, RoboTorqCertificate>>,
    proofs: RwLock<HashMap<String, RoboTorqProof>>,
}

impl MintArchive {
    pub fn new(enabled: bool) -> Arc<Self> {
        Arc::new(Self {
            enabled,
            certificates: RwLock::new(HashMap::new()),
            proofs: RwLock::new(HashMap::new()),
        })
    }

    /// Store certificate & proof atomically (best-effort). No-op if disabled.
    pub fn store(&self, cert: &RoboTorqCertificate, proof: &RoboTorqProof) {
        if !self.enabled { return; }
        {
            let mut c_map = self.certificates.write().unwrap();
            c_map.insert(cert.cert_id.clone(), cert.clone());
        }
        {
            let mut p_map = self.proofs.write().unwrap();
            p_map.insert(proof.proof_id.clone(), proof.clone());
        }
    }

    pub fn get_certificate(&self, cert_id: &str) -> Option<RoboTorqCertificate> {
        if !self.enabled { return None; }
        self.certificates.read().unwrap().get(cert_id).cloned()
    }

    pub fn get_proof(&self, proof_id: &str) -> Option<RoboTorqProof> {
        if !self.enabled { return None; }
        self.proofs.read().unwrap().get(proof_id).cloned()
    }

    pub fn len_certificates(&self) -> usize { self.certificates.read().unwrap().len() }
    pub fn len_proofs(&self) -> usize { self.proofs.read().unwrap().len() }
    pub fn enabled(&self) -> bool { self.enabled }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use crate::models::robotorq_certificate::{RoboTorqCertificate, CertStatus};
    use crate::models::robo_torq_proof::{RoboTorqProof, ProofSignature};
    use common::triples::Triple;

    fn dummy_certificate(id: &str) -> RoboTorqCertificate {
        RoboTorqCertificate {
            cert_id: id.to_string(),
            robotorq_proof_id: format!("proof-{}", id),
            merkle_root: "r".repeat(64),
            contract_ids: vec!["c1".into()],
            timestamp_nanos: 123,
            hash: "h".repeat(64),
            status: CertStatus::Digital,
            bearer_bond_id: None,
            total_jouletorq: 3600,
            total_stake_jouletorq: 1800, // dummy stake for test fixture
            total_triple: Triple::zero(),
        }
    }

    fn dummy_proof(id: &str) -> RoboTorqProof {
        RoboTorqProof {
            proof_id: id.to_string(),
            certificate_id: id.to_string(),
            merkle_root: "r".repeat(64),
            merkle_tree: vec![vec!["leaf".into()]],
            ingot_hashes: vec!["leaf".into()],
            signatures: vec![ProofSignature { signer_id: "mint".into(), algorithm: "none".into(), signature: vec![], message_hash: "m".repeat(64), key_fingerprint: "k".repeat(64), public_key: vec![], timestamp: SystemTime::now() }],
            timestamp: SystemTime::now(),
            proof_hash: "p".repeat(64),
            total_jouletorq: 3600,
            total_triple: Triple::zero(),
        }
    }

    #[test]
    fn test_archive_store_and_get() {
        let archive = MintArchive::new(true);
        let cert = dummy_certificate("cert-1");
        let proof = dummy_proof("proof-cert-1");
        archive.store(&cert, &proof);
        assert_eq!(archive.len_certificates(), 1);
        assert_eq!(archive.len_proofs(), 1);
        assert!(archive.get_certificate(&cert.cert_id).is_some());
        assert!(archive.get_proof(&proof.proof_id).is_some());
    }

    #[test]
    fn test_archive_disabled() {
        let archive = MintArchive::new(false);
        let cert = dummy_certificate("cert-2");
        let proof = dummy_proof("proof-cert-2");
        archive.store(&cert, &proof); // no-op
        assert_eq!(archive.len_certificates(), 0);
        assert_eq!(archive.len_proofs(), 0);
        assert!(archive.get_certificate(&cert.cert_id).is_none());
        assert!(archive.get_proof(&proof.proof_id).is_none());
    }
}
