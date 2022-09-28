//! Dash Drive
//!
//! Decentralized storage hosted by Dash masternodes
//!

#![cfg_attr(docsrs, feature(doc_cfg))]
// Coding conventions
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Common module
pub mod common;
/// Contract module
pub mod contract;
/// Drive module
pub mod drive;
/// Error module
pub mod error;
/// Fee module
pub mod fee;
/// Fee pools module
pub mod fee_pools;
/// Query module
pub mod query;
/// DPP module
pub use dpp;
/// GroveDB module
pub use grovedb;
