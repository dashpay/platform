//! A context provider that knows one data contract itself.

use dash_context_provider::{ContextProvider, ContextProviderError, QuorumKeyFuture};
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::version::PlatformVersion;
use std::sync::Arc;

/// `provider`, except that the data contract known under `contract`'s id is `contract`.
///
/// A context provider keeps the latest version of each contract it is given, and any
/// operation may give it a newer one. A proof that is verified against one given version of
/// a contract, as the proof of a delta contract update is against the contract the delta
/// was built on, is verified in this context instead: the contract is the one the operation
/// started with, whatever the provider holds by the time the proof arrives. The provider is
/// not written, so other operations keep seeing its latest version.
pub(crate) struct WithContract<'a> {
    provider: &'a dyn ContextProvider,
    contract: &'a Arc<DataContract>,
}

impl<'a> WithContract<'a> {
    pub(crate) fn new(provider: &'a dyn ContextProvider, contract: &'a Arc<DataContract>) -> Self {
        Self { provider, contract }
    }
}

impl ContextProvider for WithContract<'_> {
    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        if *id == self.contract.id() {
            return Ok(Some(Arc::clone(self.contract)));
        }
        self.provider.get_data_contract(id, platform_version)
    }

    fn register_data_contract(&self, contract: Arc<DataContract>) {
        self.provider.register_data_contract(contract)
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        self.provider.get_token_configuration(token_id)
    }

    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        self.provider
            .get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
    }

    fn fetch_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Option<QuorumKeyFuture> {
        self.provider
            .fetch_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        self.provider.get_platform_activation_height()
    }

    fn network(&self) -> Option<Network> {
        self.provider.network()
    }

    fn requires_proofs(&self) -> bool {
        self.provider.requires_proofs()
    }

    fn core_height_verified(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) {
        self.provider
            .core_height_verified(quorum_type, quorum_hash, core_chain_locked_height)
    }
}
