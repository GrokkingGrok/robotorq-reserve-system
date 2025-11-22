// JTU Storage Manager - SQLite database for local proof ownership
// Phase 1: Digger Rewrite - Day 2

use rusqlite::{Connection, params, Result as SqliteResult};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

/// JouleTorqUnit – atomic record for a single token's work.
///
/// A JTU represents the cross product of energy consumed while that
/// token was being processed (power draw × elapsed seconds → joules)
/// plus its stake cost at time of execution. There is one JTU per
/// token index, but the joules field can span the entire execution
/// interval for that token (it is not a trivial 1:1 sample; it is the
/// accumulated consumption for that token's lifecycle slice).
///
/// Invariants:
/// * `joules_consumed` = total joules for THIS token's processing window
/// * `robo_stake_paid` = stake cost allocated to THIS token
/// * `hash` commits to all other fields (stable canonical encoding)
/// * No post‑hoc mutation; disputes rely on immutability
///
/// Purpose:
/// * Local retention (≈30 days) for dispute proofs / rehash verification
/// * Source material for downstream ingot assembly (transmit only hashes)
///
/// Future (phase‑4 crypto): replace `signature` with Falcon‑1024 and
/// populate `digger_id` from registration before persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JouleTorqUnit {
    pub hash: String,              // SHA256 hash (32 bytes, 64 hex chars)
    pub signature: Vec<u8>,        // TODO(phase-4-crypto): Replace with Falcon-1024!
                                   // - Currently 64-byte placeholder
                                   // - Must be 1280 bytes for Falcon-1024
                                   // - Must be signed by robot's private key
                                   // - Proves robot identity and work authenticity
    pub digger_id: String,         // TODO(phase-4-crypto): Wire this up!
                                   // - Currently unused (not passed from API!)
                                   // - Must come from robot registry
                                   // - Critical for multi-robot contracts
                                   // - Enables accountability and dispute resolution
    pub contract_id: String,       // Which contract
    pub token_index: i64,          // Token number within milestone
    pub milestone_index: i64,      // Milestone number
    pub timestamp: i64,            // Unix timestamp (seconds)
    pub joules_consumed: f64,      // Energy for THIS token (cross product!)
    pub robo_stake_paid: f64,      // Cost of THIS token
}

/// JtuStorageManager – per‑contract SQLite shard for JTU persistence.
///
/// Design goals:
/// * Fast lookups for dispute resolution (hash → full JTU)
/// * Parallelism: separate DB file removes cross‑contract contention
/// * Lifecycle: whole contract data can be pruned / archived atomically
/// * Simplicity: plain file copy for backup, easy deletion for expiry
///
/// Not a global DB on purpose; size & isolation keep downstream hashing
/// predictable and simplify cleanup.
pub struct JtuStorageManager {
    storage_path: PathBuf,
}

impl JtuStorageManager {
    /// Construct a manager rooted at `storage_path`.
    /// Ensures directory exists. Does *not* open any DB until needed.
    pub fn new(storage_path: PathBuf) -> SqliteResult<Self> {
        std::fs::create_dir_all(&storage_path)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        
        Ok(Self { storage_path })
    }
    
    /// Get (and lazily initialize) a connection for `contract_id`.
    /// Creates schema & indexes on first use. Returns a fresh `Connection`.
    /// Caller should keep the connection short‑lived (no pooling required
    /// at current scale; SQLite handles file locking internally).
    pub fn get_connection(&self, contract_id: &str) -> SqliteResult<Connection> {
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
                timestamp INTEGER,
                joules_consumed REAL,
                robo_stake_paid REAL
            )",
            [],
        )?;
        
        // Create indexes for common queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON jtus(timestamp)",
            [],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_milestone ON jtus(milestone_index)",
            [],
        )?;
        
        Ok(conn)
    }
    
    /// Insert a single JTU (fails if `hash` already exists).
    /// Local proof retained up to retention window (see `prune_old`).
    pub fn insert_jtu(&self, jtu: &JouleTorqUnit) -> SqliteResult<()> {
        let conn = self.get_connection(&jtu.contract_id)?;
        
        conn.execute(
            "INSERT INTO jtus VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                jtu.hash,
                jtu.signature,
                jtu.digger_id,
                jtu.contract_id,
                jtu.token_index,
                jtu.milestone_index,
                jtu.timestamp,
                jtu.joules_consumed,
                jtu.robo_stake_paid,
            ],
        )?;
        
        Ok(())
    }
    
    /// Insert many JTUs atomically inside one transaction.
    /// Approx throughput: ~5k inserts/sec typical dev hardware.
    pub fn insert_batch(&self, jtus: &[JouleTorqUnit]) -> SqliteResult<()> {
        if jtus.is_empty() {
            return Ok(());
        }
        
        let contract_id = &jtus[0].contract_id;
        let conn = self.get_connection(contract_id)?;
        
        let tx = conn.unchecked_transaction()?;
        
        for jtu in jtus {
            tx.execute(
                "INSERT INTO jtus VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    jtu.hash,
                    jtu.signature,
                    jtu.digger_id,
                    jtu.contract_id,
                    jtu.token_index,
                    jtu.milestone_index,
                    jtu.timestamp,
                    jtu.joules_consumed,
                    jtu.robo_stake_paid,
                ],
            )?;
        }
        
        tx.commit()?;
        Ok(())
    }
    
    /// Return all JTU hashes for `contract_id`, ordered by timestamp.
    /// Only lightweight hash list (used for refinery transmission).
    pub fn get_all_hashes(&self, contract_id: &str) -> SqliteResult<Vec<String>> {
        let conn = self.get_connection(contract_id)?;
        
        let mut stmt = conn.prepare("SELECT hash FROM jtus ORDER BY timestamp")?;
        let hashes = stmt.query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        
        Ok(hashes)
    }
    
    /// Count total JTUs stored for `contract_id`.
    pub fn get_jtu_count(&self, contract_id: &str) -> SqliteResult<i64> {
        let conn = self.get_connection(contract_id)?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM jtus", [], |row| row.get(0))?;
        Ok(count)
    }
    
    /// Fetch full JTU for `hash`. Returns `Ok(None)` if not present.
    /// Used to satisfy dispute proofs beyond bare hash transmission.
    pub fn get_jtu_by_hash(&self, contract_id: &str, hash: &str) -> SqliteResult<Option<JouleTorqUnit>> {
        let conn = self.get_connection(contract_id)?;
        
        let mut stmt = conn.prepare(
            "SELECT hash, signature, digger_id, contract_id, token_index, 
                    milestone_index, timestamp, joules_consumed, robo_stake_paid 
             FROM jtus WHERE hash = ?1"
        )?;
        
        let result = stmt.query_row([hash], |row| {
            Ok(JouleTorqUnit {
                hash: row.get(0)?,
                signature: row.get(1)?,
                digger_id: row.get(2)?,
                contract_id: row.get(3)?,
                token_index: row.get(4)?,
                milestone_index: row.get(5)?,
                timestamp: row.get(6)?,
                joules_consumed: row.get(7)?,
                robo_stake_paid: row.get(8)?,
            })
        });
        
        match result {
            Ok(jtu) => Ok(Some(jtu)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
    
    /// Delete JTUs older than `days` for given contract. Returns count removed.
    /// Recommended default: 30 days (post‑dispute window).
    pub fn prune_old(&self, contract_id: &str, days: i64) -> SqliteResult<usize> {
        let conn = self.get_connection(contract_id)?;
        let cutoff = chrono::Utc::now().timestamp() - (days * 86400);
        
        conn.execute(
            "DELETE FROM jtus WHERE timestamp < ?1",
            params![cutoff],
        )
    }
    
    /// Aggregate simple stats (count, total joules, total stake) for contract.
    pub fn get_stats(&self, contract_id: &str) -> SqliteResult<StorageStats> {
        let conn = self.get_connection(contract_id)?;
        
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM jtus", [], |row| row.get(0))?;
        
        let total_joules: f64 = conn.query_row(
            "SELECT COALESCE(SUM(joules_consumed), 0.0) FROM jtus", 
            [], 
            |row| row.get(0)
        )?;
        
        let total_stake: f64 = conn.query_row(
            "SELECT COALESCE(SUM(robo_stake_paid), 0.0) FROM jtus", 
            [], 
            |row| row.get(0)
        )?;
        
        Ok(StorageStats {
            jtu_count: count,
            total_joules,
            total_robo_stake: total_stake,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub jtu_count: i64,
    pub total_joules: f64,
    pub total_robo_stake: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_jtu(index: i64) -> JouleTorqUnit {
        JouleTorqUnit {
            hash: format!("hash-{:04}", index),
            signature: vec![0u8; 64],
            digger_id: "test-digger".to_string(),
            contract_id: "test-contract".to_string(),
            token_index: index,
            milestone_index: 0,
            timestamp: 1731734400 + index,
            joules_consumed: 4.17,
            robo_stake_paid: 0.0000139,
        }
    }

    #[test]
    fn test_create_storage_manager() {
        let temp_dir = TempDir::new().unwrap();
        let _storage = JtuStorageManager::new(temp_dir.path().to_path_buf()).unwrap();
        
        // Should create directory
        assert!(temp_dir.path().exists());
    }

    #[test]
    fn test_insert_and_retrieve() {
        let temp_dir = TempDir::new().unwrap();
        let storage = JtuStorageManager::new(temp_dir.path().to_path_buf()).unwrap();
        
        let jtu = create_test_jtu(1);
        storage.insert_jtu(&jtu).unwrap();
        
        let retrieved = storage.get_jtu_by_hash("test-contract", "hash-0001").unwrap();
        assert!(retrieved.is_some());
        
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.hash, "hash-0001");
        assert_eq!(retrieved.token_index, 1);
    }

    #[test]
    fn test_batch_insert() {
        let temp_dir = TempDir::new().unwrap();
        let storage = JtuStorageManager::new(temp_dir.path().to_path_buf()).unwrap();
        
        let jtus: Vec<JouleTorqUnit> = (0..100).map(create_test_jtu).collect();
        storage.insert_batch(&jtus).unwrap();
        
        let count = storage.get_jtu_count("test-contract").unwrap();
        assert_eq!(count, 100);
    }

    #[test]
    fn test_get_all_hashes() {
        let temp_dir = TempDir::new().unwrap();
        let storage = JtuStorageManager::new(temp_dir.path().to_path_buf()).unwrap();
        
        let jtus: Vec<JouleTorqUnit> = (0..10).map(create_test_jtu).collect();
        storage.insert_batch(&jtus).unwrap();
        
        let hashes = storage.get_all_hashes("test-contract").unwrap();
        assert_eq!(hashes.len(), 10);
        assert_eq!(hashes[0], "hash-0000");
        assert_eq!(hashes[9], "hash-0009");
    }

    #[test]
    fn test_get_stats() {
        let temp_dir = TempDir::new().unwrap();
        let storage = JtuStorageManager::new(temp_dir.path().to_path_buf()).unwrap();
        
        let jtus: Vec<JouleTorqUnit> = (0..100).map(create_test_jtu).collect();
        storage.insert_batch(&jtus).unwrap();
        
        let stats = storage.get_stats("test-contract").unwrap();
        assert_eq!(stats.jtu_count, 100);
        assert!((stats.total_joules - 417.0).abs() < 0.01); // 100 * 4.17
        assert!((stats.total_robo_stake - 0.00139).abs() < 0.0001); // 100 * 0.0000139
    }

    #[test]
    fn test_prune_old() {
        let temp_dir = TempDir::new().unwrap();
        let storage = JtuStorageManager::new(temp_dir.path().to_path_buf()).unwrap();
        
        // Insert JTUs with different timestamps
        let old_jtu = JouleTorqUnit {
            timestamp: chrono::Utc::now().timestamp() - (40 * 86400), // 40 days ago
            ..create_test_jtu(1)
        };
        
        let new_jtu = JouleTorqUnit {
            timestamp: chrono::Utc::now().timestamp(), // Now
            ..create_test_jtu(2)
        };
        
        storage.insert_jtu(&old_jtu).unwrap();
        storage.insert_jtu(&new_jtu).unwrap();
        
        // Prune anything older than 30 days
        let deleted = storage.prune_old("test-contract", 30).unwrap();
        assert_eq!(deleted, 1);
        
        let count = storage.get_jtu_count("test-contract").unwrap();
        assert_eq!(count, 1); // Only new JTU remains
    }
}
