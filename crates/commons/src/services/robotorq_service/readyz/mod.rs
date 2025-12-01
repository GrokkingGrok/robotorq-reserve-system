//! Readiness probe handler (`/readyz`).
//!
//! Exposes a simple readiness endpoint backed by a shared `Arc<AtomicBool>`.
//! - Returns HTTP 200 with "Ready" when the flag is `true`.
//! - Returns HTTP 503 with "Not Ready" when the flag is `false`.
//!
//! Intended to be toggled by the server lifecycle once dependencies (e.g.,
//! storage, messaging, crypto material) are initialized and the service can
//! safely accept traffic.
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
pub async fn readyz_handler(flag: Arc<AtomicBool>) -> impl IntoResponse {
    if flag.load(Ordering::Relaxed) {
        (StatusCode::OK, "Ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "Not Ready")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    /// Test that readyz_handler returns 200 OK with "Ready" when the flag is true.
    #[tokio::test]
    async fn test_readyz_handler_ready() {
        let flag = Arc::new(AtomicBool::new(true));
        let response = readyz_handler(flag).await;
        let (parts, body) = response.into_response().into_parts();
        assert_eq!(parts.status, StatusCode::OK);
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        assert_eq!(&body_bytes[..], b"Ready");
    }

    /// Test that readyz_handler returns 503 SERVICE_UNAVAILABLE with "Not Ready" when the flag is false.
    #[tokio::test]
    async fn test_readyz_handler_not_ready() {
        let flag = Arc::new(AtomicBool::new(false));
        let response = readyz_handler(flag).await;
        let (parts, body) = response.into_response().into_parts();
        assert_eq!(parts.status, StatusCode::SERVICE_UNAVAILABLE);
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        assert_eq!(&body_bytes[..], b"Not Ready");
    }
}
