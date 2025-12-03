# Rust Docstring Guide (rustdoc + VS Code)

This guide shows how to write clear, attractive, and robust Rust documentation comments that render beautifully in VS Code and rustdoc, and don’t break your build with doctests.

## Core Patterns
- /// Line docs: Use on items (functions, structs, enums, traits, methods).
- //! Module/file docs: Use at the top of a file to document the entire module or crate.
- Fenced blocks: Prefer code fences for examples.
  - Use ```rust for syntax highlighting.
  - Use modifiers: `rust,ignore` (do not compile), `rust,no_run` (compile-only), `rust,should_panic`.
- Concise first line: Start with a one-sentence summary; rustdoc uses this in overviews.
- Short paragraphs: Keep lines ~80-100 chars for readability in editors.

## Safe Example Snippets
Choose the right fence modifier for examples:
- `rust,ignore`: Best default when examples reference crate-local paths or optional features.
- `rust,no_run`: Compiles, but won’t run; use when paths resolve and you want compiler validation.
- `rust`: Full doctest; only use when you know imports and environment compile and run.

Example:
```rust,ignore
/// Export current metrics in Prometheus text format.
///
/// Example usage:
/// ```rust,ignore
/// use commons::util::metrics::metrics::MetricsHandler;
/// use commons::robot_gateway_metrics::RobotGatewayMetrics;
/// use commons::RobotId;
///
/// let handler = MetricsHandler::new();
/// handler.register_schema_version_gauges("commons");
/// let gw_metrics = RobotGatewayMetrics::new(&handler, "robot_gateway");
/// let gw = commons::robot_gateway::RobotGateway::single(RobotId::new()).with_metrics(gw_metrics);
/// let text = handler.export_text();
/// ```
pub fn export_text() { /* ... */ }
```

## Module-Level Docs (`//!`)
Place at the top of the file to describe the module purpose, key types, and usage.

Example (`services/robot_gateway/robot_gateway.rs`):
```rust
//! Robot gateway service
//!
//! - Registers robots and coordinates unmapped batch capture.
//! - Emits metrics via `RobotGatewayMetrics` when attached.
//!
//! Quick start:
//! ```rust,ignore
//! use commons::{RobotId};
//! use commons::robot_gateway::RobotGateway;
//! let gw = RobotGateway::single(RobotId::new());
//! ```
```

## Structure and Formatting Tips
- Headings: Use `#`, `##`, `###` in doc comments for sections.
- Lists: Use `-` or `*` for bullets; keep items short.
- Links: Use Markdown links to other items or docs.
  - Intra-doc links: `[RobotGateway](crate::robot_gateway::RobotGateway)`.
- Tables: Keep minimal; prefer lists where possible for readability.

## Error and Invariant Docs
- State failure modes explicitly and reference error enums.
- Example:
```rust
/// Captures a batch for `robot_id`.
///
/// Errors
/// - `RobotGatewayError::UnknownRobotId`: when the robot is not registered.
/// - `BatchError::Validation(...)`: if batch validation fails.
///
/// Note: The legacy `InvariantError` type has been removed. Prefer domain-specific
/// error enums (for example `TokenError`, `TripleTorqError`, `RobotError`,
/// `BatchError`) and map them to `ServiceError` at public service boundaries.
```

## Metrics Docs
- Name metrics consistently: `{prefix}_{metric}`.
- Document units: seconds, bytes, counts.
- Provide a one-line purpose and optional sample.

## Testing-Friendly Docs
- Prefer `rust,ignore` unless you maintain imports that make examples compile.
- For compile-only validation, switch to `rust,no_run` and add minimal imports.
- Avoid heavy setup in examples; keep them focused.

## Style Checklist
- Clear summary line
- Purpose and context
- Example (fenced block)
- Failure modes (Errors section)
- Links to related modules/types
- Avoid panics and `expect` in examples

## Quick Template
```rust
/// One-line summary
///
/// Longer explanation (1-2 short paragraphs).
///
/// Example
/// ```rust,ignore
/// // Minimal example
/// ```
///
/// Errors
/// - Prefer domain-specific errors, e.g. `DomainError::X`: reason
pub fn your_api(...) { /* ... */ }
```

## References
- Rust doc comments: https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html
- Intra-doc links: https://doc.rust-lang.org/rustdoc/linking-to-items-by-name.html
- Doctests: https://doc.rust-lang.org/rustdoc/documentation-tests.html
