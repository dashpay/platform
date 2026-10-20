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

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Weak};

use arc_swap::ArcSwapOption;
use dpp::dashcore::Network;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::version::PlatformVersion;

use crate::{platform_activation_height, ContextProvider, ContextProviderError, QuorumKeyFuture};

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

    /// Keep `contract` so later proofs over it can be verified.
    fn register_data_contract(&self, contract: Arc<DataContract>);
}

/// Where an [`AdaptiveContextProvider`] takes quorum keys from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ContextProviderMode {
    /// SPV once its source is published and synced, trusted until then.
    Auto = 0,
    /// SPV only; lookups fail until its source is published and synced.
    Spv = 1,
    /// The trusted provider only.
    Trusted = 2,
}

impl TryFrom<u8> for ContextProviderMode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Auto),
            1 => Ok(Self::Spv),
            2 => Ok(Self::Trusted),
            _ => Err(()),
        }
    }
}

/// The source an [`AdaptiveContextProvider`] would ask right now.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AdaptiveContextProviderSource {
    /// The trusted provider.
    Trusted = 0,
    /// The SPV client.
    Spv = 1,
}

/// Whether a published SPV client is synced enough to answer quorum lookups.
pub type Readiness = Arc<dyn Fn() -> Result<bool, ContextProviderError> + Send + Sync>;

/// A published SPV client: its quorum lookups and whether it is synced.
#[derive(Clone)]
struct SpvContextSource {
    provider: Arc<dyn ContextProvider>,
    is_ready: Readiness,
}

/// One lease's hold on the slot; `source` is set once the holder publishes.
struct SlotClaim {
    lease_id: u64,
    source: Option<SpvContextSource>,
}

/// The slot an SPV client publishes its quorum keys into.
///
/// Only one lease holds the slot at a time, so two SPV clients can never both
/// answer for the same SDK.
pub struct SpvSourceSlot {
    claim: ArcSwapOption<SlotClaim>,
    next_lease_id: AtomicU64,
}

impl Default for SpvSourceSlot {
    fn default() -> Self {
        Self {
            claim: ArcSwapOption::empty(),
            next_lease_id: AtomicU64::new(1),
        }
    }
}

/// Returned by [`SpvSourceSlot::claim`] when another lease holds the
/// slot.
#[derive(Debug, thiserror::Error)]
#[error("another SPV client already serves this SDK's quorum keys")]
pub struct SpvSourceSlotTaken;

impl SpvSourceSlot {
    /// Claim the slot. Only an empty slot can be claimed; the lease empties it
    /// again when dropped.
    pub fn claim(self: &Arc<Self>) -> Result<SpvSourceLease, SpvSourceSlotTaken> {
        let lease_id = self.next_lease_id.fetch_add(1, Ordering::Relaxed);
        let claim = Arc::new(SlotClaim {
            lease_id,
            source: None,
        });
        let previous = self
            .claim
            .compare_and_swap(std::ptr::null::<SlotClaim>(), Some(claim));
        if previous.is_some() {
            return Err(SpvSourceSlotTaken);
        }
        Ok(SpvSourceLease {
            slot: Arc::downgrade(self),
            lease_id,
        })
    }

    fn publish(&self, lease_id: u64, source: SpvContextSource) {
        self.claim.rcu(|current| match current {
            Some(claim) if claim.lease_id == lease_id => Some(Arc::new(SlotClaim {
                lease_id,
                source: Some(source.clone()),
            })),
            _ => current.clone(),
        });
    }

    fn release(&self, lease_id: u64) {
        self.claim.rcu(|current| match current {
            Some(claim) if claim.lease_id == lease_id => None,
            _ => current.clone(),
        });
    }

    /// The published source, if it is synced.
    ///
    /// Every "no usable source" outcome, an unknown readiness included, is
    /// [`ContextProviderError::QuorumSourceUnavailable`]: it is this side's
    /// state, so the SDK must not ban the node whose proof it could not check.
    fn ready_source(&self) -> Result<SpvContextSource, ContextProviderError> {
        let source = self
            .claim
            .load_full()
            .and_then(|claim| claim.source.clone())
            .ok_or_else(|| {
                ContextProviderError::QuorumSourceUnavailable(
                    "SPV context source is not active".to_string(),
                )
            })?;
        match (source.is_ready)() {
            Ok(true) => Ok(source),
            Ok(false) => Err(ContextProviderError::QuorumSourceUnavailable(
                "SPV context source is not ready".to_string(),
            )),
            Err(error) => Err(ContextProviderError::QuorumSourceUnavailable(format!(
                "SPV context source readiness is unknown: {error}"
            ))),
        }
    }

    fn is_ready(&self) -> bool {
        self.ready_source().is_ok()
    }
}

/// A claim on an [`SpvSourceSlot`] slot. Dropping it empties the slot,
/// unless a later lease holds it by then.
pub struct SpvSourceLease {
    slot: Weak<SpvSourceSlot>,
    lease_id: u64,
}

impl SpvSourceLease {
    /// Serve `provider`'s quorum keys while `is_ready` reports it synced.
    pub fn publish(&self, provider: Arc<dyn ContextProvider>, is_ready: Readiness) {
        if let Some(slot) = self.slot.upgrade() {
            slot.publish(self.lease_id, SpvContextSource { provider, is_ready });
        }
    }
}

impl Drop for SpvSourceLease {
    fn drop(&mut self) {
        if let Some(slot) = self.slot.upgrade() {
            slot.release(self.lease_id);
        }
    }
}

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

/// Routes each quorum lookup to an SPV client or a trusted provider by
/// [`ContextProviderMode`]; contracts and tokens come from a
/// [`ContractSource`].
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
    pub fn new(
        trusted: Arc<dyn ContextProvider>,
        contracts: Arc<dyn ContractSource>,
        network: Network,
    ) -> Self {
        Self {
            trusted,
            spv: Arc::default(),
            contracts,
            mode: AtomicU8::new(ContextProviderMode::Auto as u8),
            network,
        }
    }

    /// The slot an SPV client publishes its quorum keys into.
    pub fn source_slot(&self) -> Arc<SpvSourceSlot> {
        Arc::clone(&self.spv)
    }

    /// Change where later lookups go.
    pub fn set_mode(&self, mode: ContextProviderMode) {
        self.mode.store(mode as u8, Ordering::Release);
    }

    /// Where lookups go.
    pub fn mode(&self) -> ContextProviderMode {
        ContextProviderMode::try_from(self.mode.load(Ordering::Acquire))
            .expect("context-provider mode is only written from validated values")
    }

    /// The source a lookup would ask right now.
    pub fn active_source(&self) -> AdaptiveContextProviderSource {
        match self.mode() {
            ContextProviderMode::Trusted => AdaptiveContextProviderSource::Trusted,
            ContextProviderMode::Spv => AdaptiveContextProviderSource::Spv,
            ContextProviderMode::Auto if self.spv.is_ready() => AdaptiveContextProviderSource::Spv,
            ContextProviderMode::Auto => AdaptiveContextProviderSource::Trusted,
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
            ContextProviderMode::Spv => self.spv.ready_source()?.provider.get_quorum_public_key(
                quorum_type,
                quorum_hash,
                core_chain_locked_height,
            ),
            ContextProviderMode::Auto => match self.spv.ready_source() {
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
        match self.active_source() {
            AdaptiveContextProviderSource::Trusted => self.trusted.fetch_quorum_public_key(
                quorum_type,
                quorum_hash,
                core_chain_locked_height,
            ),
            AdaptiveContextProviderSource::Spv => None,
        }
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(platform_activation_height(self.network))
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
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use dpp::dashcore::Network;
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
    use dpp::version::PlatformVersion;

    use super::{
        AdaptiveContextProvider, AdaptiveContextProviderSource, ContextProviderMode,
        ContractSource, SpvContextProvider, SpvSourceLease, SpvSourceSlot,
    };
    use crate::{ContextProvider, ContextProviderError, QuorumKeyFuture};

    /// A quorum source answering every lookup with `[key_byte; 48]`, or failing.
    struct TestQuorums {
        name: &'static str,
        key_byte: u8,
        fail: bool,
    }

    impl ContextProvider for TestQuorums {
        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            if self.fail {
                return Err(ContextProviderError::InvalidQuorum(format!(
                    "{} quorum miss",
                    self.name
                )));
            }
            Ok([self.key_byte; 48])
        }

        fn fetch_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _height: u32,
        ) -> Option<QuorumKeyFuture> {
            let key = [self.key_byte; 48];
            Some(Box::pin(async move { Ok(Some(key)) }))
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(999)
        }

        fn get_data_contract(
            &self,
            _id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            Err(ContextProviderError::Generic(format!(
                "{} contract route",
                self.name
            )))
        }

        fn get_token_configuration(
            &self,
            _id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Err(ContextProviderError::Generic(format!(
                "{} token route",
                self.name
            )))
        }
    }

    /// A contract source whose reads fail with a recognisable message.
    struct TestContracts;

    impl ContractSource for TestContracts {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            Err(ContextProviderError::Generic(
                "local contract route".to_string(),
            ))
        }

        fn get_token_configuration(
            &self,
            _token_id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Err(ContextProviderError::Generic(
                "local token route".to_string(),
            ))
        }

        fn register_data_contract(&self, _contract: Arc<DataContract>) {}
    }

    fn quorums(name: &'static str, key_byte: u8) -> Arc<dyn ContextProvider> {
        Arc::new(TestQuorums {
            name,
            key_byte,
            fail: false,
        })
    }

    fn failing_quorums(name: &'static str) -> Arc<dyn ContextProvider> {
        Arc::new(TestQuorums {
            name,
            key_byte: 0,
            fail: true,
        })
    }

    fn adaptive() -> AdaptiveContextProvider {
        AdaptiveContextProvider::new(
            quorums("trusted", 0x11),
            Arc::new(TestContracts),
            Network::Testnet,
        )
    }

    fn quorum_key(provider: &impl ContextProvider) -> Result<[u8; 48], ContextProviderError> {
        provider.get_quorum_public_key(1, [2; 32], 3)
    }

    /// Claim `slot` and publish `provider` into it, with `ready` as its
    /// readiness.
    fn publish(
        slot: &Arc<SpvSourceSlot>,
        provider: Arc<dyn ContextProvider>,
        ready: bool,
    ) -> SpvSourceLease {
        let lease = slot.claim().expect("empty slot");
        lease.publish(provider, Arc::new(move || Ok(ready)));
        lease
    }

    fn is_unavailable(result: Result<[u8; 48], ContextProviderError>) -> bool {
        matches!(
            result,
            Err(ContextProviderError::QuorumSourceUnavailable(_))
        )
    }

    #[test]
    fn should_route_quorum_lookups_by_mode_and_readiness() {
        let adaptive = adaptive();
        let slot = adaptive.source_slot();

        assert_eq!(quorum_key(&adaptive).unwrap(), [0x11; 48]);
        assert_eq!(
            adaptive.active_source(),
            AdaptiveContextProviderSource::Trusted
        );

        adaptive.set_mode(ContextProviderMode::Spv);
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
        assert_eq!(adaptive.active_source(), AdaptiveContextProviderSource::Spv);

        ready.store(true, Ordering::Release);
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x22; 48]);

        adaptive.set_mode(ContextProviderMode::Trusted);
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x11; 48]);

        adaptive.set_mode(ContextProviderMode::Auto);
        ready.store(false, Ordering::Release);
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x11; 48]);
        assert_eq!(
            adaptive.active_source(),
            AdaptiveContextProviderSource::Trusted
        );
        ready.store(true, Ordering::Release);
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x22; 48]);
        assert_eq!(adaptive.active_source(), AdaptiveContextProviderSource::Spv);
    }

    #[test]
    fn should_read_contracts_and_tokens_from_the_contract_source_in_every_mode() {
        let adaptive = adaptive();
        let _lease = publish(&adaptive.source_slot(), quorums("spv", 0x22), true);
        let id = Identifier::new([7; 32]);

        for mode in [
            ContextProviderMode::Auto,
            ContextProviderMode::Spv,
            ContextProviderMode::Trusted,
        ] {
            adaptive.set_mode(mode);
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
    fn should_let_one_lease_hold_the_slot_until_it_is_dropped() {
        let slot = Arc::new(SpvSourceSlot::default());
        let lease = publish(&slot, quorums("spv", 0x22), true);
        assert!(slot.claim().is_err());

        drop(lease);
        let _replacement = publish(&slot, quorums("other", 0x33), true);
        assert_eq!(
            slot.ready_source()
                .unwrap()
                .provider
                .get_quorum_public_key(1, [2; 32], 3)
                .unwrap(),
            [0x33; 48]
        );
    }

    #[test]
    fn should_not_retry_a_ready_spv_miss_against_trusted() {
        let adaptive = adaptive();
        let _lease = publish(&adaptive.source_slot(), failing_quorums("spv"), true);

        let error = quorum_key(&adaptive).unwrap_err().to_string();
        assert!(error.contains("spv quorum miss"));
    }

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

        adaptive.set_mode(ContextProviderMode::Spv);
        assert!(
            fetch(&adaptive).is_none(),
            "SPV without a source must not fetch"
        );

        let _lease = publish(&adaptive.source_slot(), quorums("spv", 0x22), true);
        assert!(fetch(&adaptive).is_none(), "SPV must not fetch");

        adaptive.set_mode(ContextProviderMode::Auto);
        assert!(
            fetch(&adaptive).is_none(),
            "Auto on ready SPV must not fetch"
        );

        adaptive.set_mode(ContextProviderMode::Trusted);
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
        adaptive.set_mode(ContextProviderMode::Spv);

        assert!(is_unavailable(quorum_key(&fixed)));
        assert!(is_unavailable(quorum_key(&adaptive)));

        let unready = publish(&fixed.source_slot(), quorums("spv", 0x22), false);
        let adaptive_unready = publish(&adaptive.source_slot(), quorums("spv", 0x22), false);
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
        let _fixed_lease = failing_readiness(&fixed.source_slot());
        let _adaptive_lease = failing_readiness(&adaptive.source_slot());
        assert!(is_unavailable(quorum_key(&fixed)));
        assert!(is_unavailable(quorum_key(&adaptive)));
    }

    /// The empty-slot swap is the only thing keeping two SPV clients from
    /// answering for one SDK, so concurrent claims must yield exactly one
    /// lease.
    #[test]
    fn should_grant_exactly_one_of_concurrent_claims() {
        let slot = Arc::new(SpvSourceSlot::default());
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let claims: Vec<_> = (0..8)
            .map(|_| {
                let slot = Arc::clone(&slot);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    slot.claim().ok()
                })
            })
            .collect();
        let leases: Vec<_> = claims
            .into_iter()
            .filter_map(|claim| claim.join().expect("claim thread"))
            .collect();
        assert_eq!(leases.len(), 1);
    }

    /// An SDK may be destroyed before the wallet manager holding its slot's
    /// lease; publishing through or dropping that lease must then do nothing.
    #[test]
    fn should_ignore_a_lease_whose_slot_is_gone() {
        let slot = Arc::new(SpvSourceSlot::default());
        let lease = slot.claim().expect("empty slot");
        drop(slot);

        lease.publish(quorums("spv", 0x22), Arc::new(|| Ok(true)));
        drop(lease);
    }

    /// Auto mode treats an SPV client that cannot say whether it is synced as
    /// not ready, and answers from the trusted provider.
    #[test]
    fn should_fall_back_to_trusted_in_auto_mode_when_readiness_is_unknown() {
        let adaptive = adaptive();
        let lease = adaptive.source_slot().claim().expect("empty slot");
        lease.publish(
            quorums("spv", 0x22),
            Arc::new(|| Err(ContextProviderError::Generic("no runtime".to_string()))),
        );

        assert_eq!(
            adaptive.active_source(),
            AdaptiveContextProviderSource::Trusted
        );
        assert_eq!(quorum_key(&adaptive).expect("trusted key"), [0x11; 48]);
    }
}
