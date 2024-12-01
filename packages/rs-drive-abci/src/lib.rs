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

/// Errors module
pub mod error;

/// Execution module
pub mod execution;

/// Platform configuration
pub mod config;

/// Logging and tracing
pub mod logging;

/// Anything related to 3rd party RPC
pub mod rpc;

/// Core utilities
pub mod core;

/// Metrics subsystem
pub mod metrics;

/// Test helpers and fixtures
#[cfg(any(feature = "mocks", test))]
pub mod test;

/// Mimic of block execution for tests
#[cfg(any(feature = "mocks", test))]
pub mod mimic;
/// Platform module
pub mod platform_types;
/// Querying
pub mod query;
/// Various utils
pub mod utils;

/// Drive server
pub mod server;
