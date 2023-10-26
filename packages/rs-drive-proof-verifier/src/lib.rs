//! Proof verification library for Dash Drive
#![warn(missing_docs)]

/// Errors that can occur during proof verification
mod error;
/// Implementation of proof verification
mod proof;
mod provider;
pub mod types;
mod verify;
pub use error::Error;
pub use proof::FromProof;
#[cfg(feature = "mocks")]
pub use provider::MockQuorumInfoProvider;
pub use provider::QuorumInfoProvider;
