#![cfg(feature = "persistence")]

use axum::http::StatusCode;
use axum::response::IntoResponse;
use commons::util::config::persistance::PersistenceConfig;
use commons::util::persistence::{ctx_with_timeout_ms, make_driver};

/// End-to-end test for the `DriverFactory` producing a concrete `SQLite` driver
/// and integrating with the readiness handler.
#[tokio::test]
async fn driver_factory_e2e_sqlite_readyz_integration() {
    // Build a PersistenceConfig for an in-memory SQLite database
    let cfg = PersistenceConfig {
        backend: commons::util::config::persistance::PersistenceBackend::Sqlite,
        database_url: "sqlite://:memory:".to_string(),
        run_migrations: false, // keep it fast and isolated for CI
        ..Default::default()
    };

    // Create a concrete driver via the factory
    let boxed = make_driver(&cfg).await.expect("make sqlite driver");

    // Basic liveness/health assertions from the produced driver
    let health = boxed.health();
    assert!(health.ready, "driver health should report ready");

    let ctx = ctx_with_timeout_ms(1000);
    let res = boxed.ping(&ctx).await;
    assert!(res.is_ok(), "driver ping should succeed");

    // Convert into an Arc so it can be passed to the readyz helper
    let arc_drv: std::sync::Arc<dyn commons::util::persistence::PersistenceDriver> =
        std::sync::Arc::from(boxed);

    // The readiness handler expects Arc<AtomicBool> flag and an Option<Arc<dyn PersistenceDriver>>
    let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let response = commons::services::robotorq_service::readyz::readyz_handler_with_driver(
        std::sync::Arc::clone(&flag),
        Some(std::sync::Arc::clone(&arc_drv)),
    );

    let (parts, body) = response.into_response().into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    assert_eq!(&body_bytes[..], b"Ready");

    // Shutdown should be a no-op but callable
    arc_drv.shutdown();
}
