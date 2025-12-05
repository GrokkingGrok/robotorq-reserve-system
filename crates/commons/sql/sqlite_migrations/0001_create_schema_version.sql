-- SQLite migration: create schema_version and set expected version to 1
BEGIN TRANSACTION;
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER NOT NULL
);
DELETE FROM schema_version;
INSERT INTO schema_version(version) VALUES (1);
COMMIT;