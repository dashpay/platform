//! Context providers for Dash Platform SDK
//!
//! This crate provides the core traits for context providers that are used
//! to fetch network state, data contracts, and quorum public keys, and the
//! [`routing`] providers that choose between an SPV client and a trusted
//! provider for quorum keys.

pub mod error;
pub mod provider;
pub mod routing;

pub use error::ContextProviderError;
pub use provider::{
    platform_activation_height, ContextProvider, DataContractProvider, QuorumKeyFuture,
};

#[cfg(feature = "mocks")]
pub use provider::MockContextProvider;
