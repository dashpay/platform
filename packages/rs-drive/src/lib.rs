//! Dash Drive
//!
//! Decentralized storage hosted by Dash masternodes
//!

#![cfg_attr(docsrs, feature(doc_cfg))]
// Coding conventions
#![forbid(unsafe_code)]
#![deny(missing_docs)]

#[cfg(feature = "server")]
extern crate core;

#[cfg(any(feature = "server", feature = "verify"))]
pub mod common;
/// Drive module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod drive;
/// Error module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod error;
/// Fee pools module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod fee_pools;
/// Query module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod query;

/// DPP module
#[cfg(feature = "server")]
pub use dpp;
/// GroveDB module
#[cfg(feature = "server")]
pub use grovedb;

#[cfg(feature = "server")]
pub use grovedb_path;

#[cfg(feature = "server")]
pub use grovedb_costs;

#[cfg(feature = "server")]
pub use grovedb_storage;
#[cfg(feature = "server")]
mod fee;
/// State transition action module
#[cfg(feature = "server")]
pub mod state_transition_action;
/// Test helpers
#[cfg(feature = "fixtures-and-mocks")]
pub mod tests;
