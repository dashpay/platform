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

#[cfg(feature = "full")]
pub mod common;
/// Contract module
#[cfg(feature = "full")]
pub mod contract;
/// Drive module
#[cfg(feature = "full")]
pub mod drive;
/// Error module
#[cfg(feature = "full")]
pub mod error;
/// Fee module
#[cfg(feature = "full")]
pub mod fee;
/// Fee pools module
#[cfg(feature = "full")]
pub mod fee_pools;
/// Query module
#[cfg(feature = "full")]
pub mod query;

/// DPP module
#[cfg(feature = "full")]
pub use dpp;
/// GroveDB module
#[cfg(feature = "full")]
pub use grovedb;

/// Test helpers
#[cfg(feature = "fixtures-and-mocks")]
pub mod tests;
