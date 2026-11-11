//! Context providers that choose where quorum keys come from.
//!
//! [`SpvContextProvider`] takes quorum keys only from an SPV client.
//! [`AdaptiveContextProvider`] routes each lookup to an SPV client or to a
//! trusted provider by a mode that can change while the SDK runs. Both read
//! data contracts and token configurations from a [`ContractSource`], which has
//! no quorum keys to fall back on.
//!
//! The SPV client is not known when the SDK is built. Each provider owns an
//! [`SpvSourceSlot`] slot; whoever runs the SPV client claims it with
//! [`SpvSourceSlot::claim`] and publishes its quorum keys through the
//! returned [`SpvSourceLease`]. Until then, and once the lease is dropped, an
//! SPV lookup fails with [`ContextProviderError::QuorumSourceUnavailable`].

use std::sync::Arc;

use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{DataContract, Identifier};
use dpp::version::PlatformVersion;

use crate::ContextProviderError;

mod adaptive;
mod slot;
mod spv;
#[cfg(test)]
mod test_support;

pub use adaptive::{AdaptiveContextProvider, AdaptiveContextProviderSource, ContextProviderMode};
pub use slot::{Readiness, SpvSourceLease, SpvSourceSlot, SpvSourceSlotTaken};
pub use spv::SpvContextProvider;

/// The data contracts and token configurations a provider resolves locally.
///
/// It has no quorum keys, so a provider reading contracts from it cannot answer
/// a quorum lookup from the same place.
pub trait ContractSource: Send + Sync {
    /// The data contract with `id`, if this source has it.
    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError>;

    /// The token configuration of `token_id`, if this source has it.
    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError>;

    /// Keep `contract` so later proofs over it can be verified, unless the
    /// source holds one of a higher version under its id, as
    /// [`ContextProvider::register_data_contract`](crate::ContextProvider::register_data_contract)
    /// asks.
    fn register_data_contract(&self, contract: Arc<DataContract>);
}
