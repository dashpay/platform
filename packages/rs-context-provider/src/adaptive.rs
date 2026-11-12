//! [`AdaptiveContextProvider`] routes each quorum lookup to an SPV client or
//! to a trusted provider, by a [`ContextProviderMode`] that can change while
//! the SDK runs. Its SPV route reads the same kind of [`SpvSourceSlot`] as
//! [`SpvContextProvider`](crate::SpvContextProvider).

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use dpp::dashcore::Network;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::version::PlatformVersion;

use crate::{
    platform_activation_height, ContextProvider, ContextProviderError, ContextProviderMode,
    ContractSource, QuorumKeyFuture, QuorumKeySource, SpvSourceSlot,
};

/// Routes each quorum lookup to an SPV client or a trusted provider by
/// [`ContextProviderMode`]; contracts and tokens come from a
/// [`ContractSource`].
///
/// In [`ContextProviderMode::Auto`] it answers from the trusted provider
/// whenever the SPV client is not ready: before it has synced, and again
/// whenever it loses its peers or its sync fails. Someone who can cut the
/// client off from its SPV peers can therefore move Auto onto the trusted
/// provider, so Auto is only as strong as the trusted provider. Clients that
/// must never verify proofs with the trusted provider's keys use
/// [`ContextProviderMode::Spv`], which fails closed instead.
///
/// It does not pass on [`ContextProvider::core_height_verified`]: it does not
/// record which source supplied a quorum's key, and a verified height vouches
/// only for that source.
pub struct AdaptiveContextProvider {
    trusted: Arc<dyn ContextProvider>,
    spv: Arc<SpvSourceSlot>,
    contracts: Arc<dyn ContractSource>,
    mode: AtomicU8,
    network: Network,
}

impl AdaptiveContextProvider {
    /// A provider in [`ContextProviderMode::Auto`] that answers from `trusted`
    /// until an SPV client is published into its slot and synced.
    ///
    /// Fails with [`ContextProviderError::Config`] when `trusted` declares a
    /// network other than `network`: the SDK checks only the network this
    /// provider reports, and would verify proofs with the other network's
    /// quorum keys whenever it routes to `trusted`.
    pub fn new(
        trusted: Arc<dyn ContextProvider>,
        contracts: Arc<dyn ContractSource>,
        network: Network,
    ) -> Result<Self, ContextProviderError> {
        if let Some(trusted_network) = trusted.network() {
            if trusted_network != network {
                return Err(ContextProviderError::Config(format!(
                    "the trusted provider serves {trusted_network:?} quorums, but the adaptive provider routes {network:?} lookups"
                )));
            }
        }
        Ok(Self {
            trusted,
            spv: Arc::default(),
            contracts,
            mode: AtomicU8::new(ContextProviderMode::Auto as u8),
            network,
        })
    }

    /// Where lookups go.
    pub fn mode(&self) -> ContextProviderMode {
        ContextProviderMode::try_from(self.mode.load(Ordering::Acquire))
            .expect("context-provider mode is only written from validated values")
    }

    /// The source a lookup would ask right now.
    fn current_source(&self) -> QuorumKeySource {
        match self.mode() {
            ContextProviderMode::Trusted => QuorumKeySource::Trusted,
            ContextProviderMode::Spv => QuorumKeySource::Spv,
            ContextProviderMode::Auto if self.spv.is_ready(self.network) => QuorumKeySource::Spv,
            ContextProviderMode::Auto => QuorumKeySource::Trusted,
        }
    }
}

impl ContextProvider for AdaptiveContextProvider {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        match self.mode() {
            ContextProviderMode::Trusted => self.trusted.get_quorum_public_key(
                quorum_type,
                quorum_hash,
                core_chain_locked_height,
            ),
            ContextProviderMode::Spv => self
                .spv
                .ready_source(self.network)?
                .provider
                .get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height),
            ContextProviderMode::Auto => match self.spv.ready_source(self.network) {
                Ok(source) => source.provider.get_quorum_public_key(
                    quorum_type,
                    quorum_hash,
                    core_chain_locked_height,
                ),
                Err(_) => self.trusted.get_quorum_public_key(
                    quorum_type,
                    quorum_hash,
                    core_chain_locked_height,
                ),
            },
        }
    }

    /// Only the trusted route can fetch a key missing from its cache. The SPV
    /// route never fetches, so SPV mode never falls back to trusted keys.
    fn fetch_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Option<QuorumKeyFuture> {
        match self.current_source() {
            QuorumKeySource::Trusted => self.trusted.fetch_quorum_public_key(
                quorum_type,
                quorum_hash,
                core_chain_locked_height,
            ),
            QuorumKeySource::Spv => None,
        }
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(platform_activation_height(self.network))
    }

    fn network(&self) -> Option<Network> {
        Some(self.network)
    }

    /// The trusted provider's requirement. It is the only source known when
    /// the SDK is given this provider, and the route Auto falls back to.
    fn requires_proofs(&self) -> bool {
        self.trusted.requires_proofs()
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

    /// In [`ContextProviderMode::Auto`], SPV only while the client published
    /// into the slot says it is ready and declares no network other than this
    /// provider's. A client that cannot say, or that declares another
    /// network, counts as not ready, as it does for lookups.
    fn quorum_source(&self) -> Option<QuorumKeySource> {
        Some(self.current_source())
    }

    fn set_quorum_mode(&self, mode: ContextProviderMode) -> Result<(), ContextProviderError> {
        self.mode.store(mode as u8, Ordering::Release);
        Ok(())
    }

    fn spv_source_slot(&self) -> Option<Arc<SpvSourceSlot>> {
        Some(Arc::clone(&self.spv))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use dpp::dashcore::Network;
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
    use dpp::version::PlatformVersion;

    use super::AdaptiveContextProvider;
    use crate::test_support::{
        failing_quorums, is_unavailable, network_quorums, publish, quorum_key, quorums, slot_of,
        TestContracts,
    };
    use crate::{ContextProvider, ContextProviderError, ContextProviderMode, QuorumKeySource};
    use crate::{SpvContextProvider, SpvSourceSlot};

    fn adaptive() -> AdaptiveContextProvider {
        AdaptiveContextProvider::new(
            quorums("trusted", 0x11),
            Arc::new(TestContracts),
            Network::Testnet,
        )
        .expect("a trusted provider without a network")
    }

    #[test]
    fn should_route_quorum_lookups_by_mode_and_readiness() {
        let adaptive = adaptive();
        let slot = slot_of(&adaptive);

        assert_eq!(quorum_key(&adaptive).unwrap(), [0x11; 48]);
        assert_eq!(adaptive.quorum_source(), Some(QuorumKeySource::Trusted));

        adaptive.set_quorum_mode(ContextProviderMode::Spv).unwrap();
        assert!(is_unavailable(quorum_key(&adaptive)));

        let ready = Arc::new(AtomicBool::new(false));
        let ready_for_callback = Arc::clone(&ready);
        let lease = slot.claim().unwrap();
        lease.publish(
            quorums("spv", 0x22),
            Arc::new(move || Ok(ready_for_callback.load(Ordering::Acquire))),
        );
        assert!(
            is_unavailable(quorum_key(&adaptive)),
            "unready SPV must fail closed"
        );
        assert_eq!(adaptive.quorum_source(), Some(QuorumKeySource::Spv));

        ready.store(true, Ordering::Release);
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x22; 48]);

        adaptive
            .set_quorum_mode(ContextProviderMode::Trusted)
            .unwrap();
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x11; 48]);

        adaptive.set_quorum_mode(ContextProviderMode::Auto).unwrap();
        ready.store(false, Ordering::Release);
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x11; 48]);
        assert_eq!(adaptive.quorum_source(), Some(QuorumKeySource::Trusted));
        ready.store(true, Ordering::Release);
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x22; 48]);
        assert_eq!(adaptive.quorum_source(), Some(QuorumKeySource::Spv));
    }

    /// The SDK keeps its provider behind wrappers, and the FFI asks the
    /// provider it reaches through them for its source, changes its mode and
    /// takes its SPV slot. A wrapper answering with the defaults would report
    /// a custom provider, refuse a mode change the provider supports, and
    /// hand out no slot, so no wallet manager could serve it SPV keys.
    #[test]
    fn should_answer_for_its_quorum_source_through_every_wrapper() {
        let inner: Arc<dyn ContextProvider> = Arc::new(adaptive());
        let boxed: Arc<Box<dyn ContextProvider>> =
            Arc::new(Box::new(Arc::clone(&inner)) as Box<dyn ContextProvider>);
        let mutex = Mutex::new(Arc::clone(&inner));
        let wrappers: [&dyn ContextProvider; 3] = [&inner, &boxed, &mutex];
        let slot = slot_of(&inner);

        for provider in wrappers {
            provider
                .set_quorum_mode(ContextProviderMode::Spv)
                .expect("the adaptive provider has a mode");
            assert_eq!(provider.quorum_source(), Some(QuorumKeySource::Spv));
            provider
                .set_quorum_mode(ContextProviderMode::Trusted)
                .expect("the adaptive provider has a mode");
            assert_eq!(provider.quorum_source(), Some(QuorumKeySource::Trusted));
            assert!(Arc::ptr_eq(
                &provider.spv_source_slot().expect("an SPV slot"),
                &slot
            ));
        }
    }

    #[test]
    fn should_read_contracts_and_tokens_from_the_contract_source_in_every_mode() {
        let adaptive = adaptive();
        let _lease = publish(&slot_of(&adaptive), quorums("spv", 0x22), true);
        let id = Identifier::new([7; 32]);

        for mode in [
            ContextProviderMode::Auto,
            ContextProviderMode::Spv,
            ContextProviderMode::Trusted,
        ] {
            adaptive.set_quorum_mode(mode).unwrap();
            let contract_error = adaptive
                .get_data_contract(&id, PlatformVersion::latest())
                .unwrap_err()
                .to_string();
            let token_error = adaptive
                .get_token_configuration(&id)
                .unwrap_err()
                .to_string();
            assert!(contract_error.contains("local contract route"));
            assert!(token_error.contains("local token route"));
        }
    }

    #[test]
    fn should_not_retry_a_ready_spv_miss_against_trusted() {
        let adaptive = adaptive();
        let _lease = publish(&slot_of(&adaptive), failing_quorums("spv"), true);

        let error = quorum_key(&adaptive).unwrap_err().to_string();
        assert!(error.contains("spv quorum miss"));
    }

    /// The SDK fetches a quorum key missing from the trusted cache through
    /// `fetch_quorum_public_key`. That fetch must follow the lookup's route:
    /// a trusted route fetches from the trusted service, an SPV route never
    /// does, or SPV mode would accept a trusted key after an SPV miss.
    #[tokio::test]
    async fn should_fetch_missing_quorum_keys_only_on_the_trusted_route() {
        let adaptive = adaptive();
        let fetch =
            |adaptive: &AdaptiveContextProvider| adaptive.fetch_quorum_public_key(1, [2; 32], 3);

        let fetched = fetch(&adaptive).expect("Auto without SPV fetches from trusted");
        assert_eq!(fetched.await.unwrap(), Some([0x11; 48]));

        adaptive.set_quorum_mode(ContextProviderMode::Spv).unwrap();
        assert!(
            fetch(&adaptive).is_none(),
            "SPV without a source must not fetch"
        );

        let _lease = publish(&slot_of(&adaptive), quorums("spv", 0x22), true);
        assert!(fetch(&adaptive).is_none(), "SPV must not fetch");

        adaptive.set_quorum_mode(ContextProviderMode::Auto).unwrap();
        assert!(
            fetch(&adaptive).is_none(),
            "Auto on ready SPV must not fetch"
        );

        adaptive
            .set_quorum_mode(ContextProviderMode::Trusted)
            .unwrap();
        let fetched = fetch(&adaptive).expect("Trusted fetches");
        assert_eq!(fetched.await.unwrap(), Some([0x11; 48]));
    }

    /// While the SPV source is missing, unready, or cannot say whether it is
    /// ready, the failure is this side's, so it must be
    /// `QuorumSourceUnavailable`: the SDK bans nodes for any other quorum-key
    /// error.
    #[test]
    fn should_report_an_unusable_spv_source_without_blaming_the_node() {
        let fixed = SpvContextProvider::new(Arc::new(TestContracts), Network::Testnet);
        let adaptive = adaptive();
        adaptive.set_quorum_mode(ContextProviderMode::Spv).unwrap();

        assert!(is_unavailable(quorum_key(&fixed)));
        assert!(is_unavailable(quorum_key(&adaptive)));

        let unready = publish(&slot_of(&fixed), quorums("spv", 0x22), false);
        let adaptive_unready = publish(&slot_of(&adaptive), quorums("spv", 0x22), false);
        assert!(is_unavailable(quorum_key(&fixed)));
        assert!(is_unavailable(quorum_key(&adaptive)));
        drop((unready, adaptive_unready));

        let failing_readiness = |slot: &Arc<SpvSourceSlot>| {
            let lease = slot.claim().unwrap();
            lease.publish(
                quorums("spv", 0x22),
                Arc::new(|| Err(ContextProviderError::Generic("no runtime".to_string()))),
            );
            lease
        };
        let _fixed_lease = failing_readiness(&slot_of(&fixed));
        let _adaptive_lease = failing_readiness(&slot_of(&adaptive));
        assert!(is_unavailable(quorum_key(&fixed)));
        assert!(is_unavailable(quorum_key(&adaptive)));
    }

    /// The SPV client published into a provider's slot must serve that
    /// provider's network. One serving another network would have its
    /// Platform quorums vouch for this network's proofs, so strict SPV fails
    /// closed without blaming the node, and Auto treats it as not ready and
    /// answers from the trusted provider. A client that serves the same
    /// network, or does not say, is used.
    #[test]
    fn should_not_use_a_published_spv_client_of_another_network() {
        let fixed = SpvContextProvider::new(Arc::new(TestContracts), Network::Testnet);
        let adaptive = adaptive();
        let _fixed_lease = publish(
            &slot_of(&fixed),
            network_quorums("mainnet spv", 0x33, Network::Mainnet),
            true,
        );
        let _adaptive_lease = publish(
            &slot_of(&adaptive),
            network_quorums("mainnet spv", 0x33, Network::Mainnet),
            true,
        );

        assert!(is_unavailable(quorum_key(&fixed)));
        assert_eq!(adaptive.quorum_source(), Some(QuorumKeySource::Trusted));
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x11; 48]);
        adaptive
            .set_quorum_mode(ContextProviderMode::Spv)
            .expect("the adaptive provider has a mode");
        assert!(is_unavailable(quorum_key(&adaptive)));

        for source in [
            network_quorums("testnet spv", 0x22, Network::Testnet),
            quorums("spv", 0x22),
        ] {
            let fixed = SpvContextProvider::new(Arc::new(TestContracts), Network::Testnet);
            let _lease = publish(&slot_of(&fixed), source, true);
            assert_eq!(quorum_key(&fixed).unwrap(), [0x22; 48]);
        }
    }

    /// A quorum source that states requirements and records the verified
    /// heights it is told about.
    #[derive(Default)]
    struct Requiring {
        proofs: bool,
        network: Option<Network>,
        verified: Mutex<Vec<(u32, [u8; 32], u32)>>,
    }

    impl ContextProvider for Requiring {
        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            Ok([0x44; 48])
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(999)
        }

        fn get_data_contract(
            &self,
            _id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            Ok(None)
        }

        fn get_token_configuration(
            &self,
            _id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Ok(None)
        }

        fn requires_proofs(&self) -> bool {
            self.proofs
        }

        fn network(&self) -> Option<Network> {
            self.network
        }

        fn core_height_verified(&self, quorum_type: u32, quorum_hash: [u8; 32], height: u32) {
            self.verified
                .lock()
                .expect("test state lock")
                .push((quorum_type, quorum_hash, height));
        }
    }

    /// The SDK refuses a provider for another network, or one that requires
    /// proofs while proofs are off, by asking the provider it is built or set with.
    /// A routing provider that answered with the defaults would let the SDK
    /// accept one it must refuse.
    #[test]
    fn should_forward_provider_requirements_through_every_wrapper() {
        let fixed = SpvContextProvider::new(Arc::new(TestContracts), Network::Testnet);
        assert_eq!(fixed.network(), Some(Network::Testnet));
        assert!(!fixed.requires_proofs());

        for proofs in [false, true] {
            let trusted = Arc::new(Requiring {
                proofs,
                ..Default::default()
            });
            let adaptive =
                AdaptiveContextProvider::new(trusted, Arc::new(TestContracts), Network::Testnet)
                    .expect("a trusted provider without a network");
            assert_eq!(adaptive.network(), Some(Network::Testnet));
            assert_eq!(adaptive.requires_proofs(), proofs);
        }
    }

    /// The SDK checks the network of the provider it is built or set with, and the
    /// adaptive provider reports its own. Over a trusted provider for another
    /// network it would pass that check, then verify proofs with that
    /// network's quorum keys whenever it routes to the trusted provider, so
    /// it refuses one. A trusted provider that declares no network, or the
    /// same one, is accepted.
    #[test]
    fn should_refuse_a_trusted_provider_for_another_network() {
        let trusted = |network| {
            Arc::new(Requiring {
                network,
                ..Default::default()
            }) as Arc<dyn ContextProvider>
        };

        let refusal = AdaptiveContextProvider::new(
            trusted(Some(Network::Testnet)),
            Arc::new(TestContracts),
            Network::Mainnet,
        )
        .err();
        assert!(
            matches!(
                &refusal,
                Some(ContextProviderError::Config(message))
                    if message.contains("Testnet") && message.contains("Mainnet")
            ),
            "{refusal:?}"
        );

        for network in [None, Some(Network::Mainnet)] {
            let adaptive = AdaptiveContextProvider::new(
                trusted(network),
                Arc::new(TestContracts),
                Network::Mainnet,
            )
            .expect("a trusted provider for no other network");
            assert_eq!(adaptive.network(), Some(Network::Mainnet));
        }
    }

    /// A verified height vouches only for the source whose key verified the
    /// response. The adaptive provider does not record which source supplied
    /// a key, and its route can change between the lookup and the call, so
    /// it tells neither: the trusted provider must not hear of a response an
    /// SPV key verified.
    #[test]
    fn should_not_report_a_verified_height_to_a_source_that_may_not_have_supplied_the_key() {
        let trusted = Arc::new(Requiring::default());
        let adaptive = AdaptiveContextProvider::new(
            Arc::clone(&trusted) as Arc<dyn ContextProvider>,
            Arc::new(TestContracts),
            Network::Testnet,
        )
        .expect("a trusted provider without a network");
        let _lease = publish(&slot_of(&adaptive), quorums("spv", 0x22), true);
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x22; 48]);

        adaptive.core_height_verified(1, [2; 32], 3);
        assert!(trusted.verified.lock().expect("test state lock").is_empty());
    }

    /// Auto mode treats an SPV client that cannot say whether it is synced as
    /// not ready, and answers from the trusted provider.
    #[test]
    fn should_fall_back_to_trusted_in_auto_mode_when_readiness_is_unknown() {
        let adaptive = adaptive();
        let lease = slot_of(&adaptive).claim().expect("empty slot");
        lease.publish(
            quorums("spv", 0x22),
            Arc::new(|| Err(ContextProviderError::Generic("no runtime".to_string()))),
        );

        assert_eq!(adaptive.quorum_source(), Some(QuorumKeySource::Trusted));
        assert_eq!(quorum_key(&adaptive).expect("trusted key"), [0x11; 48]);
    }
}
