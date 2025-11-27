Below is an inconsistency and cleanup audit for the `commons` library. I grouped findings by category and gave concrete remediation suggestions.

**Structure & Module Paths**
- Path Attributes Everywhere: Extensive use of `#[path = ...]` in lib.rs. Adds maintenance overhead; conventional `mod` + directory naming is simpler. Suggest migrating gradually (start with `types` modules).
- Mixed Directory Naming: `services/robot-gateway` uses a hyphen; Rust module naming convention prefers underscores (`robot_gateway`). Rename directory and adjust path attr.
- Removed Legacy `merkel/` vs `merkle/`: Good, but verify no references remain (grep for `merkel/`). If any, clean them.
- Upper/Lower Case: Editor context showed contract.rs earlier; actual folder is lowercase `contract/`. Ensure casing consistent for cross-platform clarity.

**Domain Modeling**
- RobotGateway Error Handling: Uses `RobotError::InvalidRobotName` as placeholder when robot ID unknown. Introduce a `GatewayError` with variants like `UnknownRobotId` and extend `InvariantError`.
- Contract Validation: Uses `ConfigError::Invalid(String)` for domain-specific contract issues. Consider a dedicated `ContractError` enum for clarity.
- TokenError::NegativeEnergy Unused: No code maps or sets energy via `f64`. Remove or implement the feature (e.g., energy normalization phase).
- TripleTorq Intended Use: Well separated from ore measurement, but ensure no confusion: add doc comment clarifying it represents monetary circulation only, not raw production.
- Schema Version: Present in `UnmappedOreBatch` and `ContractTerms`, absent in `Token`, `Robot`, `TripleTorq`, `Contract` (root). Decide: either every hashed struct carries `schema_version` or keep version only at batch/message envelope level. Consistency benefits indexing and migrations.
- Hash Field Naming: All hashed structs use `hash` except Merkle constructs (using `root`, `leaf`). Fine, but consider adding `content_hash` or `struct_hash` naming if semantic separation needed later.

**Error Taxonomy**
- InvariantError Aggregation: Good pattern, but mixing configuration parsing (`ConfigError`) and business invariants. Consider splitting into `DomainInvariantError` and `ConfigError` separately to keep semantics clean.
- Using Generic Invalid(String): Harder to test exhaustively; explicit variants (`EmptyParties`, `EmptyPartyName`, `ZeroMaxBatchTokens`) improve reliability.
- No Metrics Errors: Metrics handler silently ignores registration errors (`ok()`). Capture/propagate registration failures.

**API Consistency**
- Constructors Patterns:
  - Some return `Result<Self, InvariantError>` (Robot, Token, UnmappedOreBatch, TripleTorq::new, Contract::new).
  - Others return plain `Self` (`TripleTorq::from_smallest_units`, `from_components_unchecked`, `RobotGateway::single/new/from_config`). Ensure naming differentiates fallible vs infallible constructors (`*_unchecked` or `try_new`).
- Method Naming:
  - `capture_unmapped_batch_any` vs `capture_unmapped_batch_for`: Consider `capture_for(robot_id, ...)` and `capture_first(...)` or `capture_default(...)`.
- Using `expect` Removed: Good. Confirm no remaining internal `expect` that could panic (grep for `expect(`).
- IDs: Thin newtypes with `.new()` only. For future validation or classification, you might want typed constructors (e.g., `RobotId::from_uuid(uuid)`).

**Performance & Concurrency**
- Merkle Async: Uses `spawn_blocking`; fine for CPU-bound hashing. If batch sizes rise significantly, consider chunked parallel hashing (rayon or task partitioning).
- TripleTorq Arithmetic: Works in `u128`; potential overflow if extremely large additions. If upper economic bounds exist, add checked operations or saturating alternatives.

**Logging**
- `init_logging(false, ...)` still produces JSON (with ANSI). Misleading boolean semantic. Either:
  - Change `json: bool` branch to produce compact text when false, or
  - Rename parameter to `structured: bool`.
- Two initializations executed in tests produce duplicate outputs (both pretty + json). Acceptable, but can isolate to avoid clutter.

**Metrics**
- Separation Achieved: `MetricsHandler` + service-specific `MerkleMetrics`. Good.
- Naming Consistency: `merkle_leaves_total` vs economic invariants—may want `unmapped_batches_total`, `unmapped_tokens_total` later.
- No robot gateway metrics yet: Consider `robot_gateway_registered_robots` gauge and `robot_gateway_batches_captured_total`.
- Export Format Only: Provide `encode()` returning bytes for HTTP endpoint; text is fine but might want future OpenMetrics compatibility.

**Documentation & Comments**
- Some comments state “operational status” for `Token::joule_count`; that’s vague. Clarify: “total joules of work mapped to this token.”
- Missing module-level docs for `robot_gateway`.
- Economic invariants critical; consider a `doc` module or single `economics.rs` centralizing constants (re-export those rather than duplicating numeric literals like 3600 in multiple places).

**Consistency in Constants**
- TripleTorq constants: `TOKEN_TORQS_PER_ROBOTORQ` vs `JOULE_TORQS_PER_TOKEN_TORQ`. These are clear, but consider shortening to `TOKEN_PER_ROBOT` and `JOULE_PER_TOKEN` and deriving the third. Provide a public accessor rather than duplicating math in downstream code.

**Potential Dead or Missing Items**
- `currency/` directory present (from `list_dir`) but not referenced. Inspect and either wire or remove.
- No direct usage of `active_contract` logic in RobotGateway; currently gateway only routes `robot_id`. If `active_contract` matters, integrate validation that robot’s `active_contract` matches `contract_id`.

**Security / Integrity**
- Hash JSON serialization of entire struct—including `hash` field currently zeroed—fine, but ensure deterministic field order (serde defaults OK).
- Public keys in `Party` are raw `String`: Validate format (base58/base64/hex) to avoid mismatch later.

**Suggested Priority Fixes**
1. Introduce dedicated error enums (`ContractError`, `RobotGatewayError`) and remove generic `ConfigError::Invalid(String)` cases.
2. Restore semantic difference in logging initializer: `init_logging(json=true)` => JSON; `json=false` => text compact.
3. Unify schema_version placement: add to `Contract`, `Token`, `TripleTorq` or decide to confine versioning to batch/message layer and remove from `ContractTerms` if redundant.
4. Add gateway metrics module (`services/robot-gateway/metrics_robot_gateway.rs`).
5. Rename `services/robot-gateway` → `services/robot_gateway` for idiomatic module naming.
6. Replace placeholder unknown-robot error with explicit variant.
7. Remove unused `TokenError::NegativeEnergy` or implement energy mapping logic.

**Optional Enhancements**
- Provide a `Hashable` trait implemented by all hashed domain structs, centralizing hashing logic.
- Add `serde(tag = "type")` polymorphism if you foresee variant payload handling for tokens/ingots/certificates.

Let me know which subset you want implemented first (e.g., error taxonomy cleanup, logging boolean semantic fix, schema version strategy), and I’ll apply targeted patches.