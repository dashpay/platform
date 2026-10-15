//! A [`ContextProvider`] over trust anchors the embedder pushes into it.
//!
//! Embedders that already hold an authoritative view of the Core chain, such
//! as a full node or an SPV client that verifies quorum commitments, should
//! not ask a remote service which quorums sign Platform proofs. With
//! [`LocalContextProvider`] they push what they know instead:
//!
//! * the full set of active Platform quorum public keys, replacing the
//!   previous set on every push ([`LocalContextProvider::set_quorum_keys`]);
//! * their best ChainLock height, which only moves forward
//!   ([`LocalContextProvider::set_chain_locked_height`]).
//!
//! The provider does no I/O. It never fetches a key it was not given, so a
//! proof signed by a quorum the embedder has not pushed fails verification.

use crate::error::ContextProviderError;
use crate::provider::ContextProvider;
use dpp::bls::PublicKey;
use dpp::dashcore::sml::llmq_type::LLMQType;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
#[cfg(feature = "system-contracts")]
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::version::PlatformVersion;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// A [`ContextProvider`] that verifies proofs only against quorum keys and a
/// ChainLock height the embedder supplies.
///
/// A quorum key lookup runs these checks in order:
///
/// 1. A proof signed by any quorum type other than the Platform type given to
///    [`new`](Self::new) is refused with
///    [`ContextProviderError::InvalidQuorum`].
/// 2. Until the embedder has pushed a quorum key set, and, with a ChainLock
///    floor ([`with_chain_lock_floor`](Self::with_chain_lock_floor)), a
///    ChainLock height, the lookup fails with
///    [`ContextProviderError::QuorumSourceUnavailable`]. The provider cannot
///    judge the proof yet, so the SDK does not hold it against the node.
/// 3. With a floor, a proof whose signed core chain-locked height trails the
///    pushed height by more than the floor allows is refused as stale with
///    [`ContextProviderError::InvalidQuorum`]. There is no ceiling: a node
///    that has seen a newer ChainLock than the embedder is honest.
/// 4. A quorum hash that is not in the pushed set is refused with
///    [`ContextProviderError::InvalidQuorum`]. With a floor, a proof signed
///    above the pushed ChainLock height may name a quorum the embedder has not
///    learned yet, so that case fails with
///    [`ContextProviderError::QuorumSourceUnavailable`] instead.
/// 5. Otherwise the pushed key is returned.
///
/// `QuorumSourceUnavailable` fails the request without a health ban on the
/// node; the SDK may step over it briefly to ask another one. Every
/// `InvalidQuorum` is the node's fault, and the SDK bans it.
///
/// On every update, push the quorum keys before the ChainLock height they go
/// with, so a lookup never judges a new height against an old key set.
///
/// Data contracts are served from those registered with
/// [`ContextProvider::register_data_contract`], then from the system
/// contracts chosen with `with_system_contracts` (feature
/// `system-contracts`), built once per protocol version. Any other contract is
/// unknown.
///
/// The provider is meant to be shared: wrap it in an [`Arc`], keep one clone
/// to push updates through, and give other clones to every SDK built over it.
/// An embedder that rebuilds its SDK, for example after its endpoint set
/// changed, passes the same provider to the new SDK and keeps its anchors.
pub struct LocalContextProvider {
    platform_quorum_type: LLMQType,
    max_chain_lock_lag: Option<CoreBlockHeight>,
    platform_activation_height: Option<CoreBlockHeight>,
    #[cfg(feature = "system-contracts")]
    system_contracts: Vec<SystemDataContract>,
    /// Quorum hash in proof byte order -> BLS public key; `None` until the
    /// first push.
    quorum_keys: RwLock<Option<HashMap<[u8; 32], [u8; 48]>>>,
    /// The embedder's best ChainLock height; 0 until one is pushed.
    chain_locked_height: AtomicU32,
    registered_contracts: RwLock<HashMap<Identifier, Arc<DataContract>>>,
    #[cfg(feature = "system-contracts")]
    /// (contract id, protocol version) -> the built system contract.
    system_contract_cache: RwLock<HashMap<(Identifier, u32), Arc<DataContract>>>,
}

impl LocalContextProvider {
    /// Creates a provider that accepts proofs signed by quorums of
    /// `platform_quorum_type` only, for example
    /// `NetworkLLMQExt::platform_type(&network)`.
    ///
    /// It holds no quorum keys and no ChainLock height until the embedder
    /// pushes them, and no ChainLock floor unless one is configured.
    pub fn new(platform_quorum_type: LLMQType) -> Self {
        Self {
            platform_quorum_type,
            max_chain_lock_lag: None,
            platform_activation_height: None,
            #[cfg(feature = "system-contracts")]
            system_contracts: Vec::new(),
            quorum_keys: RwLock::default(),
            chain_locked_height: AtomicU32::new(0),
            registered_contracts: RwLock::default(),
            #[cfg(feature = "system-contracts")]
            system_contract_cache: RwLock::default(),
        }
    }

    /// Refuses proofs whose signed core chain-locked height trails the pushed
    /// ChainLock height by more than `max_lag` blocks, and refuses to judge
    /// any proof until a ChainLock height has been pushed.
    ///
    /// How much lag is normal depends on the network and on how the embedder
    /// follows the chain, so there is no default.
    pub fn with_chain_lock_floor(mut self, max_lag: CoreBlockHeight) -> Self {
        self.max_chain_lock_lag = Some(max_lag);
        self
    }

    /// Serves `contracts`, built for the protocol version each lookup asks
    /// for. Each one must be compiled into dpp (for example through this
    /// crate's `dpns-contract` and `dashpay-contract` features); a lookup of
    /// one that is not fails with
    /// [`ContextProviderError::DataContractFailure`].
    #[cfg(feature = "system-contracts")]
    pub fn with_system_contracts(
        mut self,
        contracts: impl IntoIterator<Item = SystemDataContract>,
    ) -> Self {
        self.system_contracts = contracts.into_iter().collect();
        self
    }

    /// The core height at which Platform activated on this network.
    /// Without it [`ContextProvider::get_platform_activation_height`] fails.
    pub fn with_platform_activation_height(mut self, height: CoreBlockHeight) -> Self {
        self.platform_activation_height = Some(height);
        self
    }

    /// Replaces the Platform quorum keys with `keys`: each quorum's hash and
    /// its BLS public key.
    ///
    /// The hash is in the byte order proofs carry, which is the order Dash
    /// Core's RPC prints it in (`quorum list`), the reverse of its internal
    /// `uint256` bytes.
    ///
    /// Push the whole set every time. A quorum missing from `keys` no longer
    /// verifies any proof, so rotated-out quorums stop being trusted. With a
    /// ChainLock floor, include every quorum that was active at some height in
    /// the floor's window, since a proof signed that far back may still use it.
    ///
    /// Every key must be a usable BLS public key: one that parses and is not
    /// the identity point. A key that fails would be returned as trusted, and
    /// the proof's signature check would then blame the node that sent it. If
    /// any key fails, this returns [`ContextProviderError::InvalidQuorum`]
    /// naming its quorum, and the previous set stays in place.
    pub fn set_quorum_keys(
        &self,
        keys: impl IntoIterator<Item = ([u8; 32], [u8; 48])>,
    ) -> Result<(), ContextProviderError> {
        let keys = keys
            .into_iter()
            .map(|(hash, key)| validate_quorum_key(&hash, &key).map(|()| (hash, key)))
            .collect::<Result<_, _>>()?;
        *write(&self.quorum_keys) = Some(keys);
        Ok(())
    }

    /// Records the embedder's best ChainLock height. A height below the one
    /// already recorded is ignored, and 0 is not a height.
    pub fn set_chain_locked_height(&self, height: CoreBlockHeight) {
        self.chain_locked_height.fetch_max(height, Ordering::AcqRel);
    }

    /// The highest ChainLock height pushed so far, if any.
    pub fn chain_locked_height(&self) -> Option<CoreBlockHeight> {
        match self.chain_locked_height.load(Ordering::Acquire) {
            0 => None,
            height => Some(height),
        }
    }

    #[cfg(feature = "system-contracts")]
    fn system_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        // Proof verification can ask for ids taken from a proof it has not
        // checked yet, so only contracts that exist are cached.
        let Some(&contract) = self.system_contracts.iter().find(|c| c.id() == *id) else {
            return Ok(None);
        };
        let cache_key = (*id, platform_version.protocol_version);
        if let Some(cached) = read(&self.system_contract_cache).get(&cache_key) {
            return Ok(Some(Arc::clone(cached)));
        }
        let built = Arc::new(
            load_system_data_contract(contract, platform_version).map_err(|e| {
                ContextProviderError::DataContractFailure(format!(
                    "cannot build system contract {contract:?} for protocol version {}: {e}",
                    platform_version.protocol_version
                ))
            })?,
        );
        write(&self.system_contract_cache).insert(cache_key, Arc::clone(&built));
        Ok(Some(built))
    }

    #[cfg(not(feature = "system-contracts"))]
    fn system_contract(
        &self,
        _id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok(None)
    }
}

impl ContextProvider for LocalContextProvider {
    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        if let Some(contract) = read(&self.registered_contracts).get(id) {
            return Ok(Some(Arc::clone(contract)));
        }
        self.system_contract(id, platform_version)
    }

    fn register_data_contract(&self, contract: Arc<DataContract>) {
        write(&self.registered_contracts).insert(contract.id(), contract);
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }

    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        let platform_type = u8::from(self.platform_quorum_type);
        if quorum_type != u32::from(platform_type) {
            return Err(ContextProviderError::InvalidQuorum(format!(
                "proof signed by quorum type {quorum_type}; Platform proofs on this network are \
                 signed by type {platform_type}"
            )));
        }
        let quorum_keys = read(&self.quorum_keys);
        let Some(quorum_keys) = quorum_keys.as_ref() else {
            return Err(ContextProviderError::QuorumSourceUnavailable(
                "no Platform quorum keys have been pushed yet".to_string(),
            ));
        };
        // `Some(local)` exactly when a floor is set.
        let local = match self.max_chain_lock_lag {
            None => None,
            Some(max_lag) => {
                let Some(local) = self.chain_locked_height() else {
                    return Err(ContextProviderError::QuorumSourceUnavailable(
                        "no ChainLock height has been pushed yet".to_string(),
                    ));
                };
                if local.saturating_sub(core_chain_locked_height) > max_lag {
                    return Err(ContextProviderError::InvalidQuorum(format!(
                        "stale proof: signed core chain-locked height {core_chain_locked_height} \
                         trails the local ChainLock height {local} by more than {max_lag} blocks"
                    )));
                }
                Some(local)
            }
        };
        if let Some(key) = quorum_keys.get(&quorum_hash) {
            return Ok(*key);
        }
        let quorum_hash = hex::encode(quorum_hash);
        match local {
            Some(local) if core_chain_locked_height > local => {
                Err(ContextProviderError::QuorumSourceUnavailable(format!(
                    "quorum {quorum_hash} is not known locally, and the proof was signed at core \
                     chain-locked height {core_chain_locked_height}, above the local ChainLock \
                     height {local}"
                )))
            }
            _ => Err(ContextProviderError::InvalidQuorum(format!(
                "no locally known Platform quorum with hash {quorum_hash}"
            ))),
        }
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        self.platform_activation_height.ok_or_else(|| {
            ContextProviderError::Config("no Platform activation height was configured".to_string())
        })
    }
}

fn validate_quorum_key(hash: &[u8; 32], key: &[u8; 48]) -> Result<(), ContextProviderError> {
    let reason = match PublicKey::try_from(key.as_slice()) {
        Ok(public_key) if public_key.is_identity() => "it is the identity point".to_string(),
        Ok(_) => return Ok(()),
        Err(error) => error.to_string(),
    };
    Err(ContextProviderError::InvalidQuorum(format!(
        "quorum {} has an unusable BLS public key: {reason}",
        hex::encode(hash)
    )))
}

// Every write replaces or inserts a whole value, so a panic while a lock is
// held cannot leave a half-updated map behind.
fn read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLATFORM_TYPE: LLMQType = LLMQType::LlmqtypeTestnetPlatform;
    const PLATFORM: u32 = 106;
    const QUORUM: [u8; 32] = [1; 32];
    /// A usable BLS public key, distinct for each seed.
    fn key(seed: u8) -> [u8; 48] {
        dpp::bls::SecretKey::from_ikm(&[seed; 32])
            .expect("32 bytes of keying material")
            .public_key()
            .to_bytes()
    }

    fn anchored(max_lag: CoreBlockHeight, local: CoreBlockHeight) -> LocalContextProvider {
        let provider = LocalContextProvider::new(PLATFORM_TYPE).with_chain_lock_floor(max_lag);
        provider.set_quorum_keys([(QUORUM, key(7))]).unwrap();
        provider.set_chain_locked_height(local);
        provider
    }

    #[test]
    fn should_serve_a_pushed_key_by_its_proof_order_hash_only() {
        let mut proof_order = [0u8; 32];
        proof_order[0] = 0xaa;
        proof_order[31] = 0xbb;
        let provider = LocalContextProvider::new(PLATFORM_TYPE);
        provider.set_quorum_keys([(proof_order, key(7))]).unwrap();

        assert_eq!(
            provider
                .get_quorum_public_key(PLATFORM, proof_order, 1)
                .expect("the pushed key"),
            key(7)
        );
        let mut reversed = proof_order;
        reversed.reverse();
        assert!(matches!(
            provider.get_quorum_public_key(PLATFORM, reversed, 1),
            Err(ContextProviderError::InvalidQuorum(_))
        ));
    }

    /// The checks run in the documented order, observable through which
    /// variant wins when several would fail.
    #[test]
    fn should_check_type_then_anchor_then_staleness_then_hash() {
        let unanchored = LocalContextProvider::new(PLATFORM_TYPE).with_chain_lock_floor(10);
        // Wrong type beats a key set that was never pushed.
        assert!(matches!(
            unanchored.get_quorum_public_key(PLATFORM + 1, QUORUM, 5000),
            Err(ContextProviderError::InvalidQuorum(_))
        ));
        unanchored.set_quorum_keys([(QUORUM, key(7))]).unwrap();
        // Wrong type beats the missing anchor.
        assert!(matches!(
            unanchored.get_quorum_public_key(PLATFORM + 1, QUORUM, 5000),
            Err(ContextProviderError::InvalidQuorum(_))
        ));
        // The missing anchor beats an unknown hash: nobody is blamed.
        assert!(matches!(
            unanchored.get_quorum_public_key(PLATFORM, [9; 32], 5000),
            Err(ContextProviderError::QuorumSourceUnavailable(_))
        ));
        assert!(matches!(
            unanchored.get_quorum_public_key(PLATFORM, QUORUM, 5000),
            Err(ContextProviderError::QuorumSourceUnavailable(_))
        ));

        let provider = anchored(10, 5000);
        // Wrong type is refused even for a pushed hash.
        assert!(matches!(
            provider.get_quorum_public_key(PLATFORM + 1, QUORUM, 5000),
            Err(ContextProviderError::InvalidQuorum(_))
        ));
        // A stale proof is refused even for a pushed hash.
        let stale = provider
            .get_quorum_public_key(PLATFORM, QUORUM, 4989)
            .unwrap_err();
        assert!(
            matches!(&stale, ContextProviderError::InvalidQuorum(m) if m.contains("stale")),
            "got {stale:?}"
        );
        // Staleness beats an unknown hash.
        let stale_and_unknown = provider
            .get_quorum_public_key(PLATFORM, [9; 32], 4989)
            .unwrap_err();
        assert!(
            matches!(&stale_and_unknown, ContextProviderError::InvalidQuorum(m) if m.contains("stale")),
            "got {stale_and_unknown:?}"
        );
        let unknown = provider
            .get_quorum_public_key(PLATFORM, [9; 32], 5000)
            .unwrap_err();
        assert!(
            matches!(&unknown, ContextProviderError::InvalidQuorum(m) if m.contains("no locally known")),
            "got {unknown:?}"
        );
    }

    /// Before the embedder pushes any key set the provider cannot judge a
    /// proof, with or without a floor. A pushed empty set is a judgement.
    #[test]
    fn should_not_judge_proofs_before_the_first_key_set() {
        for provider in [
            LocalContextProvider::new(PLATFORM_TYPE),
            LocalContextProvider::new(PLATFORM_TYPE).with_chain_lock_floor(10),
        ] {
            provider.set_chain_locked_height(5000);
            assert!(matches!(
                provider.get_quorum_public_key(PLATFORM, QUORUM, 5000),
                Err(ContextProviderError::QuorumSourceUnavailable(_))
            ));
            provider.set_quorum_keys([]).unwrap();
            assert!(matches!(
                provider.get_quorum_public_key(PLATFORM, QUORUM, 5000),
                Err(ContextProviderError::InvalidQuorum(_))
            ));
        }
    }

    /// A proof signed above the embedder's ChainLock height may use a quorum
    /// the embedder has not learned yet, so an unknown hash there blames
    /// nobody. At or below that height the embedder knows every quorum.
    #[test]
    fn should_not_blame_an_unknown_quorum_signed_above_the_local_chain_lock() {
        let provider = anchored(10, 5000);
        assert!(matches!(
            provider.get_quorum_public_key(PLATFORM, [9; 32], 5001),
            Err(ContextProviderError::QuorumSourceUnavailable(_))
        ));
        assert!(matches!(
            provider.get_quorum_public_key(PLATFORM, [9; 32], 5000),
            Err(ContextProviderError::InvalidQuorum(_))
        ));

        // Without a floor there is no local height to compare against.
        let unfloored = LocalContextProvider::new(PLATFORM_TYPE);
        unfloored.set_quorum_keys([(QUORUM, key(7))]).unwrap();
        unfloored.set_chain_locked_height(5000);
        assert!(matches!(
            unfloored.get_quorum_public_key(PLATFORM, [9; 32], 5001),
            Err(ContextProviderError::InvalidQuorum(_))
        ));
    }

    #[test]
    fn should_enforce_the_chain_lock_floor_without_a_ceiling() {
        let provider = anchored(288, 5000);
        let lookup = |signed| provider.get_quorum_public_key(PLATFORM, QUORUM, signed);

        assert_eq!(lookup(5000 - 288).expect("at the floor"), key(7));
        assert!(matches!(
            lookup(5000 - 289),
            Err(ContextProviderError::InvalidQuorum(_))
        ));
        assert_eq!(lookup(6000).expect("a node ahead of us is honest"), key(7));
    }

    #[test]
    fn should_only_move_the_chain_lock_height_forward() {
        let provider = anchored(10, 5000);
        provider.set_chain_locked_height(10);
        provider.set_chain_locked_height(0);
        assert_eq!(provider.chain_locked_height(), Some(5000));
        // The floor still judges against the higher height.
        let stale = provider
            .get_quorum_public_key(PLATFORM, QUORUM, 20)
            .unwrap_err();
        assert!(
            matches!(&stale, ContextProviderError::InvalidQuorum(m) if m.contains("stale")),
            "got {stale:?}"
        );

        provider.set_chain_locked_height(6000);
        assert_eq!(provider.chain_locked_height(), Some(6000));
        assert!(matches!(
            provider.get_quorum_public_key(PLATFORM, QUORUM, 5000),
            Err(ContextProviderError::InvalidQuorum(_))
        ));
    }

    /// Without a floor the ChainLock height is neither required nor checked.
    #[test]
    fn should_ignore_chain_lock_heights_without_a_floor() {
        let provider = LocalContextProvider::new(PLATFORM_TYPE);
        provider.set_quorum_keys([(QUORUM, key(7))]).unwrap();
        assert_eq!(provider.chain_locked_height(), None);
        assert_eq!(
            provider.get_quorum_public_key(PLATFORM, QUORUM, 1).unwrap(),
            key(7)
        );
        provider.set_chain_locked_height(1_000_000);
        assert_eq!(
            provider.get_quorum_public_key(PLATFORM, QUORUM, 1).unwrap(),
            key(7)
        );
    }

    /// An unusable key is refused with its quorum named, and the whole batch
    /// is refused with it: the previous keys keep verifying.
    #[test]
    fn should_refuse_a_batch_with_an_unusable_key_and_keep_the_previous_set() {
        let mut identity = [0u8; 48];
        identity[0] = 0xc0;
        let provider = anchored(10, 1);
        for unusable in [[0u8; 48], identity] {
            for batch in [
                vec![([2; 32], unusable)],
                vec![([3; 32], key(9)), ([2; 32], unusable)],
            ] {
                let error = provider.set_quorum_keys(batch).unwrap_err();
                assert!(
                    matches!(&error, ContextProviderError::InvalidQuorum(m) if m.contains(&hex::encode([2u8; 32]))),
                    "got {error:?}"
                );
                assert_eq!(
                    provider.get_quorum_public_key(PLATFORM, QUORUM, 1).unwrap(),
                    key(7),
                    "the previous set is untouched"
                );
                assert!(provider
                    .get_quorum_public_key(PLATFORM, [3; 32], 1)
                    .is_err());
            }
        }
    }

    #[test]
    fn should_forget_rotated_out_quorums_when_keys_are_replaced() {
        let provider = anchored(10, 1);
        provider.set_quorum_keys([([2; 32], key(9))]).unwrap();
        assert!(matches!(
            provider.get_quorum_public_key(PLATFORM, QUORUM, 1),
            Err(ContextProviderError::InvalidQuorum(_))
        ));
        assert_eq!(
            provider
                .get_quorum_public_key(PLATFORM, [2; 32], 1)
                .expect("the replacement key"),
            key(9)
        );
    }

    #[test]
    fn should_report_the_platform_activation_height_only_when_configured() {
        assert!(matches!(
            LocalContextProvider::new(PLATFORM_TYPE).get_platform_activation_height(),
            Err(ContextProviderError::Config(_))
        ));
        assert_eq!(
            LocalContextProvider::new(PLATFORM_TYPE)
                .with_platform_activation_height(1320)
                .get_platform_activation_height()
                .unwrap(),
            1320
        );
    }

    #[cfg(all(feature = "dpns-contract", feature = "dashpay-contract"))]
    mod contracts {
        use super::*;

        #[test]
        fn should_serve_configured_system_contracts_once_per_protocol_version() {
            let provider = LocalContextProvider::new(PLATFORM_TYPE)
                .with_system_contracts([SystemDataContract::DPNS, SystemDataContract::Dashpay]);
            let latest = PlatformVersion::latest();
            let first = PlatformVersion::first();

            for contract in [SystemDataContract::DPNS, SystemDataContract::Dashpay] {
                let served = provider
                    .get_data_contract(&contract.id(), latest)
                    .unwrap()
                    .expect("a configured system contract");
                assert_eq!(served.id(), contract.id());
                let again = provider
                    .get_data_contract(&contract.id(), latest)
                    .unwrap()
                    .expect("cached");
                assert!(Arc::ptr_eq(&served, &again), "built once per version");
                let other_version = provider
                    .get_data_contract(&contract.id(), first)
                    .unwrap()
                    .expect("built for the other version");
                assert!(!Arc::ptr_eq(&served, &other_version));
            }
        }

        #[test]
        fn should_not_serve_system_contracts_that_were_not_configured() {
            let provider = LocalContextProvider::new(PLATFORM_TYPE)
                .with_system_contracts([SystemDataContract::DPNS]);
            let version = PlatformVersion::latest();

            assert!(provider
                .get_data_contract(&SystemDataContract::Dashpay.id(), version)
                .unwrap()
                .is_none());
            assert!(provider
                .get_data_contract(&Identifier::from([0x33; 32]), version)
                .unwrap()
                .is_none());
        }

        /// A registered contract is served ahead of the system contracts and
        /// replaces an unknown answer the provider gave before.
        #[test]
        fn should_serve_a_registered_contract_after_reporting_it_unknown() {
            let version = PlatformVersion::latest();
            let dashpay =
                Arc::new(load_system_data_contract(SystemDataContract::Dashpay, version).unwrap());
            let provider = LocalContextProvider::new(PLATFORM_TYPE);
            assert!(provider
                .get_data_contract(&dashpay.id(), version)
                .unwrap()
                .is_none());

            provider.register_data_contract(Arc::clone(&dashpay));

            let served = provider
                .get_data_contract(&dashpay.id(), version)
                .unwrap()
                .expect("the registered contract");
            assert!(Arc::ptr_eq(&served, &dashpay));
        }

        #[test]
        fn should_prefer_a_registered_contract_over_the_system_one() {
            let version = PlatformVersion::latest();
            let registered =
                Arc::new(load_system_data_contract(SystemDataContract::DPNS, version).unwrap());
            let provider = LocalContextProvider::new(PLATFORM_TYPE)
                .with_system_contracts([SystemDataContract::DPNS]);
            provider.register_data_contract(Arc::clone(&registered));

            let served = provider
                .get_data_contract(&registered.id(), version)
                .unwrap()
                .unwrap();
            assert!(Arc::ptr_eq(&served, &registered));
        }
    }
}
