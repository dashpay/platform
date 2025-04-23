//! Proof verification library for Dash Drive
#![warn(missing_docs)]

/// Errors that can occur during proof verification
pub mod error;
/// Implementation of proof verification
mod proof;
mod provider;
pub mod types;
mod verify;
pub use error::Error;
pub use proof::{FromProof, Length};
#[cfg(feature = "mocks")]
pub use provider::MockContextProvider;
pub use provider::{ContextProvider, DataContractProvider};
/// From Request
pub mod from_request;
/// Implementation of unproved verification
pub mod unproved;

// Needed for #[derive(PlatformSerialize, PlatformDeserialize)]
#[cfg(feature = "mocks")]
use dpp::serialization;
