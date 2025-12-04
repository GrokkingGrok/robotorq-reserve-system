use commons::util::logging::init_test_logging;
use commons::util::persistence::{Context, DbAttributes, with_db_span};
use std::time::Duration;

#[tokio::test]
async fn with_db_span_records_timeout_and_runs_future() {
    // Initialize test logging/tracing subscriber once for span recording.
    let _ = init_test_logging("info");

    let ctx = Context {
        trace_headers: Default::default(),
        deadline: Some(tokio::time::Instant::now() + Duration::from_millis(50)),
        metadata: None,
    };

    let attrs = DbAttributes {
        driver: Some("memory".to_string()),
        op: Some("read".to_string()),
        entity: Some("unit".to_string()),
        statement: Some("stmt#1".to_string()),
    };

    let result = with_db_span(&ctx, &attrs, async {
        Ok::<_, commons::util::persistence::PersistenceError>(42)
    })
    .await;
    assert_eq!(result, Ok(42));
}

#[tokio::test]
async fn with_db_span_emits_fields_to_subscriber() {
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt;

    // Create a buffer writer for the subscriber to capture formatted output.
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));

    struct MutexWriter(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for MutexWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let mut g = self.0.lock().unwrap();
            g.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let make_writer = {
        let b = buf.clone();
        move || MutexWriter(b.clone())
    };

    use tracing_subscriber::fmt::format::FmtSpan;
    let subscriber = fmt::fmt()
        .with_span_events(FmtSpan::FULL)
        .with_writer(make_writer)
        .finish();

    let ctx = Context {
        trace_headers: Default::default(),
        deadline: Some(tokio::time::Instant::now() + std::time::Duration::from_millis(500)),
        metadata: None,
    };

    let attrs = DbAttributes {
        driver: Some("memory".to_string()),
        op: Some("read".to_string()),
        entity: Some("unit".to_string()),
        statement: Some("stmt#1".to_string()),
    };

    tracing::subscriber::with_default(subscriber, || {
        // Run the instrumented future inside the temporary subscriber on this thread
        let fut = async {
            with_db_span(&ctx, &attrs, async {
                Ok::<_, commons::util::persistence::PersistenceError>(1usize)
            })
            .await
        };
        let res = futures::executor::block_on(fut);
        assert_eq!(res, Ok(1usize));
    });

    // Inspect buffer contents for our attribute keys/values
    let out = {
        let g = buf.lock().unwrap();
        String::from_utf8_lossy(&g[..]).to_string()
    };

    // Print captured output for easier debugging on CI/local runs
    println!("CAPTURED OUTPUT:\n{}", out);

    assert!(
        out.contains("db.driver"),
        "output did not include db.driver: {}",
        out
    );
    assert!(
        out.contains("memory"),
        "output did not include driver value: {}",
        out
    );
    assert!(out.contains("db.operation"), "output missing db.operation");
    assert!(out.contains("read"), "output missing operation value");
    assert!(out.contains("db.entity"), "output missing db.entity");
    assert!(out.contains("unit"), "output missing entity value");
}
