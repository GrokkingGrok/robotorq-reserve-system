//! Legacy minimal Axum server utilities.
//!
//! Provides a small Axum-based HTTP server exposing health and metrics endpoints
//! without requiring a concrete `RoboTorqService` implementation. This is handy
//! for very small binaries, tests, and demos. For production or richer lifecycle
//! management, prefer the structured `HttpServer`.
//!
//! Endpoints typically exposed:
//! - Health: returns HTTP 200 with a simple body
//! - Metrics: returns Prometheus text exposition format
//!
//! Notes:
//! - CORS is permissive by default (`CorsLayer::permissive`).
//! - Shutdown is best done via proper signal handling; a legacy TCP-based helper
//!   is provided here for compatibility.
//!
//! Example minimal router wiring:
//! ```rust,ignore
//! use std::sync::Arc;
//! use axum::{routing::get, Router};
//! use commons::util::metrics::MetricsHandler;
//! use commons::services::http::{HttpService, HttpEndpoint};
//!
//! #[tokio::main]
//! async fn main() {
//!     let handler = Arc::new(MetricsHandler::new());
//!     let service = HttpService { address: "127.0.0.1".into(), port: 8080 };
//!     let health = HttpEndpoint("/healthz".into());
//!     let metrics = HttpEndpoint("/metrics".into());
//!
//!     // See `start_basic_http_server_async` for a ready-to-run helper.
//!     let _ = (handler, service, health, metrics);
//! }
//! ```
use std::sync::Arc;
use axum::{http::StatusCode, routing::get, Router};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use crate::util::error::{InvariantError, logging_error::LoggingError};
use crate::util::metrics::MetricsHandler;
use super::{HttpEndpoint, HttpServerConfig, HttpService};
/// Minimal Axum server exposing health and metrics.
pub async fn start_basic_http_server_async(
    handler: Arc<MetricsHandler>,
    service: HttpService,
    health: HttpEndpoint,
    metrics: HttpEndpoint,
) -> Result<(), InvariantError> {
    //! Start a minimal Axum HTTP server.
    //!
    //! # Arguments
    //! - `handler`: Shared Prometheus metrics handler used to export metrics.
    //! - `service`: Host and port to bind the HTTP server to.
    //! - `health`: Path for the health endpoint (e.g., `/healthz`).
    //! - `metrics`: Path for the metrics endpoint (e.g., `/metrics`).
    //!
    //! # Returns
    //! - `Ok(())` once the server has shut down gracefully.
    //!
    //! # Errors
    //! - Returns `InvariantError::Logging` if binding the listener fails or if
    //!   Axum serving returns an error.
    //!
    //! # Panics
    //! - Does not panic under normal operation.
    //!
    //! # Examples
    //! ```rust,ignore
    //! use std::sync::Arc;
    //! use commons::services::http::{HttpService, HttpEndpoint, start_basic_http_server_async};
    //! use commons::util::metrics::MetricsHandler;
    //!
    //! #[tokio::main]
    //! async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //!     let handler = Arc::new(MetricsHandler::new());
    //!     let service = HttpService { address: "127.0.0.1".into(), port: 8080 };
    //!     let health = HttpEndpoint("/healthz".into());
    //!     let metrics = HttpEndpoint("/metrics".into());
    //!
    //!     start_basic_http_server_async(handler, service, health, metrics).await?;
    //!     Ok(())
    //! }
    //! ```
    let addr = format!("{}:{}", service.address, service.port);
    let app = Router::new()
        .route(health.0.as_str(), get(move || async move { (StatusCode::OK, "OK".to_string()) }))
        .route(metrics.0.as_str(), get(move || async move { (StatusCode::OK, handler.export_text()) }))
        .layer(CorsLayer::permissive());

    let listener = TcpListener::bind(&addr).await
        .map_err(|e| InvariantError::Logging(LoggingError::from(e.to_string())))?;

    tracing::info!("Basic HTTP server listening on {}", addr);
    axum::serve(listener, app).await
        .map_err(|e| InvariantError::Logging(LoggingError::from(e.to_string())))?;
    Ok(())
}

/// Start server using a structured config (async version).
/// This is a legacy function for backward compatibility - prefer HttpServer for new code.
///
/// This function is a convenience wrapper around `start_basic_http_server_async`
/// that takes a structured `HttpServerConfig` instead of individual parameters.
///
/// # Arguments
///
/// * `handler` - Metrics handler that provides the metrics data
/// * `cfg` - Complete HTTP server configuration
///
/// # Returns
///
/// Returns `Ok(())` if the server shuts down gracefully, or `Err(error)` if
/// it fails to start or encounters an unrecoverable error.
///
/// # Errors
/// - Binding to the configured address/port fails.
/// - Axum server returns an unrecoverable error.
///
/// # Panics
/// - Not expected to panic during normal operation.
///
/// # Examples
///
/// ```rust,ignore
/// use commons::services::http::{HttpServerConfig, start_basic_http_server_with_config_async};
/// use commons::util::metrics::MetricsHandler;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Before: Metrics handler and config exist separately
///     let handler = MetricsHandler::new();
///     let config = HttpServerConfig::local_defaults(8080);
///
///     // After: HTTP server is running with the combined configuration
///     // The config provides address, port, and endpoint paths
///     start_basic_http_server_with_config_async(handler, config).await?;
///
///     Ok(())
/// }
/// ```
pub async fn start_basic_http_server_with_config_async(
    handler: Arc<MetricsHandler>,
    cfg: HttpServerConfig,
) -> Result<(), InvariantError> {
    start_basic_http_server_async(handler, cfg.service, cfg.health, cfg.metrics).await
}

/// Request shutdown by hitting `/shutdown`.
/// Legacy synchronous helper; for async servers, use proper signaling.
pub fn request_graceful_shutdown(service: &HttpService) -> Result<(), InvariantError> {
    //! Attempt to trigger a graceful shutdown by sending an HTTP request to
    //! the server's internal shutdown endpoint.
    //!
    //! This is a best-effort, legacy helper intended for simple servers that
    //! support a `GET /shutdown` path. Prefer explicit shutdown signals or
    //! cooperative cancellation in async contexts.
    //!
    //! # Arguments
    //! - `service`: Target host and port for the running HTTP server.
    //!
    //! # Returns
    //! - `Ok(())` if the request bytes were written to the TCP stream.
    //!
    //! # Errors
    //! - Connection errors (e.g., refused, unreachable).
    //! - I/O errors writing the request.
    //!
    //! # Panics
    //! - Not expected to panic under normal operation.
    //!
    //! # Examples
    //! ```rust,ignore
    //! use commons::services::http::{HttpService, request_graceful_shutdown};
    //!
    //! fn main() -> Result<(), Box<dyn std::error::Error>> {
    //!     let http = HttpService { address: "127.0.0.1".into(), port: 8080 };
    //!     request_graceful_shutdown(&http)?;
    //!     Ok(())
    //! }
    //! ```
    use std::io::Write;
    use std::net::TcpStream;
    let addr = format!("{}:{}", service.address, service.port);
    let mut stream = TcpStream::connect(&addr)
        .map_err(|e| InvariantError::Logging(LoggingError::from(e.to_string())))?;
    let req = format!("GET /shutdown HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", addr);
    stream.write_all(req.as_bytes())
        .map_err(|e| InvariantError::Logging(LoggingError::from(e.to_string())))?;
    Ok(())
}
