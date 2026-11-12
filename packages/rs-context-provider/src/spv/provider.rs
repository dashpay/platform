use std::sync::Arc;

use dpp::dashcore::Network;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::version::PlatformVersion;

use super::SpvSourceSlot;
use crate::{
    platform_activation_height, ContextProvider, ContextProviderError, ContractSource,
    QuorumKeySource,
};

/// Strict SPV: quorum keys come only from the SPV client published into
/// [`ContextProvider::spv_source_slot`]; contracts and tokens come from a
/// [`ContractSource`]. It has no mode: every
/// [`ContextProvider::set_quorum_mode`] is refused.
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

    fn quorum_source(&self) -> Option<QuorumKeySource> {
        Some(QuorumKeySource::Spv)
    }

    fn spv_source_slot(&self) -> Option<Arc<SpvSourceSlot>> {
        Some(Arc::clone(&self.spv))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use dpp::dashcore::Network;
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;

    use super::SpvContextProvider;
    use crate::test_support::{
        is_unavailable, publish, quorum_key, quorums, slot_of, TestContracts,
    };
    use crate::{ContextProvider, ContextProviderError, ContextProviderMode, QuorumKeySource};

    #[test]
    fn should_answer_strict_spv_lookups_only_from_the_published_source() {
        let fixed = SpvContextProvider::new(Arc::new(TestContracts), Network::Testnet);
        let slot = slot_of(&fixed);

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

    /// Strict SPV has one quorum source and no mode. It reports SPV whether
    /// or not a client is published, and refuses every mode change, Trusted
    /// included: nothing reaching it through the SDK can turn it onto
    /// trusted quorum keys.
    #[test]
    fn should_report_spv_and_refuse_every_mode_change() {
        let fixed = SpvContextProvider::new(Arc::new(TestContracts), Network::Testnet);
        assert_eq!(fixed.quorum_source(), Some(QuorumKeySource::Spv));

        for mode in [
            ContextProviderMode::Auto,
            ContextProviderMode::Spv,
            ContextProviderMode::Trusted,
        ] {
            let refusal = fixed.set_quorum_mode(mode);
            assert!(
                matches!(refusal, Err(ContextProviderError::Config(_))),
                "{mode:?}: {refusal:?}"
            );
        }

        let _lease = publish(&slot_of(&fixed), quorums("spv", 0x22), true);
        assert_eq!(quorum_key(&fixed).unwrap(), [0x22; 48]);
        assert_eq!(fixed.quorum_source(), Some(QuorumKeySource::Spv));
    }
}
