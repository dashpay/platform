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

/// Contracts module
pub mod contracts;

/// Errors module
pub mod error;

/// Execution module
pub mod execution;

/// Platform module
pub mod platform;

/// Functions related to IdentityCreditWithdrawalTransaction
pub mod identity_credit_withdrawal;

/// Platform configuration
pub mod config;

/// Logging and tracing
pub mod logging;

/// Platform state
pub mod state;

/// Platform constants
pub mod constants;

/// Anything related to 3rd party RPC
pub mod rpc;

// TODO We should compile it only for tests
/// Asset Lock
pub mod asset_lock;
/// Core utilities
pub mod core;
/// Metrics subsystem
pub mod metrics;
/// Querying
pub mod query;
/// Test helpers and fixtures
pub mod test;
/// Validator Set
pub mod validator_set;
