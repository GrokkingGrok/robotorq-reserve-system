use anyhow::Result;
use serde::Deserialize;
use tracing::debug;

#[derive(Clone)]
pub struct KlipperClient {
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct PrintStatsResponse {
    result: PrintStatsResult,
}

#[derive(Debug, Deserialize)]
struct PrintStatsResult {
    status: PrintStatsStatus,
}

#[derive(Debug, Deserialize)]
struct PrintStatsStatus {
    print_stats: PrintStats,
}

#[derive(Debug, Deserialize)]
struct PrintStats {
    state: String,
}

impl KlipperClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap(),
            base_url: base_url.to_string(),
        }
    }
    
    pub async fn is_printing(&self) -> Result<bool> {
        let url = format!("{}/printer/objects/query?print_stats", self.base_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
        
        let data: PrintStatsResponse = response.json().await?;
        let state = &data.result.status.print_stats.state;
        
        debug!("Klipper state: {}", state);
        
        // Klipper states:
        // - "standby": Idle, no print
        // - "printing": Actively printing
        // - "paused": Print paused (still consuming capacity)
        // - "complete": Print just finished
        // - "cancelled": Print was cancelled
        // - "error": Print error
        
        Ok(state == "printing" || state == "paused")
    }
    
    pub async fn get_print_info(&self) -> Result<PrintInfo> {
        let url = format!("{}/printer/objects/query?print_stats", self.base_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
        
        let data: PrintStatsResponse = response.json().await?;
        let stats = &data.result.status.print_stats;
        
        Ok(PrintInfo {
            state: stats.state.clone(),
        })
    }
}

#[derive(Debug)]
pub struct PrintInfo {
    pub state: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_klipper_client_creation() {
        let client = KlipperClient::new("http://localhost:7125");
        assert_eq!(client.base_url, "http://localhost:7125");
    }
}
