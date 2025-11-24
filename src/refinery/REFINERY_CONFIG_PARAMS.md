# Refinery Configuration Parameters (Rust)

This document defines the configuration surface for the Rust `refinery` service, derived from the current Go implementation and the Rust Mint configuration patterns.

Keep these as environment variables in production and allow a `RefineryConfig::from_env()` convenience constructor in `config.rs`.

---

## Required parameters

- `REFINERY_NATS_URL` (string)
  - Default: `nats://nats:4222`
  - Description: NATS server URL used to subscribe to hash subjects and publish ingots.

- `REFINERY_MINT_URL` (string)
  - Default: `http://mint:8080` (or NATS subject `mint.ingots` depending on transport)
  - Description: Endpoint or subject for sending completed ingots to Mint.

- `REFINERY_HASH_SUBJECT` (string)
  - Default: `refinery.hashes`
  - Description: NATS subject to subscribe for incoming hash batches from Digger.

- `REFINERY_INGOT_SUBJECT` (string)
  - Default: `mint.ingots`
  - Description: NATS subject to publish assembled ingots (or HTTP endpoint if Mint accepts HTTP).

## Queue and batching

- `REFINERY_QUEUE_CAPACITY` (usize)
  - Default: `10000`
  - Description: Maximum number of hash entries buffered in memory before backpressure / reject.

- `REFINERY_HASHES_PER_INGOT` (usize)
  - Default: `3600` (RoboTorq spec)
  - Description: Number of JTUs/hashes per TokenTorqIngot. Usually constant.

- `REFINERY_INGOT_BATCH_TIMEOUT_SECONDS` (i64)
  - Default: `60`
  - Description: Max wait time before assembling an ingot when not enough hashes are available (optional fallback behavior).

- `REFINERY_ASSEMBLER_WORKERS` (usize)
  - Default: `1` (can be increased on multi-core machines)
  - Description: Number of concurrent ingot assembler tasks that pull from the queue.

## Crypto and signatures

- `REFINERY_ENABLE_CRYPTO` (bool)
  - Default: `false`
  - Description: Toggle signing of ingots (Falcon-1024 / configured algorithm).

- `REFINERY_SIGNATURE_ALGORITHM` (string)
  - Default: `falcon1024`
  - Description: Algorithm to use when signing (e.g., `falcon1024`, `dilithium5`, `sphincs+`, `none`).

- `REFINERY_KEY_STORAGE_PATH` (string)
  - Default: `/data/refinery/keys`
  - Description: Filesystem path or KMS identifier for private keys used to sign ingots.

## Archive & persistence

- `REFINERY_ENABLE_ARCHIVE` (bool)
  - Default: `true`
  - Description: Persist assembled ingot metadata and merkle roots (in-memory vs on-disk depending on configuration).

- `REFINERY_ARCHIVE_PATH` (string)
  - Default: `/data/refinery/archive`
  - Description: Path for persisted ingot records, if archive enabled.

- `REFINERY_ARCHIVE_RETENTION_DAYS` (u32)
  - Default: `30`
  - Description: How long to retain archive entries before cleanup.

## Observability & HTTP

- `REFINERY_METRICS_HOST` (string)
  - Default: `0.0.0.0`

- `REFINERY_METRICS_PORT` (u16)
  - Default: `8081`
  - Description: HTTP port exposing `/metrics` and `/health` endpoints.

- `REFINERY_LOG_LEVEL` (string)
  - Default: `info`
  - Description: Logging level for structured tracing (trace, debug, info, warn, error).

## Prometheus metric names (recommended)

- `refinery_hash_queue_size` (gauge)
- `refinery_hashes_queued_total` (counter)
- `refinery_hashes_dequeued_total` (counter)
- `refinery_merkle_trees_built_total` (counter)
- `refinery_merkle_build_duration_seconds` (histogram)
- `refinery_phase2_ingots_assembled_total` (counter)
- `refinery_robostake_aggregated_total` (counter)
- `refinery_ingot_publish_errors_total` (counter)
- `refinery_service_uptime_seconds` (gauge)

## Retry, backoff & error policies

- `REFINERY_PUBLISH_RETRY_MAX` (u32)
  - Default: `5`
  - Description: Max attempts to publish ingot to Mint before marking failure.

- `REFINERY_PUBLISH_RETRY_BASE_MS` (u64)
  - Default: `200`
  - Description: Base backoff in milliseconds; exponential backoff used.

## Simulation parameters (mirror `mint` simulation config)

Include these behind a `simulation` feature flag in the Rust crate, or accept them as env vars and ignore when simulation disabled.

- `SIMULATION_MODE` (bool)
  - Default: `false`
  - Description: Run in simulation mode (fast-forwarded time, fake ingot arrivals).

- `SIMULATION_TIME_COMPRESSION` (f64)
  - Default: `1.0`
  - Description: Multiply delays by this factor to speed/slow simulated time.

- `REFINERY_INGOT_GENERATION_RATE` (f64)
  - Default: `10.0` (ingots/second in simulation time)
  - Description: When sim mode active, generate ingots at this rate instead of real ingestion.

- `REFINERY_BATCH_PROCESSING_DELAY_MS` (i64)
  - Default: `100`
  - Description: Artificial delays for assembly/processing in sim mode.

- `REFINERY_PROOF_SIGNING_DELAY_MS` (i64)
  - Default: `500`
  - Description: Artificial signing delays in sim mode.

- `SIMULATION_SCENARIO_ID` (string)
  - Default: none
  - Description: Tag used to group simulated runs and test fixtures.

## Defaults & notes

- Hash-to-ingot mapping should remain `3600` by default to preserve proof chain semantics.
- Use environment variables for all runtime tunables; provide `RefineryConfig::default()` returning safe defaults.
- Keep simulation fields behind a Cargo feature flag `simulation` to avoid shipping test-only options in production builds.

## Example `RefineryConfig` Rust struct (skeleton)

```rust
#[derive(Clone, Debug)]
pub struct RefineryConfig {
    // Connectivity
    pub nats_url: String,
    pub hash_subject: String,
    pub ingot_subject: String,
    pub mint_url: Option<String>,

    // Queue & batching
    pub queue_capacity: usize,
    pub hashes_per_ingot: usize,
    pub assembler_workers: usize,
    pub ingot_batch_timeout_seconds: i64,

    // Crypto
    pub enable_crypto: bool,
    pub signature_algorithm: String,
    pub key_storage_path: Option<String>,

    // Archive
    pub enable_archive: bool,
    pub archive_path: Option<String>,
    pub archive_retention_days: u32,

    // Observability
    pub metrics_host: String,
    pub metrics_port: u16,
    pub log_level: String,

    // Retry
    pub publish_retry_max: u32,
    pub publish_retry_base_ms: u64,

    // Simulation (cfg(feature = "simulation"))
    #[cfg(feature = "simulation")]
    pub simulation_mode: bool,
    #[cfg(feature = "simulation")]
    pub simulation_time_compression: f64,
    #[cfg(feature = "simulation")]
    pub ingot_generation_rate: f64,
    #[cfg(feature = "simulation")]
    pub batch_processing_delay_ms: i64,
    #[cfg(feature = "simulation")]
    pub proof_signing_delay_ms: i64,
    #[cfg(feature = "simulation")]
    pub simulation_scenario_id: Option<String>,
}
```

## Next steps

- Add `src/refinery/src/config.rs` implementing `RefineryConfig::from_env()` and `Default` using the above
- Wire the config into the rewrite plan (optional small doc update)
- Add unit tests mirroring `src/mint/src/config.rs` simulation tests

---

Created from the analysis of the current Go `refinery` implementation and the Rust `mint` configuration patterns.
