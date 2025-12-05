use std::sync::Arc;
use tokio::sync::Mutex;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Variant integration test: same flow as the primary test but a separate test
/// target to run in parallel / verify isolation.
#[tokio::test]
async fn http_server_readyz_integration_variant() {
    use commons::services::robotorq_service::{
        HealthContributor, HttpServerBuilder, MetricsContributor, RoboTorqService, ServiceLifecycle,
    };

    // Minimal test service that implements the lifecycle traits used by the server.
    struct TestSvc;
    impl ServiceLifecycle for TestSvc {}
    impl HealthContributor for TestSvc {
        fn health_status(&self) -> String {
            "ok".to_string()
        }
    }
    impl MetricsContributor for TestSvc {}
    impl RoboTorqService for TestSvc {}

    let svc = Arc::new(Mutex::new(TestSvc));

    // Pick a (likely) free port by binding and releasing a listener.
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind for port pick");
        let port = l.local_addr().unwrap().port();
        drop(l);
        port
    };

    // Build the server with that port (ephemeral-ish)
    let server = HttpServerBuilder::new(Arc::clone(&svc))
        .with_port(port)
        .build();

    // Create a oneshot to trigger graceful shutdown and spawn the server.
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let srv_task = tokio::spawn(async move {
        let shutdown_future = async move {
            let _ = rx.await;
        };
        // start the server (consumes `server`) and run until `shutdown_future` completes
        let _ = server.start_with_shutdown(shutdown_future).await;
    });

    let addr = format!("127.0.0.1:{port}");

    // Wait for the server to start accepting connections (bounded retry)
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
    assert!(connected, "server did not start listening on {addr}");

    // Query /readyz over a raw TCP connection and assert we get HTTP 200
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
            "unexpected response: {resp}"
        );
    }

    // Trigger shutdown and wait for server task to complete
    let _ = tx.send(());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), srv_task)
        .await
        .expect("server task join");

    // After shutdown the socket should no longer accept connections (connection refused)
    if let Ok(_) = tokio::net::TcpStream::connect(&addr).await {
        panic!("server still accepting connections after shutdown")
    }
}
