//! Proof verification library for Dash Drive
#![warn(missing_docs)]

/// Errors that can occur during proof verification
pub mod error;
pub mod ordered_btreemap;
/// Implementation of proof verification
mod proof;
mod provider;
pub mod types;
mod verify;
pub use error::Error;
pub use proof::{FromProof, Length};
pub use provider::ContextProvider;
#[cfg(feature = "mocks")]
pub use provider::MockContextProvider;
pub mod from_request;

// Needed for #[derive(PlatformSerialize, PlatformDeserialize)]
#[cfg(feature = "mocks")]
use dpp::serialization;
