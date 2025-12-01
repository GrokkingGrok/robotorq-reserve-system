//! Security configuration for RoboTorq services.
//!
//! This module defines configuration options for TLS, authentication,
//! authorization, and other security-related settings.

use serde::{Deserialize, Serialize};

/// Security configuration.
///
/// Configures TLS, authentication, and authorization for RoboTorq services.
/// Security is critical for protecting sensitive financial and operational data.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::security::{SecurityConfig, TlsConfig, AuthConfig, AuthMethod, JwtConfig};
///
/// // Development security (minimal)
/// let dev_security = SecurityConfig {
///     ..Default::default()
/// };
///
/// // Production security with TLS and JWT
/// let prod_security = SecurityConfig {
///     tls: TlsConfig {
///         http_enabled: true,
///         certificate_path: Some("/etc/ssl/certs/robotorq.crt".to_string()),
///         private_key_path: Some("/etc/ssl/private/robotorq.key".to_string()),
///         ..Default::default()
///     },
///     auth: AuthConfig {
///         method: AuthMethod::Jwt,
///         jwt: JwtConfig {
///             public_key: Some("/etc/robotorq/jwt.pem".to_string()),
///             issuer: Some("https://auth.robotorq.com".to_string()),
///             audience: Some("robotorq-services".to_string()),
///             ..Default::default()
///         },
///         ..Default::default()
///     },
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    /// TLS configuration for network connections.
    #[serde(default)]
    pub tls: TlsConfig,

    /// Authentication configuration.
    #[serde(default)]
    pub auth: AuthConfig,

    /// Authorization configuration.
    #[serde(default)]
    pub authz: AuthzConfig,

    /// Secrets management configuration.
    #[serde(default)]
    pub secrets: SecretsConfig,
}

/// TLS configuration.
///
/// Configures Transport Layer Security for encrypted network communications.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::security::{TlsConfig, ClientCertMode, TlsVersion};
///
/// // Basic TLS for HTTP
/// let http_tls = TlsConfig {
///     http_enabled: true,
///     certificate_path: Some("/etc/ssl/certs/server.crt".to_string()),
///     private_key_path: Some("/etc/ssl/private/server.key".to_string()),
///     min_version: TlsVersion::Tls13,
///     ..Default::default()
/// };
///
/// // Mutual TLS with client certificates
/// let mtls_config = TlsConfig {
///     http_enabled: true,
///     nats_enabled: true,
///     certificate_path: Some("/etc/ssl/certs/server.crt".to_string()),
///     private_key_path: Some("/etc/ssl/private/server.key".to_string()),
///     ca_certificate_path: Some("/etc/ssl/certs/ca.crt".to_string()),
///     client_cert_mode: ClientCertMode::Required,
///     min_version: TlsVersion::Tls13,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Whether TLS is enabled for HTTP connections.
    ///
    /// When enabled, the HTTP server will use HTTPS instead of HTTP.
    /// Requires certificate and key configuration.
    #[serde(default = "default_tls_http_enabled")]
    pub http_enabled: bool,

    /// Whether TLS is enabled for NATS connections.
    ///
    /// When enabled, NATS client connections will be encrypted.
    #[serde(default = "default_tls_nats_enabled")]
    pub nats_enabled: bool,

    /// Whether TLS is enabled for database connections.
    ///
    /// When enabled, database connections will be encrypted.
    /// Support depends on the database backend.
    #[serde(default = "default_tls_database_enabled")]
    pub database_enabled: bool,

    /// Path to TLS certificate file.
    ///
    /// PEM-encoded certificate file for server authentication.
    /// Required when TLS is enabled for any service.
    #[serde(default)]
    pub certificate_path: Option<String>,

    /// Path to TLS private key file.
    ///
    /// PEM-encoded private key file corresponding to the certificate.
    /// Required when TLS is enabled for any service.
    #[serde(default)]
    pub private_key_path: Option<String>,

    /// Path to TLS certificate authority file.
    ///
    /// PEM-encoded CA certificate for client certificate verification.
    /// Optional, uses system CA store if not provided.
    #[serde(default)]
    pub ca_certificate_path: Option<String>,

    /// Client certificate verification mode.
    ///
    /// Controls whether and how client certificates are verified.
    #[serde(default)]
    pub client_cert_mode: ClientCertMode,

    /// Minimum TLS version to accept.
    ///
    /// Rejects connections using TLS versions below this level.
    #[serde(default)]
    pub min_version: TlsVersion,

    /// Allowed cipher suites.
    ///
    /// List of acceptable TLS cipher suites. Empty list allows all secure suites.
    #[serde(default = "default_cipher_suites")]
    pub cipher_suites: Vec<String>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            http_enabled: default_tls_http_enabled(),
            nats_enabled: default_tls_nats_enabled(),
            database_enabled: default_tls_database_enabled(),
            certificate_path: None,
            private_key_path: None,
            ca_certificate_path: None,
            client_cert_mode: ClientCertMode::default(),
            min_version: TlsVersion::default(),
            cipher_suites: default_cipher_suites(),
        }
    }
}

/// Client certificate verification modes.
///
/// Controls whether and how client certificates are verified.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::security::ClientCertMode;
///
/// // No client certificate verification (standard HTTPS)
/// let none = ClientCertMode::None;
///
/// // Request client certificates but don't require them
/// let optional = ClientCertMode::Optional;
///
/// // Require and verify client certificates (mutual TLS)
/// let required = ClientCertMode::Required;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClientCertMode {
    /// No client certificate verification
    #[default]
    None,
    /// Client certificates are requested but not required
    Optional,
    /// Client certificates are required and verified
    Required,
}

/// TLS versions.
///
/// Minimum TLS version to accept for connections.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::security::TlsVersion;
///
/// // TLS 1.2 (wider compatibility)
/// let tls12 = TlsVersion::Tls12;
///
/// // TLS 1.3 (modern, recommended)
/// let tls13 = TlsVersion::Tls13;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TlsVersion {
    /// TLS 1.2
    #[default]
    Tls12,
    /// TLS 1.3
    Tls13,
}

/// Authentication configuration.
///
/// Configures how users and services authenticate with RoboTorq services.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::security::{AuthConfig, AuthMethod, JwtConfig, ApiKeyConfig};
///
/// // JWT-based authentication
/// let jwt_auth = AuthConfig {
///     method: AuthMethod::Jwt,
///     jwt: JwtConfig {
///         public_key: Some("/etc/robotorq/jwt.pem".to_string()),
///         issuer: Some("https://auth.robotorq.com".to_string()),
///         audience: Some("robotorq-services".to_string()),
///         ..Default::default()
///     },
///     ..Default::default()
/// };
///
/// // API key authentication
/// let api_key_auth = AuthConfig {
///     method: AuthMethod::ApiKey,
///     api_key: ApiKeyConfig {
///         header_name: "X-API-Key".to_string(),
///         allow_header: true,
///         allow_query: false,
///         ..Default::default()
///     },
///     ..Default::default()
/// };
///
/// // No authentication (development only)
/// let no_auth = AuthConfig {
///     method: AuthMethod::None,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    /// Authentication method.
    ///
    /// Determines how authentication is performed.
    #[serde(default)]
    pub method: AuthMethod,

    /// JWT configuration (when using JWT auth).
    #[serde(default)]
    pub jwt: JwtConfig,

    /// API key configuration (when using API key auth).
    #[serde(default)]
    pub api_key: ApiKeyConfig,

    /// Session configuration.
    #[serde(default)]
    pub session: SessionConfig,
}

/// Authentication methods.
///
/// Determines how authentication is performed for incoming requests.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::security::AuthMethod;
///
/// // No authentication required (insecure, development only)
/// let none = AuthMethod::None;
///
/// // JSON Web Token authentication
/// let jwt = AuthMethod::Jwt;
///
/// // API key authentication
/// let api_key = AuthMethod::ApiKey;
///
/// // Mutual TLS client certificate authentication
/// let mtls = AuthMethod::Mtls;
///
/// // Multiple authentication methods (first successful wins)
/// let multi = AuthMethod::Multi;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    /// No authentication required
    #[default]
    None,
    /// JSON Web Tokens
    Jwt,
    /// API keys
    ApiKey,
    /// Mutual TLS client certificates
    Mtls,
    /// Multiple methods (first successful wins)
    Multi,
}

/// JWT authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// Path to JWT public key or secret.
    ///
    /// For RSA/ECDSA: path to PEM public key file
    /// For HMAC: base64-encoded secret
    #[serde(default)]
    pub public_key: Option<String>,

    /// JWT issuer validation.
    ///
    /// Expected "iss" claim in JWT tokens.
    #[serde(default)]
    pub issuer: Option<String>,

    /// JWT audience validation.
    ///
    /// Expected "aud" claim in JWT tokens.
    #[serde(default)]
    pub audience: Option<String>,

    /// Token expiration leeway in seconds.
    ///
    /// Allows for clock skew between systems.
    #[serde(default = "default_jwt_leeway_seconds")]
    pub leeway_seconds: u64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            public_key: None,
            issuer: None,
            audience: None,
            leeway_seconds: default_jwt_leeway_seconds(),
        }
    }
}

/// API key authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// API key header name.
    ///
    /// HTTP header containing the API key.
    #[serde(default = "default_api_key_header")]
    pub header_name: String,

    /// API key query parameter name.
    ///
    /// URL query parameter containing the API key.
    #[serde(default = "default_api_key_query_param")]
    pub query_param: String,

    /// Whether to accept API keys in headers.
    #[serde(default = "default_api_key_allow_header")]
    pub allow_header: bool,

    /// Whether to accept API keys in query parameters.
    #[serde(default = "default_api_key_allow_query")]
    pub allow_query: bool,
}

impl Default for ApiKeyConfig {
    fn default() -> Self {
        Self {
            header_name: default_api_key_header(),
            query_param: default_api_key_query_param(),
            allow_header: default_api_key_allow_header(),
            allow_query: default_api_key_allow_query(),
        }
    }
}

/// Session configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Session timeout in seconds.
    ///
    /// How long sessions remain valid without activity.
    #[serde(default = "default_session_timeout_seconds")]
    pub timeout_seconds: u64,

    /// Whether to use secure cookies.
    ///
    /// Requires HTTPS when enabled.
    #[serde(default = "default_session_secure_cookies")]
    pub secure_cookies: bool,

    /// Session cookie name.
    #[serde(default = "default_session_cookie_name")]
    pub cookie_name: String,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: default_session_timeout_seconds(),
            secure_cookies: default_session_secure_cookies(),
            cookie_name: default_session_cookie_name(),
        }
    }
}

/// Authorization configuration.
///
/// Configures access control and permissions for authenticated users.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::security::{AuthzConfig, AuthzMethod, RbacConfig};
/// use std::collections::HashMap;
///
/// // Role-based access control
/// let mut roles = HashMap::new();
/// roles.insert("admin".to_string(), vec!["*".to_string()]);
/// roles.insert("user".to_string(), vec!["read".to_string(), "write".to_string()]);
/// roles.insert("readonly".to_string(), vec!["read".to_string()]);
///
/// let rbac_authz = AuthzConfig {
///     method: AuthzMethod::Rbac,
///     rbac: RbacConfig {
///         default_role: "user".to_string(),
///         roles,
///     },
///     ..Default::default()
/// };
///
/// // Allow all authenticated requests
/// let allow_all = AuthzConfig {
///     method: AuthzMethod::AllowAll,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthzConfig {
    /// Authorization method.
    ///
    /// Determines how authorization decisions are made.
    #[serde(default)]
    pub method: AuthzMethod,

    /// Role-based access control configuration.
    #[serde(default)]
    pub rbac: RbacConfig,

    /// Policy file path (for OPA, etc.).
    #[serde(default)]
    pub policy_path: Option<String>,
}

/// Authorization methods.
///
/// Determines how authorization decisions are made for authenticated requests.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::security::AuthzMethod;
///
/// // Allow all authenticated requests (simple)
/// let allow_all = AuthzMethod::AllowAll;
///
/// // Deny all requests by default (explicit allow only)
/// let deny_all = AuthzMethod::DenyAll;
///
/// // Role-based access control
/// let rbac = AuthzMethod::Rbac;
///
/// // Open Policy Agent for complex policies
/// let opa = AuthzMethod::Opa;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthzMethod {
    /// Allow all authenticated requests
    #[default]
    AllowAll,
    /// Deny all requests (explicit allow only)
    DenyAll,
    /// Role-based access control
    Rbac,
    /// Open Policy Agent
    Opa,
}

/// Role-based access control configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacConfig {
    /// Default role for authenticated users.
    #[serde(default = "default_rbac_default_role")]
    pub default_role: String,

    /// Role definitions with permissions.
    #[serde(default = "default_rbac_roles")]
    pub roles: std::collections::HashMap<String, Vec<String>>,
}

impl Default for RbacConfig {
    fn default() -> Self {
        Self {
            default_role: default_rbac_default_role(),
            roles: default_rbac_roles(),
        }
    }
}

/// Secrets management configuration.
///
/// Configures how sensitive data like keys and passwords are stored and accessed.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::security::{SecretsConfig, SecretsBackend};
///
/// // Environment variable secrets
/// let env_secrets = SecretsConfig {
///     backend: SecretsBackend::Env,
///     prefix: "ROBOTORQ_".to_string(),
///     ..Default::default()
/// };
///
/// // HashiCorp Vault secrets
/// let vault_secrets = SecretsConfig {
///     backend: SecretsBackend::Vault,
///     prefix: "robotorq/prod".to_string(),
///     server_url: Some("https://vault.robotorq.internal:8200".to_string()),
///     auth_token: Some("hvs.CAES...".to_string()),
/// };
///
/// // AWS Secrets Manager
/// let aws_secrets = SecretsConfig {
///     backend: SecretsBackend::Aws,
///     prefix: "/robotorq/prod/".to_string(),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    /// Secrets backend.
    ///
    /// Determines where secrets are stored and how they're accessed.
    #[serde(default)]
    pub backend: SecretsBackend,

    /// Path prefix for secrets.
    ///
    /// Namespace for secrets to avoid conflicts.
    #[serde(default = "default_secrets_prefix")]
    pub prefix: String,

    /// Secrets server URL (for external backends).
    #[serde(default)]
    pub server_url: Option<String>,

    /// Authentication token for secrets backend.
    #[serde(default)]
    pub auth_token: Option<String>,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            backend: SecretsBackend::default(),
            prefix: default_secrets_prefix(),
            server_url: None,
            auth_token: None,
        }
    }
}

/// Secrets backends.
///
/// Defines the external system used for secrets management.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::security::SecretsBackend;
///
/// // Use environment variables for secrets (development)
/// let backend = SecretsBackend::Env;
///
/// // Use local files for secrets (single node)
/// let backend = SecretsBackend::File;
///
/// // Use HashiCorp Vault for enterprise secrets management
/// let backend = SecretsBackend::Vault;
///
/// // Use AWS Secrets Manager for cloud-native deployments
/// let backend = SecretsBackend::Aws;
///
/// // Use Azure Key Vault for Azure deployments
/// let backend = SecretsBackend::Azure;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SecretsBackend {
    /// Environment variables
    #[default]
    Env,
    /// Local files
    File,
    /// HashiCorp Vault
    Vault,
    /// AWS Secrets Manager
    Aws,
    /// Azure Key Vault
    Azure,
}

// Default value functions

/// Default TLS enablement for HTTP endpoints.
///
/// Returns `false` to disable TLS by default for development.
/// Enable TLS in production environments.
fn default_tls_http_enabled() -> bool { false }

/// Default TLS enablement for NATS connections.
///
/// Returns `false` to disable TLS by default for development.
/// Enable TLS for secure inter-service communication.
fn default_tls_nats_enabled() -> bool { false }

/// Default TLS enablement for database connections.
///
/// Returns `false` to disable TLS by default for development.
/// Enable TLS for secure database communication.
fn default_tls_database_enabled() -> bool { false }

/// Default cipher suites for TLS connections.
///
/// Returns an empty vector, allowing the TLS library to choose secure defaults.
/// Specify custom cipher suites for compliance requirements.
fn default_cipher_suites() -> Vec<String> { Vec::new() }

/// Default JWT leeway in seconds.
///
/// Returns 30 seconds to allow for clock skew between systems.
/// Adjust based on your infrastructure's time synchronization.
fn default_jwt_leeway_seconds() -> u64 { 30 }

/// Default API key header name.
///
/// Returns "X-API-Key" as the standard header for API keys.
/// This follows common API key authentication conventions.
fn default_api_key_header() -> String { "X-API-Key".to_string() }

/// Default API key query parameter name.
///
/// Returns "api_key" for URL query parameter authentication.
/// Useful for API clients that can't set custom headers.
fn default_api_key_query_param() -> String { "api_key".to_string() }

/// Default API key header allowance.
///
/// Returns `true` to allow API keys in HTTP headers by default.
/// This is the most common and secure method.
fn default_api_key_allow_header() -> bool { true }

/// Default API key query parameter allowance.
///
/// Returns `false` to disable query parameter API keys by default.
/// Query parameters are less secure than headers.
fn default_api_key_allow_query() -> bool { false }

/// Default session timeout in seconds.
///
/// Returns 3600 seconds (1 hour) for session validity.
/// Adjust based on your security and usability requirements.
fn default_session_timeout_seconds() -> u64 { 3600 } // 1 hour

/// Default secure cookie setting.
///
/// Returns `false` to allow non-HTTPS cookies in development.
/// Set to `true` in production with HTTPS.
fn default_session_secure_cookies() -> bool { false }

/// Default session cookie name.
///
/// Returns "robotorq_session" for session identification.
/// Choose a unique name to avoid conflicts with other applications.
fn default_session_cookie_name() -> String { "robotorq_session".to_string() }

/// Default RBAC role for authenticated users.
///
/// Returns "user" as the baseline role for authenticated users.
/// Define roles based on your application's permission model.
fn default_rbac_default_role() -> String { "user".to_string() }

/// Default RBAC role definitions.
///
/// Returns a basic role structure with admin, user, and readonly roles.
/// Customize roles and permissions for your specific use case.
fn default_rbac_roles() -> std::collections::HashMap<String, Vec<String>> {
    let mut roles = std::collections::HashMap::new();
    roles.insert("admin".to_string(), vec!["*".to_string()]);
    roles.insert("user".to_string(), vec!["read".to_string()]);
    roles
}

/// Default secrets prefix.
///
/// Returns "robotorq" as the namespace for secrets.
/// Use different prefixes for different environments or applications.
fn default_secrets_prefix() -> String { "robotorq".to_string() }