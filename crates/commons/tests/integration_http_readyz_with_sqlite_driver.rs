#![cfg(all(feature = "persistence", feature = "test_helpers"))]

use std::sync::Arc;
use tokio::sync::Mutex;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use commons::util::persistence::ctx_with_timeout_ms;

/// Integration test: start an `HttpServer` using `start_autoload` so that commons
/// constructs and attaches the concrete SQLite driver, then query `/readyz`.
#[tokio::test]
async fn http_readyz_with_sqlite_driver_integration() {
    use commons::services::robotorq_service::{
        HealthContributor, HttpServerBuilder, MetricsContributor, RoboTorqService, ServiceLifecycle,
    };
    use commons::util::config::RoboTorqConfig;

    // Create a PersistenceConfig programmatically for an in-memory SQLite and no migrations
    let mut p_cfg = commons::util::config::persistance::PersistenceConfig::default();
    p_cfg.backend = commons::util::config::persistance::PersistenceBackend::Sqlite;
    p_cfg.database_url = "sqlite::memory:".to_string();
    p_cfg.run_migrations = false;

    // Example service that stores the injected driver and calls ping during initialize
    struct ExampleSvc {
        drv: Option<std::sync::Arc<dyn commons::util::persistence::PersistenceDriver>>,
    }
    impl ExampleSvc {
        fn new() -> Self {
            Self { drv: None }
        }
    }
    impl ServiceLifecycle for ExampleSvc {}
    impl HealthContributor for ExampleSvc {
        fn health_status(&self) -> String {
            "ok".to_string()
        }
    }
    impl MetricsContributor for ExampleSvc {}
    impl RoboTorqService for ExampleSvc {
        fn set_persistence_driver(
            &mut self,
            drv: Option<std::sync::Arc<dyn commons::util::persistence::PersistenceDriver>>,
        ) {
            self.drv = drv;
        }

        async fn initialize(
            &mut self,
            _cfg: &RoboTorqConfig,
        ) -> Result<(), commons::util::error::ServiceError> {
            if let Some(d) = &self.drv {
                let ctx = ctx_with_timeout_ms(1000);
                d.ping(&ctx).await.map_err(|e| {
                    commons::util::error::ServiceError::Other(format!("ping failed: {:?}", e))
                })?;
            }
            Ok(())
        }
    }

    // Choose a free port to bind so we can connect reliably
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind for port pick");
        let port = l.local_addr().unwrap().port();
        drop(l);
        port
    };

    let svc = Arc::new(Mutex::new(ExampleSvc::new()));
    let builder = HttpServerBuilder::new(Arc::clone(&svc)).with_port(port);

    // Load config and create a concrete driver here so we can attach it to the server
    let boxed = commons::util::persistence::make_driver(&p_cfg)
        .await
        .expect("make driver");
    let arc_drv: std::sync::Arc<dyn commons::util::persistence::PersistenceDriver> =
        std::sync::Arc::from(boxed);

    // Before starting, inject the driver into the service and initialize it so
    // ExampleSvc::initialize runs and validates the ping.
    {
        let mut locked = svc.lock().await;
        locked.set_persistence_driver(Some(std::sync::Arc::clone(&arc_drv)));
        // Build a RoboTorqConfig and set persistence section so initialize sees it
        let mut cfg = commons::util::config::load_robotorq_config(None).expect("load cfg");
        cfg.persistence = p_cfg.clone();
        locked.initialize(&cfg).await.expect("service initialize");
    }

    // Build server, attach persistence driver, and start with an owned shutdown future
    let server = builder
        .build()
        .with_persistence_driver(std::sync::Arc::clone(&arc_drv));
    let (_tx, rx) = tokio::sync::oneshot::channel::<()>();
    let srv_task = tokio::spawn(async move {
        let shutdown_future = async move {
            let _ = rx.await;
        };
        let _ = server.start_with_shutdown(shutdown_future).await;
    });
    let addr = format!("127.0.0.1:{}", port);

    // Wait for server to accept connections
    let mut connected = false;
    for _ in 0..40 {
        match tokio::net::TcpStream::connect(&addr).await {
            Ok(_) => {
                connected = true;
                break;
            }
            Err(_) => tokio::time::sleep(std::time::Duration::from_millis(50)).await,
        }
    }
    assert!(connected, "server did not start listening on {}", addr);

    // Query /readyz and expect 200 (driver attached and ready)
    {
        let mut stream = tokio::net::TcpStream::connect(&addr)
            .await
            .expect("connect to server");
        let req = b"GET /readyz HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
        stream.write_all(req).await.expect("write request");
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.expect("read response");
        let resp = String::from_utf8_lossy(&buf);
        assert!(
            resp.starts_with("HTTP/1.1 200") || resp.contains(" 200 "),
            "unexpected response: {}",
            resp
        );
    }

    // Stop server by aborting the task (triggers cleanup when dropped)
    srv_task.abort();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), srv_task).await;

    // done
}
