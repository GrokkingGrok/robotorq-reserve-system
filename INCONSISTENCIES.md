Below is an inconsistency and cleanup audit for the `commons` library. I grouped findings by category and gave concrete remediation suggestions.

Status update (2025-11-27):
- Robot gateway logging init now uses unified `InvariantError::Logging` and reports via `tracing::error!`.
- Robot gateway metrics implemented and wired; directory normalized to `services/robot_gateway`.
- Schema version constants centralized in `util/schema`; gauges exported via `MetricsHandler::register_schema_version_gauges` and covered by tests.
- Merkle warning fixed (removed unused `mut`); glob re-exports removed across util/types/services/lib.

**Structure & Module Paths**
- Path Attributes Everywhere: Extensive use of `#[path = ...]` in lib.rs. Adds maintenance overhead; conventional `mod` + directory naming is simpler. Suggest migrating gradually (start with `types` modules).
- Status: Completed for `types` and `util`; `lib.rs` now uses `pub mod util;` and standard modules.
- Mixed Directory Naming: `services/robot-gateway` uses a hyphen; Rust module naming convention prefers underscores (`robot_gateway`). Rename directory and adjust path attr.
- Status: Completed. Renamed to `services/robot_gateway`.
- Removed Legacy `merkel/` vs `merkle/`: Good, but verify no references remain (grep for `merkel/`). If any, clean them.
- Status: Verified; no code references remain.
- Upper/Lower Case: Editor context showed contract.rs earlier; actual folder is lowercase `contract/`. Ensure casing consistent for cross-platform clarity.
- Status: Contracts removed for now; marked done per request.

**Domain Modeling**
- RobotGateway Error Handling: Uses `RobotError::InvalidRobotName` as placeholder when robot ID unknown. Introduce a `GatewayError` with variants like `UnknownRobotId` and extend `InvariantError`.
- Status: Completed. Added `RobotGatewayError::UnknownRobotId`; integrated into `InvariantError` and gateway.
- Contract Validation: Uses `ConfigError::Invalid(String)` for domain-specific contract issues. Consider a dedicated `ContractError` enum for clarity.
- Status: Deferred; contracts removed (treated as done for now).
- TokenError::NegativeEnergy Unused: No code maps or sets energy via `f64`. Remove or implement the feature (e.g., energy normalization phase).
- Status: Pending. Token still has `NegativeEnergy(f64)`; decide remove or implement. Next: either remove the variant or add normalization in token construction with tests.
- TripleTorq Intended Use: Well separated from ore measurement, but ensure no confusion: add doc comment clarifying it represents monetary circulation only, not raw production.
- Status: Pending. Doc comment can be added to `types/triple_torq/mod.rs`.
- Schema Version: Present in `UnmappedOreBatch` and `ContractTerms`, absent in `Token`, `Robot`, `TripleTorq`, `Contract` (root). Decide: either every hashed struct carries `schema_version` or keep version only at batch/message envelope level. Consistency benefits indexing and migrations.
- Status: Completed. Added schema_version to `Token`, `Robot`, `TripleTorq`; central constants in `util/schema`.
- Hash Field Naming: All hashed structs use `hash` except Merkle constructs (using `root`, `leaf`). Fine, but consider adding `content_hash` or `struct_hash` naming if semantic separation needed later.
- Status: No change (acceptable as-is).

**Error Taxonomy**
- InvariantError Aggregation: Good pattern, but mixing configuration parsing (`ConfigError`) and business invariants. Consider splitting into `DomainInvariantError` and `ConfigError` separately to keep semantics clean.
- Status: Pending. Currently using unified `InvariantError`.
- Using Generic Invalid(String): Harder to test exhaustively; explicit variants (`EmptyParties`, `EmptyPartyName`, `ZeroMaxBatchTokens`) improve reliability.
- Status: Pending (contracts deferred).
- No Metrics Errors: Metrics handler silently ignores registration errors (`ok()`). Capture/propagate registration failures.
- Status: Addressed without breaking callers. Added `*_result` variants (e.g., `register_counter_result`, `register_gauge_result`, `register_schema_version_gauges_result`) that return `Result<_, prometheus::Error>`. Existing methods remain for convenience.

**API Consistency**
- Constructors Patterns:
  - Some return `Result<Self, InvariantError>` (Robot, Token, UnmappedOreBatch, TripleTorq::new, Contract::new).
  - Others return plain `Self` (`TripleTorq::from_smallest_units`, `from_components_unchecked`, `RobotGateway::single/new/from_config`). Ensure naming differentiates fallible vs infallible constructors (`*_unchecked` or `try_new`).
- Status: Completed. Standardized `Token::new(...)`; kept `*_unchecked` for infallible paths.
- Method Naming:
  - `capture_unmapped_batch_any` vs `capture_unmapped_batch_for`: Consider `capture_for(robot_id, ...)` and `capture_first(...)` or `capture_default(...)`.
- Status: Pending (current names retained). Next: rename for clarity and add deprecation notices.
- Using `expect` Removed: Good. Confirm no remaining internal `expect` that could panic (grep for `expect(`).
- Status: Verified; no critical `expect` in core paths.
- IDs: Thin newtypes with `.new()` only. For future validation or classification, you might want typed constructors (e.g., `RobotId::from_uuid(uuid)`).
- Status: Pending; IDs kept simple with schema constants added.

**Performance & Concurrency**
- Merkle Async: Uses `spawn_blocking`; fine for CPU-bound hashing. If batch sizes rise significantly, consider chunked parallel hashing (rayon or task partitioning).
- Status: No change; acceptable.
- TripleTorq Arithmetic: Works in `u128`; potential overflow if extremely large additions. If upper economic bounds exist, add checked operations or saturating alternatives.
- Status: No change; considered acceptable.

**Logging**
- `init_logging(false, ...)` still produces JSON (with ANSI). Misleading boolean semantic. Either:
  - Change `json: bool` branch to produce compact text when false, or
  - Rename parameter to `structured: bool`.
- Status: Completed. `init_logging` fixed to return `Result`; tests adjusted; semantics clarified in code paths.
- Two initializations executed in tests produce duplicate outputs (both pretty + json). Acceptable, but can isolate to avoid clutter.
- Status: Completed. Tests handle dispatcher already-set gracefully.

**Metrics**
- Separation Achieved: `MetricsHandler` + service-specific `MerkleMetrics`. Good.
- Naming Consistency: `merkle_leaves_total` vs economic invariants—may want `unmapped_batches_total`, `unmapped_tokens_total` later.
- Status: Pending; convenience methods added (`error counter`, `batch gauge`, `processing histogram`).
- No robot gateway metrics yet: Consider `robot_gateway_registered_robots` gauge and `robot_gateway_batches_captured_total`.
- Status: Completed. Added `RobotGatewayMetrics` with `registered_robots`, `batches_captured_total`, and `batches_rejected_total` and wired into gateway.
- Export Format Only: Provide `encode()` returning bytes for HTTP endpoint; text is fine but might want future OpenMetrics compatibility.
- Status: Pending. Next: add `export_bytes()` and consider OpenMetrics encoder.

**Documentation & Comments**
- Some comments state “operational status” for `Token::joule_count`; that’s vague. Clarify: “total joules of work mapped to this token.”
- Status: Pending.
- Missing module-level docs for `robot_gateway`.
- Status: Pending. Next: add module docs describing responsibilities and metrics.
- Economic invariants critical; consider a `doc` module or single `economics.rs` centralizing constants (re-export those rather than duplicating numeric literals like 3600 in multiple places).
- Status: Pending. Next: add `commons::economics` with constants and re-exports; add unit tests enforcing invariants.

**Consistency in Constants**
- TripleTorq constants: `TOKEN_TORQS_PER_ROBOTORQ` vs `JOULE_TORQS_PER_TOKEN_TORQ`. These are clear, but consider shortening to `TOKEN_PER_ROBOT` and `JOULE_PER_TOKEN` and deriving the third. Provide a public accessor rather than duplicating math in downstream code.
- Status: Pending.

**Potential Dead or Missing Items**
- `currency/` directory present (from `list_dir`) but not referenced. Inspect and either wire or remove.
- Status: Pending.
- No direct usage of `active_contract` logic in RobotGateway; currently gateway only routes `robot_id`. If `active_contract` matters, integrate validation that robot’s `active_contract` matches `contract_id`.
- Status: Pending.

**Security / Integrity**
- Hash JSON serialization of entire struct—including `hash` field currently zeroed—fine, but ensure deterministic field order (serde defaults OK).
- Status: Completed; hashing via `hash_struct` standardized.
- Public keys in `Party` are raw `String`: Validate format (base58/base64/hex) to avoid mismatch later.
- Status: Pending.

**Suggested Priority Fixes**
1. Introduce dedicated error enums (`ContractError`, `RobotGatewayError`) and remove generic `ConfigError::Invalid(String)` cases.
   - Status: Partially completed (GatewayError done; ContractError deferred).
 2. Restore semantic difference in logging initializer: `init_logging(json=true)` => JSON; `json=false` => text compact.
   - Status: Completed.
3. Unify schema_version placement: add to `Contract`, `Token`, `TripleTorq` or decide to confine versioning to batch/message layer and remove from `ContractTerms` if redundant.
   - Status: Completed (Token, Robot, TripleTorq, Config), contracts deferred.
4. Add gateway metrics module (`services/robot-gateway/metrics_robot_gateway.rs`).
   - Status: Completed. Implemented as `services/robot_gateway/metrics_robot_gateway.rs` and exposed via `robot_gateway_metrics` module.
5. Rename `services/robot-gateway` → `services/robot_gateway` for idiomatic module naming.
   - Status: Completed.
6. Replace placeholder unknown-robot error with explicit variant.
   - Status: Completed.
7. Remove unused `TokenError::NegativeEnergy` or implement energy mapping logic.
   - Status: Pending.
 8. Clean warnings in `merkle`: remove unused `mut` on `build_levels` parameter.
    - Status: Completed.

**Optional Enhancements**
- Provide a `Hashable` trait implemented by all hashed domain structs, centralizing hashing logic.
- Status: Pending.
- Add `serde(tag = "type")` polymorphism if you foresee variant payload handling for tokens/ingots/certificates.
- Status: Pending.

Let me know which subset you want implemented first (e.g., error taxonomy cleanup, logging boolean semantic fix, schema version strategy), and I’ll apply targeted patches.