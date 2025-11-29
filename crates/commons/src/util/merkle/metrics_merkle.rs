use crate::metrics::MetricsHandler;
use prometheus::{Counter, Histogram};

/// Metrics dedicated to Merkle operations.
#[derive(Clone)]
pub struct MerkleMetrics {
	pub leaves_total: Counter,
	pub build_duration_seconds: Histogram,
}

impl MerkleMetrics {
	/// Register Merkle-specific metrics into the provided handler.
	pub fn register(handler: &MetricsHandler) -> Self {
		let leaves_total = handler.register_counter(
			"merkle_leaves_total",
			"Total number of leaves added to Merkle builders",
		);
		let build_duration_seconds = handler.register_histogram(
			"merkle_build_duration_seconds",
			"Time to finalize Merkle trees (seconds)",
			&[0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0],
		);
		Self { leaves_total, build_duration_seconds }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::metrics::MetricsHandler;

	#[test]
	fn merkle_metrics_register_and_use() {
		let handler = MetricsHandler::new();
		let m = MerkleMetrics::register(&handler);
		m.leaves_total.inc();
		m.build_duration_seconds.observe(0.02);
		let _ = handler.export_text();
	}
}
