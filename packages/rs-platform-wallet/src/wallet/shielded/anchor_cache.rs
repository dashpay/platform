//! Short-lived cache of Platform's recorded shielded-anchor set.
//!
//! Every shielded spend must be built against a commitment-tree root that
//! Platform has recorded. Finding one means downloading and proof-verifying
//! the whole recorded set (`ShieldedAnchors`), a round trip that otherwise
//! sits serially in front of proving on every send. [`RecordedAnchorCache`]
//! keeps the most recent set so a send can skip it when a previous send or
//! the reservation-release pass just fetched one.
//!
//! # Why a cached set is safe to spend against
//!
//! Platform records at most one anchor per block, in the block-end
//! transaction that appends the block's notes, and prunes only by age (every
//! `shielded_anchor_pruning_interval` blocks it drops anchors older than
//! `shielded_anchor_retention_blocks`, always keeping the newest). So every
//! anchor in a recent set is still accepted unless it was already within the
//! TTL of the retention edge, a risk a fresh fetch shares because proving
//! takes comparable time. Two guards keep the cached set from selecting a
//! different anchor than a fresh fetch would:
//!
//! - **TTL** ([`RECORDED_ANCHOR_CACHE_TTL`]), measured from when the request
//!   was sent, on both the monotonic and the wall clock (the monotonic clock
//!   stops while a phone sleeps).
//! - **Tree-size pin**: a set is used only while the local tree still has the
//!   leaf count `N` read before its request was sent. Holding leaf `N - 1`
//!   means its block was committed, so every root of a tree of size `<= N`
//!   that Platform will ever record already was. Any tree growth may add a
//!   newer checkpoint the set lacks, so it retires the set.
//!
//! Both guards compare against a fresh fetch from an equally current node. A
//! node the SDK still accepts as up to date may lag a block behind and omit
//! the newest checkpoint; the cache then keeps that older, still recorded
//! anchor for up to the TTL, where a fresh fetch would retry the node's luck.
//!
//! A third guard ties the set to how it was verified. A request records the
//! SDK's context provider and its
//! [`policy_revision`](ContextProvider::policy_revision), which advances on
//! every policy change, such as a quorum mode switch, before it is sent. The set is stored only if
//! both are unchanged when it arrives, and returned only while they still
//! match. A provider that reports no revision, which is the default, is
//! never cached against, so a spend never reuses evidence that an earlier
//! verification policy accepted.
//!
//! An anchor pruned after the cached fetch can still be selected, where a
//! fresh fetch would already omit it. Platform then rejects the spend after
//! proving. That failure invalidates the cache; the notes stay reserved until
//! sync resolves the spend.
//!
//! The spend path also refetches when the cached set covers none of the
//! wallet's checkpoints. Failures during anchor selection, proving, or
//! broadcast handling invalidate the cache; earlier failures do not. The
//! stranded-reservation release pass never reads the cache (its fund-safety
//! argument needs a set fetched right before its note scan); it only fills it.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock, PoisonError, Weak};
use std::time::{Duration, Instant, SystemTime};

use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
use dash_sdk::platform::ContextProvider;

/// How long a fetched recorded-anchor set may be reused by the spend path.
///
/// At most a few percent of the 1000-block retention window even at one block
/// per second, and the same order as a spend's own proving time, so it does
/// not change which anchors can expire mid-send.
pub const RECORDED_ANCHOR_CACHE_TTL: Duration = Duration::from_secs(30);

/// Recorded-anchor set shared by the spend path. See the module docs.
#[derive(Debug)]
pub struct RecordedAnchorCache {
    ttl: Duration,
    sdk: OnceLock<Weak<dash_sdk::Sdk>>,
    state: Mutex<CacheState>,
}

/// What verifies the SDK's proofs: the installed context provider and its
/// policy revision, which advances on every policy change.
#[derive(Debug, Clone)]
struct ProofPolicy {
    provider: Weak<dyn ContextProvider>,
    revision: u64,
}

impl ProofPolicy {
    /// `None` when the provider reports no revision.
    fn of(sdk: &dash_sdk::Sdk) -> Option<Self> {
        let provider = sdk.context_provider_arc();
        Some(Self {
            revision: provider.policy_revision()?,
            provider: Arc::downgrade(&provider),
        })
    }
}

impl PartialEq for ProofPolicy {
    fn eq(&self, other: &Self) -> bool {
        self.provider.ptr_eq(&other.provider) && self.revision == other.revision
    }
}

/// The policy a set is bound to: its SDK's [`ProofPolicy`], or none for a
/// cache never bound with [`RecordedAnchorCache::for_sdk`].
#[derive(Debug, Clone, PartialEq)]
enum PolicyKey {
    Unbound,
    Bound(ProofPolicy),
}

#[derive(Debug, Default)]
struct CacheState {
    entry: Option<CachedAnchorSet>,
    /// When [`RecordedAnchorCache::invalidate`] last ran. Requests sent
    /// before it may carry the set it meant to discard, so they are dropped.
    invalidated_at: Option<Instant>,
}

#[derive(Debug, Clone)]
struct CachedAnchorSet {
    anchors: Arc<HashSet<[u8; 32]>>,
    /// When the request that produced `anchors` was sent. The set reflects
    /// Platform's state at some later instant, so ageing from here is
    /// conservative.
    fetch_started: FetchStamp,
    /// Local commitment-tree leaf count read before the request was sent.
    tree_size: u64,
    /// Proved height of this exact set, retained for pending-spend recovery.
    height: u64,
    policy: PolicyKey,
}

/// When a recorded-anchor request was sent, on two clocks.
///
/// `Instant` alone is not enough: on iOS / macOS it is an uptime clock and on
/// Android / Linux `CLOCK_MONOTONIC`, neither of which advances while the
/// device sleeps, so a set fetched before the phone was locked for hours
/// would still look seconds old. The wall clock keeps running through sleep
/// but can be set backwards. A set's age is the larger of the two readings,
/// and a wall clock that moved before the stamp counts as expired.
///
/// It also records the verification policy in force when the request was
/// sent, `None` when it could not be read.
#[derive(Debug, Clone)]
pub(crate) struct FetchStamp {
    monotonic: Instant,
    wall: SystemTime,
    policy: Option<PolicyKey>,
}

impl FetchStamp {
    /// Stamp a request to an unbound cache, about to be sent.
    #[cfg(test)]
    pub(crate) fn now() -> Self {
        Self {
            monotonic: Instant::now(),
            wall: SystemTime::now(),
            policy: Some(PolicyKey::Unbound),
        }
    }

    /// A stamp `age` in the past on both clocks.
    #[cfg(test)]
    pub(crate) fn ago(age: Duration) -> Self {
        Self {
            monotonic: Instant::now()
                .checked_sub(age)
                .expect("monotonic clock is past the requested age"),
            wall: SystemTime::now()
                .checked_sub(age)
                .expect("wall clock is past the requested age"),
            policy: Some(PolicyKey::Unbound),
        }
    }

    /// Whether this stamp is younger than `ttl` on both clocks.
    fn is_younger_than(&self, ttl: Duration) -> bool {
        self.monotonic.elapsed() < ttl && self.wall.elapsed().is_ok_and(|wall_age| wall_age < ttl)
    }
}

impl Default for RecordedAnchorCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RecordedAnchorCache {
    /// An empty cache with the default [`RECORDED_ANCHOR_CACHE_TTL`].
    pub fn new() -> Self {
        Self::with_ttl(RECORDED_ANCHOR_CACHE_TTL)
    }

    /// An empty cache with a shorter TTL, capped at
    /// [`RECORDED_ANCHOR_CACHE_TTL`]. A zero TTL disables reuse: every spend
    /// fetches the set, as it did before this cache existed.
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            ttl: ttl.min(RECORDED_ANCHOR_CACHE_TTL),
            sdk: OnceLock::new(),
            state: Mutex::new(CacheState::default()),
        }
    }

    /// Bind the cache to `sdk`, whose proofs every cached set must have been
    /// verified by. `None` when it is bound to another SDK.
    pub(crate) fn for_sdk(&self, sdk: &Arc<dash_sdk::Sdk>) -> Option<&Self> {
        let owner = self.sdk.get_or_init(|| Arc::downgrade(sdk));
        owner.ptr_eq(&Arc::downgrade(sdk)).then_some(self)
    }

    /// The policy in force now, `None` when it cannot be read or the bound
    /// SDK is gone.
    fn current_policy(&self) -> Option<PolicyKey> {
        match self.sdk.get() {
            None => Some(PolicyKey::Unbound),
            Some(sdk) => ProofPolicy::of(&*sdk.upgrade()?).map(PolicyKey::Bound),
        }
    }

    /// Stamp a request about to be sent.
    pub(crate) fn stamp(&self) -> FetchStamp {
        FetchStamp {
            monotonic: Instant::now(),
            wall: SystemTime::now(),
            policy: self.current_policy(),
        }
    }

    /// The cached set, if one is younger than the TTL, was fetched while the
    /// local tree had exactly `tree_size` leaves, and was verified under the
    /// policy in force now.
    pub(crate) fn get(&self, tree_size: u64) -> Option<(Arc<HashSet<[u8; 32]>>, u64)> {
        let policy = self.current_policy()?;
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state
            .entry
            .as_ref()
            .filter(|e| {
                e.tree_size == tree_size
                    && e.policy == policy
                    && e.fetch_started.is_younger_than(self.ttl)
            })
            .map(|e| (Arc::clone(&e.anchors), e.height))
    }

    /// Record a freshly fetched set. `fetch_started` is its request's
    /// [`stamp`](Self::stamp), `tree_size` the local tree's leaf count read
    /// before that, and `height` the proved height of this set.
    /// Concurrent fetches resolve to the one sent last. A fetch sent before
    /// the last [`invalidate`](Self::invalidate), or whose policy changed or
    /// could not be read between its stamp and now, is dropped.
    pub(crate) fn insert(
        &self,
        anchors: Arc<HashSet<[u8; 32]>>,
        fetch_started: FetchStamp,
        tree_size: u64,
        height: u64,
    ) {
        let Some(policy) = fetch_started.policy.clone() else {
            return;
        };
        if self.current_policy().as_ref() != Some(&policy) {
            return;
        }
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let sent = fetch_started.monotonic;
        if state.invalidated_at.is_some_and(|at| sent <= at)
            || state
                .entry
                .as_ref()
                .is_some_and(|current| current.fetch_started.monotonic > sent)
        {
            return;
        }
        state.entry = Some(CachedAnchorSet {
            anchors,
            fetch_started,
            tree_size,
            height,
            policy,
        });
    }

    /// Drop any cached set, and any still in flight; the next spend fetches
    /// a fresh one.
    pub fn invalidate(&self) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.entry = None;
        state.invalidated_at = Some(Instant::now());
    }
}

/// Fetch and proof-verify Platform's current recorded anchor set for the
/// credit shielded pool.
pub(crate) async fn fetch_recorded_anchor_set(
    sdk: &dash_sdk::Sdk,
) -> Result<(HashSet<[u8; 32]>, u64), dash_sdk::Error> {
    let (dash_sdk::query_types::ShieldedAnchors(anchors), metadata) =
        dash_sdk::query_types::ShieldedAnchors::fetch_current_with_metadata(sdk).await?;
    Ok((anchors.into_iter().collect(), metadata.height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dpp::data_contract::TokenConfiguration;
    use dash_sdk::dpp::prelude::{DataContract, Identifier};
    use dash_sdk::dpp::version::PlatformVersion;
    use dash_sdk::error::ContextProviderError;
    use dash_sdk::platform::QuorumKeySource;
    use std::sync::atomic::Ordering;

    fn set(bytes: &[u8]) -> Arc<HashSet<[u8; 32]>> {
        Arc::new(bytes.iter().map(|b| [*b; 32]).collect())
    }

    #[test]
    fn should_reject_a_cache_proved_by_another_sdk() {
        let first = Arc::new(dash_sdk::SdkBuilder::new_mock().build().unwrap());
        let other = Arc::new(dash_sdk::SdkBuilder::new_mock().build().unwrap());
        let cache = RecordedAnchorCache::new();
        assert!(cache.for_sdk(&first).is_some());
        cache.insert(set(&[1]), cache.stamp(), 10, 100);
        assert!(cache.for_sdk(&other).is_none());
        assert_eq!(cache.for_sdk(&first).unwrap().get(10).unwrap().1, 100);
        drop(first);
        assert!(cache.for_sdk(&other).is_none());
        assert!(cache.get(10).is_none(), "the proving SDK is gone");
    }

    #[derive(Default)]
    struct SwitchingProvider {
        revision: std::sync::atomic::AtomicU64,
        busy: std::sync::atomic::AtomicBool,
    }

    impl ContextProvider for SwitchingProvider {
        fn get_data_contract(
            &self,
            _: &Identifier,
            _: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            Ok(None)
        }

        fn get_token_configuration(
            &self,
            _: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Ok(None)
        }

        fn get_quorum_public_key(
            &self,
            _: u32,
            _: [u8; 32],
            _: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            Ok([0; 48])
        }

        fn get_platform_activation_height(&self) -> Result<u32, ContextProviderError> {
            Ok(0)
        }

        fn quorum_source(&self) -> Option<QuorumKeySource> {
            panic!("the cache must not probe source readiness")
        }

        fn policy_revision(&self) -> Option<u64> {
            (!self.busy.load(Ordering::SeqCst)).then(|| self.revision.load(Ordering::SeqCst))
        }
    }

    fn sdk_with(provider: Arc<SwitchingProvider>) -> Arc<dash_sdk::Sdk> {
        Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_context_provider_arc(provider)
                .build()
                .unwrap(),
        )
    }

    #[test]
    fn should_drop_a_set_verified_before_the_provider_changed_policy() {
        let provider = Arc::new(SwitchingProvider::default());
        let sdk = sdk_with(Arc::clone(&provider));
        let cache = RecordedAnchorCache::new();
        let cache = cache.for_sdk(&sdk).unwrap();
        cache.insert(set(&[1]), cache.stamp(), 10, 100);
        assert!(cache.get(10).is_some());

        provider.revision.fetch_add(1, Ordering::SeqCst);

        assert!(cache.get(10).is_none());
    }

    #[test]
    fn should_drop_a_set_verified_by_a_replaced_context_provider() {
        let sdk = sdk_with(Arc::new(SwitchingProvider::default()));
        let cache = RecordedAnchorCache::new();
        let cache = cache.for_sdk(&sdk).unwrap();
        cache.insert(set(&[1]), cache.stamp(), 10, 100);

        sdk.set_context_provider(SwitchingProvider::default())
            .unwrap();

        assert!(cache.get(10).is_none());
    }

    #[test]
    fn should_not_store_a_fetch_that_spanned_two_policy_changes() {
        let provider = Arc::new(SwitchingProvider::default());
        let sdk = sdk_with(Arc::clone(&provider));
        let cache = RecordedAnchorCache::new();
        let cache = cache.for_sdk(&sdk).unwrap();
        let sent = cache.stamp();

        provider.revision.fetch_add(2, Ordering::SeqCst);
        cache.insert(set(&[1]), sent, 10, 100);
        assert!(cache.get(10).is_none());

        cache.insert(set(&[2]), cache.stamp(), 10, 100);
        assert!(
            cache.get(10).is_some(),
            "a fetch sent after the change is stored"
        );
    }

    #[test]
    fn should_bypass_the_cache_while_the_policy_cannot_be_read() {
        let provider = Arc::new(SwitchingProvider::default());
        let sdk = sdk_with(Arc::clone(&provider));
        let cache = RecordedAnchorCache::new();
        let cache = cache.for_sdk(&sdk).unwrap();
        cache.insert(set(&[1]), cache.stamp(), 10, 100);

        provider.busy.store(true, Ordering::SeqCst);
        assert!(cache.get(10).is_none());
        let sent = cache.stamp();
        provider.busy.store(false, Ordering::SeqCst);
        cache.insert(set(&[2]), sent, 10, 100);

        let hit = cache.get(10).expect("the earlier set's policy still holds");
        assert!(hit.0.contains(&[1; 32]) && !hit.0.contains(&[2; 32]));
    }

    #[test]
    fn should_never_cache_against_a_mutex_wrapped_provider() {
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_context_provider_arc(Arc::new(Mutex::new(SwitchingProvider::default())))
                .build()
                .unwrap(),
        );
        let cache = RecordedAnchorCache::new();
        let cache = cache.for_sdk(&sdk).unwrap();
        cache.insert(set(&[1]), cache.stamp(), 10, 100);
        assert!(cache.get(10).is_none());

        let plain = sdk_with(Arc::new(SwitchingProvider::default()));
        let control = RecordedAnchorCache::new();
        let control = control.for_sdk(&plain).unwrap();
        control.insert(set(&[1]), control.stamp(), 10, 100);
        assert!(control.get(10).is_some());
    }

    #[test]
    fn should_drop_a_fetch_stamped_before_the_cache_was_bound() {
        let sdk = sdk_with(Arc::new(SwitchingProvider::default()));
        let cache = RecordedAnchorCache::new();
        let sent = cache.stamp();
        let cache = cache.for_sdk(&sdk).unwrap();
        cache.insert(set(&[1]), sent, 10, 100);
        assert!(cache.get(10).is_none());
    }

    #[test]
    fn should_cap_a_custom_ttl_at_the_default() {
        let cache = RecordedAnchorCache::with_ttl(Duration::from_secs(3600));
        cache.insert(set(&[1]), FetchStamp::ago(Duration::from_secs(31)), 10, 100);
        assert!(cache.get(10).is_none());
    }

    #[test]
    fn should_return_a_fresh_set_for_the_same_tree_size() {
        let cache = RecordedAnchorCache::new();
        assert!(cache.get(10).is_none(), "an empty cache has nothing");

        cache.insert(set(&[1, 2]), FetchStamp::now(), 10, 100);

        let hit = cache.get(10).expect("fresh set for the pinned tree size");
        assert!(hit.0.contains(&[1; 32]) && hit.0.contains(&[2; 32]));
        assert_eq!(hit.1, 100, "the set retains its own proved height");
    }

    #[test]
    fn should_miss_when_the_tree_grew_since_the_fetch() {
        let cache = RecordedAnchorCache::new();
        cache.insert(set(&[1]), FetchStamp::now(), 10, 100);

        assert!(
            cache.get(11).is_none(),
            "a sync appended after the fetch: the set may lack the newest checkpoint"
        );
    }

    #[test]
    fn should_expire_after_the_ttl() {
        let cache = RecordedAnchorCache::with_ttl(Duration::from_secs(30));
        cache.insert(set(&[1]), FetchStamp::ago(Duration::from_secs(31)), 10, 100);

        assert!(cache.get(10).is_none(), "never used past the TTL");
    }

    /// The monotonic clock stops while a phone sleeps: a set fetched before
    /// hours of sleep reads as seconds old on it. The wall clock must still
    /// expire it.
    #[test]
    fn should_expire_across_device_sleep_by_the_wall_clock() {
        let cache = RecordedAnchorCache::with_ttl(Duration::from_secs(30));
        let slept = FetchStamp {
            monotonic: Instant::now(),
            wall: SystemTime::now() - Duration::from_secs(3 * 60 * 60),
            ..FetchStamp::now()
        };
        cache.insert(set(&[1]), slept, 10, 100);

        assert!(cache.get(10).is_none(), "hours of wall time have passed");
    }

    /// A wall clock set backwards past the stamp gives no trustworthy age:
    /// treat the set as expired rather than as fresh.
    #[test]
    fn should_expire_when_the_wall_clock_moved_backwards() {
        let cache = RecordedAnchorCache::with_ttl(Duration::from_secs(30));
        let future_wall = FetchStamp {
            monotonic: Instant::now(),
            wall: SystemTime::now() + Duration::from_secs(60 * 60),
            ..FetchStamp::now()
        };
        cache.insert(set(&[1]), future_wall, 10, 100);

        assert!(cache.get(10).is_none());
    }

    #[test]
    fn should_never_hit_with_a_zero_ttl() {
        let cache = RecordedAnchorCache::with_ttl(Duration::ZERO);
        cache.insert(set(&[1]), FetchStamp::now(), 10, 100);

        assert!(cache.get(10).is_none());
    }

    #[test]
    fn should_keep_the_most_recently_sent_fetch() {
        let cache = RecordedAnchorCache::new();
        let now = FetchStamp::now();
        let earlier = FetchStamp::ago(Duration::from_secs(1));

        cache.insert(set(&[2]), now, 10, 100);
        // A slower request sent earlier completes afterwards: it must not
        // overwrite the newer snapshot.
        cache.insert(set(&[1]), earlier, 10, 100);

        let hit = cache.get(10).expect("newer set retained");
        assert!(hit.0.contains(&[2; 32]) && !hit.0.contains(&[1; 32]));
    }

    #[test]
    fn should_drop_the_set_on_invalidate() {
        let cache = RecordedAnchorCache::new();
        cache.insert(set(&[1]), FetchStamp::now(), 10, 100);

        cache.invalidate();

        assert!(cache.get(10).is_none());
    }

    /// A fetch sent before a failed spend invalidated the cache may return
    /// the very anchor that spend had rejected: it must not refill the cache.
    #[test]
    fn should_drop_a_fetch_sent_before_invalidate() {
        let cache = RecordedAnchorCache::new();
        let in_flight = FetchStamp::now();

        cache.invalidate();
        cache.insert(set(&[1]), in_flight, 10, 100);
        assert!(cache.get(10).is_none(), "stale in-flight fetch dropped");

        std::thread::sleep(Duration::from_millis(1));
        cache.insert(set(&[2]), FetchStamp::now(), 10, 100);
        assert!(cache.get(10).is_some(), "a fetch sent afterwards is kept");
    }
}
