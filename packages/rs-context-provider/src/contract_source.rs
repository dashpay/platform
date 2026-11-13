use std::sync::Arc;

use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{DataContract, Identifier};
use dpp::version::PlatformVersion;

use crate::ContextProviderError;

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
