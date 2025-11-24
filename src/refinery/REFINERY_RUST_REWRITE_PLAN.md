# Refinery Rust Rewrite Plan

## Overview

This document outlines the plan for rewriting the RoboTorq Refinery service from Go to Rust, following the architectural patterns established in the Rust Mint implementation. The Refinery is responsible for aggregating JouleTorqUnit (JTU) hashes into TokenTorqIngots and managing the Phase 2 hash-only processing pipeline.

## Current Go Refinery Architecture

### Core Components
- **QueueManager**: Thread-safe hash-based FIFO queue with blocking retrieval
- **Phase2IngotAssembler**: Aggregates 3600 hashes into ingots with merkle trees
- **NatsSubscriber**: Receives hash batches from Digger service
- **MintClient**: Sends completed ingots to Mint service
- **Merkle Tree Builder**: Constructs merkle roots from hash collections

### Key Responsibilities
1. Receive hash batches from Digger (Phase 2: hash-only, not full JTUs)
2. Buffer hashes in thread-safe queue with capacity limits
3. Aggregate 3600 hashes into ingots with merkle root calculation
4. Sign ingots with Falcon-1024 (Phase 5)
5. Send signed ingots to Mint service
6. Provide health/metrics endpoints

## Rust Mint Architecture Analysis

### Architectural Patterns to Adopt

#### 1. Async Runtime & Concurrency
- **Tokio**: Primary async runtime with multi-threaded executor
- **Channels**: `tokio::sync::mpsc` for inter-component communication
- **Arc/Clone**: Shared ownership for config, metrics, and engines
- **Graceful Shutdown**: Context-based cancellation throughout

#### 2. Modular Structure
```
src/
├── main.rs          # Service orchestration & HTTP server
├── lib.rs           # Module declarations
├── config.rs        # Environment-based configuration
├── metrics.rs       # Prometheus metrics definitions
├── models/          # Data structures (ingots, hashes, etc.)
├── engine/          # Core business logic
│   ├── mod.rs
│   ├── queue_manager.rs
│   ├── ingot_assembler.rs
│   ├── merkle_builder.rs
│   └── proof_engine.rs
├── handlers/        # NATS message handlers
│   ├── mod.rs
│   ├── hash_subscriber.rs
│   └── ingot_publisher.rs
├── nats_client.rs   # NATS connection management
└── archive.rs       # Optional data persistence
```

#### 3. Component Lifecycle
- **Initialization**: Config → Metrics → Engines → Handlers
- **Background Tasks**: Spawn tokio tasks for continuous processing
- **Signal Handling**: Graceful shutdown on SIGTERM/SIGINT
- **Health Checks**: HTTP endpoints with service status

#### 4. Error Handling
- **Anyhow**: Ergonomic error handling with context
- **Tracing**: Structured logging with spans and fields
- **Result Types**: Explicit error propagation

#### 5. Metrics & Observability
- **Prometheus**: Gauges, counters, histograms
- **HTTP Server**: `/metrics` and `/health` endpoints
- **Service Uptime**: Continuous uptime tracking

## Refinery Rust Rewrite Implementation Plan

### Phase 1: Core Infrastructure

#### 1.1 Project Setup
```rust
// Cargo.toml
[package]
name = "refinery"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
serde = { version = "1.0", features = ["derive"] }
async-nats = "0.35"
prometheus = "0.13"
reqwest = "0.12"  # For Mint client HTTP calls
```

#### 1.2 Configuration Module
```rust
// src/config.rs
#[derive(Debug, Clone)]
pub struct RefineryConfig {
    pub nats_url: String,
    pub mint_url: String,
    pub queue_capacity: usize,
    pub enable_archive: bool,
    pub crypto_enabled: bool,
    pub falcon_key_path: Option<String>,
}
```

#### 1.3 Metrics Module
```rust
// src/metrics.rs
pub struct RefineryMetrics {
    pub service_uptime_seconds: Counter,
    pub hashes_received_total: Counter,
    pub ingots_assembled_total: Counter,
    pub merkle_operations_total: Counter,
    pub queue_size: Gauge,
    pub assembly_duration: Histogram,
}
```

### Phase 2: Data Models

#### 2.1 Hash Entry Model
```rust
// src/models/hash_entry.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashEntry {
    pub hash: String,           // SHA256 hex of JTU
    pub contract_id: String,
    pub digger_id: String,
    pub robo_stake_paid: f64,
    pub timestamp: DateTime<Utc>,
    pub index: i64,
}
```

#### 2.2 Ingot Model
```rust
// src/models/ingot.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTorqIngot {
    pub id: String,
    pub branch_hash: String,    // Merkle root
    pub hash_count: usize,      // Always 3600
    pub contract_ids: Vec<String>,
    pub digger_ids: Vec<String>,
    pub robo_stake_total: f64,
    pub timestamp: DateTime<Utc>,
    pub signature: Option<String>,  // Falcon-1024
    pub public_key: Option<String>,
}
```

### Phase 3: Core Engine Components

#### 3.1 Queue Manager
```rust
// src/engine/queue_manager.rs
pub struct QueueManager {
    hash_queue: Vec<HashEntry>,
    capacity: usize,
    not_empty: Arc<Notify>,
    metrics: Arc<RefineryMetrics>,
}

impl QueueManager {
    pub async fn add_hash(&self, entry: HashEntry) -> Result<(), QueueError> {
        // Thread-safe hash addition with backpressure
    }

    pub async fn get_hashes(&self, count: usize) -> Result<Vec<HashEntry>, QueueError> {
        // Blocking retrieval with timeout
    }
}
```

#### 3.2 Merkle Builder
```rust
// src/engine/merkle_builder.rs
pub struct MerkleBuilder;

impl MerkleBuilder {
    pub fn build_tree(hashes: &[String]) -> Result<String, MerkleError> {
        // Construct merkle tree and return root hash
    }
}
```

#### 3.3 Ingot Assembler
```rust
// src/engine/ingot_assembler.rs
pub struct IngotAssembler {
    queue_manager: Arc<QueueManager>,
    merkle_builder: MerkleBuilder,
    proof_engine: Arc<ProofEngine>,
    metrics: Arc<RefineryMetrics>,
}

impl IngotAssembler {
    pub async fn assemble_ingots(&self) -> Result<(), AssemblyError> {
        loop {
            // Get 3600 hashes from queue
            // Build merkle tree
            // Create signed ingot
            // Send to Mint
        }
    }
}
```

### Phase 4: Message Handlers

#### 4.1 Hash Subscriber
```rust
// src/handlers/hash_subscriber.rs
pub struct HashSubscriber {
    nats_client: Arc<NatsClient>,
    queue_manager: Arc<QueueManager>,
    metrics: Arc<RefineryMetrics>,
}

impl HashSubscriber {
    pub async fn start(&self) -> Result<()> {
        // Subscribe to "refinery.hashes" subject
        // Parse incoming hash batches
        // Add to queue manager
    }
}
```

#### 4.2 Ingot Publisher
```rust
// src/handlers/ingot_publisher.rs
pub struct IngotPublisher {
    nats_client: Arc<NatsClient>,
    metrics: Arc<RefineryMetrics>,
}

impl IngotPublisher {
    pub async fn publish_ingot(&self, ingot: TokenTorqIngot) -> Result<()> {
        // Serialize ingot to JSON
        // Publish to "mint.ingots" subject
    }
}
```

### Phase 5: Service Orchestration

#### 5.1 Main Service Loop
```rust
// src/main.rs
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = Arc::new(RefineryConfig::from_env()?);
    let metrics = Arc::new(RefineryMetrics::new());

    // Initialize components
    let nats_client = nats_client::connect(&config.nats_url).await?;
    let queue_manager = Arc::new(QueueManager::new(config.queue_capacity, metrics.clone()));
    let proof_engine = Arc::new(ProofEngine::new(config.clone(), metrics.clone()));

    // Start HTTP server for metrics/health
    start_http_server(metrics.clone()).await;

    // Start hash subscriber
    let subscriber = HashSubscriber::new(nats_client.clone(), queue_manager.clone(), metrics.clone());
    tokio::spawn(async move {
        subscriber.start().await?;
    });

    // Start ingot assembler
    let assembler = IngotAssembler::new(queue_manager, proof_engine, metrics.clone());
    tokio::spawn(async move {
        assembler.assemble_ingots().await?;
    });

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    Ok(())
}
```

## Migration Strategy

### Phase 1: Parallel Implementation
- Implement Rust Refinery alongside existing Go version
- Use different NATS subjects for testing (`refinery.hashes.rust`, `mint.ingots.rust`)
- Compare outputs and performance metrics

### Phase 2: Integration Testing
- Route subset of traffic to Rust version
- Compare ingot quality, throughput, and error rates
- Validate merkle tree construction and signatures

### Phase 3: Full Migration
- Switch NATS subjects to production names
- Remove Go version after successful operation
- Update Docker Compose and deployment scripts

## Benefits of Rust Rewrite

### Performance
- **Zero-cost abstractions**: High-performance concurrent processing
- **Memory safety**: Compile-time guarantees prevent data races
- **Async efficiency**: Tokio provides scalable async runtime

### Maintainability
- **Strong typing**: Catch errors at compile time
- **Modular architecture**: Clear separation of concerns
- **Comprehensive testing**: Built-in test frameworks

### Observability
- **Structured logging**: Consistent log formats with tracing
- **Rich metrics**: Detailed performance and health monitoring
- **Health endpoints**: Standardized service health checks

## Risk Mitigation

### Testing Strategy
- **Unit tests**: Test individual components in isolation
- **Integration tests**: End-to-end pipeline validation
- **Performance benchmarks**: Compare with Go implementation
- **Chaos testing**: Simulate network failures and queue overflows

### Rollback Plan
- Keep Go version deployed alongside Rust
- Feature flags to route traffic between implementations
- Automated canary deployments with gradual traffic increase

### Monitoring
- **Golden signals**: Latency, throughput, errors, saturation
- **Business metrics**: Ingots assembled, queue depths, signature success rates
- **Alerting**: Automatic rollback triggers on error rate thresholds

## Implementation Timeline

### Week 1-2: Infrastructure & Models
- Project setup, dependencies, basic structure
- Data models (HashEntry, TokenTorqIngot)
- Configuration and metrics systems

### Week 3-4: Core Engine
- Queue manager with async blocking operations
- Merkle tree builder
- Ingot assembler with signature support

### Week 5-6: Message Handling
- NATS subscriber for hash batches
- Ingot publisher to Mint service
- Error handling and retry logic

### Week 7-8: Integration & Testing
- HTTP server for metrics/health
- Comprehensive test suite
- Performance benchmarking vs Go version

### Week 9-10: Production Readiness
- Documentation and deployment scripts
- Parallel operation testing
- Gradual traffic migration

## Success Criteria

- **Functional parity**: Produces identical ingots to Go version
- **Performance**: ≥95% of Go throughput with lower memory usage
- **Reliability**: Zero data loss, graceful error handling
- **Observability**: Complete metrics coverage with alerting
- **Maintainability**: Clear code structure with comprehensive tests

This rewrite will modernize the Refinery service with Rust's performance and safety guarantees while maintaining full compatibility with the existing RoboTorq pipeline.