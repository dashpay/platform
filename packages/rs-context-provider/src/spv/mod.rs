//! Quorum keys from an SPV client.
//!
//! [`SpvContextProvider`] takes quorum keys only from an SPV client and never
//! from a trusted service. The SPV client is not known when the SDK is built:
//! the provider owns an [`SpvSourceSlot`], whoever runs the SPV client claims
//! it with [`SpvSourceSlot::claim`] and publishes its quorum keys through the
//! returned [`SpvSourceLease`]. Until then, and once the lease is dropped, a
//! lookup fails with
//! [`ContextProviderError::QuorumSourceUnavailable`](crate::ContextProviderError::QuorumSourceUnavailable).

mod provider;
mod slot;

pub use provider::SpvContextProvider;
pub use slot::{Readiness, SpvSourceLease, SpvSourceSlot, SpvSourceSlotTaken};
