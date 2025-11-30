//! Ore-level types representing raw JouleTorqOre batches.
//!
//! These are the lowest-level work proof artifacts produced by robots
//! before aggregation into higher economic units.
pub mod unmapped_ore_batch;

pub use unmapped_ore_batch::UnmappedOreBatch;