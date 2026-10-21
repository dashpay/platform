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
use dash_context_provider::ContextProviderError;
use dash_core_proof::bootstrap::{self, RecordKind};
use dash_core_proof::{parse_commitment, rpc_hex, State};
use futures::future::{self, select, BoxFuture, Either, FutureExt, Shared};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::sleep;
use lru::LruCache;
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::num::NonZeroUsize;
#[cfg(not(target_arch = "wasm32"))]
use std::panic::AssertUnwindSafe;
use std::pin::pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use url::Url;
use web_time::Instant;

/// A quorum's BLS public key.
pub(crate) type QuorumKey = [u8; 48];

/// A quorum: its LLMQ type and its hash in the order the SDK uses (Core's RPC
/// display order).
type QuorumId = (u32, [u8; 32]);

/// Most requests one fetch sends, across sources and retries.
const MAX_REQUESTS_PER_FETCH: usize = 8;
/// Most requests of one fetch in flight at once: the first and one hedge.
const MAX_REQUESTS_IN_FLIGHT: usize = 2;
/// Most distinct quorums being proven at once. Each costs a relay a Core
/// call, and the quorums come from unverified responses.
const MAX_QUORUMS_IN_FLIGHT: usize = 2;
/// Most quorums with a running or recently failed fetch remembered.
const MAX_FETCH_SLOTS: usize = 64;
/// Verified states kept besides the checkpoint, to start later proofs from.
const MAX_RECENT_STATES: usize = 16;
/// The retry delay after a relay's 502 doubles this many times.
const MAX_RETRY_DOUBLINGS: u32 = 3;
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
    /// First delay before asking again after a relay's 502.
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

/// A proven key and the highest target height that proved it.
struct Cached {
    key: QuorumKey,
    active_at: u32,
}

type SharedFetch = Shared<BoxFuture<'static, Result<(), String>>>;

/// The latest fetch of one quorum.
enum Slot {
    Running {
        id: u64,
        height: u32,
        fetch: SharedFetch,
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
    fetches: HashMap<QuorumId, Slot>,
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
    Proven,
    /// The relay could not serve the proof yet; ask again after `after`, for
    /// the other height.
    Retry {
        reason: String,
        after: Duration,
    },
    /// The source failed and cools down.
    SourceFailed(String),
    /// A valid proof of the relay's latest ChainLock does not reach the
    /// height: asking again cannot help until the chain grows.
    BeyondTip(String),
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
            fetches: HashMap::new(),
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
    pub(crate) async fn fetch(
        self: Arc<Self>,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        height: u32,
    ) -> Result<QuorumKey, ContextProviderError> {
        if let Ok(key) = self.lookup(quorum_type, quorum_hash, height) {
            return Ok(key);
        }
        let fetch = self
            .join_or_start(quorum_type, quorum_hash, height)
            .map_err(ContextProviderError::QuorumSourceUnavailable)?;
        fetch
            .await
            .map_err(ContextProviderError::QuorumSourceUnavailable)?;
        // A fetch this one joined may have proven the quorum only for a lower
        // height; the lookup then reports the source as unavailable.
        self.lookup(quorum_type, quorum_hash, height)
    }

    fn lock(&self) -> MutexGuard<'_, Proven> {
        // Every update under the lock completes before it is released.
        self.proven.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The running fetch of this quorum, or a new one if the limits allow.
    ///
    /// A fetch or a failure counts only for heights within the validity
    /// window of its own, since the height comes from an unverified response:
    /// a bogus height must not hold back, or cool down, a request for an
    /// honest one.
    fn join_or_start(
        self: &Arc<Self>,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        height: u32,
    ) -> Result<SharedFetch, String> {
        let quorum = (quorum_type, quorum_hash);
        let near = |other: u32| other.abs_diff(height) <= self.window;
        let now = Instant::now();
        let forgotten_after = self.timing.max_quorum_cooldown.saturating_mul(2);
        let mut proven = self.lock();
        // A fetch every waiter dropped is not running natively; it is
        // replaced rather than joined or counted.
        proven.fetches.retain(|_, slot| match slot {
            Slot::Running { fetch, .. } => fetch.strong_count() != Some(1),
            Slot::Failed { at, .. } => now.saturating_duration_since(*at) < forgotten_after,
        });
        // A fetch that finished since the caller's lookup has nothing to add.
        if let Some(cached) = proven.keys.peek(&quorum) {
            if height <= cached.active_at.saturating_add(self.window) {
                return Ok(future::ready(Ok(())).boxed().shared());
            }
        }
        let failures = match proven.fetches.get(&quorum) {
            Some(Slot::Running {
                fetch,
                height: running,
                ..
            }) if near(*running) => {
                return Ok(fetch.clone());
            }
            Some(Slot::Running {
                height: running, ..
            }) => {
                return Err(format!(
                    "quorum {} is being proven for core height {running}; retry",
                    hex::encode(quorum_hash)
                ));
            }
            Some(Slot::Failed {
                at,
                failures,
                height: failed,
            }) if near(*failed) => {
                let cooldown = backoff(
                    self.timing.quorum_cooldown,
                    self.timing.max_quorum_cooldown,
                    *failures,
                );
                let wait = cooldown.saturating_sub(now.saturating_duration_since(*at));
                if !wait.is_zero() {
                    return Err(format!(
                        "quorum {} was asked for too recently, retry in {} s",
                        hex::encode(quorum_hash),
                        wait.as_millis().div_ceil(1000)
                    ));
                }
                *failures
            }
            Some(Slot::Failed { .. }) | None => 0,
        };
        let running = proven
            .fetches
            .values()
            .filter(|slot| matches!(slot, Slot::Running { .. }))
            .count();
        if running >= MAX_QUORUMS_IN_FLIGHT {
            return Err(format!(
                "{running} other quorums are being proven; not starting another"
            ));
        }
        if !proven.fetches.contains_key(&quorum) && proven.fetches.len() >= MAX_FETCH_SLOTS {
            return Err("too many quorums were asked for recently".to_string());
        }
        proven.next_fetch_id += 1;
        let id = proven.next_fetch_id;
        // Built under the lock, so it must do nothing until it is polled.
        let fetch = Arc::clone(self).start(quorum, height, id, failures);
        proven.fetches.insert(
            quorum,
            Slot::Running {
                id,
                height,
                fetch: fetch.clone(),
            },
        );
        Ok(fetch)
    }

    /// A fetch every waiter of this quorum shares.
    fn start(
        self: Arc<Self>,
        quorum: QuorumId,
        height: u32,
        id: u64,
        failures: u32,
    ) -> SharedFetch {
        async move {
            let prover = Arc::clone(&self);
            let result = contained(move || async move {
                let result = prover.prove(quorum, height).await;
                // Recorded here too, so a browser fetch that outlives its
                // waiters still frees its slot.
                prover.finish(quorum, height, id, failures, &result);
                result
            })
            .await;
            // Covers a fetch that panicked; a no-op otherwise.
            self.finish(quorum, height, id, failures, &result);
            result
        }
        .boxed()
        .shared()
    }

    /// Record how fetch `id` of a quorum ended, if its slot is still its own.
    fn finish(
        &self,
        quorum: QuorumId,
        height: u32,
        id: u64,
        failures: u32,
        result: &Result<(), String>,
    ) {
        let mut proven = self.lock();
        let ours = matches!(
            proven.fetches.get(&quorum),
            Some(Slot::Running { id: running, .. }) if *running == id
        );
        if ours && result.is_ok() {
            proven.fetches.remove(&quorum);
        } else if ours {
            let failed = Slot::Failed {
                at: Instant::now(),
                failures: failures.saturating_add(1),
                height,
            };
            proven.fetches.insert(quorum, failed);
        }
    }

    /// Ask the sources for a proof of `quorum` until one verifies, the
    /// request budget is spent or the deadline passes.
    async fn prove(&self, quorum: QuorumId, height: u32) -> Result<(), String> {
        let started = Instant::now();
        let deadline = started + self.timing.deadline;
        let mut requests = FuturesUnordered::new();
        let mut asked = vec![0u32; self.endpoints.len()];
        let mut busy = Vec::new();
        let mut sent = 0;
        let mut retries = 0;
        let mut alternate = false;
        let mut next_start = started;
        let mut failures = Vec::new();
        loop {
            let now = Instant::now();
            if now >= deadline {
                failures.push("deadline passed".to_string());
                break;
            }
            let mut wake = deadline;
            if requests.len() < MAX_REQUESTS_IN_FLIGHT && sent < MAX_REQUESTS_PER_FETCH {
                if now < next_start {
                    wake = next_start.min(deadline);
                } else if let Some(source) = self.pick_source(now, &asked, &busy) {
                    let ask = choose_ask(
                        &self.lock().states,
                        self.checkpoint.height,
                        height,
                        alternate,
                    )
                    .unwrap_or_else(|| Ask {
                        checkpoint: self.checkpoint.clone(),
                        height,
                    });
                    let body = request_body(&ask, quorum);
                    let remaining = deadline.saturating_duration_since(now);
                    let timeout = self.timing.request_timeout.min(remaining);
                    // A request cut short by the deadline says nothing about
                    // its source.
                    let cut_short = timeout < self.timing.request_timeout;
                    asked[source] += 1;
                    busy.push(source);
                    sent += 1;
                    next_start = now + self.timing.hedge_after;
                    let endpoint = &self.endpoints[source];
                    requests.push(async move {
                        let reply = self.transport.post(endpoint, body, timeout).await;
                        (source, ask, now, cut_short, reply)
                    });
                    continue;
                } else if requests.is_empty() {
                    failures.push("every source is cooling down after failures".to_string());
                    break;
                }
            } else if requests.is_empty() {
                break;
            }

            let wait = wake.saturating_duration_since(now);
            if requests.is_empty() {
                sleep(wait).await;
                continue;
            }
            let answered = match select(requests.next(), pin!(sleep(wait))).await {
                Either::Left((Some(answered), _)) => answered,
                Either::Left((None, _)) | Either::Right(_) => continue,
            };
            let (source, ask, asked_at, cut_short, reply) = answered;
            busy.retain(|busy| *busy != source);
            let origin = self.endpoints[source].origin().ascii_serialization();
            let elapsed = asked_at.elapsed().as_millis();
            let verdict = match reply {
                Err(PostError::Failed(reason)) if cut_short => Verdict::Retry {
                    reason,
                    after: Duration::ZERO,
                },
                reply => self.judge(quorum, height, &ask, reply, retries),
            };
            match verdict {
                Verdict::Proven => {
                    self.source_answered(source);
                    tracing::debug!(
                        quorum_type = quorum.0,
                        quorum_hash = %hex::encode(quorum.1),
                        height,
                        %origin,
                        elapsed_ms = elapsed,
                        "proved quorum key"
                    );
                    return Ok(());
                }
                Verdict::Retry { reason, after } => {
                    failures.push(format!("{origin}: {reason} after {elapsed} ms"));
                    retries += 1;
                    alternate = !alternate;
                    next_start = Instant::now() + after;
                }
                Verdict::SourceFailed(reason) => {
                    failures.push(format!("{origin}: {reason} after {elapsed} ms"));
                    self.source_failed(source);
                    next_start = Instant::now();
                }
                Verdict::BeyondTip(reason) => {
                    failures.push(format!("{origin}: {reason}"));
                    break;
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
        Err(reason)
    }

    /// Judge a relay's answer, publishing the key if it proves one.
    ///
    /// Only what an honest relay never does cools a source down: failed or
    /// oversized transfers, redirects, error statuses other than 502, and
    /// proofs that do not verify. A valid proof that does not serve the
    /// request may be the fault of the unverified height or quorum, so it is
    /// retried instead.
    fn judge(
        &self,
        quorum: QuorumId,
        height: u32,
        ask: &Ask,
        reply: Result<Reply, PostError>,
        retries: u32,
    ) -> Verdict {
        let retry = |reason: String| Verdict::Retry {
            reason,
            after: self
                .timing
                .first_retry
                .saturating_mul(1 << retries.min(MAX_RETRY_DOUBLINGS)),
        };
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
            // The relay's Core could not serve this proof (yet): worth asking
            // again, for the other height.
            502 => return retry(format!("HTTP 502{}", body_excerpt(body))),
            // Anything else, 503 (workers busy) included, cools the source down.
            status => return Verdict::SourceFailed(format!("HTTP {status}{}", body_excerpt(body))),
        }
        let verified = match bootstrap::verify(body, &ask.checkpoint, 0) {
            Ok(verified) => verified,
            Err(error) => return Verdict::SourceFailed(format!("invalid proof: {error}")),
        };
        let target = verified.state().height;
        let needed = height.saturating_sub(self.window);
        if target < needed && ask.height == 0 {
            return Verdict::BeyondTip(format!(
                "its latest ChainLock, at core height {target}, is below the {needed} that core \
                 height {height} needs"
            ));
        }
        if target < needed {
            return retry(format!(
                "proof ends at core height {target}, below {needed}"
            ));
        }
        let mut keys = Vec::new();
        // EvoNode records are not asked for; any that come back are ignored.
        for record in verified.records() {
            if record.kind != RecordKind::Quorum {
                continue;
            }
            match parse_commitment(record.leaf) {
                Ok(commitment) => {
                    let mut hash = commitment.quorum_hash;
                    hash.reverse();
                    if (u32::from(commitment.quorum_type), hash) == quorum {
                        keys.push(commitment.public_key);
                    }
                }
                // Records of other quorums do not matter; one that names the
                // requested quorum must parse.
                Err(error) if names(record.leaf, quorum) => {
                    return retry(format!("unparsable record of the quorum: {error}"));
                }
                Err(_) => {}
            }
        }
        let [key] = keys.as_slice() else {
            return retry(format!(
                "proof opens the requested quorum {} times, not once",
                keys.len()
            ));
        };
        match self.publish(quorum, *key, verified.state()) {
            Ok(()) => Verdict::Proven,
            Err(reason) => retry(reason),
        }
    }

    /// Cache a proven key and remember the state that proved it.
    fn publish(&self, quorum: QuorumId, key: QuorumKey, target: &State) -> Result<(), String> {
        let mut proven = self.lock();
        match proven.keys.get_mut(&quorum) {
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
            Some(cached) => cached.active_at = cached.active_at.max(target.height),
            None => {
                proven.keys.put(
                    quorum,
                    Cached {
                        key,
                        active_at: target.height,
                    },
                );
            }
        }
        match proven.states.get(&target.height) {
            Some(known) if known != target => tracing::error!(
                height = target.height,
                known = %rpc_hex(&known.block_hash),
                proven = %rpc_hex(&target.block_hash),
                "two proofs verified different blocks at one height; keeping the first"
            ),
            Some(_) => {}
            None => {
                proven.states.insert(target.height, target.clone());
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
        }
        Ok(())
    }

    /// The source to ask next: a healthy one with no request of this fetch
    /// in flight, asked the fewest times so far.
    fn pick_source(&self, now: Instant, asked: &[u32], busy: &[usize]) -> Option<usize> {
        let proven = self.lock();
        proven
            .sources
            .iter()
            .enumerate()
            .filter(|(source, health)| {
                !busy.contains(source) && health.failed_until.is_none_or(|until| until <= now)
            })
            .min_by_key(|(source, _)| asked[*source])
            .map(|(source, _)| source)
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

/// Whether a commitment's leading bytes (version, type, hash) name `quorum`.
fn names(leaf: &[u8], (quorum_type, quorum_hash): QuorumId) -> bool {
    let mut wire = quorum_hash;
    wire.reverse();
    leaf.get(2).map(|t| u32::from(*t)) == Some(quorum_type) && leaf.get(3..35) == Some(&wire[..])
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

/// The start of an error body, for the failure reason.
fn body_excerpt(body: &[u8]) -> String {
    let text: String = String::from_utf8_lossy(body)
        .chars()
        .filter(|c| !c.is_control())
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
async fn contained<F, Fut>(prove: F) -> Result<(), String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<(), String>>,
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
async fn contained<F, Fut>(prove: F) -> Result<(), String>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), String>> + 'static,
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
