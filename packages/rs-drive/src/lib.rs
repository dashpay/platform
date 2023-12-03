//! Dash Drive
//!
//! Decentralized storage hosted by Dash masternodes
//!

#![cfg_attr(docsrs, feature(doc_cfg))]
// Coding conventions
#![forbid(unsafe_code)]
#![deny(missing_docs)]

#[cfg(feature = "full")]
extern crate core;

#[cfg(any(feature = "full", feature = "verify"))]
pub mod common;
/// Drive module
#[cfg(any(feature = "full", feature = "verify"))]
pub mod drive;
/// Error module
#[cfg(any(feature = "full", feature = "verify"))]
pub mod error;
/// Fee pools module
#[cfg(feature = "full")]
pub mod fee_pools;
/// Query module
#[cfg(any(feature = "full", feature = "verify"))]
pub mod query;

/// DPP module
#[cfg(feature = "full")]
pub use dpp;
/// GroveDB module
#[cfg(feature = "full")]
pub use grovedb;
#[cfg(feature = "full")]
mod fee;
/// State transition action module
#[cfg(feature = "full")]
pub mod state_transition_action;
/// Test helpers
#[cfg(feature = "fixtures-and-mocks")]
pub mod tests;
