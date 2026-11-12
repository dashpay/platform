//! Context providers for Dash Platform SDK
//!
//! This crate provides the core traits for context providers that are used
//! to fetch network state, data contracts, and quorum public keys, and two
//! providers built on them: [`SpvContextProvider`], which takes quorum keys
//! only from an SPV client, and [`AdaptiveContextProvider`], which routes
//! them between an SPV client and a trusted provider.

pub mod adaptive;
pub mod contract_source;
pub mod error;
pub mod provider;
pub mod quorum_source;
pub mod spv;
#[cfg(test)]
mod test_support;

pub use adaptive::AdaptiveContextProvider;
pub use contract_source::ContractSource;
pub use error::ContextProviderError;
pub use provider::{
    platform_activation_height, ContextProvider, DataContractProvider, QuorumKeyFuture,
};
pub use quorum_source::{ContextProviderMode, QuorumKeySource};
pub use spv::{Readiness, SpvContextProvider, SpvSourceLease, SpvSourceSlot, SpvSourceSlotTaken};

#[cfg(feature = "mocks")]
pub use provider::MockContextProvider;
