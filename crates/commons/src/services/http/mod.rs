use std::thread::JoinHandle;
use std::thread;
use tiny_http::{Server, Response};
use crate::util::metrics::MetricsHandler;
use crate::util::error::{InvariantError, logging_error::LoggingError};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct HttpService {
    pub address: String,
    pub port: u16,
}

impl HttpService {
    pub fn new<S: Into<String>>(address: S, port: u16) -> Self {
        Self { address: address.into(), port }
    }
}

#[derive(Clone, Debug)]
pub struct HttpEndpoint(pub String);

impl HttpEndpoint {
    pub fn new<S: Into<String>>(path: S) -> Self { Self(path.into()) }
}
// Re-export HTTP types so services can import from services::http

/// Start a minimal HTTP server serving health and metrics endpoints.
/// - Health: responds 200 "OK"
/// - Metrics: responds with Prometheus text from MetricsHandler::export_text()
/// Returns a JoinHandle for the background server thread.
pub fn start_basic_http_server(
    handler: Arc<MetricsHandler>,
    service: HttpService,
    health: HttpEndpoint,
    metrics: HttpEndpoint,
) -> Result<JoinHandle<()>, InvariantError> {
    let addr = format!("{}:{}", service.address, service.port);
    let server = Server::http(&addr)
        .map_err(|e| InvariantError::Logging(LoggingError::from(e.to_string())))?;

    let health_path = health.0.clone();
    let metrics_path = metrics.0.clone();
    let shutdown_path = String::from("/shutdown");

    let handle = thread::spawn(move || {
        for request in server.incoming_requests() {
            let url = request.url();
            if url == health_path {
                let _ = request.respond(Response::from_string("OK"));
            } else if url == metrics_path {
                let body = handler.export_text();
                let _ = request.respond(Response::from_string(body).with_status_code(200));
            } else if url == shutdown_path {
                let _ = request.respond(Response::from_string("bye").with_status_code(200));
                break;
            } else {
                let _ = request.respond(Response::from_string("not found").with_status_code(404));
            }
        }
    });

    Ok(handle)
}

#[derive(Clone, Debug)]
pub struct HttpServerConfig {
    pub service: HttpService,
    pub health: HttpEndpoint,
    pub metrics: HttpEndpoint,
}

impl HttpServerConfig {
    pub fn new(service: HttpService, health: HttpEndpoint, metrics: HttpEndpoint) -> Self {
        Self { service, health, metrics }
    }

    /// Convenience for local development defaults.
    pub fn local_defaults(port: u16) -> Self {
        Self {
            service: HttpService::new("127.0.0.1", port),
            health: HttpEndpoint::new("/health"),
            metrics: HttpEndpoint::new("/metrics"),
        }
    }
}

/// Start server using a structured config.
pub fn start_basic_http_server_with_config(
    handler: Arc<MetricsHandler>,
    cfg: HttpServerConfig,
) -> Result<JoinHandle<()>, InvariantError> {
    start_basic_http_server(handler, cfg.service, cfg.health, cfg.metrics)
}

/// Gracefully request shutdown by calling the internal `/shutdown` endpoint.
pub fn request_graceful_shutdown(service: &HttpService) -> Result<(), InvariantError> {
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


