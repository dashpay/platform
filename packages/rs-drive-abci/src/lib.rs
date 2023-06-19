//! Dash ABCI
//!
//! ABCI is an interface that defines the boundary between the replication engine (the blockchain),
//! and the state machine (the application). Using a socket protocol, a consensus engine running
//! in one process can manage an application state running in another.
//!

#![cfg_attr(docsrs, feature(doc_cfg))]
// Coding conventions
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// ABCI module
pub mod abci;

/// Block module
mod block;

/// Validation module
pub mod validation;

/// Errors module
pub mod error;

/// Execution module
pub mod execution;

/// Platform module
pub mod platform;

/// Platform configuration
pub mod config;

/// Logging and tracing
pub mod logging;

/// Platform constants
pub mod constants;

/// Anything related to 3rd party RPC
pub mod rpc;

/// Core utilities
pub mod core;
/// Metrics subsystem
pub mod metrics;
/// Test helpers and fixtures
pub mod test;

/// Mimic of block execution for tests
pub mod mimic;
