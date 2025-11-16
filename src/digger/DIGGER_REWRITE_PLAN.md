# Digger Rewrite Plan: Pure Backend First

**Date**: November 15, 2025  
**Branch**: `feature/digger-refactor`  
**Status**: 🔴 REWRITE REQUIRED - Tauri blocking critical proof chain work

---

## 🎯 Core Problem

**Tauri frontend is stealing the backend's thunder.**

The GUI is **NOT** needed for the proof chain to work. We need to:
1. Strip out ALL Tauri code
2. Keep pure HTTP server for testing
3. Focus 100% on correct proof generation
4. Add frontend later when architecture is solid

---

## ✅ What We're Keeping

### HTTP Server (Essential)
```rust
// src/http_api.rs - KEEP THIS
pub struct ApiServer {
    digger: Arc<Mutex<Digger>>,
    contracts: Arc<Mutex<HashMap<String, Contract>>>,
}

// Routes we need:
POST   /contracts/create    - Create new contract
POST   /contracts/stake     - Pay RoboStake for contract  
POST   /contracts/execute   - Start work on contract
GET    /contracts/{id}      - Get contract status
GET    /status              - Digger health/stats
GET    /jtus/{contract_id}  - Query stored JTUs (for testing)
```

### Core Digger Logic (Essential)
```rust
// src/digger.rs - KEEP THIS (with modifications)
pub struct Digger {
    pub id: String,
    pub nats_client: NatsClient,
    pub jtu_storage: JtuStorageManager,  // NEW
    pub contracts: HashMap<String, ContractState>,
}

// Keep these methods:
- execute_contract()
- process_milestone()  
- send_ore_batch()     // Will change to send_hash_batch()
```

### Configuration (Essential)
```rust
// src/config.rs or environment variables
struct DiggerConfig {
    digger_id: String,
    nats_url: String,
    http_port: u16,
    storage_path: String,  // NEW: "./jtu_storage"
    batch_interval_sec: u64,  // NEW: 60 seconds
}
```

---

## ❌ What We're Removing

### Tauri GUI (Complete Removal)
- **tauri.conf.json** - DELETE
- **build.rs** (Tauri-specific parts) - SIMPLIFY
- **Cargo.toml** tauri dependencies - REMOVE
- **src/main.rs** (Tauri app setup) - REPLACE with pure HTTP server
- **ALL** `#[tauri::command]` functions - CONVERT to HTTP endpoints

### Frontend Code (Not Needed Yet)
- **src-ui/** directory - IGNORE (don't delete, just don't build)
- Any JavaScript/Svelte compilation - SKIP

---

## 🏗️ New Architecture

### Directory Structure
```
src/digger-app/digger/
├── Cargo.toml              # Simplified (no Tauri)
├── src/
│   ├── main.rs             # Pure HTTP server (Axum or Actix)
│   ├── http_api.rs         # API routes
│   ├── digger.rs           # Core work logic
│   ├── config.rs           # Configuration
│   ├── jtu_storage.rs      # NEW: SQLite storage manager
│   ├── jtu_hasher.rs       # NEW: Hash generation
│   ├── contract_state.rs   # NEW: Per-contract state machine
│   └── lib.rs              # Shared library code
├── jtu_storage/            # NEW: Local JTU database (gitignored)
│   ├── contract-001.db
│   ├── contract-002.db
│   └── ...
└── tests/
    ├── integration_test.rs
    └── jtu_storage_test.rs
```

---

## 📦 New Dependencies (Cargo.toml)

```toml
[package]
name = "digger"
version = "0.2.0"
edition = "2021"

[[bin]]
name = "digger"
path = "src/main.rs"

[dependencies]
# HTTP Server
axum = "0.7"           # Or actix-web if you prefer
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Database
rusqlite = { version = "0.32", features = ["bundled"] }

# Hashing
sha2 = "0.10"
hex = "0.4"

# NATS
async-nats = "0.36"

# Configuration
dotenvy = "0.15"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Time
chrono = { version = "0.4", features = ["serde"] }

# Crypto (placeholder for now, Falcon later)
# pqcrypto-falcon = "0.3"  # Add in Phase 4

[dev-dependencies]
reqwest = { version = "0.12", features = ["json"] }
```

---

## 🔨 Implementation Steps

### Step 1: Tag Current State (Safety Net)
```powershell
git checkout feature/digger-refactor
git tag digger-with-tauri-backup
git push origin digger-with-tauri-backup
```

### Step 2: Simplify Cargo.toml
```powershell
# Edit Cargo.toml to remove ALL tauri dependencies
# Keep only: axum, tokio, serde, rusqlite, sha2, async-nats, chrono
```

### Step 3: Create New main.rs (Pure HTTP Server)
```rust
// src/main.rs
use axum::{
    routing::{get, post},
    Router,
    Json,
    extract::{State, Path},
};
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

mod http_api;
mod digger;
mod config;
mod jtu_storage;
mod jtu_hasher;
mod contract_state;

use digger::Digger;
use config::DiggerConfig;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Load config
    let config = DiggerConfig::from_env();
    
    // Initialize Digger
    let digger = Arc::new(Mutex::new(Digger::new(config).await.unwrap()));
    
    // Build HTTP routes
    let app = Router::new()
        .route("/status", get(http_api::get_status))
        .route("/contracts/create", post(http_api::create_contract))
        .route("/contracts/stake", post(http_api::stake_contract))
        .route("/contracts/execute", post(http_api::execute_contract))
        .route("/contracts/:id", get(http_api::get_contract))
        .route("/jtus/:contract_id", get(http_api::get_jtus))
        .with_state(digger);
    
    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.http_port));
    tracing::info!("Digger HTTP server listening on {}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

### Step 4: Create JTU Storage Manager
```rust
// src/jtu_storage.rs
use rusqlite::{Connection, params};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JouleTorqUnit {
    pub hash: String,
    pub signature: Vec<u8>,      // Placeholder for now
    pub digger_id: String,
    pub contract_id: String,
    pub token_index: i64,
    pub milestone_index: i64,
    pub timestamp: i64,
}

pub struct JtuStorageManager {
    storage_path: PathBuf,
}

impl JtuStorageManager {
    pub fn new(storage_path: PathBuf) -> Self {
        std::fs::create_dir_all(&storage_path).unwrap();
        Self { storage_path }
    }
    
    pub fn get_connection(&self, contract_id: &str) -> rusqlite::Result<Connection> {
        let db_path = self.storage_path.join(format!("{}.db", contract_id));
        let conn = Connection::open(db_path)?;
        
        // Create table if not exists
        conn.execute(
            "CREATE TABLE IF NOT EXISTS jtus (
                hash TEXT PRIMARY KEY,
                signature BLOB,
                digger_id TEXT,
                contract_id TEXT,
                token_index INTEGER,
                milestone_index INTEGER,
                timestamp INTEGER
            )",
            [],
        )?;
        
        Ok(conn)
    }
    
    pub fn insert_jtu(&self, jtu: &JouleTorqUnit) -> rusqlite::Result<()> {
        let conn = self.get_connection(&jtu.contract_id)?;
        
        conn.execute(
            "INSERT INTO jtus VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                jtu.hash,
                jtu.signature,
                jtu.digger_id,
                jtu.contract_id,
                jtu.token_index,
                jtu.milestone_index,
                jtu.timestamp,
            ],
        )?;
        
        Ok(())
    }
    
    pub fn get_all_hashes(&self, contract_id: &str) -> rusqlite::Result<Vec<String>> {
        let conn = self.get_connection(contract_id)?;
        
        let mut stmt = conn.prepare("SELECT hash FROM jtus ORDER BY timestamp")?;
        let hashes = stmt.query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        
        Ok(hashes)
    }
    
    pub fn get_jtu_count(&self, contract_id: &str) -> rusqlite::Result<i64> {
        let conn = self.get_connection(contract_id)?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM jtus", [], |row| row.get(0))?;
        Ok(count)
    }
    
    pub fn prune_old(&self, contract_id: &str, days: i64) -> rusqlite::Result<usize> {
        let conn = self.get_connection(contract_id)?;
        let cutoff = chrono::Utc::now().timestamp() - (days * 86400);
        
        conn.execute(
            "DELETE FROM jtus WHERE timestamp < ?1",
            params![cutoff],
        )
    }
}
```

### Step 5: Create JTU Hasher
```rust
// src/jtu_hasher.rs
use sha2::{Sha256, Digest};

pub fn calculate_jtu_hash(
    token_id: &str,
    joules: f64,
    timestamp: i64,
    contract_id: &str,
    digger_id: &str,
) -> String {
    let data = format!(
        "{}:{}:{}:{}:{}",
        token_id, joules, timestamp, contract_id, digger_id
    );
    
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    
    hex::encode(result)
}

pub fn create_placeholder_signature() -> Vec<u8> {
    // TODO: Replace with real Falcon-1024 signature in Phase 4
    vec![0u8; 64]  // 64-byte placeholder
}
```

### Step 6: Update Digger to Use Storage
```rust
// src/digger.rs (modified)
use crate::jtu_storage::{JtuStorageManager, JouleTorqUnit};
use crate::jtu_hasher::{calculate_jtu_hash, create_placeholder_signature};

pub struct Digger {
    pub id: String,
    pub nats_client: async_nats::Client,
    pub storage: JtuStorageManager,
    pub contracts: HashMap<String, ContractState>,
}

impl Digger {
    // When processing a token (inside execute_contract loop)
    pub fn create_and_store_jtu(
        &mut self,
        contract_id: &str,
        token_index: i64,
        milestone_index: i64,
        joules: f64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let timestamp = chrono::Utc::now().timestamp();
        let token_id = format!("{}-m{}-t{}", contract_id, milestone_index, token_index);
        
        // Calculate hash (joules included in hash, then discarded!)
        let hash = calculate_jtu_hash(
            &token_id,
            joules,
            timestamp,
            contract_id,
            &self.id,
        );
        
        // Create JTU
        let jtu = JouleTorqUnit {
            hash: hash.clone(),
            signature: create_placeholder_signature(),
            digger_id: self.id.clone(),
            contract_id: contract_id.to_string(),
            token_index,
            milestone_index,
            timestamp,
        };
        
        // Store locally
        self.storage.insert_jtu(&jtu)?;
        
        // Return hash for network transmission
        Ok(hash)
    }
    
    // Send hash batch to Refinery (every 60 seconds)
    pub async fn send_hash_batch(
        &self,
        contract_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let hashes = self.storage.get_all_hashes(contract_id)?;
        
        // Send to NATS (hash-only message)
        let message = serde_json::json!({
            "contract_id": contract_id,
            "digger_id": self.id,
            "hashes": hashes,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        self.nats_client
            .publish("ore.batch", message.to_string().into())
            .await?;
        
        tracing::info!(
            "Sent {} JTU hashes for contract {}",
            hashes.len(),
            contract_id
        );
        
        Ok(())
    }
}
```

### Step 7: Implement Contract State Machine
```rust
// src/contract_state.rs
#[derive(Debug, Clone)]
pub enum ApprovalStatus {
    PendingStake,     // Contract created, no stake paid
    StakeApproved,    // Stake paid, can send to Refinery
    ExecutionComplete, // All milestones done
}

pub struct ContractState {
    pub contract_id: String,
    pub robo_stake_paid: f64,
    pub approval_status: ApprovalStatus,
    pub jtu_count: i64,
    pub last_hash_send: Option<i64>,  // Unix timestamp
}

impl ContractState {
    pub fn should_send_hashes(&self, interval_sec: u64) -> bool {
        if self.approval_status != ApprovalStatus::StakeApproved {
            return false;
        }
        
        match self.last_hash_send {
            None => true,  // Never sent before
            Some(last) => {
                let now = chrono::Utc::now().timestamp();
                (now - last) >= interval_sec as i64
            }
        }
    }
}
```

---

## 🧪 Testing Strategy

### Manual Testing (cURL)
```bash
# Start Digger
cargo run --bin digger

# Create contract
curl -X POST http://localhost:9000/contracts/create \
  -H "Content-Type: application/json" \
  -d '{
    "contract_id": "test-001",
    "milestones": 5,
    "tokens_per_milestone": 100,
    "power_watts": 2000
  }'

# Pay stake (unlocks hash sending)
curl -X POST http://localhost:9000/contracts/stake \
  -H "Content-Type: application/json" \
  -d '{
    "contract_id": "test-001",
    "amount": 0.05
  }'

# Execute contract
curl -X POST http://localhost:9000/contracts/execute \
  -H "Content-Type: application/json" \
  -d '{
    "contract_id": "test-001"
  }'

# Check status
curl http://localhost:9000/status

# Query stored JTUs
curl http://localhost:9000/jtus/test-001
```

### Integration Test
```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_jtu_storage_and_hash_transmission() {
    // 1. Start Digger HTTP server
    // 2. Create contract
    // 3. Pay stake
    // 4. Execute contract
    // 5. Wait for hash batch send
    // 6. Verify hashes in database
    // 7. Verify NATS message sent
}
```

---

## 📊 Success Metrics

### Phase 1 Complete When:
- [ ] Tauri completely removed (no frontend dependencies)
- [ ] HTTP server starts cleanly on port 9000
- [ ] Can create contract via cURL
- [ ] Can pay stake via cURL
- [ ] JTUs stored in SQLite (verified with SQLite browser)
- [ ] Hashes sent to NATS (verified with `nats sub ore.batch`)
- [ ] Storage grows at correct rate (cross product: tokens × watts)
- [ ] 30-day cleanup works (prunes old JTUs)

### Performance Targets:
- **Startup time**: <1 second
- **Hash generation**: >10,000 JTU/sec
- **Storage write**: >5,000 JTU/sec (SQLite batch insert)
- **Memory usage**: <100 MB for 1M stored JTUs
- **Disk usage**: ~450 bytes per JTU (uncompressed)

---

## 🚀 Execution Plan

### Day 1: Setup
- [x] Tag current state: `digger-with-tauri-backup`
- [ ] Checkout `feature/digger-refactor`
- [ ] Simplify Cargo.toml (remove Tauri deps)
- [ ] Create new main.rs (Axum server)
- [ ] Verify it compiles

### Day 2: Storage
- [ ] Create jtu_storage.rs
- [ ] Create jtu_hasher.rs
- [ ] Write unit tests for both
- [ ] Verify SQLite databases created correctly

### Day 3: Integration
- [ ] Update digger.rs to use storage
- [ ] Implement hash-only NATS messages
- [ ] Add contract state machine
- [ ] Test with cURL

### Day 4: Testing & Polish
- [ ] Write integration tests
- [ ] Test with real NATS + Refinery (if ready)
- [ ] Add tracing/logging
- [ ] Document API endpoints

### Day 5: Merge
- [ ] Code review
- [ ] Merge to `v0`
- [ ] Deploy to testnet
- [ ] Begin Phase 2 (Refinery update)

---

## 🎓 Key Decisions

### Why Strip Tauri Now?
1. **Frontend blocks backend** - Can't test proof chain without fighting UI
2. **Complexity burden** - Tauri adds 50+ dependencies we don't need yet
3. **Headless mode is hacky** - Better to have clean HTTP API
4. **Can re-add later** - Once backend is solid, wrap it with Tauri

### Why SQLite?
1. **Embedded** - No external database needed
2. **Fast** - 5,000+ inserts/sec with transactions
3. **Reliable** - ACID guarantees, won't lose data
4. **Portable** - Single file per contract, easy to backup

### Why Hash-Only Transmission?
1. **Bandwidth** - 93% reduction (9 MB/sec → 640 KB/sec)
2. **Separation** - Digger owns proofs, Refinery builds merkle trees
3. **Dispute** - Can provide full JTU on demand (merkle proof)
4. **Scalability** - Essential at 1M+ robots

---

## 📚 Next After Phase 1

Once Digger rewrite is complete:
1. **Phase 2**: Update Refinery to receive hash batches
2. **Phase 3**: Implement merkle tree in Refinery
3. **Phase 4**: Add real Falcon-1024 signatures
4. **Phase 5**: E2E test (Digger → Refinery → Mint)

---

**Ready to start?** Let's tag the current state and begin the rewrite! 🚀

*"Backend first. Frontend follows."* 💪
