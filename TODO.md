# Todo

1. Implement the sandbox runtime integration so the new execution environment mirrors production hooks while remaining safely isolated.
2. Refresh shared crate dependencies (especially `tokio`, `prost`, and `serde`) to bring in the latest fixes and verify the Cargo lockstep consistency.
3. Expand automated tests covering the sandbox workflows and the new dependency behavior in `robot-gateway`, including both unit and integration checks.
