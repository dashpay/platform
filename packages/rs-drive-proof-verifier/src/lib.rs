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
pub use provider::ContextProvider;
#[cfg(feature = "mocks")]
pub use provider::MockContextProvider;
