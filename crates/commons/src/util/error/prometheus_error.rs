use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrometheusError {
    #[error("prometheus error: {0}")]
    Inner(#[from] prometheus::Error),
}
