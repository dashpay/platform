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

/// Drive module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod drive;
/// Error module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod error;
/// Query module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod query;

/// DPP module
#[cfg(feature = "server")]
pub use dpp;
/// GroveDB module
#[cfg(any(feature = "server", feature = "verify"))]
pub use grovedb;

#[cfg(feature = "server")]
pub use grovedb_path;

#[cfg(feature = "server")]
pub use grovedb_costs;

#[cfg(feature = "server")]
pub use grovedb_storage;
#[cfg(feature = "server")]
mod fees;
/// State transition action module
#[cfg(feature = "server")]
pub mod state_transition_action;
/// Util module
#[cfg(any(feature = "server", feature = "verify", feature = "fixtures-and-mocks"))]
pub mod util;
#[cfg(feature = "server")]
mod open;
/// Drive Cache
#[cfg(feature = "server")]
pub mod cache;
#[cfg(any(feature = "server", feature = "verify"))]
pub mod config;
#[cfg(feature = "server")]
mod prove;
/// Contains a set of useful grovedb proof verification functions
#[cfg(feature = "verify")]
pub mod verify;
