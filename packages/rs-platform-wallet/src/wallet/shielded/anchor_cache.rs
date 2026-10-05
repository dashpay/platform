//! Short-lived cache of Platform's recorded shielded-anchor set.
//!
//! Every shielded spend must be built against a commitment-tree root that
//! Platform has recorded (`validate_anchor_exists`). The spend path learns
//! which of the wallet's checkpoint roots qualify by downloading and
//! proof-verifying the whole recorded set (`ShieldedAnchors`, up to
//! `shielded_anchor_retention_blocks + shielded_anchor_pruning_interval`
//! entries) — a network round trip that otherwise sits serially in front of
//! proving on every send.
//!
//! [`RecordedAnchorCache`] keeps the most recently fetched set so a send can
//! skip that round trip when a fresh one is already in hand (a send-screen
//! prefetch, a previous send, or the coordinator's reservation-release pass).
//!
//! # Why a cached set is safe to spend against
//!
//! Platform records at most one anchor per block and removes anchors only by
//! age: on every `shielded_anchor_pruning_interval` (100) boundary it drops
//! those recorded more than `shielded_anchor_retention_blocks` (1000) blocks
//! earlier, always keeping the newest. An anchor therefore stays accepted for
//! at least 1000 blocks after it is recorded, and every anchor in a set
//! fetched a moment ago is still recorded unless it was pruned since. An
//! anchor chosen from a cached set can only have been pruned if it was already
//! within the TTL of the retention edge when the set was fetched; an anchor
//! that close to expiry is at the same risk with a fresh fetch, because
//! proving takes comparable time between selecting it and broadcasting. Such a
//! rejected spend invalidates the cache, so the retry selects afresh.
//!
//! Two guards keep a cached set from ever selecting a different anchor than a
//! fresh fetch would:
//!
//! - **TTL** ([`RECORDED_ANCHOR_CACHE_TTL`]): a set is never used once it is
//!   older than the TTL, measured from when its request was *sent* on both
//!   the monotonic and the wall clock (the monotonic clock stops while an
//!   iOS / Android device sleeps; the wall clock does not).
//! - **Tree-size pin**: each set is tagged with the local commitment tree's
//!   leaf count `N` read *before* its request was sent, and is used only while
//!   the tree still has exactly `N` leaves. Holding leaf `N - 1` means the
//!   block that appended it was already committed, and Platform records a
//!   block's anchor in that same block-end transaction — so every root of a
//!   tree of size `≤ N` that Platform will ever record was already recorded
//!   when the request was sent. Hence, at size `N`, the cached set contains
//!   every checkpoint root a fresh fetch could find, except anchors pruned
//!   since. This rests only on the leaf count, not on when or under which
//!   lock the wallet created its checkpoints. Once a sync grows the tree, its
//!   new checkpoint may be recorded after the snapshot; the cached set would
//!   lack it and the probe would settle for an older checkpoint, so any
//!   growth retires the set.
//!
//! The spend path additionally refetches when the cached set covers none of
//! the wallet's checkpoints (`ShieldedNoRecordedAnchor`), and drops the cache
//! after any failed spend so a retry always works from a fresh set.
//!
//! The cache is never consulted by the stranded-reservation release pass: its
//! fund-safety argument needs a set fetched immediately before the note scan,
//! and an older set would lack anchors armed after it was fetched. That pass
//! only *populates* the cache.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;

/// How long a fetched recorded-anchor set may be reused by the spend path.
///
/// 30 s is a small slice of the retention window at any realistic block rate:
/// Tenderdash produces a block every 3 min when idle and, under load, no
/// faster than one consensus round (propose → prevote → precommit, on the
/// order of a second or more), so 30 s is at most a few percent of the
/// 1000-block retention. It is also the same order as the time a spend
/// already takes to prove between choosing its anchor and broadcasting, so it
/// does not change the class of anchors that can expire mid-send. Long enough
/// to cover "open the send screen → confirm" and back-to-back sends.
pub const RECORDED_ANCHOR_CACHE_TTL: Duration = Duration::from_secs(30);

/// Recorded-anchor set shared by the spend path. See the module docs.
#[derive(Debug)]
pub struct RecordedAnchorCache {
    ttl: Duration,
    entry: Mutex<Option<CachedAnchorSet>>,
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
            entry: Mutex::new(None),
        }
    }

    /// The cached set, if one is younger than the TTL and was fetched while
    /// the local tree had exactly `tree_size` leaves.
    pub(crate) fn get(&self, tree_size: u64) -> Option<Arc<HashSet<[u8; 32]>>> {
        let entry = self.entry.lock().unwrap_or_else(PoisonError::into_inner);
        entry
            .as_ref()
            .filter(|e| e.tree_size == tree_size && e.fetch_started.is_younger_than(self.ttl))
            .map(|e| Arc::clone(&e.anchors))
    }

    /// Record a freshly fetched set. `fetch_started` is when its request was
    /// sent and `tree_size` the local tree's leaf count read before that.
    /// Concurrent fetches resolve to the one sent last.
    pub(crate) fn insert(
        &self,
        anchors: Arc<HashSet<[u8; 32]>>,
        fetch_started: FetchStamp,
        tree_size: u64,
    ) {
        let mut entry = self.entry.lock().unwrap_or_else(PoisonError::into_inner);
        if entry
            .as_ref()
            .is_some_and(|current| current.fetch_started.monotonic > fetch_started.monotonic)
        {
            return;
        }
        *entry = Some(CachedAnchorSet {
            anchors,
            fetch_started,
            tree_size,
        });
    }

    /// Drop any cached set; the next spend fetches a fresh one.
    pub fn invalidate(&self) {
        *self.entry.lock().unwrap_or_else(PoisonError::into_inner) = None;
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
}
