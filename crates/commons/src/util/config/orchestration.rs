

use serde::{Deserialize, Serialize};
use crate::util::config::mode::Mode;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Orchestration {
    pub mode: Mode,
    pub nats_url: String,
    pub prometheus_bind: Option<String>,
}