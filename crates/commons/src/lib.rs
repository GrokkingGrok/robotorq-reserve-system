//! Commons crate for RoboTorq: shared types, utilities, and service interfaces.
//!
//! This crate defines the core data types, error handling, configuration, metrics,
//! and service abstractions used across the RoboTorq Reserve System. It preserves
//! the economic invariants of the unit hierarchy:
//! - 1 TokenTorqIngot = 3,600 JouleTorqOre units
//! - 1 RoboTorq Certificate = 1,000 ingots = 3,600,000 Ore units
//!
//! Use this crate to build service implementations in downstream crates while
//! keeping business logic decoupled from HTTP and transport layers.
#![warn(missing_docs)]
pub mod types;
pub mod util;
pub mod services;

pub use types::Robot;
pub use types::Token;
pub use types::UnmappedOreBatch;
pub use types::ids::{RobotId, TokenId, UnmappedOreBatchId, TripleTorqId};
pub use types::triple_torq::TripleTorq;


