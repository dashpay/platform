//! Proving Platform quorum keys with proofs fetched from relays.
//!
//! A key is believed only after a `DASHNC02` bootstrap proof, checked from a
//! state this provider already trusts, opens the quorum's commitment under a
//! ChainLocked quorum root. Relays are untrusted byte sources: one that lies,
//! stalls or fails costs time, bounded here, but never changes a key.
//!
//! A proven key is immutable: under ChainLocks each quorum has one mined
//! commitment. What expires is the evidence that the quorum is still active.
//! A key proven by a target at height `T` is served for heights up to
//! `T + window`, where `window` is how long a Platform quorum stays in the
//! active set. The height comes from the response being verified and is not
//! authenticated, so the window is only a trigger to prove again, not a
//! security boundary.
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
// A `Send` timer on every target, for callers outside the local executor.
use crate::quorum_refresh::sleep as caller_sleep;
use dash_context_provider::ContextProviderError;
use dash_core_proof::bootstrap::{self, RecordKind};
use dash_core_proof::{parse_commitment, rpc_hex, ErrorKind, State};
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

/// Most requests one fetch sends, across sources and retries.
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
/// stops asking it: one for the height and one for the tip.
const MAX_CANNOT_SERVE: u32 = 2;
/// Busy answers in a row after which a fetch stops asking a source. A relay
/// that stays busy is not cooled down for other fetches, but it must not keep
/// one running, and holding a running slot, for its whole deadline.
const MAX_BUSY: u32 = 3;
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

/// A proven key and the highest target height that proved it. The key is
/// served for heights up to `active_at` plus the validity window.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cached {
    key: QuorumKey,
    active_at: u32,
}

/// What a fetch hands every caller waiting on it: the key it proved and the
/// height that covers, so that a caller does not depend on the cache still
/// holding it.
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
/// The proof starts from the newest verified state below `height`, so it is
/// as short as it can be, and asks for a target at `height`. Once a state
/// past the checkpoint is verified, the checkpoint is never used again: a
/// later quorum is proven from the latest state, asking for the tip.
/// `alternate` flips between asking for `height` and for the tip, which a
/// relay that could not serve one may still serve.
fn choose_ask(
    states: &BTreeMap<u32, State>,
    checkpoint_height: u32,
    height: u32,
    alternate: bool,
) -> Option<Ask> {
    let (_, latest) = states.last_key_value()?;
    let ask = match states.range(..height).next_back() {
        Some((&below, state)) if below != checkpoint_height || latest.height == below => Ask {
            checkpoint: state.clone(),
            height,
        },
        _ => Ask {
            checkpoint: latest.clone(),
            height: 0,
        },
    };
    Some(if alternate && ask.height != 0 {
        Ask { height: 0, ..ask }
    } else {
        ask
    })
}

/// How one answer from a relay counts.
enum Verdict {
    Proven(Cached),
    /// The relay cannot serve this quorum for the height asked: a 502, or a
    /// valid proof that does not serve the request. Asked again once, for the
    /// other height, after a delay; then left out of the fetch.
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
    /// Ask for the tip rather than the height next time.
    ask_tip: bool,
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
            ask_tip: false,
            dropped: false,
        }
    }
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
    /// from `endpoints`, serving each for `window` blocks past the target
    /// that proved it.
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

    /// The cached key of a quorum, if it was proven active recently enough
    /// for `height`.
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
            Some(cached) if height <= cached.active_at.saturating_add(self.window) => {
                Ok(cached.key)
            }
            Some(cached) => Err(ContextProviderError::QuorumSourceUnavailable(format!(
                "quorum {} was proven active at core height {}, not yet for {height}",
                hex::encode(quorum_hash),
                cached.active_at
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
    /// A caller may first join a fetch for a lower height near its own, which
    /// may prove the quorum only up to that height; it then starts a fetch of
    /// its own, once.
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
        // One deadline for the caller, across the lower fetch it may wait on
        // and the fetch of its own that may follow.
        let deadline = Instant::now() + self.timing.deadline;
        let mut may_join_lower = true;
        loop {
            let (fetch, covers) = self
                .join_or_start_for(quorum_type, quorum_hash, height, may_join_lower)
                .map_err(unavailable)?;
            let remaining = deadline.saturating_duration_since(Instant::now());
            let proven = match select(fetch, pin!(caller_sleep(remaining))).await {
                Either::Left((proven, _)) => proven.map_err(unavailable)?,
                Either::Right(_) => {
                    return Err(unavailable(format!(
                        "quorum {} was not proven within {} ms",
                        hex::encode(quorum_hash),
                        self.timing.deadline.as_millis()
                    )));
                }
            };
            if height <= proven.active_at.saturating_add(self.window) {
                return Ok(proven.key);
            }
            if covers || !may_join_lower {
                return Err(unavailable(format!(
                    "quorum {} was proven active at core height {}, not for {height}",
                    hex::encode(quorum_hash),
                    proven.active_at
                )));
            }
            may_join_lower = false;
        }
    }

    fn lock(&self) -> MutexGuard<'_, Proven> {
        // Every update under the lock completes before it is released.
        self.proven.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// [`join_or_start_for`](Self::join_or_start_for), joining a fetch for a
    /// lower height if there is one.
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
    /// allow, and whether its result is sure to cover `height`.
    ///
    /// A fetch for height `H` proves a target at or above `H` minus the
    /// window, so its key is sure to be served for heights up to `H`. A
    /// caller joins such a fetch for a height at most a window below its
    /// own; with `may_join_lower`, it otherwise joins one for a height at
    /// most a window above, and must check what it proved.
    ///
    /// A fetch or a failure counts only for heights within the validity
    /// window of its own, since the height comes from an unverified response:
    /// a fetch or failure at a bogus height must not hold back, or cool down,
    /// a request for an honest one, which runs its own fetch.
    fn join_or_start_for(
        self: &Arc<Self>,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        height: u32,
        may_join_lower: bool,
    ) -> Result<(SharedFetch, bool), String> {
        let quorum = (quorum_type, quorum_hash);
        let window = self.window;
        let near = |other: u32| other.abs_diff(height) <= self.window;
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
            if height <= cached.active_at.saturating_add(self.window) {
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
                    let covers = height <= *running && *running - height <= window;
                    let lower = may_join_lower && *running < height && height - *running <= window;
                    (covers || lower)
                        .then(|| fetch.upgrade())
                        .flatten()
                        .map(|fetch| (fetch, covers))
                }
                _ => None,
            })
            .max_by_key(|(_, covers)| *covers);
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
                } if *other == quorum && near(*failed) => Some((*at, *failures)),
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
        // The earlier failures near this height stay until the new fetch
        // ends, so a fetch every caller abandons does not erase them.
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
        let window = self.window;
        proven.fetches.retain(|(quorum, slot)| match slot {
            Slot::Failed { height, .. } => {
                *quorum != attempt.quorum || height.abs_diff(attempt.height) > window
            }
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
                            let ask = choose_ask(
                                &self.lock().states,
                                self.checkpoint.height,
                                height,
                                sources[source].ask_tip,
                            )
                            .unwrap_or_else(|| Ask {
                                checkpoint: self.checkpoint.clone(),
                                height,
                            });
                            let body = request_body(&ask, quorum);
                            let remaining = deadline.saturating_duration_since(now);
                            let timeout = self.timing.request_timeout.min(remaining);
                            waiting_for_place = false;
                            sources[source].asked += 1;
                            sources[source].in_flight = true;
                            sent += 1;
                            hedge_at = now + self.timing.hedge_after;
                            let endpoint = &self.endpoints[source];
                            requests.push(async move {
                                let reply = self.transport.post(endpoint, body, timeout).await;
                                (source, ask, now, timeout, reply)
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
            let (source, ask, asked_at, timeout, reply) = answered;
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
                    // Next time, ask this source for the other height.
                    state.ask_tip = ask.height != 0;
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
        let needed = height.saturating_sub(self.window);
        // The target height is checked before any signature, so a bogus
        // height costs no BLS work. Only this source is affected if it lies.
        let verified = match bootstrap::verify(body, &ask.checkpoint, needed) {
            Ok(verified) => verified,
            Err(error) if error.kind() == ErrorKind::StaleTarget && ask.height == 0 => {
                return Verdict::BeyondTip(format!(
                    "its latest ChainLock is below the core height {needed} that {height} needs"
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
            Ok(proven) => Verdict::Proven(proven),
            Err(reason) => Verdict::CannotServe(reason),
        }
    }

    /// Cache a proven key and remember the state that proved it, and return
    /// what the cache now holds for the quorum. Nothing is cached if either
    /// conflicts with what is already known.
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
                cached.active_at = cached.active_at.max(target.height);
                *cached
            }
            None => {
                let cached = Cached {
                    key,
                    active_at: target.height,
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
    fn should_ask_from_the_newest_state_below_the_height_but_never_fall_back_to_the_checkpoint() {
        let cases = [
            // (verified heights, height, alternate, expected, why)
            (&[100][..], 150, false, ask(100, 150), "checkpoint below"),
            (&[100], 150, true, ask(100, 0), "alternate asks for the tip"),
            (&[100], 100, false, ask(100, 0), "nothing below: the tip"),
            (
                &[100],
                50,
                false,
                ask(100, 0),
                "height under the checkpoint",
            ),
            (&[100, 200], 250, false, ask(200, 250), "later state below"),
            (
                &[100, 200],
                150,
                false,
                ask(200, 0),
                "no downgrade to checkpoint",
            ),
            (
                &[100, 200, 300],
                250,
                false,
                ask(200, 250),
                "verified state below",
            ),
            (
                &[100, 200, 300],
                250,
                true,
                ask(200, 0),
                "alternate from it",
            ),
        ];
        for (heights, height, alternate, expected, why) in cases {
            assert_eq!(
                choose_ask(&states(heights), 100, height, alternate),
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
    fn should_join_a_lower_fetch_only_when_asked_to_and_say_it_may_not_cover() {
        // The fetch for 150 keeps its place, and the one for 400 takes the
        // second of the two, so starting it is not refused.
        let fetches = Arc::new(fetches());
        let _lower = fetches.join_or_start(6, [1; 32], 150).expect("starts");
        let id = running_id(&fetches, (6, [1; 32]), 150);
        let (_, covers) = fetches.join_or_start(6, [1; 32], 100).expect("joins");
        assert!(covers, "a fetch for 150 covers 100");
        let (_, covers) = fetches.join_or_start(6, [1; 32], 400).expect("joins");
        assert!(!covers, "a fetch for 150 may prove only up to 150");
        assert_eq!(running_id(&fetches, (6, [1; 32]), 400), None);
        let (_, covers) = fetches
            .join_or_start_for(6, [1; 32], 400, false)
            .expect("its own fetch");
        assert!(covers);
        assert!(running_id(&fetches, (6, [1; 32]), 400).is_some());
        assert_eq!(running_id(&fetches, (6, [1; 32]), 150), id);
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
        assert_eq!(fetches.lookup(6, [9; 32], 800).ok(), Some([1; 48]));
        assert!(
            fetches.lookup(6, [9; 32], 801).is_err(),
            "the conflicting proof must not extend the window"
        );
        assert_eq!(fetches.latest_state(), state(200));
    }

    #[test]
    fn should_never_lower_a_quorums_active_height() {
        let fetches = fetches();
        let quorum = (6, [9; 32]);
        fetches
            .publish(quorum, [1; 48], &state(300))
            .expect("later");
        fetches
            .publish(quorum, [1; 48], &state(200))
            .expect("earlier");
        assert_eq!(fetches.lookup(6, [9; 32], 900).ok(), Some([1; 48]));
        assert_eq!(fetches.latest_state(), state(300));
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
