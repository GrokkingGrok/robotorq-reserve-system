//! Integration tests for the HTTP service template.
//!
//! These tests verify the full end-to-end functionality of the HttpServer
//! with a test service, including metrics, health checks, and config loading.

use std::sync::Arc;
use tokio::sync::Mutex;

use commons::{
    services::http::{HttpServer, HttpServerConfig, label_source::build_label_set, RoboTorqService},
    util::config::load_robotorq_config,
    util::error::InvariantError,
    util::metrics::{MetricsRegistry, PrometheusRegistry},
};

/// Test service for integration tests.
struct TestService {
    health_counter: Box<dyn commons::util::metrics::MetricCounter + Send + Sync>,
    registry: Arc<PrometheusRegistry>,
}

impl TestService {
    fn new(labels: &commons::services::http::label_source::LabelSet) -> Self {
        let registry = Arc::new(PrometheusRegistry::new(&labels.service, &labels.component, &labels.version));
        let health_counter = registry.counter("test_health_checks_total", "Test health checks", &[]);
        Self {
            health_counter,
            registry,
        }
    }
}

impl RoboTorqService for TestService {
    fn health_check(&self) -> Result<String, InvariantError> {
        self.health_counter.inc();
        Ok("Test service healthy".to_string())
    }

    fn export_metrics(&self) -> String {
        self.registry.export_text()
    }

    async fn start(&self) -> Result<(), InvariantError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Client;

    fn extract_metric_value(text: &str, name: &str) -> Option<f64> {
        for line in text.lines() {
            if line.starts_with('#') { continue; }
            if line.starts_with(name) && let Some(idx) = line.rfind(' ') {
                let val_str = &line[idx+1..];
                if let Ok(v) = val_str.trim().parse::<f64>() { return Some(v); }
            }
        }
        None
    }

    /// Test full server lifecycle with HTTP requests.
    #[tokio::test]
    async fn test_http_integration() {
        // Use a fixed port for testing (assume 9999 is free)
        const TEST_PORT: u16 = 9999;

        // Load config and derive labels
        let config = load_robotorq_config(None).expect("config should load");
        let labels = build_label_set(&config);

        // Create registry with derived labels
        let registry: Arc<dyn MetricsRegistry> = Arc::new(PrometheusRegistry::new(&labels.service, &labels.component, &labels.version));

        // Build HTTP config with test port
        let http_config = HttpServerConfig::local_defaults(TEST_PORT)
            .with_metrics_registry(Arc::clone(&registry));

        // Create test service
        let service = Arc::new(Mutex::new(TestService::new(&labels)));

        // Build server
        let server = HttpServer::new(Arc::clone(&service), http_config);

        // Start server in background
        let server_handle = tokio::spawn(async move {
            server.start(&config).await
        });

        // Wait for server to start
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Create HTTP client
        let client = Client::new();
        let base_url = format!("http://127.0.0.1:{}", TEST_PORT);

        // Test /healthz
        let resp = client.get(format!("{}/healthz", base_url))
            .send()
            .await
            .expect("healthz request failed");
        assert_eq!(resp.status(), 200);
        let body = resp.text().await.expect("failed to read healthz body");
        assert_eq!(body, "Test service healthy");

        // Test /metrics
        let resp = client.get(format!("{}/metrics", base_url))
            .send()
            .await
            .expect("metrics request failed");
        assert_eq!(resp.status(), 200);
        let body = resp.text().await.expect("failed to read metrics body");
        assert!(body.contains("test_health_checks_total")); // Metric present
        let hc = extract_metric_value(&body, "test_health_checks_total").expect("health check metric value present");
        assert!((hc - 1.0).abs() < f64::EPSILON);
        let ready = extract_metric_value(&body, "http_ready").expect("http_ready metric present");
        assert!((ready - 1.0).abs() < f64::EPSILON);
        // New HTTP-level metric should be present and incremented
        let http_h = extract_metric_value(&body, "http_health_requests_total").expect("http health requests metric present");
        assert!(http_h >= 1.0);
        assert!(body.contains(&format!("service=\"{}\"", labels.service)));
        assert!(body.contains("component=\"http\""));
        assert!(body.contains(&format!("version=\"{}\"", labels.version)));

        // Test CORS headers
        let resp = client.get(format!("{}/healthz", base_url))
            .header("Origin", "http://example.com")
            .send()
            .await
            .expect("CORS request failed");
        assert_eq!(resp.status(), 200);
        assert!(resp.headers().contains_key("access-control-allow-origin"));

        // Server should shut down when we drop the handle, but for clean test, we can abort
        server_handle.abort();
    }

    /// Test config-derived labels are applied correctly.
    #[tokio::test]
    async fn test_config_derived_labels_integration() {
        let config = load_robotorq_config(None).expect("config should load");
        let labels = build_label_set(&config);

        println!("Labels: service={}, component={}, version={}, subject={}", labels.service, labels.component, labels.version, labels.subject);

        // Verify labels are derived from config
        assert!(!labels.service.is_empty());
        assert_eq!(labels.component, "http");
        assert!(!labels.version.is_empty());
        assert!(labels.subject == "core" || labels.subject == "sim");

        // Create service with these labels and perform a health check so the counter is instantiated
        let service = TestService::new(&labels);
        let _ = service.health_check();
        let metrics = service.export_metrics();

        println!("Metrics: {}", metrics);

        // Verify metrics use the derived labels
        assert!(metrics.contains(&format!("service=\"{}\"", labels.service)));
        assert!(metrics.contains("component=\"http\""));
        assert!(metrics.contains(&format!("version=\"{}\"", labels.version)));
    }

    /// Test that health checks increment the counter.
    #[tokio::test]
    async fn test_health_check_counter() {
        let config = load_robotorq_config(None).expect("config should load");
        let labels = build_label_set(&config);
        let service = TestService::new(&labels);
        
        // After first health check, counter should be 1
        service.health_check().unwrap();
        let after = service.export_metrics();
        let v1 = extract_metric_value(&after, "test_health_checks_total").expect("metric present after first health");
        assert!((v1 - 1.0).abs() < f64::EPSILON);

        // After another, counter should be 2
        service.health_check().unwrap();
        let final_metrics = service.export_metrics();
        let v2 = extract_metric_value(&final_metrics, "test_health_checks_total").expect("metric present after second health");
        assert!((v2 - 2.0).abs() < f64::EPSILON);
    }
}