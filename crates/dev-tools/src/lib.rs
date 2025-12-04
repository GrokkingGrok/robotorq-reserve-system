//! `dev-tools`
//!
//! Tiny crate to provide a workspace-level `dev` feature flag.
//!
//! This crate intentionally contains no runtime code. It exists so member crates
//! can add an optional dependency on `dev-tools` and expose a `dev` feature that
//! can be enabled with `--features dev` when testing or running development-only
//! integrations. See `docs/DEV_FEATURES.md` for usage notes.

#[cfg(feature = "dev")]
#[doc(hidden)]
pub fn _dev_feature_marker() {
    // no-op marker function; only exists so the feature has at least one usage
}

#[cfg(not(feature = "dev"))]
#[doc(hidden)]
pub fn _dev_feature_marker() {}
