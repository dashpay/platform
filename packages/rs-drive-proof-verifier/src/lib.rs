//! Proof verification library for Dash Drive
#![warn(missing_docs)]
#![allow(clippy::result_large_err)]

/// Errors that can occur during proof verification
pub mod error;
/// Implementation of proof verification
mod proof;
pub mod types;
mod verify;
pub use error::Error;
pub use proof::{FromProof, Length};

// Re-export context provider types from dash-context-provider
#[cfg(feature = "mocks")]
pub use dash_context_provider::MockContextProvider;
pub use dash_context_provider::{ContextProvider, ContextProviderError, DataContractProvider};

/// From Request
pub mod from_request;
/// Implementation of unproved verification
pub mod unproved;

// Needed for #[derive(PlatformSerialize, PlatformDeserialize)]
#[cfg(feature = "mocks")]
use dpp::serialization;
