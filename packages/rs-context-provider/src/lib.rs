//! Context provider traits for Dash Platform SDK
//!
//! This crate provides the core traits for context providers that are used
//! to fetch network state, data contracts, and quorum public keys.

pub mod error;
pub mod provider;

pub use error::ContextProviderError;
pub use provider::{ContextProvider, DataContractProvider, QuorumKeyFuture};

#[cfg(feature = "mocks")]
pub use provider::MockContextProvider;
