//! Cryptographic Configuration for RoboTorq Reserve System
//!
//! This module provides configuration for cryptographic operations including
//! digital signatures, key management, and certificate handling. It supports
//! both classical and post-quantum cryptographic algorithms.
//!
//! # Supported Algorithms
//!
//! ## Post-Quantum Signatures
//! - **Falcon**: Lattice-based signature scheme, fast signing/verification
//! - **SPHINCS+**: Stateless hash-based signatures, maximum security
//!
//! ## Classical Signatures (for compatibility)
//! - **Ed25519**: Edwards-curve Digital Signature Algorithm
//! - **RSA**: Rivest-Shamir-Adleman (with PSS padding)
//!
//! # Key Management
//!
//! The system supports multiple key storage backends:
//! - **File System**: Local key files with optional encryption
//! - **Hardware Security Modules (HSM)**: Tamper-resistant key storage
//! - **Cloud KMS**: Managed key services (AWS KMS, Azure Key Vault)
//!
//! # Certificate Management
//!
//! Support for X.509 certificates and certificate chains for:
//! - Service identity verification
//! - Certificate authority validation
//! - Certificate revocation checking

use serde::{Deserialize, Serialize};

/// Cryptographic configuration for the RoboTorq system.
///
/// Configures digital signatures, key management, and certificate handling
/// for securing economic transactions and service communication.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::crypto::{CryptoConfig, SignatureAlgorithm, KeyBackend};
///
/// // Production configuration with Falcon signatures
/// let prod_crypto = CryptoConfig {
///     signature_algorithm: SignatureAlgorithm::Falcon,
///     key_backend: KeyBackend::Hsm,
///     key_path: Some("/etc/robotorq/keys".to_string()),
///     certificate_path: Some("/etc/ssl/certs/robotorq.crt".to_string()),
///     ..Default::default()
/// };
///
/// // Development configuration with Ed25519
/// let dev_crypto = CryptoConfig {
///     signature_algorithm: SignatureAlgorithm::Ed25519,
///     key_backend: KeyBackend::File,
///     key_path: Some("./keys".to_string()),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CryptoConfig {
    /// Primary signature algorithm for signing operations.
    ///
    /// Determines which cryptographic algorithm to use for generating
    /// and verifying digital signatures throughout the system.
    #[serde(default)]
    pub signature_algorithm: SignatureAlgorithm,

    /// Key storage backend.
    ///
    /// Defines where cryptographic keys are stored and how they're accessed.
    #[serde(default)]
    pub key_backend: KeyBackend,

    /// Path to key storage location.
    ///
    /// Directory or file path where keys are stored. Interpretation depends
    /// on the key backend (directory for file backend, PKCS#11 URI for HSM).
    #[serde(default)]
    pub key_path: Option<String>,

    /// Path to certificate file.
    ///
    /// PEM-encoded X.509 certificate file for service identity.
    #[serde(default)]
    pub certificate_path: Option<String>,

    /// Path to certificate chain file.
    ///
    /// PEM-encoded certificate chain for certificate validation.
    #[serde(default)]
    pub certificate_chain_path: Option<String>,

    /// Certificate revocation settings.
    #[serde(default)]
    pub revocation: RevocationConfig,

    /// Key rotation settings.
    #[serde(default)]
    pub rotation: RotationConfig,

    /// Hardware security module configuration.
    #[serde(default)]
    pub hsm: HsmConfig,

    /// Cloud KMS configuration.
    #[serde(default)]
    pub kms: KmsConfig,
}

/// Supported signature algorithms.
///
/// Defines the cryptographic algorithms available for digital signatures.
/// Includes both post-quantum and classical algorithms.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::crypto::SignatureAlgorithm;
///
/// // Post-quantum algorithms (recommended for long-term security)
/// let falcon = SignatureAlgorithm::Falcon;
/// let sphincs = SignatureAlgorithm::Sphincs;
///
/// // Classical algorithms (for compatibility)
/// let ed25519 = SignatureAlgorithm::Ed25519;
/// let rsa = SignatureAlgorithm::RsaPss;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SignatureAlgorithm {
    /// Falcon lattice-based signature scheme (post-quantum).
    ///
    /// Fast signing and verification, good for high-throughput applications.
    /// Provides 128-bit post-quantum security.
    #[default]
    Falcon,

    /// SPHINCS+ stateless hash-based signatures (post-quantum).
    ///
    /// Maximum security with 256-bit post-quantum security.
    /// Slower but provides the highest security guarantees.
    Sphincs,

    /// Ed25519 Edwards-curve signatures (classical).
    ///
    /// Fast, secure, and widely supported. Good for development and compatibility.
    Ed25519,

    /// RSA with PSS padding (classical).
    ///
    /// Traditional RSA signatures with provably secure padding.
    /// Slower and requires larger keys than ECC algorithms.
    RsaPss,
}

/// Key storage backends.
///
/// Defines where cryptographic keys are stored and managed.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::crypto::KeyBackend;
///
/// // Local file system storage
/// let file = KeyBackend::File;
///
/// // Hardware security module
/// let hsm = KeyBackend::Hsm;
///
/// // AWS Key Management Service
/// let aws = KeyBackend::AwsKms;
///
/// // Azure Key Vault
/// let azure = KeyBackend::AzureKeyVault;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum KeyBackend {
    /// Local file system storage.
    ///
    /// Keys stored as encrypted files on local disk.
    /// Suitable for development and single-node deployments.
    #[default]
    File,

    /// Hardware Security Module.
    ///
    /// Tamper-resistant key storage with PKCS#11 interface.
    /// Required for production deployments with high security requirements.
    Hsm,

    /// AWS Key Management Service.
    ///
    /// Cloud-based key management with AWS KMS.
    /// Suitable for AWS deployments.
    AwsKms,

    /// Azure Key Vault.
    ///
    /// Cloud-based key management with Azure Key Vault.
    /// Suitable for Azure deployments.
    AzureKeyVault,

    /// Google Cloud KMS.
    ///
    /// Cloud-based key management with Google Cloud KMS.
    /// Suitable for GCP deployments.
    GcpKms,
}

/// Certificate revocation configuration.
///
/// Configures how certificate revocation is checked and handled.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::crypto::{RevocationConfig, RevocationMethod};
///
/// // Check Certificate Revocation Lists
/// let crl_config = RevocationConfig {
///     method: RevocationMethod::Crl,
///     crl_path: Some("/etc/ssl/crl.pem".to_string()),
///     ..Default::default()
/// };
///
/// // Use Online Certificate Status Protocol
/// let ocsp_config = RevocationConfig {
///     method: RevocationMethod::Ocsp,
///     ocsp_url: Some("http://ocsp.example.com".to_string()),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationConfig {
    /// Revocation checking method.
    #[serde(default)]
    pub method: RevocationMethod,

    /// Path to Certificate Revocation List file.
    #[serde(default)]
    pub crl_path: Option<String>,

    /// OCSP responder URL.
    #[serde(default)]
    pub ocsp_url: Option<String>,

    /// Whether to require successful revocation checking.
    ///
    /// If true, certificates that cannot be checked will be rejected.
    /// If false, revocation checking failures are logged but allowed.
    #[serde(default = "default_revocation_required")]
    pub required: bool,
}

impl Default for RevocationConfig {
    fn default() -> Self {
        Self {
            method: RevocationMethod::default(),
            crl_path: None,
            ocsp_url: None,
            required: default_revocation_required(),
        }
    }
}

/// Certificate revocation methods.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RevocationMethod {
    /// No revocation checking
    #[default]
    None,
    /// Certificate Revocation List
    Crl,
    /// Online Certificate Status Protocol
    Ocsp,
}

/// Key rotation configuration.
///
/// Configures automatic key rotation for security maintenance.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::crypto::RotationConfig;
///
/// // Rotate keys every 30 days
/// let rotation = RotationConfig {
///     enabled: true,
///     interval_days: 30,
///     overlap_days: 7, // Allow 7 days for old keys to expire
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationConfig {
    /// Whether automatic key rotation is enabled.
    #[serde(default = "default_rotation_enabled")]
    pub enabled: bool,

    /// Key rotation interval in days.
    #[serde(default = "default_rotation_interval_days")]
    pub interval_days: u32,

    /// Key overlap period in days.
    ///
    /// Number of days to keep old keys valid after rotation
    /// to allow for signature verification of in-flight messages.
    #[serde(default = "default_rotation_overlap_days")]
    pub overlap_days: u32,
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            enabled: default_rotation_enabled(),
            interval_days: default_rotation_interval_days(),
            overlap_days: default_rotation_overlap_days(),
        }
    }
}

/// Hardware Security Module configuration.
///
/// Configuration for PKCS#11 compatible HSMs.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::crypto::HsmConfig;
///
/// let hsm = HsmConfig {
///     library_path: "/usr/lib/x86_64-linux-gnu/pkcs11/libsofthsm2.so".to_string(),
///     slot_id: Some(0),
///     pin: Some("1234".to_string()), // In production, use environment variables
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HsmConfig {
    /// Path to PKCS#11 library.
    #[serde(default)]
    pub library_path: String,

    /// HSM slot ID.
    #[serde(default)]
    pub slot_id: Option<u64>,

    /// HSM PIN for authentication.
    #[serde(default)]
    pub pin: Option<String>,

    /// Token label for key identification.
    #[serde(default)]
    pub token_label: Option<String>,
}

/// Cloud Key Management Service configuration.
///
/// Configuration for cloud-based KMS services.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::crypto::KmsConfig;
///
/// // AWS KMS configuration
/// let aws_kms = KmsConfig {
///     provider: "aws".to_string(),
///     region: Some("us-east-1".to_string()),
///     key_id: Some("alias/robotorq-signing-key".to_string()),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KmsConfig {
    /// KMS provider name.
    #[serde(default)]
    pub provider: String,

    /// Cloud region or location.
    #[serde(default)]
    pub region: Option<String>,

    /// Key identifier or alias.
    #[serde(default)]
    pub key_id: Option<String>,

    /// Service account or authentication credentials.
    #[serde(default)]
    pub credentials_path: Option<String>,
}

// Default value functions

/// Default revocation checking requirement.
///
/// Returns `false` to allow certificates that cannot be checked,
/// logging warnings instead of rejecting them.
fn default_revocation_required() -> bool {
    false
}

/// Default key rotation enablement.
///
/// Returns `true` to enable automatic key rotation for security.
fn default_rotation_enabled() -> bool {
    true
}

/// Default key rotation interval.
///
/// Returns 90 days for key rotation, balancing security and operational overhead.
fn default_rotation_interval_days() -> u32 {
    90
}

/// Default key overlap period.
///
/// Returns 30 days to allow time for old signatures to be verified
/// and systems to update to new keys.
fn default_rotation_overlap_days() -> u32 {
    30
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypto_config_default() {
        let config = CryptoConfig::default();
        assert_eq!(config.signature_algorithm, SignatureAlgorithm::Falcon);
        assert_eq!(config.key_backend, KeyBackend::File);
        assert!(config.key_path.is_none());
        assert!(config.certificate_path.is_none());
        assert_eq!(config.revocation.method, RevocationMethod::None);
        assert!(config.rotation.enabled);
        assert_eq!(config.rotation.interval_days, 90);
    }

    #[test]
    fn test_crypto_config_serialization() {
        let config = CryptoConfig {
            signature_algorithm: SignatureAlgorithm::Sphincs,
            key_backend: KeyBackend::Hsm,
            key_path: Some("/etc/keys".to_string()),
            certificate_path: Some("/etc/cert.pem".to_string()),
            ..Default::default()
        };

        // Test TOML serialization
        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("signature_algorithm = \"sphincs\""));
        assert!(toml.contains("key_backend = \"hsm\""));
        assert!(toml.contains("key_path = \"/etc/keys\""));
        assert!(toml.contains("certificate_path = \"/etc/cert.pem\""));

        // Test deserialization
        let deserialized: CryptoConfig = toml::from_str(&toml).unwrap();
        assert_eq!(
            deserialized.signature_algorithm,
            SignatureAlgorithm::Sphincs
        );
        assert_eq!(deserialized.key_backend, KeyBackend::Hsm);
        assert_eq!(deserialized.key_path, Some("/etc/keys".to_string()));
        assert_eq!(
            deserialized.certificate_path,
            Some("/etc/cert.pem".to_string())
        );
    }

    #[test]
    fn test_signature_algorithm_variants() {
        // Test all signature algorithm variants
        let algorithms = vec![
            SignatureAlgorithm::Falcon,
            SignatureAlgorithm::Sphincs,
            SignatureAlgorithm::Ed25519,
            SignatureAlgorithm::RsaPss,
        ];

        for algo in algorithms {
            let config = CryptoConfig {
                signature_algorithm: algo.clone(),
                ..Default::default()
            };

            let toml = toml::to_string(&config).unwrap();
            let deserialized: CryptoConfig = toml::from_str(&toml).unwrap();
            assert_eq!(deserialized.signature_algorithm, algo);
        }
    }

    #[test]
    fn test_key_backend_variants() {
        // Test all key backend variants
        let backends = vec![
            KeyBackend::File,
            KeyBackend::Hsm,
            KeyBackend::AwsKms,
            KeyBackend::AzureKeyVault,
            KeyBackend::GcpKms,
        ];

        for backend in backends {
            let config = CryptoConfig {
                key_backend: backend.clone(),
                ..Default::default()
            };

            let toml = toml::to_string(&config).unwrap();
            let deserialized: CryptoConfig = toml::from_str(&toml).unwrap();
            assert_eq!(deserialized.key_backend, backend);
        }
    }

    #[test]
    fn test_revocation_config() {
        let revocation = RevocationConfig {
            method: RevocationMethod::Ocsp,
            ocsp_url: Some("http://ocsp.example.com".to_string()),
            required: true,
            ..Default::default()
        };

        let config = CryptoConfig {
            revocation,
            ..Default::default()
        };

        let toml = toml::to_string(&config).unwrap();
        let deserialized: CryptoConfig = toml::from_str(&toml).unwrap();
        assert_eq!(deserialized.revocation.method, RevocationMethod::Ocsp);
        assert_eq!(
            deserialized.revocation.ocsp_url,
            Some("http://ocsp.example.com".to_string())
        );
        assert!(deserialized.revocation.required);
    }

    #[test]
    fn test_rotation_config() {
        let rotation = RotationConfig {
            enabled: false,
            interval_days: 60,
            overlap_days: 14,
        };

        let config = CryptoConfig {
            rotation,
            ..Default::default()
        };

        let toml = toml::to_string(&config).unwrap();
        let deserialized: CryptoConfig = toml::from_str(&toml).unwrap();
        assert!(!deserialized.rotation.enabled);
        assert_eq!(deserialized.rotation.interval_days, 60);
        assert_eq!(deserialized.rotation.overlap_days, 14);
    }
}
