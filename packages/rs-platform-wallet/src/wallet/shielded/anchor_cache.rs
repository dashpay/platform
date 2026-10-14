//! Short-lived cache of Platform's recorded shielded-anchor set.
//!
//! Every shielded spend must be built against a commitment-tree root that
//! Platform has recorded. Finding one means downloading and proof-verifying
//! the whole recorded set (`ShieldedAnchors`), a round trip that otherwise
//! sits serially in front of proving on every send. [`RecordedAnchorCache`]
//! keeps the most recent set so a send can skip it when a send-screen
//! prefetch, a previous send, or the reservation-release pass just fetched one.
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
//! The spend path also refetches when the cached set covers none of the
//! wallet's checkpoints, and drops the cache after any failed spend. The
//! stranded-reservation release pass never reads the cache (its fund-safety
//! argument needs a set fetched right before its note scan); it only fills it.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;

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
    state: Mutex<CacheState>,
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
}

/// When a recorded-anchor request was sent, on two clocks.
///
/// `Instant` alone is not enough: on iOS / macOS it is an uptime clock and on
/// Android / Linux `CLOCK_MONOTONIC`, neither of which advances while the
/// device sleeps, so a set fetched before the phone was locked for hours
/// would still look seconds old. The wall clock keeps running through sleep
/// but can be set backwards. A set's age is the larger of the two readings,
/// and a wall clock that moved before the stamp counts as expired.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FetchStamp {
    monotonic: Instant,
    wall: SystemTime,
}

impl FetchStamp {
    /// Stamp a request about to be sent.
    pub(crate) fn now() -> Self {
        Self {
            monotonic: Instant::now(),
            wall: SystemTime::now(),
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

    /// An empty cache with a custom TTL. A zero TTL disables reuse: every
    /// spend fetches the set, as it did before this cache existed.
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            ttl,
            state: Mutex::new(CacheState::default()),
        }
    }

    /// The cached set, if one is younger than the TTL and was fetched while
    /// the local tree had exactly `tree_size` leaves.
    pub(crate) fn get(&self, tree_size: u64) -> Option<Arc<HashSet<[u8; 32]>>> {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state
            .entry
            .as_ref()
            .filter(|e| e.tree_size == tree_size && e.fetch_started.is_younger_than(self.ttl))
            .map(|e| Arc::clone(&e.anchors))
    }

    /// Record a freshly fetched set. `fetch_started` is when its request was
    /// sent and `tree_size` the local tree's leaf count read before that.
    /// Concurrent fetches resolve to the one sent last, and a fetch sent
    /// before the last [`invalidate`](Self::invalidate) is dropped.
    pub(crate) fn insert(
        &self,
        anchors: Arc<HashSet<[u8; 32]>>,
        fetch_started: FetchStamp,
        tree_size: u64,
    ) {
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
) -> Result<HashSet<[u8; 32]>, dash_sdk::Error> {
    let dash_sdk::query_types::ShieldedAnchors(anchors) =
        dash_sdk::query_types::ShieldedAnchors::fetch_current(sdk).await?;
    Ok(anchors.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(bytes: &[u8]) -> Arc<HashSet<[u8; 32]>> {
        Arc::new(bytes.iter().map(|b| [*b; 32]).collect())
    }

    #[test]
    fn should_return_a_fresh_set_for_the_same_tree_size() {
        let cache = RecordedAnchorCache::new();
        assert!(cache.get(10).is_none(), "an empty cache has nothing");

        cache.insert(set(&[1, 2]), FetchStamp::now(), 10);

        let hit = cache.get(10).expect("fresh set for the pinned tree size");
        assert!(hit.contains(&[1; 32]) && hit.contains(&[2; 32]));
    }

    #[test]
    fn should_miss_when_the_tree_grew_since_the_fetch() {
        let cache = RecordedAnchorCache::new();
        cache.insert(set(&[1]), FetchStamp::now(), 10);

        assert!(
            cache.get(11).is_none(),
            "a sync appended after the fetch: the set may lack the newest checkpoint"
        );
    }

    #[test]
    fn should_expire_after_the_ttl() {
        let cache = RecordedAnchorCache::with_ttl(Duration::from_secs(30));
        cache.insert(set(&[1]), FetchStamp::ago(Duration::from_secs(31)), 10);

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
        };
        cache.insert(set(&[1]), slept, 10);

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
        };
        cache.insert(set(&[1]), future_wall, 10);

        assert!(cache.get(10).is_none());
    }

    #[test]
    fn should_never_hit_with_a_zero_ttl() {
        let cache = RecordedAnchorCache::with_ttl(Duration::ZERO);
        cache.insert(set(&[1]), FetchStamp::now(), 10);

        assert!(cache.get(10).is_none());
    }

    #[test]
    fn should_keep_the_most_recently_sent_fetch() {
        let cache = RecordedAnchorCache::new();
        let now = FetchStamp::now();
        let earlier = FetchStamp::ago(Duration::from_secs(1));

        cache.insert(set(&[2]), now, 10);
        // A slower request sent earlier completes afterwards: it must not
        // overwrite the newer snapshot.
        cache.insert(set(&[1]), earlier, 10);

        let hit = cache.get(10).expect("newer set retained");
        assert!(hit.contains(&[2; 32]) && !hit.contains(&[1; 32]));
    }

    #[test]
    fn should_drop_the_set_on_invalidate() {
        let cache = RecordedAnchorCache::new();
        cache.insert(set(&[1]), FetchStamp::now(), 10);

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
        cache.insert(set(&[1]), in_flight, 10);
        assert!(cache.get(10).is_none(), "stale in-flight fetch dropped");

        std::thread::sleep(Duration::from_millis(1));
        cache.insert(set(&[2]), FetchStamp::now(), 10);
        assert!(cache.get(10).is_some(), "a fetch sent afterwards is kept");
    }
}
