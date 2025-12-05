//! Readiness probe handler (`/readyz`).
//!
//! Exposes a simple readiness endpoint backed by a shared `Arc<AtomicBool>`.
//! - Returns HTTP 200 with "Ready" when the flag is `true`.
//! - Returns HTTP 503 with "Not Ready" when the flag is `false`.
//!
//! Intended to be toggled by the server lifecycle once dependencies (e.g.,
//! storage, messaging, crypto material) are initialized and the service can
//! safely accept traffic.
use crate::util::persistence::PersistenceDriver;
use axum::{http::StatusCode, response::IntoResponse};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Readiness handler returning 200 when ready, 503 otherwise.
///
/// # Arguments
/// - `flag`: Shared readiness indicator managed by your server lifecycle.
///
/// # Returns
/// - `200 OK` and "Ready" when ready; otherwise `503 SERVICE_UNAVAILABLE` and "Not Ready".
///
/// # Panics
/// - Not expected to panic.
///
/// # Examples
/// ```rust,ignore
/// use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
/// use axum::{routing::get, Router};
/// use commons::services::http::readyz::readyz_handler;
///
/// async fn router() -> Router {
///     let ready = Arc::new(AtomicBool::new(false));
///     let r = ready.clone();
///     Router::new()
///         .route("/readyz", get(move || readyz_handler(r.clone())))
/// }
///
/// // Later, once init completes:
/// // ready.store(true, Ordering::Relaxed);
/// ```
#[allow(clippy::needless_pass_by_value)]
pub fn readyz_handler(flag: Arc<AtomicBool>) -> impl IntoResponse {
    if flag.load(Ordering::Relaxed) {
        (StatusCode::OK, "Ready".to_string())
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "Not Ready".to_string())
    }
}

/// Readiness handler that also considers an optional persistence driver's health.
///
/// If the server readiness flag is false, returns 503. If the flag is true and a
/// persistence driver is provided, the driver's `health().ready` is consulted and
/// will cause a 503 if not ready. Otherwise returns 200.
///
/// # Arguments
/// - `flag`: Shared readiness indicator
/// - `drv`: Optional persistence driver used to determine data-plane readiness
///
/// # Returns
/// - `200 OK` with "Ready" when both service and persistence are ready
/// - `503 SERVICE_UNAVAILABLE` with a descriptive message otherwise
///
/// # Examples
/// ```rust,ignore
/// use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
/// use commons::services::http::readyz::readyz_handler_with_driver;
/// let flag = Arc::new(AtomicBool::new(true));
/// let resp = readyz_handler_with_driver(flag, None);
/// // resp is 200 OK when flag is true
/// ```
#[allow(clippy::needless_pass_by_value)]
pub fn readyz_handler_with_driver(
    flag: Arc<AtomicBool>,
    drv: Option<std::sync::Arc<dyn PersistenceDriver>>,
) -> impl IntoResponse {
    if !flag.load(Ordering::Relaxed) {
        return (StatusCode::SERVICE_UNAVAILABLE, "Not Ready".to_string());
    }

    if let Some(d) = drv {
        let h = d.health();
        if !h.ready {
            let msg = h
                .message
                .unwrap_or_else(|| "persistence not ready".to_string());
            return (StatusCode::SERVICE_UNAVAILABLE, msg);
        }
    }

    (StatusCode::OK, "Ready".to_string())
}

/// Async readiness handler that optionally performs per-request driver validation.
///
/// When the flag is true and a driver is provided, calls `driver.health_now().await`.
/// Falls back to `driver.health()` if not overridden by the backend.
#[allow(clippy::needless_pass_by_value)]
pub async fn readyz_handler_with_driver_validate(
    flag: Arc<AtomicBool>,
    drv: Option<std::sync::Arc<dyn PersistenceDriver>>,
) -> impl IntoResponse {
    if !flag.load(Ordering::Relaxed) {
        return (StatusCode::SERVICE_UNAVAILABLE, "Not Ready".to_string());
    }

    if let Some(d) = drv {
        let h = d.health_now().await;
        if !h.ready {
            let msg = h
                .message
                .unwrap_or_else(|| "persistence not ready".to_string());
            return (StatusCode::SERVICE_UNAVAILABLE, msg);
        }
    }

    (StatusCode::OK, "Ready".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    /// Test that readyz_handler returns 200 OK with "Ready" when the flag is true.
    #[tokio::test]
    async fn test_readyz_handler_ready() {
        let flag = Arc::new(AtomicBool::new(true));
        let response = readyz_handler(flag);
        let (parts, body) = response.into_response().into_parts();
        assert_eq!(parts.status, StatusCode::OK);
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        assert_eq!(&body_bytes[..], b"Ready");
    }

    /// Test that readyz_handler returns 503 SERVICE_UNAVAILABLE with "Not Ready" when the flag is false.
    #[tokio::test]
    async fn test_readyz_handler_not_ready() {
        let flag = Arc::new(AtomicBool::new(false));
        let response = readyz_handler(flag);
        let (parts, body) = response.into_response().into_parts();
        assert_eq!(parts.status, StatusCode::SERVICE_UNAVAILABLE);
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        assert_eq!(&body_bytes[..], b"Not Ready");
    }

    #[tokio::test]
    async fn test_readyz_with_driver_ready() {
        struct DummyDrv;
        impl crate::util::persistence::PersistenceDriver for DummyDrv {
            fn health(&self) -> crate::util::persistence::PersistenceHealth {
                crate::util::persistence::PersistenceHealth {
                    ready: true,
                    message: None,
                }
            }
        }
        let flag = Arc::new(AtomicBool::new(true));
        let drv: Option<std::sync::Arc<dyn PersistenceDriver>> =
            Some(std::sync::Arc::new(DummyDrv));
        let response = readyz_handler_with_driver(flag, drv);
        let (parts, body) = response.into_response().into_parts();
        assert_eq!(parts.status, StatusCode::OK);
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        assert_eq!(&body_bytes[..], b"Ready");
    }

    #[tokio::test]
    async fn test_readyz_with_driver_not_ready_reports_message() {
        struct DummyDrv;
        impl crate::util::persistence::PersistenceDriver for DummyDrv {
            fn health(&self) -> crate::util::persistence::PersistenceHealth {
                crate::util::persistence::PersistenceHealth {
                    ready: false,
                    message: Some("bad schema".to_string()),
                }
            }
        }
        let flag = Arc::new(AtomicBool::new(true));
        let drv: Option<std::sync::Arc<dyn PersistenceDriver>> =
            Some(std::sync::Arc::new(DummyDrv));
        let response = readyz_handler_with_driver(flag, drv);
        let (parts, body) = response.into_response().into_parts();
        assert_eq!(parts.status, StatusCode::SERVICE_UNAVAILABLE);
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        assert_eq!(&body_bytes[..], b"bad schema");
    }
}
