//! Proving Platform quorum keys with proofs fetched from relays.
//!
//! A key is believed only after a `DASHNC02` bootstrap proof, checked from a
//! state this provider already trusts, opens the quorum's commitment under a
//! ChainLocked quorum root. Relays are untrusted byte sources: one that lies,
//! stalls or fails costs time, bounded here, but never changes a key.
//!
//! A proven key is immutable: under ChainLocks each quorum has one mined
//! commitment. What a proof adds is evidence of when the quorum was active.
//! Platform quorums do not rotate: the active set at a height is the newest
//! quorums mined by it, so a quorum enters the set when it is mined and never
//! returns once it leaves. A quorum opened under the quorum roots of two
//! verified targets was therefore active at every height between them. Every
//! target is proven from one checkpoint through ChainLocks, so all of them lie
//! on one chain.
//!
//! A key is served for a Core height `H` when a verified target at or below
//! `H` opened the quorum, so it was mined by `H`, and one at or above `H`
//! minus one DKG interval did too, so it was active at some height in that
//! last interval up to `H`. A target above `H` is never evidence that the
//! quorum existed at `H`. The grace above the highest target is for Drive,
//! which rotates its validator set only at the end of the first block whose
//! Core height drops the current quorum
//! (`rs-drive-abci/src/execution/platform_events/block_end/validator_set_update/v2/mod.rs`):
//! that block reports a height at which Core has just rotated its signing
//! quorum out.
//!
//! The SDK treats any error from a fetch as "the source gave no answer", and
//! never bans a node for it. A fetch therefore never reports a quorum as
//! absent: an inclusion proof cannot show absence, and a relay's "not found"
//! is unauthenticated.

use crate::proof_transport::{PostError, ProofTransport, Reply};
#[cfg(target_arch = "wasm32")]
use crate::quorum_refresh::on_local_executor;
#[cfg(not(target_arch = "wasm32"))]
use crate::quorum_refresh::panic_message;
#[cfg(not(target_arch = "wasm32"))]
use crate::quorum_refresh::sleep;
#[cfg(not(target_arch = "wasm32"))]
use crate::quorum_refresh::sleep as caller_sleep;
use dash_context_provider::ContextProviderError;
use dash_core_proof::bootstrap::{self, RecordKind};
use dash_core_proof::{parse_commitment, rpc_hex, ErrorKind, State};
#[cfg(target_arch = "wasm32")]
use futures::channel::oneshot;
use futures::future::{self, select, BoxFuture, Either, FutureExt, Shared, WeakShared};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::sleep;
use lru::LruCache;
use std::collections::BTreeMap;
use std::future::Future;
use std::num::NonZeroUsize;
#[cfg(not(target_arch = "wasm32"))]
use std::panic::AssertUnwindSafe;
use std::pin::pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use url::Url;
use web_time::Instant;

/// A quorum's BLS public key.
pub(crate) type QuorumKey = [u8; 48];

/// A quorum: its LLMQ type and its hash in the order the SDK uses (Core's RPC
/// display order).
pub(crate) type QuorumId = (u32, [u8; 32]);

/// Most requests one fetch sends, across sources and retries. Once a relay
/// cannot serve the first ask, the last one is kept for the lower ask (see
/// [`choose_ask`]) until that is sent, so that it is reached with any number
/// of sources.
const MAX_REQUESTS_PER_FETCH: usize = 8;
/// Most requests of one fetch in flight at once: the first and one hedge.
const MAX_REQUESTS_IN_FLIGHT: usize = 2;
/// Most distinct quorums being proven at once. Each costs a relay a Core
/// call, and the quorums come from unverified responses.
const MAX_QUORUMS_IN_FLIGHT: usize = 2;
/// Most recently failed fetches remembered for their cooldowns. Running
/// fetches have their own bound.
const MAX_FAILED_FETCHES: usize = 64;
/// Most fetches running at once, waiting ones included. A fetch gives its
/// proving place up while it waits, so without this bound fetches of fake
/// quorums could pile up; past it, new quorums are refused until one ends.
const MAX_RUNNING_FETCHES: usize = 16;
/// Verified states kept besides the checkpoint, to start later proofs from.
const MAX_RECENT_STATES: usize = 16;
/// The delay before asking a busy relay again doubles this many times.
const MAX_RETRY_DOUBLINGS: u32 = 3;
/// Answers a source may give that it cannot serve the quorum before a fetch
/// stops asking it: one for each of the two asks [`choose_ask`] alternates
/// between.
const MAX_CANNOT_SERVE: u32 = 2;
/// Busy answers in a row after which a fetch stops asking a source. A relay
/// that stays busy is not cooled down for other fetches, but it must not keep
/// one running, and holding a running slot, for its whole deadline.
const MAX_BUSY: u32 = 3;
/// One DKG interval of the Platform quorum types (mainnet type 4, testnet
/// type 6), in Core blocks. A key is served this far above the highest
/// target that opened it, for the block on which Drive rotates away from a
/// quorum Core has just retired. A relay asked for a height may answer with
/// a later target when the height has no ChainLock of its own; it is then
/// asked for a target at most this far below the height, which serves it.
pub(crate) const DKG_INTERVAL_BLOCKS: u32 = 24;
/// How long a fetch waits before trying again to take a place to prove its
/// quorum.
const PLACE_RETRY: Duration = Duration::from_millis(250);
/// How much of an error response's body goes into the failure reason.
const MAX_REASON_BODY_CHARS: usize = 120;

/// Deadlines, delays and cooldowns of fetches.
#[derive(Clone, Debug)]
pub(crate) struct Timing {
    /// Overall deadline of one fetch.
    pub(crate) deadline: Duration,
    /// Deadline of one request. Relays give Core 60 s for a proof.
    pub(crate) request_timeout: Duration,
    /// How long a request may go unanswered before the next source is asked
    /// as well.
    pub(crate) hedge_after: Duration,
    /// Delay before asking a relay again after it could not serve a quorum,
    /// and the first delay after it was busy.
    pub(crate) first_retry: Duration,
    /// First wait before a quorum whose fetch failed is fetched again.
    pub(crate) quorum_cooldown: Duration,
    /// Longest such wait.
    pub(crate) max_quorum_cooldown: Duration,
    /// First wait before a failing source is asked again.
    pub(crate) source_cooldown: Duration,
    /// Longest such wait.
    pub(crate) max_source_cooldown: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            deadline: Duration::from_secs(70),
            request_timeout: Duration::from_secs(65),
            hedge_after: Duration::from_secs(5),
            first_retry: Duration::from_secs(1),
            quorum_cooldown: Duration::from_secs(5),
            max_quorum_cooldown: Duration::from_secs(60),
            source_cooldown: Duration::from_secs(10),
            max_source_cooldown: Duration::from_secs(120),
        }
    }
}

/// `first`, doubled for every failure after the first, capped at `max`.
fn backoff(first: Duration, max: Duration, failures: u32) -> Duration {
    let doublings = failures.saturating_sub(1).min(16);
    first.saturating_mul(1 << doublings).min(max)
}

/// A proven key and the lowest and highest verified targets whose quorum root
/// opened it. The quorum was active at every height from `lo` to `hi`. The
/// key is served from `lo` to one DKG interval past `hi`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cached {
    key: QuorumKey,
    lo: u32,
    hi: u32,
}

impl Cached {
    /// The highest Core height the key is served for.
    fn last_served(&self) -> u32 {
        self.hi.saturating_add(DKG_INTERVAL_BLOCKS)
    }

    /// Whether the key is served for `height`: the quorum was mined by it and
    /// active within one DKG interval before it. Every check of a cached or
    /// fetched key goes through this.
    fn serves(&self, height: u32) -> bool {
        (self.lo..=self.last_served()).contains(&height)
    }
}

/// What a fetch hands every caller waiting on it: the key it proved and the
/// heights its proofs serve, so that a caller does not depend on the cache
/// still holding it.
type FetchResult = Result<Cached, String>;
type FetchFuture = BoxFuture<'static, FetchResult>;
type SharedFetch = Shared<FetchFuture>;

/// A running or recently failed fetch of a quorum, for one height.
enum Slot {
    Running {
        id: u64,
        height: u32,
        /// Set while the fetch holds one of the places to prove a quorum.
        holding: Arc<AtomicBool>,
        /// Weak, because the fetch holds the provider's state: only its
        /// waiters keep it alive. Natively it runs only while they poll it;
        /// in a browser a local task drives it to its deadline.
        fetch: WeakShared<FetchFuture>,
    },
    Failed {
        at: Instant,
        failures: u32,
        height: u32,
    },
}

#[derive(Default)]
struct SourceHealth {
    failed_until: Option<Instant>,
    failures: u32,
}

/// Everything fetches change, behind one lock that is never held across an
/// await.
struct Proven {
    keys: LruCache<QuorumId, Cached>,
    /// The checkpoint and the most recent verified targets, by height.
    states: BTreeMap<u32, State>,
    fetches: Vec<(QuorumId, Slot)>,
    sources: Vec<SourceHealth>,
    next_fetch_id: u64,
}

/// What to ask a relay for: a proof from `checkpoint` to a target at or above
/// `height`, where 0 asks for Core's latest ChainLock.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Ask {
    checkpoint: State,
    height: u32,
}

/// Choose what to ask a relay for when proving a quorum for `height`.
///
/// With `alternate`, the relay is asked for a target [`DKG_INTERVAL_BLOCKS`]
/// below `height`, from the newest state below that target, so a quorum that
/// retired between that target and `height` is reached too. Any such target that opens the
/// quorum serves `height`, even on the block where Drive still signs with a
/// quorum Core has just retired. When no state is kept below that target,
/// the lowest target above the newest state below `height` is asked for.
///
/// `extending` says the cache holds the quorum from a target at or below
/// `height` that is more than one DKG interval old, so any target at or above
/// `height` minus that interval serves it. The relay is then asked for its
/// tip, which also serves the next heights Platform reports, or the lower
/// target. The tip is asked for from the newest verified state below
/// `height`, so the proof is as short as it can be, but not from the
/// checkpoint once a later state is verified: from the latest state instead.
///
/// Otherwise a target at or below `height` is needed: the relay is asked for
/// `height`, or the lower target. Only a state below `height` can lead there,
/// the checkpoint included. There is no such ask when no verified state is
/// below `height`.
fn choose_ask(
    states: &BTreeMap<u32, State>,
    checkpoint_height: u32,
    height: u32,
    extending: bool,
    alternate: bool,
) -> Option<Ask> {
    let below = states.range(..height).next_back();
    if alternate {
        // A quorum retired since `lower` is in no quorum root above it.
        let lower = height.saturating_sub(DKG_INTERVAL_BLOCKS);
        if let Some((&from, state)) = states.range(..lower).next_back().or(below) {
            return Some(Ask {
                checkpoint: state.clone(),
                height: lower.max(from + 1),
            });
        }
    }
    if !extending {
        let (_, state) = below?;
        return Some(Ask {
            checkpoint: state.clone(),
            height,
        });
    }
    let (_, latest) = states.last_key_value()?;
    Some(match below {
        Some((&from, state)) if from != checkpoint_height || latest.height == from => Ask {
            checkpoint: state.clone(),
            height: 0,
        },
        _ => Ask {
            checkpoint: latest.clone(),
            height: 0,
        },
    })
}

/// How one answer from a relay counts.
enum Verdict {
    Proven(Cached),
    /// The relay cannot serve this quorum for the height asked: a 502, or a
    /// valid proof that does not serve the request, such as one whose target
    /// is above the height. Asked again once, for the other height, after a
    /// delay; then left out of the fetch.
    CannotServe(String),
    /// The relay's workers are busy: ask again after a backoff.
    Busy(String),
    /// The source failed and cools down for every fetch.
    SourceFailed(String),
    /// A proof of the relay's latest ChainLock does not reach the height:
    /// the relay is left out of this fetch.
    BeyondTip(String),
}

/// One fetch: which quorum, for which height, and its slot.
struct Attempt {
    quorum: QuorumId,
    height: u32,
    id: u64,
    /// Earlier failures of this quorum, for its cooldown, and when the last
    /// one was.
    failures: u32,
    failed_at: Option<Instant>,
    holding: Arc<AtomicBool>,
}

/// What one fetch knows about one source.
#[derive(Clone)]
struct SourceAttempts {
    asked: u32,
    in_flight: bool,
    ready_at: Instant,
    cannot_serve: u32,
    busy: u32,
    /// Ask for the lower height next time, rather than the tip or the
    /// height. See [`choose_ask`].
    alternate: bool,
    dropped: bool,
}

impl SourceAttempts {
    fn new(now: Instant) -> Self {
        Self {
            asked: 0,
            in_flight: false,
            ready_at: now,
            cannot_serve: 0,
            busy: 0,
            alternate: false,
            dropped: false,
        }
    }
}

/// What a fetch does next, given what the cache holds for its quorum.
enum NextAsk {
    Ask(Ask),
    /// The quorum is proven for the height already.
    Proven(Cached),
    /// No proof can serve the height.
    Unreachable,
}

/// The source a fetch asks next.
enum Pick {
    Ready(usize),
    /// None is ready before this time.
    At(Instant),
    /// None is left: all are cooling down, left out or already asked.
    Nothing,
}

/// Proves quorum keys and caches them, for a provider and its clones.
pub(crate) struct ProofFetches {
    quorum_type: u32,
    window: u32,
    checkpoint: State,
    endpoints: Vec<Url>,
    transport: ProofTransport,
    timing: Timing,
    proven: Mutex<Proven>,
}

impl ProofFetches {
    /// Prove keys of `quorum_type` quorums from `checkpoint` with proofs
    /// from `endpoints`. A caller may wait on a running fetch of a quorum for
    /// a height at most `window` from its own.
    pub(crate) fn new(
        quorum_type: u32,
        window: u32,
        checkpoint: State,
        endpoints: Vec<Url>,
        cache_size: NonZeroUsize,
        transport: ProofTransport,
        timing: Timing,
    ) -> Self {
        let proven = Proven {
            keys: LruCache::new(cache_size),
            states: BTreeMap::from([(checkpoint.height, checkpoint.clone())]),
            fetches: Vec::new(),
            sources: endpoints.iter().map(|_| SourceHealth::default()).collect(),
            next_fetch_id: 0,
        };
        Self {
            quorum_type,
            window,
            checkpoint,
            endpoints,
            transport,
            timing,
            proven: Mutex::new(proven),
        }
    }

    /// The checkpoint proofs start from until a later state is verified.
    pub(crate) fn checkpoint(&self) -> &State {
        &self.checkpoint
    }

    /// The highest verified state, or the checkpoint.
    pub(crate) fn latest_state(&self) -> State {
        self.lock()
            .states
            .last_key_value()
            .map_or_else(|| self.checkpoint.clone(), |(_, state)| state.clone())
    }

    /// How many quorum keys are cached.
    pub(crate) fn cached_quorum_count(&self) -> usize {
        self.lock().keys.len()
    }

    /// Whether this provider ever proves the key of this quorum for this
    /// height: only Platform quorums, at heights Core can have.
    pub(crate) fn provable(&self, quorum_type: u32, height: u32) -> bool {
        quorum_type == self.quorum_type && height <= i32::MAX as u32
    }

    /// The cached key of a quorum, if verified proofs serve `height`.
    pub(crate) fn lookup(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        height: u32,
    ) -> Result<QuorumKey, ContextProviderError> {
        if !self.provable(quorum_type, height) {
            return Err(ContextProviderError::InvalidQuorum(format!(
                "quorum type {quorum_type} at core height {height} is not a Platform quorum this \
                 provider verifies (type {})",
                self.quorum_type
            )));
        }
        let mut proven = self.lock();
        match proven.keys.get(&(quorum_type, quorum_hash)) {
            Some(cached) if cached.serves(height) => Ok(cached.key),
            Some(cached) => Err(ContextProviderError::QuorumSourceUnavailable(format!(
                "quorum {} is proven for core heights {} to {}, not yet for {height}",
                hex::encode(quorum_hash),
                cached.lo,
                cached.last_served()
            ))),
            None => Err(ContextProviderError::QuorumSourceUnavailable(format!(
                "quorum {} is not proven yet",
                hex::encode(quorum_hash)
            ))),
        }
    }

    /// Prove a quorum's key for `height`, or join the fetch already proving
    /// it, and return it. Every failure is
    /// [`ContextProviderError::QuorumSourceUnavailable`]; this never reports
    /// the quorum as absent.
    ///
    /// A caller may first join a fetch of the quorum for another height near
    /// its own, whose proofs may not serve the caller's height, or which may
    /// fail; it then starts a fetch of its own, once, within the same
    /// deadline.
    pub(crate) async fn fetch(
        self: Arc<Self>,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        height: u32,
    ) -> Result<QuorumKey, ContextProviderError> {
        if let Ok(key) = self.lookup(quorum_type, quorum_hash, height) {
            return Ok(key);
        }
        let unavailable = ContextProviderError::QuorumSourceUnavailable;
        // One deadline for the caller, across the other fetch it may wait on
        // and the fetch of its own that may follow.
        let deadline = Instant::now() + self.timing.deadline;
        let mut may_join_other = true;
        loop {
            let (fetch, own) = self
                .join_or_start_for(quorum_type, quorum_hash, height, may_join_other)
                .map_err(unavailable)?;
            let remaining = deadline.saturating_duration_since(Instant::now());
            let proven = match select(fetch, pin!(caller_sleep(remaining))).await {
                // Another height's fetch failing says nothing about this one.
                Either::Left((Err(_), _)) if !own && may_join_other => {
                    may_join_other = false;
                    continue;
                }
                Either::Left((proven, _)) => proven.map_err(unavailable)?,
                Either::Right(_) => {
                    return Err(unavailable(format!(
                        "quorum {} was not proven within {} ms",
                        hex::encode(quorum_hash),
                        self.timing.deadline.as_millis()
                    )));
                }
            };
            if proven.serves(height) {
                return Ok(proven.key);
            }
            if own || !may_join_other {
                return Err(unavailable(format!(
                    "quorum {} is proven for core heights {} to {}, not for {height}",
                    hex::encode(quorum_hash),
                    proven.lo,
                    proven.last_served()
                )));
            }
            may_join_other = false;
        }
    }

    fn lock(&self) -> MutexGuard<'_, Proven> {
        // Every update under the lock completes before it is released.
        self.proven.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// [`join_or_start_for`](Self::join_or_start_for), joining a fetch for
    /// another height near this one if there is one.
    #[cfg(test)]
    fn join_or_start(
        self: &Arc<Self>,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        height: u32,
    ) -> Result<(SharedFetch, bool), String> {
        self.join_or_start_for(quorum_type, quorum_hash, height, true)
    }

    /// A running fetch of this quorum to wait on, or a new one if the limits
    /// allow, and whether it is the fetch for `height` itself, whose result
    /// is final for the caller.
    ///
    /// A fetch succeeds only once its proofs serve its own height, so only
    /// a fetch for `height` itself is sure to serve the caller. With
    /// `may_join_other`, a caller otherwise joins a fetch for a height at
    /// most a window from its own, and must check what it proved.
    ///
    /// A failure cools the quorum down only for its own height. Proving
    /// another height asks for other targets, and the height comes from an
    /// unverified response: a failure at a bogus height must not hold back a
    /// request for an honest one, which runs its own fetch.
    fn join_or_start_for(
        self: &Arc<Self>,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        height: u32,
        may_join_other: bool,
    ) -> Result<(SharedFetch, bool), String> {
        let quorum = (quorum_type, quorum_hash);
        let window = self.window;
        let now = Instant::now();
        let forgotten_after = self.timing.max_quorum_cooldown.saturating_mul(2);
        let mut proven = self.lock();
        proven.fetches.retain(|(_, slot)| match slot {
            // A fetch every waiter dropped no longer runs.
            Slot::Running { fetch, .. } => fetch.upgrade().is_some(),
            Slot::Failed { at, .. } => now.saturating_duration_since(*at) < forgotten_after,
        });
        // A fetch that finished since the caller's lookup has nothing to add.
        if let Some(cached) = proven.keys.peek(&quorum) {
            if cached.serves(height) {
                return Ok((future::ready(Ok(*cached)).boxed().shared(), true));
            }
        }
        let joinable = proven
            .fetches
            .iter()
            .filter_map(|(other, slot)| match slot {
                Slot::Running {
                    fetch,
                    height: running,
                    ..
                } if *other == quorum => {
                    let own = *running == height;
                    let near = running.abs_diff(height) <= window;
                    (own || (may_join_other && near))
                        .then(|| fetch.upgrade())
                        .flatten()
                        .map(|fetch| (fetch, own))
                }
                _ => None,
            })
            .max_by_key(|(_, own)| *own);
        if let Some(joined) = joinable {
            return Ok(joined);
        }
        let earlier = proven
            .fetches
            .iter()
            .filter_map(|(other, slot)| match slot {
                Slot::Failed {
                    at,
                    failures,
                    height: failed,
                } if *other == quorum && *failed == height => Some((*at, *failures)),
                _ => None,
            })
            .max_by_key(|(_, failures)| *failures);
        if let Some((at, failures)) = earlier {
            let cooldown = backoff(
                self.timing.quorum_cooldown,
                self.timing.max_quorum_cooldown,
                failures,
            );
            let wait = cooldown.saturating_sub(now.saturating_duration_since(at));
            if !wait.is_zero() {
                return Err(format!(
                    "quorum {} was asked for too recently, retry in {} s",
                    hex::encode(quorum_hash),
                    wait.as_millis().div_ceil(1000)
                ));
            }
        }
        let running = places_taken(&proven);
        if running >= MAX_QUORUMS_IN_FLIGHT {
            return Err(format!(
                "{running} other quorums are being proven; not starting another"
            ));
        }
        // Every running slot left after the pruning above is alive.
        let live = proven
            .fetches
            .iter()
            .filter(|(_, slot)| matches!(slot, Slot::Running { .. }))
            .count();
        if live >= MAX_RUNNING_FETCHES {
            return Err(format!(
                "{live} quorum fetches are running; not starting another"
            ));
        }
        // The earlier failures at this height stay until the new fetch ends,
        // so a fetch every caller abandons does not erase them.
        proven.next_fetch_id += 1;
        let id = proven.next_fetch_id;
        let holding = Arc::new(AtomicBool::new(true));
        let attempt = Attempt {
            quorum,
            height,
            id,
            failures: earlier.map_or(0, |(_, failures)| failures),
            failed_at: earlier.map(|(at, _)| at),
            holding: Arc::clone(&holding),
        };
        // Built under the lock, so it must do nothing until it is polled.
        let fetch = Arc::clone(self).start(attempt);
        // A fetch not yet polled always downgrades; were it done, it would
        // need no slot.
        let Some(weak) = fetch.downgrade() else {
            return Ok((fetch, true));
        };
        proven.fetches.push((
            quorum,
            Slot::Running {
                id,
                height,
                holding,
                fetch: weak,
            },
        ));
        // A browser fetch runs to its deadline even if every caller leaves,
        // so its slot stays joinable meanwhile.
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(fetch.clone().map(drop));
        Ok((fetch, true))
    }

    /// A fetch every waiter of this quorum shares.
    fn start(self: Arc<Self>, attempt: Attempt) -> SharedFetch {
        let attempt = Arc::new(attempt);
        async move {
            let prover = Arc::clone(&self);
            let inner = Arc::clone(&attempt);
            let result = contained(move || async move {
                let (result, asked) = prover.prove(&inner).await;
                // Recorded here too, so a browser fetch that outlives its
                // waiters still frees its slot.
                prover.finish(&inner, &result, asked);
                result
            })
            .await;
            // Covers a fetch that panicked; a no-op otherwise.
            self.finish(&attempt, &result, true);
            result
        }
        .boxed()
        .shared()
    }

    /// Record how a fetch ended, if its slot is still its own. A failure
    /// without a single request asked nothing, so it does not cool the
    /// quorum down further; the earlier failures are kept.
    fn finish(&self, attempt: &Attempt, result: &FetchResult, asked: bool) {
        attempt.holding.store(false, Ordering::Relaxed);
        let mut proven = self.lock();
        let ours = |slot: &Slot| matches!(slot, Slot::Running { id, .. } if *id == attempt.id);
        if !proven.fetches.iter().any(|(_, slot)| ours(slot)) {
            return;
        }
        // This fetch's outcome replaces the failures it carried on.
        proven.fetches.retain(|(quorum, slot)| match slot {
            Slot::Failed { height, .. } => *quorum != attempt.quorum || *height != attempt.height,
            Slot::Running { .. } => true,
        });
        let Some(index) = proven
            .fetches
            .iter()
            .position(|(_, slot)| matches!(slot, Slot::Running { id, .. } if *id == attempt.id))
        else {
            return;
        };
        let failed = match (result, asked, attempt.failed_at) {
            (Ok(_), _, _) => None,
            (Err(_), true, _) => Some((Instant::now(), attempt.failures.saturating_add(1))),
            (Err(_), false, Some(at)) => Some((at, attempt.failures)),
            (Err(_), false, None) => None,
        };
        let Some((at, failures)) = failed else {
            proven.fetches.swap_remove(index);
            return;
        };
        proven.fetches[index].1 = Slot::Failed {
            at,
            failures,
            height: attempt.height,
        };
        // Forget the oldest failure rather than remember without bound.
        let failed: Vec<_> = proven
            .fetches
            .iter()
            .enumerate()
            .filter_map(|(index, (_, slot))| match slot {
                Slot::Failed { at, .. } => Some((*at, index)),
                Slot::Running { .. } => None,
            })
            .collect();
        if failed.len() > MAX_FAILED_FETCHES {
            if let Some((_, oldest)) = failed.into_iter().min() {
                proven.fetches.swap_remove(oldest);
            }
        }
    }

    /// Take one of the places to prove a quorum, unless the fetch holds one
    /// already. A fetch gives its place up while it only waits, so the limit
    /// holds for fetches that send.
    fn take_place(&self, attempt: &Attempt) -> bool {
        if attempt.holding.load(Ordering::Relaxed) {
            return true;
        }
        let proven = self.lock();
        let free = places_taken(&proven) < MAX_QUORUMS_IN_FLIGHT;
        if free {
            attempt.holding.store(true, Ordering::Relaxed);
        }
        free
    }

    /// Ask the sources for a proof of a quorum until one verifies, the
    /// request budget is spent, every source is out or the deadline passes.
    /// Also returns whether the failure counts against the quorum: whether
    /// any request was sent, and the fetch did not end waiting for a place.
    async fn prove(&self, attempt: &Attempt) -> (FetchResult, bool) {
        let (quorum, height) = (attempt.quorum, attempt.height);
        let started = Instant::now();
        let deadline = started + self.timing.deadline;
        let mut requests = FuturesUnordered::new();
        let mut sources = vec![SourceAttempts::new(started); self.endpoints.len()];
        let mut sent = 0;
        // Whether a relay could not serve the first ask, and whether the
        // lower ask was sent since.
        let mut refused = false;
        let mut asked_lower = false;
        let mut hedge_at = started;
        let mut failures = Vec::new();
        let mut waited_for_place = false;
        let mut waiting_for_place = false;
        loop {
            let now = Instant::now();
            if now >= deadline {
                failures.push("deadline passed".to_string());
                break;
            }
            let mut wake = deadline;
            if requests.len() < MAX_REQUESTS_IN_FLIGHT && sent < MAX_REQUESTS_PER_FETCH {
                if !requests.is_empty() && now < hedge_at {
                    wake = hedge_at;
                } else {
                    match self.pick_source(now, &sources) {
                        Pick::Ready(_) if !self.take_place(attempt) => {
                            waiting_for_place = true;
                            if !waited_for_place {
                                waited_for_place = true;
                                failures.push("waited for other quorums' proofs".to_string());
                            }
                            wake = (now + PLACE_RETRY).min(deadline);
                        }
                        Pick::Ready(source) => {
                            // With more sources than half the budget, each
                            // would otherwise get the first ask and none the
                            // lower one.
                            let alternate = sources[source].alternate
                                || refused && !asked_lower && sent + 1 == MAX_REQUESTS_PER_FETCH;
                            let ask = match self.next_ask(quorum, height, alternate) {
                                NextAsk::Ask(ask) => ask,
                                // Another fetch proved it meanwhile.
                                NextAsk::Proven(proven) => return (Ok(proven), sent > 0),
                                NextAsk::Unreachable => {
                                    failures.push(format!(
                                        "no verified state is below core height {height}, so \
                                         no proof can reach it"
                                    ));
                                    break;
                                }
                            };
                            let body = request_body(&ask, quorum);
                            let remaining = deadline.saturating_duration_since(now);
                            let timeout = self.timing.request_timeout.min(remaining);
                            waiting_for_place = false;
                            sources[source].asked += 1;
                            sources[source].in_flight = true;
                            sent += 1;
                            asked_lower |= alternate;
                            hedge_at = now + self.timing.hedge_after;
                            let endpoint = &self.endpoints[source];
                            requests.push(async move {
                                let reply = self.transport.post(endpoint, body, timeout).await;
                                (source, ask, alternate, now, timeout, reply)
                            });
                            continue;
                        }
                        Pick::At(at) => wake = at.min(deadline),
                        Pick::Nothing if requests.is_empty() => {
                            failures.push(
                                "every source is cooling down or out of this fetch".to_string(),
                            );
                            break;
                        }
                        Pick::Nothing => {}
                    }
                }
            } else if requests.is_empty() {
                failures.push(format!("all {MAX_REQUESTS_PER_FETCH} requests used"));
                break;
            }

            // Browser timers round short waits down to zero.
            let wait = wake
                .saturating_duration_since(now)
                .max(Duration::from_millis(1));
            if requests.is_empty() {
                // Nothing is in flight: let another quorum be proven meanwhile.
                attempt.holding.store(false, Ordering::Relaxed);
                sleep(wait).await;
                continue;
            }
            let answered = match select(requests.next(), pin!(sleep(wait))).await {
                Either::Left((Some(answered), _)) => answered,
                Either::Left((None, _)) | Either::Right(_) => continue,
            };
            let (source, ask, alternate, asked_at, timeout, reply) = answered;
            let origin = self.endpoints[source].origin().ascii_serialization();
            let elapsed = asked_at.elapsed();
            let state = &mut sources[source];
            state.in_flight = false;
            let note =
                |reason: &str| format!("{origin}: {reason} after {} ms", elapsed.as_millis());
            // A request the deadline cut short says nothing about its source.
            let cut_short = timeout < self.timing.request_timeout && elapsed >= timeout;
            let verdict = match reply {
                Err(PostError::Failed(reason)) if cut_short => {
                    failures.push(note(&reason));
                    continue;
                }
                reply => self.judge(quorum, height, &ask, reply),
            };
            match verdict {
                Verdict::Proven(proven) => {
                    self.source_answered(source);
                    tracing::debug!(
                        quorum_type = quorum.0,
                        quorum_hash = %hex::encode(quorum.1),
                        height,
                        %origin,
                        elapsed_ms = elapsed.as_millis(),
                        "proved quorum key"
                    );
                    return (Ok(proven), true);
                }
                Verdict::CannotServe(reason) => {
                    state.busy = 0;
                    state.cannot_serve += 1;
                    refused |= !alternate;
                    // Next time, ask this source for the other height.
                    state.alternate = !alternate;
                    if state.cannot_serve >= MAX_CANNOT_SERVE {
                        state.dropped = true;
                        failures.push(note(&format!("{reason}; not asked again")));
                    } else {
                        state.ready_at = Instant::now() + self.timing.first_retry;
                        failures.push(note(&reason));
                    }
                }
                Verdict::Busy(reason) if state.busy + 1 >= MAX_BUSY => {
                    state.dropped = true;
                    failures.push(note(&format!(
                        "{reason}; busy {MAX_BUSY} times, not asked again"
                    )));
                }
                Verdict::Busy(reason) => {
                    state.busy += 1;
                    let max = self
                        .timing
                        .first_retry
                        .saturating_mul(1 << MAX_RETRY_DOUBLINGS);
                    state.ready_at =
                        Instant::now() + backoff(self.timing.first_retry, max, state.busy);
                    failures.push(note(&reason));
                }
                Verdict::SourceFailed(reason) => {
                    self.source_failed(source);
                    failures.push(note(&reason));
                }
                Verdict::BeyondTip(reason) => {
                    state.dropped = true;
                    failures.push(note(&reason));
                }
            }
        }
        let reason = format!(
            "could not prove quorum {} of type {} for core height {height} within {} ms: {}",
            hex::encode(quorum.1),
            quorum.0,
            started.elapsed().as_millis(),
            failures.join("; ")
        );
        tracing::warn!(%reason, "quorum proof fetch failed");
        // Running out of time while other quorums held the places says
        // nothing about this one.
        (Err(reason), sent > 0 && !waiting_for_place)
    }

    /// Judge a relay's answer, publishing the key if it proves one.
    ///
    /// Only what an honest relay never does cools a source down: failed or
    /// oversized transfers, redirects, error statuses other than 502 and
    /// 503, and proofs that do not verify. A valid proof that does not serve
    /// the request may be the fault of the unverified height or quorum hash.
    ///
    /// A valid proof whose target, together with the earlier proofs, does not
    /// serve `height` is still published, as evidence for later asks, but does
    /// not prove the key for `height`.
    fn judge(
        &self,
        quorum: QuorumId,
        height: u32,
        ask: &Ask,
        reply: Result<Reply, PostError>,
    ) -> Verdict {
        let reply = match reply {
            Ok(reply) => reply,
            Err(PostError::TooLarge) => {
                return Verdict::SourceFailed("answered with more than a proof can be".to_string())
            }
            Err(PostError::Failed(reason)) => return Verdict::SourceFailed(reason),
        };
        let body = &reply.body;
        match reply.status {
            200 => {}
            // The relay's Core cannot serve this quorum for this height.
            502 => return Verdict::CannotServe(format!("HTTP 502{}", body_excerpt(body))),
            // The relay's workers are busy, which is normal under load.
            503 => return Verdict::Busy(format!("HTTP 503{}", body_excerpt(body))),
            status => return Verdict::SourceFailed(format!("HTTP {status}{}", body_excerpt(body))),
        }
        // A target below the height asked for, or more than one DKG interval
        // below `height` for the tip, can never help. It is refused before any
        // signature, so a bogus height costs no BLS work. Only this source is
        // affected if it lies.
        let needed = match ask.height {
            0 => height.saturating_sub(DKG_INTERVAL_BLOCKS),
            asked => asked,
        };
        let verified = match bootstrap::verify(body, &ask.checkpoint, needed) {
            Ok(verified) => verified,
            Err(error) if error.kind() == ErrorKind::StaleTarget && ask.height == 0 => {
                return Verdict::BeyondTip(format!(
                    "its latest ChainLock is below core height {needed}, which {height} needs"
                ));
            }
            Err(error) if error.kind() == ErrorKind::StaleTarget => {
                return Verdict::CannotServe(format!("proof ends below core height {needed}"));
            }
            Err(error) => return Verdict::SourceFailed(format!("invalid proof: {error}")),
        };
        let mut keys = Vec::new();
        // EvoNode records are not asked for; any that come back are ignored.
        for record in verified.records() {
            if record.kind != RecordKind::Quorum {
                continue;
            }
            // A record that does not parse opens no key; if it was the
            // requested quorum's, the count below is short.
            if let Ok(commitment) = parse_commitment(record.leaf) {
                let mut hash = commitment.quorum_hash;
                hash.reverse();
                if (u32::from(commitment.quorum_type), hash) == quorum {
                    keys.push(commitment.public_key);
                }
            }
        }
        let [key] = keys.as_slice() else {
            return Verdict::CannotServe(format!(
                "proof opens the requested quorum {} times, not once",
                keys.len()
            ));
        };
        match self.publish(quorum, *key, verified.state()) {
            Ok(proven) if proven.serves(height) => Verdict::Proven(proven),
            Ok(proven) => Verdict::CannotServe(format!(
                "proof target {} leaves the quorum proven for core heights {} to {}, not for \
                 {height}",
                verified.state().height,
                proven.lo,
                proven.last_served()
            )),
            Err(reason) => Verdict::CannotServe(reason),
        }
    }

    /// What to ask a source next for this quorum, given what the cache holds
    /// for it now.
    fn next_ask(&self, quorum: QuorumId, height: u32, alternate: bool) -> NextAsk {
        let proven = self.lock();
        let cached = proven.keys.peek(&quorum);
        if let Some(cached) = cached.filter(|cached| cached.serves(height)) {
            return NextAsk::Proven(*cached);
        }
        let extending = cached.is_some_and(|cached| cached.lo <= height);
        choose_ask(
            &proven.states,
            self.checkpoint.height,
            height,
            extending,
            alternate,
        )
        .map_or(NextAsk::Unreachable, NextAsk::Ask)
    }

    /// Cache a proven key, widening the heights it is known active at to the
    /// target that opened it, and remember the target, and return what the
    /// cache now holds for the quorum. Nothing is cached if either conflicts
    /// with what is already known.
    pub(crate) fn publish(
        &self,
        quorum: QuorumId,
        key: QuorumKey,
        target: &State,
    ) -> Result<Cached, String> {
        let mut proven = self.lock();
        if let Some(known) = proven.states.get(&target.height) {
            if known != target {
                tracing::error!(
                    height = target.height,
                    known = %rpc_hex(&known.block_hash),
                    proven = %rpc_hex(&target.block_hash),
                    "two proofs verified different blocks at one height; keeping the first"
                );
                return Err("proves a different block than one already verified".to_string());
            }
        }
        let cached = match proven.keys.get_mut(&quorum) {
            Some(cached) if cached.key != key => {
                tracing::error!(
                    quorum_type = quorum.0,
                    quorum_hash = %hex::encode(quorum.1),
                    cached = %hex::encode(cached.key),
                    proven = %hex::encode(key),
                    "a proof opened a different key for a quorum already proven; keeping the first"
                );
                return Err("opens a different key than one already proven".to_string());
            }
            Some(cached) => {
                cached.lo = cached.lo.min(target.height);
                cached.hi = cached.hi.max(target.height);
                *cached
            }
            None => {
                let cached = Cached {
                    key,
                    lo: target.height,
                    hi: target.height,
                };
                proven.keys.put(quorum, cached);
                cached
            }
        };
        if proven
            .states
            .insert(target.height, target.clone())
            .is_none()
        {
            // One state is added at a time, so one removal keeps the bound.
            if proven.states.len() > MAX_RECENT_STATES + 1 {
                let oldest = proven
                    .states
                    .keys()
                    .copied()
                    .find(|height| *height != self.checkpoint.height);
                if let Some(height) = oldest {
                    proven.states.remove(&height);
                }
            }
        }
        Ok(cached)
    }

    /// The source to ask next: one that is not cooling down, left out of
    /// this fetch or already asked, and is ready; among those the one asked
    /// the fewest times.
    fn pick_source(&self, now: Instant, sources: &[SourceAttempts]) -> Pick {
        let proven = self.lock();
        let candidates: Vec<_> = sources
            .iter()
            .zip(&proven.sources)
            .enumerate()
            .filter(|(_, (attempts, health))| {
                !attempts.dropped
                    && !attempts.in_flight
                    && health.failed_until.is_none_or(|until| until <= now)
            })
            .map(|(source, (attempts, _))| (source, attempts))
            .collect();
        let ready = candidates
            .iter()
            .filter(|(_, attempts)| attempts.ready_at <= now)
            .min_by_key(|(_, attempts)| attempts.asked);
        match (ready, candidates.iter().map(|(_, a)| a.ready_at).min()) {
            (Some((source, _)), _) => Pick::Ready(*source),
            (None, Some(at)) => Pick::At(at),
            (None, None) => Pick::Nothing,
        }
    }

    /// How many fetches hold one of the places to prove a quorum.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) fn places(&self) -> usize {
        places_taken(&self.lock())
    }

    /// Which sources are cooling down.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) fn cooling(&self) -> Vec<bool> {
        let now = Instant::now();
        let proven = self.lock();
        let cooling = |health: &SourceHealth| health.failed_until.is_some_and(|until| until > now);
        proven.sources.iter().map(cooling).collect()
    }

    fn source_failed(&self, source: usize) {
        let first = self.timing.source_cooldown;
        let max = self.timing.max_source_cooldown;
        if let Some(health) = self.lock().sources.get_mut(source) {
            health.failures = health.failures.saturating_add(1);
            health.failed_until = Some(Instant::now() + backoff(first, max, health.failures));
        }
    }

    fn source_answered(&self, source: usize) {
        if let Some(health) = self.lock().sources.get_mut(source) {
            *health = SourceHealth::default();
        }
    }
}

/// How many fetches hold one of the places to prove a quorum.
fn places_taken(proven: &Proven) -> usize {
    proven
        .fetches
        .iter()
        .filter(|(_, slot)| match slot {
            // A fetch every waiter dropped no longer runs.
            Slot::Running { holding, fetch, .. } => {
                holding.load(Ordering::Relaxed) && fetch.upgrade().is_some()
            }
            Slot::Failed { .. } => false,
        })
        .count()
}

/// The JSON a relay's `/proofs` takes. Relays reject unknown fields.
fn request_body(ask: &Ask, (quorum_type, quorum_hash): QuorumId) -> String {
    serde_json::json!({
        "checkpoint": rpc_hex(&ask.checkpoint.block_hash),
        "height": ask.height,
        "quorumHash": hex::encode(quorum_hash),
        "llmqType": quorum_type,
        "nodeCount": 0,
    })
    .to_string()
}

/// Invisible formatting characters, such as bidirectional overrides, that
/// could make a logged reason read differently from what it says.
fn is_invisible_format(c: char) -> bool {
    matches!(
        c,
        '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}'
            | '\u{FEFF}'
    )
}

/// The start of a plain-text error body, for the failure reason. Markup,
/// such as a CDN's error page, is left out.
fn body_excerpt(body: &[u8]) -> String {
    if body.contains(&b'<') {
        return String::new();
    }
    let text: String = String::from_utf8_lossy(body)
        .chars()
        .filter(|c| !c.is_control() && !is_invisible_format(*c))
        .take(MAX_REASON_BODY_CHARS)
        .collect();
    if text.is_empty() {
        text
    } else {
        format!(": {text}")
    }
}

/// Run a fetch so that a panic becomes a failed fetch instead of being raised
/// again in every waiter of the shared future.
#[cfg(not(target_arch = "wasm32"))]
async fn contained<F, Fut>(prove: F) -> FetchResult
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = FetchResult>,
{
    match AssertUnwindSafe(async move { prove().await })
        .catch_unwind()
        .await
    {
        Ok(result) => result,
        Err(payload) => {
            let message = panic_message(payload.as_ref());
            tracing::error!(%message, "quorum proof fetch panicked");
            Err(format!("quorum proof fetch panicked: {message}"))
        }
    }
}

/// Wait for `duration` in a `Send` future, for callers outside the local
/// executor. The browser timer runs in a local task that ends, clearing the
/// timer, as soon as this future is dropped.
#[cfg(target_arch = "wasm32")]
async fn caller_sleep(duration: Duration) {
    let (cancel, cancelled) = oneshot::channel::<()>();
    let (fired, done) = oneshot::channel::<()>();
    wasm_bindgen_futures::spawn_local(async move {
        if let Either::Left(_) = select(sleep(duration), cancelled).await {
            let _ = fired.send(());
        }
    });
    let _cancel_on_drop = cancel;
    let _ = done.await;
}

/// Run a fetch on the local executor, where browser requests and timers have
/// to run, so it finishes even if every waiter goes away.
#[cfg(target_arch = "wasm32")]
async fn contained<F, Fut>(prove: F) -> FetchResult
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = FetchResult> + 'static,
{
    on_local_executor(prove)
        .await
        .unwrap_or_else(|| Err("quorum proof task was dropped".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_core_proof::ProofNetwork;

    fn state(height: u32) -> State {
        State {
            network: ProofNetwork::Testnet,
            height,
            block_hash: [height as u8; 32],
            masternode_root: [1; 32],
            quorum_root: [2; 32],
        }
    }

    fn states(heights: &[u32]) -> BTreeMap<u32, State> {
        heights
            .iter()
            .map(|height| (*height, state(*height)))
            .collect()
    }

    fn ask(from: u32, height: u32) -> Option<Ask> {
        Some(Ask {
            checkpoint: state(from),
            height,
        })
    }

    #[test]
    fn should_ask_for_the_tip_from_a_later_state_and_the_lower_target_from_below() {
        let lower = 250 - DKG_INTERVAL_BLOCKS;
        let cases = [
            // (verified heights, height, alternate, expected, why)
            (&[100][..], 150, false, ask(100, 0), "the tip, from below"),
            (&[100], 150, true, ask(100, 126), "alternate asks lower"),
            (&[100], 100, false, ask(100, 0), "nothing below: the latest"),
            (&[100], 100, true, ask(100, 0), "nothing below either way"),
            (&[100, 200], 150, false, ask(200, 0), "no downgrade"),
            (
                &[100, 200],
                150,
                true,
                ask(100, 126),
                "lower from the checkpoint",
            ),
            (&[100, 200, 300], 250, false, ask(200, 0), "state below"),
            (&[100, 200, 300], 250, true, ask(200, lower), "alternate"),
            (&[100, 240], 250, true, ask(100, lower), "from below it"),
            (
                &[100, 150, 190],
                200,
                true,
                ask(150, 200 - DKG_INTERVAL_BLOCKS),
                "a quorum retired by 190",
            ),
        ];
        for (heights, height, alternate, expected, why) in cases {
            assert_eq!(
                choose_ask(&states(heights), 100, height, true, alternate),
                expected,
                "{why}"
            );
        }
    }

    #[test]
    fn should_ask_for_the_height_or_just_below_it_from_a_state_below_it() {
        let lower = 150 - DKG_INTERVAL_BLOCKS;
        let cases = [
            // (verified heights, height, alternate, expected, why)
            (&[100][..], 150, false, ask(100, 150), "the height"),
            (&[100], 150, true, ask(100, lower), "alternate asks lower"),
            (&[100], 110, true, ask(100, 101), "not below the start"),
            (&[100, 140], 150, true, ask(100, lower), "from below it"),
            (
                &[100, 200],
                150,
                false,
                ask(100, 150),
                "only the checkpoint is below",
            ),
            (
                &[100, 200, 300],
                250,
                false,
                ask(200, 250),
                "verified state below",
            ),
            (&[100], 100, false, None, "nothing below"),
            (&[100], 50, true, None, "under the checkpoint"),
        ];
        for (heights, height, alternate, expected, why) in cases {
            assert_eq!(
                choose_ask(&states(heights), 100, height, false, alternate),
                expected,
                "{why}"
            );
        }
    }

    #[test]
    fn should_double_cooldowns_up_to_their_cap() {
        let (first, max) = (Duration::from_secs(5), Duration::from_secs(60));
        let waits: Vec<_> = (1..=6)
            .map(|failures| backoff(first, max, failures).as_secs())
            .collect();
        assert_eq!(waits, [5, 10, 20, 40, 60, 60]);
        assert_eq!(backoff(first, max, u32::MAX), max);
    }

    fn fetches() -> ProofFetches {
        ProofFetches::new(
            6,
            600,
            state(100),
            vec![Url::parse("https://relay.example").expect("a URL")],
            NonZeroUsize::new(4).expect("non-zero"),
            ProofTransport::new().expect("a client"),
            Timing::default(),
        )
    }

    fn running_id(fetches: &ProofFetches, quorum: QuorumId, height: u32) -> Option<u64> {
        fetches
            .lock()
            .fetches
            .iter()
            .find_map(|(other, slot)| match slot {
                Slot::Running { id, height: h, .. } if *other == quorum && *h == height => {
                    Some(*id)
                }
                _ => None,
            })
    }

    fn holding(fetches: &ProofFetches, quorum: QuorumId) -> Arc<AtomicBool> {
        fetches
            .lock()
            .fetches
            .iter()
            .find_map(|(other, slot)| match slot {
                Slot::Running { holding, .. } if *other == quorum => Some(Arc::clone(holding)),
                _ => None,
            })
            .expect("a running fetch")
    }

    fn attempt(quorum: QuorumId, id: u64, holding: Arc<AtomicBool>) -> Attempt {
        Attempt {
            quorum,
            height: 150,
            id,
            failures: 0,
            failed_at: None,
            holding,
        }
    }

    fn failed(at: Instant) -> Slot {
        Slot::Failed {
            at,
            failures: 1,
            height: 150,
        }
    }

    #[test]
    fn should_forget_the_oldest_failure_past_the_limit() {
        let fetches = Arc::new(fetches());
        let now = Instant::now();
        let earlier = now.checked_sub(Duration::from_secs(1)).unwrap_or(now);
        {
            let mut proven = fetches.lock();
            proven.fetches.push(((6, [0; 32]), failed(earlier)));
            for n in 1..MAX_FAILED_FETCHES {
                proven.fetches.push(((6, [n as u8; 32]), failed(now)));
            }
        }
        let _fetch = fetches.join_or_start(6, [0xff; 32], 150).expect("starts");
        let id = running_id(&fetches, (6, [0xff; 32]), 150).expect("running");
        let failure = Err("failed".to_string());
        fetches.finish(
            &attempt((6, [0xff; 32]), id, holding(&fetches, (6, [0xff; 32]))),
            &failure,
            true,
        );
        let proven = fetches.lock();
        assert_eq!(proven.fetches.len(), MAX_FAILED_FETCHES);
        assert!(
            !proven
                .fetches
                .iter()
                .any(|(quorum, _)| *quorum == (6, [0; 32])),
            "the oldest goes"
        );
    }

    #[test]
    fn should_hold_back_only_the_height_a_fetch_failed_for() {
        let fetches = Arc::new(fetches());
        fetches
            .lock()
            .fetches
            .push(((6, [1; 32]), failed(Instant::now())));
        assert!(fetches.join_or_start(6, [1; 32], 150).is_err());
        // Another height asks for other targets, so it runs its own fetch.
        for height in [149, 151] {
            assert!(
                fetches.join_or_start_for(6, [1; 32], height, false).is_ok(),
                "{height}"
            );
        }
    }

    #[test]
    fn should_keep_another_heights_failure_when_a_fetch_fails() {
        let fetches = Arc::new(fetches());
        let quorum = (6, [1; 32]);
        let failure = Slot::Failed {
            at: Instant::now(),
            failures: 4,
            height: 1000,
        };
        fetches.lock().fetches.push((quorum, failure));
        let _lower = fetches.join_or_start(6, [1; 32], 500).expect("starts");
        let id = running_id(&fetches, quorum, 500).expect("running");
        let lower = Attempt {
            height: 500,
            ..attempt(quorum, id, holding(&fetches, quorum))
        };
        fetches.finish(&lower, &Err("failed".to_string()), true);
        let mut recorded: Vec<_> = fetches
            .lock()
            .fetches
            .iter()
            .filter_map(|(_, slot)| match slot {
                Slot::Failed {
                    failures, height, ..
                } => Some((*failures, *height)),
                Slot::Running { .. } => None,
            })
            .collect();
        recorded.sort();
        assert_eq!(recorded, [(1, 500), (4, 1000)]);
        assert!(
            fetches.join_or_start(6, [1; 32], 1000).is_err(),
            "1000 is still held back"
        );
    }

    #[test]
    fn should_refuse_new_quorums_while_the_running_fetches_are_full() {
        let fetches = Arc::new(fetches());
        let mut waiting = Vec::new();
        for n in 0..MAX_RUNNING_FETCHES {
            let fetch = fetches
                .join_or_start(6, [n as u8; 32], 150)
                .expect("a place is free");
            // Each gives its place up while it waits out a relay's backoff.
            holding(&fetches, (6, [n as u8; 32])).store(false, Ordering::Relaxed);
            waiting.push(fetch);
        }
        let refused = fetches.join_or_start(6, [0xff; 32], 150).map(|_| ());
        assert!(
            refused
                .as_ref()
                .is_err_and(|reason| reason.contains("quorum fetches are running")),
            "{refused:?}"
        );
        // A running fetch can still be joined.
        assert!(fetches.join_or_start(6, [0; 32], 150).is_ok());

        let id = running_id(&fetches, (6, [1; 32]), 150).expect("running");
        let first = attempt((6, [1; 32]), id, holding(&fetches, (6, [1; 32])));
        fetches.finish(&first, &Err("failed".to_string()), true);
        assert!(fetches.join_or_start(6, [0xff; 32], 150).is_ok());
    }

    #[test]
    fn should_join_another_heights_fetch_only_when_asked_to_and_say_it_is_not_its_own() {
        // The fetch for 150 keeps its place, and the one for 400 takes the
        // second of the two, so starting it is not refused.
        let fetches = Arc::new(fetches());
        let _first = fetches.join_or_start(6, [1; 32], 150).expect("starts");
        let id = running_id(&fetches, (6, [1; 32]), 150);
        let (_, own) = fetches.join_or_start(6, [1; 32], 150).expect("joins");
        assert!(own, "the fetch for 150 is final for 150");
        for height in [100, 400] {
            let (_, own) = fetches.join_or_start(6, [1; 32], height).expect("joins");
            assert!(!own, "a fetch for 150 may not serve {height}");
            assert_eq!(running_id(&fetches, (6, [1; 32]), height), None);
        }
        let (_, own) = fetches
            .join_or_start_for(6, [1; 32], 400, false)
            .expect("its own fetch");
        assert!(own);
        assert!(running_id(&fetches, (6, [1; 32]), 400).is_some());
        assert_eq!(running_id(&fetches, (6, [1; 32]), 150), id);
        // Without joining others, a caller still joins its own height's fetch.
        let (_, own) = fetches
            .join_or_start_for(6, [1; 32], 150, false)
            .expect("joins its own height");
        assert!(own);
    }

    #[test]
    fn should_run_a_separate_fetch_for_a_height_far_from_the_running_one() {
        let fetches = Arc::new(fetches());
        let _first = fetches.join_or_start(6, [1; 32], 150).expect("starts");
        let id = running_id(&fetches, (6, [1; 32]), 150);
        let _joined = fetches.join_or_start(6, [1; 32], 750).expect("joins");
        assert_eq!(running_id(&fetches, (6, [1; 32]), 150), id);
        assert_eq!(running_id(&fetches, (6, [1; 32]), 750), None);
        let _far = fetches
            .join_or_start(6, [1; 32], 751)
            .expect("its own fetch");
        assert!(running_id(&fetches, (6, [1; 32]), 751).is_some());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn should_replace_a_fetch_every_waiter_dropped() {
        let fetches = Arc::new(fetches());
        let first = fetches.join_or_start(6, [1; 32], 150).expect("starts");
        let id = running_id(&fetches, (6, [1; 32]), 150);
        drop(first);
        let _second = fetches.join_or_start(6, [1; 32], 150).expect("restarts");
        let replaced = running_id(&fetches, (6, [1; 32]), 150);
        assert!(replaced.is_some() && replaced != id);
    }

    #[test]
    fn should_let_only_fetches_holding_a_place_send() {
        let fetches = Arc::new(fetches());
        let _first = fetches.join_or_start(6, [1; 32], 150).expect("first");
        let _second = fetches.join_or_start(6, [2; 32], 150).expect("second");
        assert!(fetches.join_or_start(6, [3; 32], 150).is_err());
        // The first gives its place up while it waits out a backoff.
        let first = holding(&fetches, (6, [1; 32]));
        first.store(false, Ordering::Relaxed);
        let _third = fetches
            .join_or_start(6, [3; 32], 150)
            .expect("a place is free");
        // Back from the wait, the first must wait for a place again.
        let attempt = attempt((6, [1; 32]), 1, first);
        assert!(!fetches.take_place(&attempt));
        holding(&fetches, (6, [2; 32])).store(false, Ordering::Relaxed);
        assert!(fetches.take_place(&attempt));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn should_not_count_a_fetch_every_waiter_dropped_against_the_places() {
        let fetches = Arc::new(fetches());
        let first = fetches.join_or_start(6, [1; 32], 150).expect("first");
        let _second = fetches.join_or_start(6, [2; 32], 150).expect("second");
        drop(first);
        let id = running_id(&fetches, (6, [2; 32]), 150).expect("running");
        let second = attempt((6, [2; 32]), id, holding(&fetches, (6, [2; 32])));
        // The second waited out a backoff and wants to send again.
        second.holding.store(false, Ordering::Relaxed);
        assert!(fetches.take_place(&second));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn should_keep_earlier_failures_when_every_caller_abandons_the_next_fetch() {
        let fetches = Arc::new(fetches());
        let quorum = (6, [1; 32]);
        let now = Instant::now();
        let long_ago = now
            .checked_sub(Duration::from_secs(11))
            .expect("the clock has been running longer than the cooldown");
        let failed = Slot::Failed {
            at: long_ago,
            failures: 2,
            height: 150,
        };
        fetches.lock().fetches.push((quorum, failed));
        let abandoned = fetches.join_or_start(6, [1; 32], 150).expect("cooled down");
        drop(abandoned);
        let _next = fetches
            .join_or_start(6, [1; 32], 150)
            .expect("starts again");
        let kept = fetches.lock().fetches.iter().any(|(other, slot)| {
            *other == quorum && matches!(slot, Slot::Failed { failures: 2, .. })
        });
        assert!(kept, "the earlier failures survive the abandoned fetch");
    }

    #[test]
    fn should_keep_earlier_failures_when_a_fetch_sent_nothing() {
        let fetches = Arc::new(fetches());
        let quorum = (6, [1; 32]);
        let at = Instant::now();
        let _fetch = fetches.join_or_start(6, [1; 32], 150).expect("starts");
        let id = running_id(&fetches, quorum, 150).expect("running");
        let attempt = Attempt {
            failures: 3,
            failed_at: Some(at),
            ..attempt(quorum, id, Arc::new(AtomicBool::new(true)))
        };
        fetches.finish(&attempt, &Err("nothing sent".to_string()), false);
        match fetches.lock().fetches.first() {
            Some((
                _,
                Slot::Failed {
                    at: kept,
                    failures: 3,
                    ..
                },
            )) => assert_eq!(*kept, at),
            _ => panic!("the earlier failures are kept"),
        };
    }

    #[test]
    fn should_leave_markup_and_invisible_characters_out_of_reasons() {
        assert_eq!(body_excerpt(b"<html>403</html>"), "");
        assert_eq!(
            body_excerpt("busy\u{202E}yrt\u{200B}\n".as_bytes()),
            ": busyyrt"
        );
    }

    #[test]
    fn should_keep_the_first_key_proven_for_a_quorum() {
        let fetches = fetches();
        let quorum = (6, [9; 32]);
        fetches
            .publish(quorum, [1; 48], &state(200))
            .expect("first proof");
        assert!(fetches.publish(quorum, [2; 48], &state(300)).is_err());
        let last = 200 + DKG_INTERVAL_BLOCKS;
        assert_eq!(fetches.lookup(6, [9; 32], last).ok(), Some([1; 48]));
        assert!(
            fetches.lookup(6, [9; 32], last + 1).is_err(),
            "the conflicting proof must not widen the interval"
        );
        assert_eq!(fetches.latest_state(), state(200));
    }

    #[test]
    fn should_serve_a_key_from_its_lowest_target_to_one_interval_past_its_highest() {
        let fetches = fetches();
        let quorum = (6, [9; 32]);
        let served = |heights: &[u32]| {
            heights
                .iter()
                .map(|height| fetches.lookup(6, [9; 32], *height).is_ok())
                .collect::<Vec<_>>()
        };
        let cached = fetches
            .publish(quorum, [1; 48], &state(300))
            .expect("first");
        assert_eq!((cached.lo, cached.hi), (300, 300));
        let grace = DKG_INTERVAL_BLOCKS;
        assert_eq!(
            served(&[0, 299, 300, 300 + grace, 301 + grace]),
            [false, false, true, true, false]
        );
        // A later target widens upward, an earlier one downward; neither is
        // ever narrowed.
        let cached = fetches
            .publish(quorum, [1; 48], &state(350))
            .expect("later");
        assert_eq!((cached.lo, cached.hi), (300, 350));
        let cached = fetches
            .publish(quorum, [1; 48], &state(200))
            .expect("earlier");
        assert_eq!((cached.lo, cached.hi), (200, 350));
        let cached = fetches
            .publish(quorum, [1; 48], &state(250))
            .expect("inside");
        assert_eq!((cached.lo, cached.hi), (200, 350));
        assert_eq!(
            served(&[199, 200, 275, 350 + grace, 351 + grace]),
            [false, true, true, true, false]
        );
        assert_eq!(fetches.latest_state(), state(350));
    }

    #[test]
    fn should_refuse_two_blocks_at_one_height() {
        let fetches = fetches();
        fetches
            .publish((6, [9; 32]), [1; 48], &state(200))
            .expect("first");
        let other = State {
            block_hash: [0xee; 32],
            ..state(200)
        };
        assert!(fetches.publish((6, [8; 32]), [2; 48], &other).is_err());
        assert!(fetches.lookup(6, [8; 32], 200).is_err());
    }

    #[test]
    fn should_tell_what_to_ask_from_the_quorums_interval() {
        let fetches = fetches();
        let quorum = (6, [9; 32]);
        let next = |height: u32, alternate: bool| match fetches.next_ask(quorum, height, alternate)
        {
            NextAsk::Ask(ask) => Some(ask),
            NextAsk::Proven(_) => None,
            NextAsk::Unreachable => panic!("reachable"),
        };
        // Nothing known: the height itself, or just below it.
        assert_eq!(next(400, false), ask(100, 400));
        assert_eq!(next(400, true), ask(100, 400 - DKG_INTERVAL_BLOCKS));
        fetches
            .publish(quorum, [1; 48], &state(300))
            .expect("a target");
        // More than one interval above it: the tip, which serves with its
        // low end.
        assert_eq!(next(400, false), ask(300, 0));
        assert_eq!(next(400, true), ask(300, 400 - DKG_INTERVAL_BLOCKS));
        // Inside it, or within one interval above it: nothing to ask.
        assert_eq!(next(300, false), None);
        assert_eq!(next(300 + DKG_INTERVAL_BLOCKS, false), None);
        assert_eq!(
            next(301 + DKG_INTERVAL_BLOCKS, false),
            ask(300, 0),
            "past the grace"
        );
        // Below it: a target at or below the height, from a state below it.
        assert_eq!(next(250, false), ask(100, 250));
        assert_eq!(next(250, true), ask(100, 250 - DKG_INTERVAL_BLOCKS));
        assert!(matches!(
            fetches.next_ask(quorum, 100, false),
            NextAsk::Unreachable
        ));
    }

    #[test]
    fn should_ask_for_the_lower_target_from_the_checkpoint_after_states_below_were_dropped() {
        // A cache that holds every quorum published here.
        let fetches = ProofFetches::new(
            6,
            600,
            state(100),
            vec![Url::parse("https://relay.example").expect("a URL")],
            NonZeroUsize::new(32).expect("non-zero"),
            ProofTransport::new().expect("a client"),
            Timing::default(),
        );
        let quorum = (6, [9; 32]);
        fetches
            .publish(quorum, [1; 48], &state(120))
            .expect("a target");
        // Later proofs of other quorums push the state at 120 out.
        for height in 300..300 + MAX_RECENT_STATES as u32 {
            fetches
                .publish((6, [height as u8; 32]), [2; 48], &state(height))
                .expect("a proof");
        }
        assert!(!fetches.lock().states.contains_key(&120));
        let latest = 300 + MAX_RECENT_STATES as u32 - 1;
        let next = |alternate| match fetches.next_ask(quorum, 200, alternate) {
            NextAsk::Ask(ask) => ask,
            _ => panic!("an ask"),
        };
        // The tip first, from the latest state; if the quorum has retired,
        // the tip cannot open it, so then a target just below the height,
        // from the checkpoint.
        assert_eq!(Some(next(false)), ask(latest, 0));
        assert_eq!(Some(next(true)), ask(100, 200 - DKG_INTERVAL_BLOCKS));
    }

    #[test]
    fn should_keep_the_checkpoint_and_the_most_recent_states() {
        let fetches = fetches();
        for height in 101..=140 {
            fetches
                .publish((6, [height as u8; 32]), [1; 48], &state(height))
                .expect("a proof");
        }
        let proven = fetches.lock();
        let heights: Vec<_> = proven.states.keys().copied().collect();
        assert_eq!(heights.len(), MAX_RECENT_STATES + 1);
        assert_eq!(heights[0], 100, "the checkpoint stays");
        assert_eq!(heights[1], 140 - MAX_RECENT_STATES as u32 + 1);
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod browser_tests {
    use super::*;
    use dash_core_proof::ProofNetwork;
    use js_sys::{Function, Reflect};
    use wasm_bindgen::JsValue;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    async fn should_end_a_callers_wait_at_its_deadline() {
        // A browser timer of the test's own, so a deadline that never fires
        // fails the test instead of leaving it pending.
        let fallback = pin!(sleep(Duration::from_secs(5)));
        let deadline = pin!(caller_sleep(Duration::from_millis(50)));
        let woke = select(fallback, deadline).await;
        assert!(matches!(woke, Either::Right(_)), "the deadline fired");
    }

    #[wasm_bindgen_test]
    async fn should_report_a_relay_that_never_answers_as_unavailable() {
        let fetch = Function::new_with_args("request", "return new Promise(() => {});");
        Reflect::set(&js_sys::global(), &JsValue::from_str("fetch"), &fetch)
            .expect("set the global fetch");
        let checkpoint = State {
            network: ProofNetwork::Testnet,
            height: 1_000_000,
            block_hash: [1; 32],
            masternode_root: [2; 32],
            quorum_root: [3; 32],
        };
        let timing = Timing {
            deadline: Duration::from_millis(300),
            request_timeout: Duration::from_millis(300),
            ..Timing::default()
        };
        let fetches = Arc::new(ProofFetches::new(
            6,
            600,
            checkpoint,
            vec![Url::parse("https://relay.test/proofs").expect("a URL")],
            NonZeroUsize::new(4).expect("non-zero"),
            ProofTransport::new().expect("a transport"),
            timing,
        ));
        let result = fetches.fetch(6, [7; 32], 1_000_100).await;
        assert!(
            matches!(
                result,
                Err(ContextProviderError::QuorumSourceUnavailable(_))
            ),
            "{result:?}"
        );
    }
}
