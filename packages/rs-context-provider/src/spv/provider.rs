use std::sync::Arc;

use dpp::dashcore::Network;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::version::PlatformVersion;

use super::SpvSourceSlot;
use crate::{platform_activation_height, ContextProvider, ContextProviderError, ContractSource};

/// Strict SPV: quorum keys come only from the SPV client published into
/// [`Self::source_slot`]; contracts and tokens come from a [`ContractSource`].
pub struct SpvContextProvider {
    spv: Arc<SpvSourceSlot>,
    contracts: Arc<dyn ContractSource>,
    network: Network,
}

impl SpvContextProvider {
    /// A provider whose quorum lookups fail until an SPV client is published
    /// into its slot.
    pub fn new(contracts: Arc<dyn ContractSource>, network: Network) -> Self {
        Self {
            spv: Arc::default(),
            contracts,
            network,
        }
    }

    /// The slot an SPV client publishes its quorum keys into.
    pub fn source_slot(&self) -> Arc<SpvSourceSlot> {
        Arc::clone(&self.spv)
    }
}

impl ContextProvider for SpvContextProvider {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        self.spv.ready_source()?.provider.get_quorum_public_key(
            quorum_type,
            quorum_hash,
            core_chain_locked_height,
        )
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(platform_activation_height(self.network))
    }

    fn network(&self) -> Option<Network> {
        Some(self.network)
    }

    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        self.contracts.get_data_contract(id, platform_version)
    }

    fn register_data_contract(&self, contract: Arc<DataContract>) {
        self.contracts.register_data_contract(contract)
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        self.contracts.get_token_configuration(token_id)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use dpp::dashcore::Network;
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;

    use super::SpvContextProvider;
    use crate::test_support::{is_unavailable, quorum_key, quorums, TestContracts};
    use crate::ContextProvider;

    #[test]
    fn should_answer_strict_spv_lookups_only_from_the_published_source() {
        let fixed = SpvContextProvider::new(Arc::new(TestContracts), Network::Testnet);
        let slot = fixed.source_slot();

        assert!(is_unavailable(quorum_key(&fixed)));
        let lease = slot.claim().unwrap();
        assert!(
            is_unavailable(quorum_key(&fixed)),
            "a claimed slot serves nothing until it is published"
        );
        lease.publish(quorums("spv", 0x22), Arc::new(|| Ok(true)));
        assert_eq!(quorum_key(&fixed).unwrap(), [0x22; 48]);
        assert!(fixed.fetch_quorum_public_key(1, [2; 32], 3).is_none());

        let id = Identifier::new([7; 32]);
        assert!(fixed
            .get_data_contract(&id, PlatformVersion::latest())
            .unwrap_err()
            .to_string()
            .contains("local contract route"));
        assert!(fixed
            .get_token_configuration(&id)
            .unwrap_err()
            .to_string()
            .contains("local token route"));
    }
}
