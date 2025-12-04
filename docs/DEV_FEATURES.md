**Dev Feature**

This repository includes a small helper crate `crates/dev-tools` which exposes a feature named
`dev`. The purpose is to provide a single, opt-in feature that member crates can depend on
to enable development-only helpers (tests, local-only integrations, or helper APIs such as
`test_helpers` used in `crates/commons`).

How it works

- Add an optional dependency on `dev-tools` in a member crate's `Cargo.toml`.
- In that member crate's `[features]` section, add a `dev` feature that references
  `dev-tools/dev` and any local test-helper features you want to enable (for example
  `test_helpers` and `persistence`).

Example (already applied for `crates/commons/Cargo.toml`):

```
[dependencies]
dev-tools = { path = "../dev-tools", optional = true }

[features]
dev = ["test_helpers", "persistence", "dev-tools/dev"]
```

Usage

- To run tests that require the dev helpers for `commons`:

```
cargo test -p commons --features "dev" -- --nocapture
```

Notes

- This `dev` feature is available per-crate. To enable the same developer UX for other
  crates, add the optional `dev-tools` dependency and a similar `dev` feature mapping in
  each crate's `Cargo.toml`.
- If you want a single-command workspace invocation, you can add a `.cargo/config.toml`
  alias or add a tiny meta crate that coordinates feature toggles across members.

If you'd like, I can add the `dev` mapping to other member crates (e.g., the simulator),
or create a small workspace-level cargo alias that runs commonly used dev-test commands.
