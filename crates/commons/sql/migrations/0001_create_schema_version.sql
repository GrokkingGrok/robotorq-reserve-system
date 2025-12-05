-- Create a simple schema_version table and set initial version to 1
CREATE TABLE IF NOT EXISTS schema_version (
    version INT NOT NULL
);

-- Ensure a single row with the expected version exists
DELETE FROM schema_version;
INSERT INTO schema_version(version) VALUES (1);
