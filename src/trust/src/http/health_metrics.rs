use crate::metrics::METRICS;
use hyper::{Body, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};

static READY: AtomicBool = AtomicBool::new(false);

pub fn set_ready() {
    READY.store(true, Ordering::SeqCst);
}

async fn router(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match (req.method().as_str(), req.uri().path()) {
        ("GET", "/health") => {
            let body = serde_json::json!({"status":"ok","ready": READY.load(Ordering::SeqCst)}).to_string();
            Ok(Response::new(Body::from(body)))
        }
        ("GET", "/ready") => {
            if READY.load(Ordering::SeqCst) {
                let body = serde_json::json!({"ready": true}).to_string();
                Ok(Response::new(Body::from(body)))
            } else {
                let body = serde_json::json!({"ready": false}).to_string();
                let mut resp = Response::new(Body::from(body));
                *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
                Ok(resp)
            }
        }
        ("GET", "/metrics") => {
            let body = METRICS.gather_prometheus();
            Ok(Response::new(Body::from(body)))
        }
        _ => {
            let mut resp = Response::new(Body::from("not found"));
            *resp.status_mut() = StatusCode::NOT_FOUND;
            Ok(resp)
        }
    }
}

pub async fn start_http_server(host: String, port: u16, mut shutdown: tokio::sync::watch::Receiver<bool>) {
    let addr = format!("{}:{}", host, port).parse().expect("invalid listen addr");

    let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(router)) });

    let server = Server::bind(&addr).serve(make_svc);

    tracing::info!(host = %host, port = port, "starting http health/metrics server");

    let graceful = server.with_graceful_shutdown(async move {
        // wait for shutdown signal
        let _ = shutdown.changed().await;
        tracing::info!("http server shutting down");
    });

    if let Err(e) = graceful.await {
        tracing::error!(error = %e, "http server error");
    }
}
