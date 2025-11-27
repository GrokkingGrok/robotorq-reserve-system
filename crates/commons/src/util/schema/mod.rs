/// Schema versioning constants for data structures.
/// Increment these when the structure changes in a backward-incompatible way.
/// Use semantic versioning: major for breaking changes, minor for additions, patch for fixes.

/// Current schema version for UnmappedOreBatch.
pub const UNMAPPED_ORE_BATCH_SCHEMA_VERSION: u32 = 1;

/// Current schema version for Token.
pub const TOKEN_SCHEMA_VERSION: u32 = 1;

/// Current schema version for Robot.
pub const ROBOT_SCHEMA_VERSION: u32 = 1;

/// Current schema version for TripleTorq.
pub const TRIPLE_TORQ_SCHEMA_VERSION: u32 = 1;

/// Current schema version for MerkleTree.
pub const MERKLE_TREE_SCHEMA_VERSION: u32 = 1;

/// Current schema version for RoboTorq runtime configuration.
pub const ROBOTORQ_CONFIG_SCHEMA_VERSION: u32 = 1;

// ID newtype schema versions (kept as constants; IDs themselves remain simple UUID wrappers).
pub const ROBOT_ID_SCHEMA_VERSION: u32 = 1;
pub const TOKEN_ID_SCHEMA_VERSION: u32 = 1;
pub const UNMAPPED_ORE_BATCH_ID_SCHEMA_VERSION: u32 = 1;
pub const TRIPLE_TORQ_ID_SCHEMA_VERSION: u32 = 1;
pub const CONTRACT_ID_SCHEMA_VERSION: u32 = 1;
pub const PARTY_ID_SCHEMA_VERSION: u32 = 1;

/// Helper to get the current version for a given type (for future migration logic).
pub fn current_schema_version(type_name: &str) -> Option<u32> {
    match type_name {
        "UnmappedOreBatch" => Some(UNMAPPED_ORE_BATCH_SCHEMA_VERSION),
        "Token" => Some(TOKEN_SCHEMA_VERSION),
        "Robot" => Some(ROBOT_SCHEMA_VERSION),
        "TripleTorq" => Some(TRIPLE_TORQ_SCHEMA_VERSION),
        "MerkleTree" => Some(MERKLE_TREE_SCHEMA_VERSION),
        "RoboTorqConfig" => Some(ROBOTORQ_CONFIG_SCHEMA_VERSION),
        // IDs
        "RobotId" => Some(ROBOT_ID_SCHEMA_VERSION),
        "TokenId" => Some(TOKEN_ID_SCHEMA_VERSION),
        "UnmappedOreBatchId" => Some(UNMAPPED_ORE_BATCH_ID_SCHEMA_VERSION),
        "TripleTorqId" => Some(TRIPLE_TORQ_ID_SCHEMA_VERSION),
        "ContractId" => Some(CONTRACT_ID_SCHEMA_VERSION),
        "PartyId" => Some(PARTY_ID_SCHEMA_VERSION),
        _ => None,
    }
}

/// Return all known schema versions as (type_name, version) pairs.
pub fn all_schema_versions() -> Vec<(&'static str, u32)> {
    vec![
        ("UnmappedOreBatch", UNMAPPED_ORE_BATCH_SCHEMA_VERSION),
        ("Token", TOKEN_SCHEMA_VERSION),
        ("Robot", ROBOT_SCHEMA_VERSION),
        ("TripleTorq", TRIPLE_TORQ_SCHEMA_VERSION),
        ("MerkleTree", MERKLE_TREE_SCHEMA_VERSION),
        ("RoboTorqConfig", ROBOTORQ_CONFIG_SCHEMA_VERSION),
        // IDs
        ("RobotId", ROBOT_ID_SCHEMA_VERSION),
        ("TokenId", TOKEN_ID_SCHEMA_VERSION),
        ("UnmappedOreBatchId", UNMAPPED_ORE_BATCH_ID_SCHEMA_VERSION),
        ("TripleTorqId", TRIPLE_TORQ_ID_SCHEMA_VERSION),
        ("ContractId", CONTRACT_ID_SCHEMA_VERSION),
        ("PartyId", PARTY_ID_SCHEMA_VERSION),
    ]
}