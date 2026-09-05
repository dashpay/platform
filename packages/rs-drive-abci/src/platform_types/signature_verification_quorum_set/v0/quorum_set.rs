use crate::config::{ChainLockConfig, QuorumLikeConfig};
use crate::platform_types::signature_verification_quorum_set::v0::quorums::Quorums;
use crate::platform_types::signature_verification_quorum_set::VerificationQuorum;
use dpp::dashcore::QuorumHash;
use dpp::dashcore_rpc::json::QuorumType;
use std::vec::IntoIter;

/// Offset for signature verification
pub const SIGN_OFFSET: u32 = 8;

/// Previously obtained quorums and heights. Required for signature verification
#[derive(Debug, Clone)]
pub(super) struct PreviousPastQuorumsV0 {
    pub(super) quorums: Quorums<VerificationQuorum>,

    /// The core height at which these quorums were last active
    pub(super) last_active_core_height: u32,

    /// The core height when the quorums were changed
    pub(super) updated_at_core_height: u32,

    /// The core height the previous chain lock validating quorums became active
    pub(super) previous_change_height: Option<u32>,
}

/// A borrowed view of the superseded quorums of a set, for callers outside this module.
pub struct PreviousPastQuorums<'q> {
    /// The superseded quorums
    pub quorums: &'q Quorums<VerificationQuorum>,
    /// The core height at which these quorums were last active
    pub last_active_core_height: u32,
    /// The core height at which the quorums were changed
    pub updated_at_core_height: u32,
    /// The core height at which the set before these became active
    pub previous_change_height: Option<u32>,
}

/// Quorums with keys for signature verification
#[derive(Debug, Clone)]
pub struct SignatureVerificationQuorumSetV0 {
    /// Quorum configuration
    pub(super) config: QuorumConfig,

    /// Current quorums
    pub(super) current_quorums: Quorums<VerificationQuorum>,

    /// The slightly old quorums used for validating ch ain locks (or instant locks), it's important to keep
    /// these because validation of signatures happens for the quorums that are 8 blocks before the
    /// height written in the chain lock. The same for instant locks
    pub(super) previous: Option<PreviousPastQuorumsV0>,
}

/// The trait defines methods for the signature verification quorums structure v0
pub trait SignatureVerificationQuorumSetV0Methods {
    /// Config
    fn config(&self) -> &QuorumConfig;

    /// Set current quorum keys
    fn set_current_quorums(&mut self, quorums: Quorums<VerificationQuorum>);

    /// Current quorum
    fn current_quorums(&self) -> &Quorums<VerificationQuorum>;

    /// Last quorum keys mutable
    fn current_quorums_mut(&mut self) -> &mut Quorums<VerificationQuorum>;

    /// Has previous quorums?
    fn has_previous_past_quorums(&self) -> bool;

    /// The superseded quorums and the core heights that bound their validity, if any.
    ///
    /// This history exists only in the platform state — it cannot be re-derived from Core
    /// — so it has to be readable to travel with a state sync snapshot.
    fn previous_past_quorums(&self) -> Option<PreviousPastQuorums<'_>>;

    /// Restores the superseded quorums verbatim, including the change height of the set
    /// before them.
    ///
    /// Unlike [`SignatureVerificationQuorumSetV0Methods::set_previous_past_quorums`], this
    /// does NOT derive `previous_change_height` from whatever this set currently holds: it
    /// is for reinstating a history that was captured elsewhere (state sync reconstruction),
    /// where deriving would silently produce a different one.
    fn restore_previous_past_quorums(
        &mut self,
        previous_quorums: Quorums<VerificationQuorum>,
        last_active_core_height: u32,
        updated_at_core_height: u32,
        previous_change_height: Option<u32>,
    );

    /// Set last quorums keys and update previous quorums
    fn replace_quorums(
        &mut self,
        quorums: Quorums<VerificationQuorum>,
        last_active_core_height: u32,
        updated_at_core_height: u32,
    );

    /// Update previous quorums
    fn set_previous_past_quorums(
        &mut self,
        previous_quorums: Quorums<VerificationQuorum>,
        last_active_core_height: u32,
        updated_at_core_height: u32,
    );

    /// Select quorums for signature verification based on sign and verification heights
    fn select_quorums(
        &self,
        signing_height: u32,
        verification_height: u32,
    ) -> SelectedQuorumSetIterator<'_>;
}

/// Iterator over selected quorum sets and specific quorums based on request_id and quorum configuration
#[derive(Clone)]
pub struct SelectedQuorumSetIterator<'q> {
    /// Quorum configuration
    config: &'q QuorumConfig,
    /// Appropriate quorum sets
    quorum_set: IntoIter<&'q Quorums<VerificationQuorum>>,
    /// Should we expect signature verification to be successful
    should_be_verifiable: bool,
}

impl<'q> Iterator for SelectedQuorumSetIterator<'q> {
    type Item = QuorumsWithConfig<'q>;

    fn next(&mut self) -> Option<Self::Item> {
        self.quorum_set.next().map(|quorums| QuorumsWithConfig {
            quorums,
            config: self.config,
        })
    }
}

/// Quorums with configuration
#[derive(Debug)]
pub struct QuorumsWithConfig<'q> {
    /// Quorums
    pub quorums: &'q Quorums<VerificationQuorum>,
    /// Config
    pub config: &'q QuorumConfig,
}

impl QuorumsWithConfig<'_> {
    /// Choose pseudorandom DIP8 or DIP24 quorum based on quorum config
    /// and request_id
    pub fn choose_quorum(
        &self,
        request_id: &[u8; 32],
    ) -> Option<(QuorumHash, &VerificationQuorum)> {
        self.quorums.choose_quorum(self.config, request_id)
    }
}

impl SelectedQuorumSetIterator<'_> {
    /// Number of quorum sets
    pub fn len(&self) -> usize {
        self.quorum_set.len()
    }

    /// Does the iterator have any quorum sets
    pub fn is_empty(&self) -> bool {
        self.quorum_set.len() == 0
    }

    /// Should we expect signature verification to be successful
    pub fn should_be_verifiable(&self) -> bool {
        self.should_be_verifiable
    }
}

/// Quorum configuration
#[derive(Debug, Clone)]
pub struct QuorumConfig {
    /// Type
    pub quorum_type: QuorumType,
    /// Active quorum signers count
    pub active_signers: u16,
    /// Is it a DIP24 rotating quorum or classic
    pub rotation: bool,
    /// DKG interval
    pub window: u32,
}

impl SignatureVerificationQuorumSetV0Methods for SignatureVerificationQuorumSetV0 {
    fn config(&self) -> &QuorumConfig {
        &self.config
    }

    fn set_current_quorums(&mut self, quorums: Quorums<VerificationQuorum>) {
        self.current_quorums = quorums;
    }

    fn current_quorums(&self) -> &Quorums<VerificationQuorum> {
        &self.current_quorums
    }

    fn current_quorums_mut(&mut self) -> &mut Quorums<VerificationQuorum> {
        &mut self.current_quorums
    }

    fn has_previous_past_quorums(&self) -> bool {
        self.previous.is_some()
    }

    fn previous_past_quorums(&self) -> Option<PreviousPastQuorums<'_>> {
        self.previous.as_ref().map(|previous| PreviousPastQuorums {
            quorums: &previous.quorums,
            last_active_core_height: previous.last_active_core_height,
            updated_at_core_height: previous.updated_at_core_height,
            previous_change_height: previous.previous_change_height,
        })
    }

    fn restore_previous_past_quorums(
        &mut self,
        previous_quorums: Quorums<VerificationQuorum>,
        last_active_core_height: u32,
        updated_at_core_height: u32,
        previous_change_height: Option<u32>,
    ) {
        self.previous = Some(PreviousPastQuorumsV0 {
            quorums: previous_quorums,
            last_active_core_height,
            updated_at_core_height,
            previous_change_height,
        });
    }

    fn replace_quorums(
        &mut self,
        quorums: Quorums<VerificationQuorum>,
        last_active_height: u32,
        updated_at_core_height: u32,
    ) {
        let previous_quorums = std::mem::replace(&mut self.current_quorums, quorums);

        self.set_previous_past_quorums(
            previous_quorums,
            last_active_height,
            updated_at_core_height,
        );
    }

    fn set_previous_past_quorums(
        &mut self,
        previous_quorums: Quorums<VerificationQuorum>,
        last_active_core_height: u32,
        updated_at_core_height: u32,
    ) {
        let previous_change_height = self
            .previous
            .as_ref()
            .map(|previous| previous.updated_at_core_height);

        self.previous = Some(PreviousPastQuorumsV0 {
            quorums: previous_quorums,
            last_active_core_height,
            updated_at_core_height,
            previous_change_height,
        });
    }

    fn select_quorums(
        &self,
        signing_height: u32,
        verification_height: u32,
    ) -> SelectedQuorumSetIterator<'_> {
        let mut quorums = Vec::new();
        let mut should_be_verifiable = false;

        if let Some(previous) = &self.previous {
            let previous_quorum_height = previous.last_active_core_height;
            let change_quorum_height = previous.updated_at_core_height;
            let previous_quorums_change_height = previous.previous_change_height;

            if signing_height > SIGN_OFFSET && verification_height >= change_quorum_height {
                // in this case we are sure that we should be targeting the current quorum
                // We updated core chain lock height from 100 to 105, new chain lock comes in for block 114
                //  ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height) ------ 106 (new chain lock verification height 114 - 8)
                // We are sure that we should use current quorums
                // If we have
                //  ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height) ------ 105 (new chain lock verification height 113 - 8)
                // We should also use current quorums, this is because at 105 we are sure new chain lock validating quorums are active
                quorums.push(&self.current_quorums);
                should_be_verifiable = true;
            } else if signing_height > SIGN_OFFSET && verification_height <= previous_quorum_height
            {
                should_be_verifiable = previous_quorums_change_height
                    .map(|previous_quorums_change_height| {
                        verification_height > previous_quorums_change_height
                    })
                    .unwrap_or(false);
                // In this case the quorums were changed recently meaning that we should use the previous quorums to verify the chain lock
                // We updated core chain lock height from 100 to 105, new chain lock comes in for block 106
                // -------- 98 (new chain lock verification height 106 - 8) ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height)
                // We are sure that we should use previous quorums
                // If we have
                // -------- 100 (new chain lock verification height 108 - 8) ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height)
                // We should also use previous quorums, this is because at 100 we are sure the old quorum set was active
                quorums.push(&previous.quorums);
            } else {
                should_be_verifiable = previous_quorums_change_height
                    .map(|previous_quorums_change_height| {
                        verification_height > previous_quorums_change_height
                    })
                    .unwrap_or(false);
                // we are in between, so we don't actually know if it was the old one or the new one to be used.
                //  ------- 100 (previous_quorum_height) ------ 104 (new chain lock verification height 112 - 8) -------105 (change_quorum_height)
                // we should just try both, starting with the current quorums
                quorums.push(&self.current_quorums);
                quorums.push(&previous.quorums);
            }
        } else {
            quorums.push(&self.current_quorums);
        }

        SelectedQuorumSetIterator {
            config: &self.config,
            quorum_set: quorums.into_iter(),
            should_be_verifiable,
        }
    }
}

impl SignatureVerificationQuorumSetV0 {
    /// New empty quorum set based on quorum configuration
    pub fn new(config: &impl QuorumLikeConfig) -> Self {
        SignatureVerificationQuorumSetV0 {
            config: QuorumConfig {
                quorum_type: config.quorum_type(),
                active_signers: config.quorum_active_signers(),
                rotation: config.quorum_rotation(),
                window: config.quorum_window(),
            },
            current_quorums: Quorums::default(),
            previous: None,
        }
    }
}

impl From<ChainLockConfig> for SignatureVerificationQuorumSetV0 {
    fn from(value: ChainLockConfig) -> Self {
        SignatureVerificationQuorumSetV0 {
            config: QuorumConfig {
                quorum_type: value.quorum_type,
                active_signers: value.quorum_active_signers,
                rotation: value.quorum_rotation,
                window: value.quorum_window,
            },
            current_quorums: Quorums::default(),
            previous: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ChainLockConfig;
    use dpp::bls_signatures::{Bls12381G2Impl, SecretKey as BlsPrivateKey};
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore_rpc::json::QuorumType;

    fn make_public_key(seed: u8) -> dpp::bls_signatures::PublicKey<Bls12381G2Impl> {
        let mut key_bytes = [0u8; 32];
        key_bytes[0] = seed;
        key_bytes[31] = 1;
        let sk =
            BlsPrivateKey::<Bls12381G2Impl>::from_be_bytes(&key_bytes).expect("valid secret key");
        sk.public_key()
    }

    fn make_verification_quorum(seed: u8, index: Option<u32>) -> VerificationQuorum {
        VerificationQuorum {
            index,
            public_key: make_public_key(seed),
        }
    }

    fn make_quorums(seeds: &[(u8, [u8; 32])]) -> Quorums<VerificationQuorum> {
        seeds
            .iter()
            .map(|(seed, hash_bytes)| {
                (
                    QuorumHash::from_byte_array(*hash_bytes),
                    make_verification_quorum(*seed, None),
                )
            })
            .collect()
    }

    fn default_chain_lock_config() -> ChainLockConfig {
        ChainLockConfig {
            quorum_type: QuorumType::Llmq400_60,
            quorum_size: 400,
            quorum_window: 288,
            quorum_active_signers: 4,
            quorum_rotation: false,
        }
    }

    // ---- Construction ----

    #[test]
    fn new_from_quorum_like_config() {
        let config = default_chain_lock_config();
        let qs = SignatureVerificationQuorumSetV0::new(&config);

        assert_eq!(qs.config().quorum_type, QuorumType::Llmq400_60);
        assert_eq!(qs.config().active_signers, 4);
        assert!(!qs.config().rotation);
        assert_eq!(qs.config().window, 288);
        assert!(qs.current_quorums().is_empty());
        assert!(!qs.has_previous_past_quorums());
    }

    #[test]
    fn from_chain_lock_config() {
        let config = ChainLockConfig {
            quorum_type: QuorumType::Llmq100_67,
            quorum_size: 100,
            quorum_window: 24,
            quorum_active_signers: 24,
            quorum_rotation: true,
        };
        let qs: SignatureVerificationQuorumSetV0 = config.into();

        assert_eq!(qs.config().quorum_type, QuorumType::Llmq100_67);
        assert_eq!(qs.config().active_signers, 24);
        assert!(qs.config().rotation);
        assert_eq!(qs.config().window, 24);
    }

    // ---- set_current_quorums / current_quorums ----

    #[test]
    fn set_and_get_current_quorums() {
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        let quorums = make_quorums(&[(1, [1u8; 32]), (2, [2u8; 32])]);
        qs.set_current_quorums(quorums);

        assert_eq!(qs.current_quorums().len(), 2);
    }

    #[test]
    fn current_quorums_mut_allows_insert() {
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        let hash = QuorumHash::from_byte_array([10u8; 32]);
        qs.current_quorums_mut()
            .insert(hash, make_verification_quorum(10, None));

        assert_eq!(qs.current_quorums().len(), 1);
        assert!(qs.current_quorums().contains_key(&hash));
    }

    // ---- has_previous_past_quorums ----

    #[test]
    fn has_previous_past_quorums_initially_false() {
        let config = default_chain_lock_config();
        let qs = SignatureVerificationQuorumSetV0::new(&config);
        assert!(!qs.has_previous_past_quorums());
    }

    // ---- set_previous_past_quorums ----

    #[test]
    fn set_previous_past_quorums_makes_has_previous_true() {
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        let prev_quorums = make_quorums(&[(1, [1u8; 32])]);
        qs.set_previous_past_quorums(prev_quorums, 100, 105);

        assert!(qs.has_previous_past_quorums());
    }

    #[test]
    fn set_previous_past_quorums_tracks_previous_change_height() {
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        // First call: previous_change_height should be None because there was no prior previous
        let q1 = make_quorums(&[(1, [1u8; 32])]);
        qs.set_previous_past_quorums(q1, 90, 100);

        // Second call: previous_change_height should be Some(100) from the first call
        let q2 = make_quorums(&[(2, [2u8; 32])]);
        qs.set_previous_past_quorums(q2, 100, 110);

        assert!(qs.has_previous_past_quorums());
        // We verify indirectly via select_quorums behavior
    }

    // ---- replace_quorums ----

    #[test]
    fn replace_quorums_moves_current_to_previous() {
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        let initial = make_quorums(&[(1, [1u8; 32])]);
        qs.set_current_quorums(initial);
        assert!(!qs.has_previous_past_quorums());

        let replacement = make_quorums(&[(2, [2u8; 32])]);
        qs.replace_quorums(replacement, 100, 105);

        assert!(qs.has_previous_past_quorums());
        // Current quorums should be the replacement
        assert_eq!(qs.current_quorums().len(), 1);
        assert!(qs
            .current_quorums()
            .contains_key(&QuorumHash::from_byte_array([2u8; 32])));
    }

    #[test]
    fn replace_quorums_twice_updates_previous_change_height() {
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        let q1 = make_quorums(&[(1, [1u8; 32])]);
        qs.set_current_quorums(q1);

        let q2 = make_quorums(&[(2, [2u8; 32])]);
        qs.replace_quorums(q2, 90, 100);

        let q3 = make_quorums(&[(3, [3u8; 32])]);
        qs.replace_quorums(q3, 100, 110);

        // After two replacements, current should be q3, previous should contain q2,
        // and the previous_change_height inside previous should be Some(100).
        assert_eq!(qs.current_quorums().len(), 1);
        assert!(qs
            .current_quorums()
            .contains_key(&QuorumHash::from_byte_array([3u8; 32])));
        assert!(qs.has_previous_past_quorums());
    }

    // ---- select_quorums ----

    #[test]
    fn select_quorums_no_previous_returns_current_only() {
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        let current = make_quorums(&[(1, [1u8; 32])]);
        qs.set_current_quorums(current);

        let iter = qs.select_quorums(20, 10);
        assert_eq!(iter.len(), 1);
        assert!(!iter.should_be_verifiable());
    }

    #[test]
    fn select_quorums_verification_above_change_height_returns_current_and_verifiable() {
        // Scenario from code comments:
        //  ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height) ------ 106 (verification_height)
        // signing_height must be > SIGN_OFFSET (8)
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        let initial = make_quorums(&[(1, [1u8; 32])]);
        qs.set_current_quorums(initial);

        let replacement = make_quorums(&[(2, [2u8; 32])]);
        qs.replace_quorums(replacement, 100, 105);

        // signing_height=114, verification_height=106 >= change_quorum_height=105
        let iter = qs.select_quorums(114, 106);
        assert_eq!(iter.len(), 1);
        assert!(iter.should_be_verifiable());
    }

    #[test]
    fn select_quorums_verification_at_change_height_returns_current_and_verifiable() {
        // verification_height == change_quorum_height
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        let initial = make_quorums(&[(1, [1u8; 32])]);
        qs.set_current_quorums(initial);

        let replacement = make_quorums(&[(2, [2u8; 32])]);
        qs.replace_quorums(replacement, 100, 105);

        let iter = qs.select_quorums(113, 105);
        assert_eq!(iter.len(), 1);
        assert!(iter.should_be_verifiable());
    }

    #[test]
    fn select_quorums_verification_below_previous_height_returns_previous() {
        // Scenario:
        // -------- 98 (verification_height) ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height)
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        let initial = make_quorums(&[(1, [1u8; 32])]);
        qs.set_current_quorums(initial);

        let replacement = make_quorums(&[(2, [2u8; 32])]);
        qs.replace_quorums(replacement, 100, 105);

        // signing_height=106, verification_height=98 <= previous_quorum_height=100
        let iter = qs.select_quorums(106, 98);
        assert_eq!(iter.len(), 1);
        // should_be_verifiable is false because previous_change_height is None
        assert!(!iter.should_be_verifiable());
    }

    #[test]
    fn select_quorums_verification_at_previous_height_returns_previous() {
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        let initial = make_quorums(&[(1, [1u8; 32])]);
        qs.set_current_quorums(initial);

        let replacement = make_quorums(&[(2, [2u8; 32])]);
        qs.replace_quorums(replacement, 100, 105);

        // verification_height == previous_quorum_height
        let iter = qs.select_quorums(108, 100);
        assert_eq!(iter.len(), 1);
        assert!(!iter.should_be_verifiable());
    }

    #[test]
    fn select_quorums_verification_between_previous_and_change_returns_both() {
        // Scenario:
        //  ------- 100 (previous_quorum_height) ------ 104 (verification_height) -------105 (change_quorum_height)
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        let initial = make_quorums(&[(1, [1u8; 32])]);
        qs.set_current_quorums(initial);

        let replacement = make_quorums(&[(2, [2u8; 32])]);
        qs.replace_quorums(replacement, 100, 105);

        // verification_height=104, between 100 and 105
        let iter = qs.select_quorums(112, 104);
        assert_eq!(iter.len(), 2);
        assert!(!iter.should_be_verifiable());
    }

    #[test]
    fn select_quorums_signing_at_or_below_offset_with_previous() {
        // When signing_height <= SIGN_OFFSET, none of the first two branches match
        // (both require signing_height > SIGN_OFFSET), so we fall to the else
        // which pushes both current and previous.
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        let initial = make_quorums(&[(1, [1u8; 32])]);
        qs.set_current_quorums(initial);

        let replacement = make_quorums(&[(2, [2u8; 32])]);
        qs.replace_quorums(replacement, 100, 105);

        // signing_height == SIGN_OFFSET (8), not > SIGN_OFFSET
        let iter = qs.select_quorums(SIGN_OFFSET, 106);
        assert_eq!(iter.len(), 2);
    }

    #[test]
    fn select_quorums_verifiable_with_previous_change_height() {
        // When there's a previous_change_height (from two replacements),
        // should_be_verifiable depends on verification_height > previous_change_height.
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        let q1 = make_quorums(&[(1, [1u8; 32])]);
        qs.set_current_quorums(q1);

        // First replacement: creates previous with previous_change_height = None
        let q2 = make_quorums(&[(2, [2u8; 32])]);
        qs.replace_quorums(q2, 90, 100);

        // Second replacement: creates previous with previous_change_height = Some(100)
        let q3 = make_quorums(&[(3, [3u8; 32])]);
        qs.replace_quorums(q3, 100, 110);

        // Case: verification_height (95) <= previous_quorum_height (100),
        // and 95 < previous_change_height (100), so NOT verifiable
        let iter = qs.select_quorums(106, 95);
        assert_eq!(iter.len(), 1); // previous quorums only
        assert!(!iter.should_be_verifiable());

        // Case: verification_height (101) > previous_change_height (100), so verifiable
        // and 101 between previous_quorum_height(100) and change_quorum_height(110)
        let iter2 = qs.select_quorums(112, 101);
        assert_eq!(iter2.len(), 2); // both current and previous
        assert!(iter2.should_be_verifiable());
    }

    // ---- SelectedQuorumSetIterator ----

    #[test]
    fn selected_quorum_set_iterator_len_and_is_empty() {
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        let current = make_quorums(&[(1, [1u8; 32])]);
        qs.set_current_quorums(current);

        let iter = qs.select_quorums(20, 10);
        assert_eq!(iter.len(), 1);
        assert!(!iter.is_empty());
    }

    #[test]
    fn selected_quorum_set_iterator_iteration() {
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        let current = make_quorums(&[(1, [1u8; 32])]);
        qs.set_current_quorums(current);

        let replacement = make_quorums(&[(2, [2u8; 32])]);
        qs.replace_quorums(replacement, 100, 105);

        // Get both quorum sets by falling into the "between" branch
        let iter = qs.select_quorums(112, 104);
        let items: Vec<_> = iter.collect();
        assert_eq!(items.len(), 2);
        // Each item should have a reference to the config
        for item in &items {
            assert_eq!(item.config.quorum_type, QuorumType::Llmq400_60);
        }
    }

    // ---- QuorumsWithConfig::choose_quorum ----

    #[test]
    fn quorums_with_config_choose_quorum_delegates() {
        let config = default_chain_lock_config();
        let mut qs = SignatureVerificationQuorumSetV0::new(&config);

        let current = make_quorums(&[(1, [1u8; 32])]);
        qs.set_current_quorums(current);

        let mut iter = qs.select_quorums(20, 10);
        let quorums_with_config = iter.next().unwrap();

        let request_id = [0u8; 32];
        let result = quorums_with_config.choose_quorum(&request_id);
        assert!(result.is_some());
    }

    // ---- SIGN_OFFSET constant ----

    #[test]
    fn sign_offset_is_8() {
        assert_eq!(SIGN_OFFSET, 8);
    }
}
