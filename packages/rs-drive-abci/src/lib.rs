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

/// Common functions module
pub mod common;

/// Contracts module
pub mod contracts;

/// Errors module
pub mod error;

/// Execution module
pub mod execution;

pub mod platform;

/// Functions related to IdentityCreditWithdrawalTransaction  
pub mod identity_credit_withdrawal;
