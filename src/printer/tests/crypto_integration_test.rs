use common::crypto::{new_algorithm, CryptoKind, SignatureAlgorithm};

#[test]
fn test_common_crypto_sign_verify() {
    let algo = new_algorithm(CryptoKind::Falcon1024);
    let (public, secret) = algo.generate_keypair().expect("keypair");
    let msg = b"printer-common-crypto-integration";
    let sig = algo.sign(msg, &secret).expect("sign");
    assert!(algo.verify(msg, &sig, &public).expect("verify"));
    assert!(!algo.verify(b"tampered", &sig, &public).expect("verify tampered"));
}