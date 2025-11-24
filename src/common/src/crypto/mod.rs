use anyhow::Result;

// Trait every signature algorithm must implement
pub trait SignatureAlgorithm: Send + Sync {
    fn generate_keypair(&self) -> Result<(Vec<u8>, Vec<u8>)>; // (public, secret)
    fn sign(&self, msg: &[u8], secret: &[u8]) -> Result<Vec<u8>>; // detached signature bytes
    fn verify(&self, msg: &[u8], sig: &[u8], public: &[u8]) -> Result<bool>; // detached signature verification
    fn algorithm_name(&self) -> &'static str;
}

// Supported algorithms enumeration
pub enum CryptoKind {
    Falcon1024,
    // Future: SphincsPlus128,
}

// Falcon implementation (using pqcrypto-falcon)
#[cfg(feature = "falcon")]
pub struct Falcon1024Algo;

#[cfg(feature = "falcon")]
impl SignatureAlgorithm for Falcon1024Algo {
    fn generate_keypair(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        use pqcrypto_falcon::falcon1024::keypair;
        use pqcrypto_traits::sign::{PublicKey as _, SecretKey as _};
        let (pubk, seck) = keypair();
        Ok((pubk.as_bytes().to_vec(), seck.as_bytes().to_vec()))
    }
    fn sign(&self, msg: &[u8], secret: &[u8]) -> Result<Vec<u8>> {
        use pqcrypto_falcon::falcon1024::{sign, SecretKey, SignedMessage};
        use pqcrypto_traits::sign::{SecretKey as _, SignedMessage as _};
        let sk = SecretKey::from_bytes(secret)?;
        let signed: SignedMessage = sign(msg, &sk);
        Ok(signed.as_bytes().to_vec()) // signature+message blob
    }
    fn verify(&self, msg: &[u8], sig: &[u8], public: &[u8]) -> Result<bool> {
        use pqcrypto_falcon::falcon1024::{open, PublicKey, SignedMessage};
        use pqcrypto_traits::sign::{PublicKey as _, SignedMessage as _};
        let pk = PublicKey::from_bytes(public)?;
        let signed = SignedMessage::from_bytes(sig)?;
        match open(&signed, &pk) {
            Ok(decrypted) => Ok(decrypted.as_slice() == msg),
            Err(_) => Ok(false),
        }
    }
    fn algorithm_name(&self) -> &'static str { "falcon1024" }
}

pub fn parse_kind(name: &str) -> Result<CryptoKind> {
    match name.to_lowercase().as_str() {
        "falcon" | "falcon1024" => Ok(CryptoKind::Falcon1024),
        other => anyhow::bail!("unsupported algorithm: {}", other),
    }
}

pub fn new_algorithm(kind: CryptoKind) -> Box<dyn SignatureAlgorithm> {
    match kind {
        CryptoKind::Falcon1024 => {
            #[cfg(feature = "falcon")]
            { Box::new(Falcon1024Algo) }
            #[cfg(not(feature = "falcon"))]
            { panic!("falcon feature not enabled") }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_falcon_sign_verify() {
        let algo = new_algorithm(CryptoKind::Falcon1024);
        let (public, secret) = algo.generate_keypair().expect("keypair");
        let msg = b"robotorq-test-message";
        let sig = algo.sign(msg, &secret).expect("sign");
        assert!(algo.verify(msg, &sig, &public).expect("verify"));
        // Negative test
        assert!(!algo.verify(b"tampered", &sig, &public).expect("verify tampered"));
    }
}
