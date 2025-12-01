//! Tests for persistence configuration structures.
//!
//! Exercises Postgres SSL mode variants and SQLite journal/synchronous settings.
use commons::util::config::persistance::{PostgresConfig, PostgresSslMode, SqliteConfig, SqliteJournalMode, SqliteSynchronousMode};

#[test]
fn postgres_ssl_mode_variants() {
    /// Ensures all `PostgresSslMode` enum variants are constructible and matchable.
    let mut pg = PostgresConfig::default();
    pg.ssl_mode = PostgresSslMode::Disable;
    assert!(matches!(pg.ssl_mode, PostgresSslMode::Disable));
    pg.ssl_mode = PostgresSslMode::Require;
    assert!(matches!(pg.ssl_mode, PostgresSslMode::Require));
    pg.ssl_mode = PostgresSslMode::Prefer;
    assert!(matches!(pg.ssl_mode, PostgresSslMode::Prefer));
    pg.ssl_mode = PostgresSslMode::Allow;
    assert!(matches!(pg.ssl_mode, PostgresSslMode::Allow));
}

#[test]
fn sqlite_mode_variants() {
    /// Ensures SQLite journal and synchronous modes accept and reflect variant assignments.
    let mut db = SqliteConfig::default();
    db.journal_mode = SqliteJournalMode::Wal;
    assert!(matches!(db.journal_mode, SqliteJournalMode::Wal));
    db.journal_mode = SqliteJournalMode::Delete;
    assert!(matches!(db.journal_mode, SqliteJournalMode::Delete));
    db.synchronous = SqliteSynchronousMode::Full;
    assert!(matches!(db.synchronous, SqliteSynchronousMode::Full));
    db.synchronous = SqliteSynchronousMode::Normal;
    assert!(matches!(db.synchronous, SqliteSynchronousMode::Normal));
}
