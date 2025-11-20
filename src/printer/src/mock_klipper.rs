use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Mock Klipper client that simulates printer state without real hardware
pub struct MockKlipperClient {
    state: Arc<RwLock<MockPrinterState>>,
}

#[derive(Debug, Clone)]
struct MockPrinterState {
    is_printing: bool,
    print_started_at: Option<std::time::Instant>,
    print_duration_secs: f64,
}

impl MockKlipperClient {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(MockPrinterState {
                is_printing: false,
                print_started_at: None,
                print_duration_secs: 0.0,
            })),
        }
    }

    /// Get current printer state (idle or printing)
    pub async fn get_printer_state(&self) -> Result<String> {
        let state = self.state.read().await;
        Ok(if state.is_printing {
            "printing".to_string()
        } else {
            "idle".to_string()
        })
    }

    /// Check if printer is currently printing
    pub async fn is_printing(&self) -> Result<bool> {
        let state = self.state.read().await;
        Ok(state.is_printing)
    }

    /// Get printer info (simulated)
    pub async fn get_printer_info(&self) -> Result<serde_json::Value> {
        let state = self.state.read().await;
        Ok(json!({
            "state": if state.is_printing { "printing" } else { "idle" },
            "state_message": if state.is_printing { "Printing from mock" } else { "Ready" },
            "hostname": "mock-klipper",
            "software_version": "mock-v1.0.0",
        }))
    }

    /// Simulate starting a print job
    pub async fn start_print(&self) -> Result<()> {
        let mut state = self.state.write().await;
        state.is_printing = true;
        state.print_started_at = Some(std::time::Instant::now());
        Ok(())
    }

    /// Simulate completing a print job
    pub async fn complete_print(&self) -> Result<()> {
        let mut state = self.state.write().await;
        if let Some(started) = state.print_started_at {
            state.print_duration_secs = started.elapsed().as_secs_f64();
        }
        state.is_printing = false;
        state.print_started_at = None;
        Ok(())
    }

    /// Get print duration (if currently printing or just completed)
    pub async fn get_print_duration(&self) -> Result<f64> {
        let state = self.state.read().await;
        if let Some(started) = state.print_started_at {
            Ok(started.elapsed().as_secs_f64())
        } else {
            Ok(state.print_duration_secs)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_initial_state() {
        let client = MockKlipperClient::new();
        assert_eq!(client.get_printer_state().await.unwrap(), "idle");
        assert!(!client.is_printing().await.unwrap());
    }

    #[tokio::test]
    async fn test_mock_start_print() {
        let client = MockKlipperClient::new();
        client.start_print().await.unwrap();
        assert_eq!(client.get_printer_state().await.unwrap(), "printing");
        assert!(client.is_printing().await.unwrap());
    }

    #[tokio::test]
    async fn test_mock_complete_print() {
        let client = MockKlipperClient::new();
        client.start_print().await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        client.complete_print().await.unwrap();
        
        assert_eq!(client.get_printer_state().await.unwrap(), "idle");
        assert!(!client.is_printing().await.unwrap());
        
        let duration = client.get_print_duration().await.unwrap();
        assert!(duration >= 0.1, "Duration should be at least 100ms");
    }

    #[tokio::test]
    async fn test_mock_printer_info() {
        let client = MockKlipperClient::new();
        let info = client.get_printer_info().await.unwrap();
        assert_eq!(info["state"], "idle");
        assert_eq!(info["hostname"], "mock-klipper");
    }
}
