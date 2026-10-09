//! Refreshes of the quorum lists that misses share.
//!
//! When proof verification meets a quorum the provider has not cached, the
//! SDK asks the provider for its key through
//! [`ContextProvider::fetch_quorum_public_key`](dash_context_provider::ContextProvider::fetch_quorum_public_key).
//! Many requests meet a new quorum at about the same time, and any node can
//! name a quorum that does not exist in every response it sends, so misses
//! share refreshes: a miss joins a refresh that is still running, and the
//! starts of two refreshes are at least [`MIN_GAP`] apart.
//!
//! Every refresh gets the next generation number. A miss notes the latest
//! generation when the SDK asks, which is after the response naming the
//! quorum arrived, so a refresh with a higher generation started after that
//! response. Only such a refresh may show that the quorum is absent.

use crate::types::QuorumData;
use futures::future::{BoxFuture, FutureExt, Shared};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use web_time::Instant;

/// Timeout of each quorum list request. Proof verification waits on these
/// requests when it meets a quorum newer than the caches.
pub(crate) const QUORUM_LIST_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// How long a miss may join a refresh that has not finished. Past the
/// requests' timeout, with a margin, an unfinished refresh has been abandoned
/// by every waiter: a shared future only runs while it is polled, and resuming
/// it would only report a timeout.
const JOIN_WINDOW: Duration = Duration::from_secs(QUORUM_LIST_REQUEST_TIMEOUT.as_secs() + 1);

/// Minimum time between the starts of two refreshes that misses ask for.
/// This bounds the load quorum hashes that do not exist can put on the quorum
/// service, whether or not the SDK bans the nodes that send them, and spaces
/// out attempts while the service fails. A finished refresh, failed or not, is
/// reused for this long.
pub(crate) const MIN_GAP: Duration = Duration::from_secs(1);

/// Both quorum lists, as one refresh fetched them.
pub(crate) struct FetchedQuorums {
    pub(crate) current: Result<Vec<QuorumData>, String>,
    pub(crate) previous: Result<Vec<QuorumData>, String>,
}

impl FetchedQuorums {
    fn failed(reason: &str) -> Self {
        Self {
            current: Err(reason.to_string()),
            previous: Err(reason.to_string()),
        }
    }

    /// Matching quorums from both lists, current first, with their list origin.
    pub(crate) fn matching<'a>(
        &'a self,
        quorum_hash: &'a [u8; 32],
    ) -> impl Iterator<Item = (&'a QuorumData, bool)> + 'a {
        [(&self.current, true), (&self.previous, false)]
            .into_iter()
            .flat_map(move |(quorums, current)| {
                quorums.iter().flatten().filter_map(move |quorum| {
                    (hex::decode(&quorum.quorum_hash).ok().as_deref()
                        == Some(quorum_hash.as_slice()))
                    .then_some((quorum, current))
                })
            })
    }

    /// Why these lists cannot show that a quorum is absent, if they cannot.
    pub(crate) fn incomplete(&self) -> Option<String> {
        match (&self.current, &self.previous) {
            (Ok(_), Ok(_)) => None,
            (Err(current), Ok(_)) => Some(format!("current quorums: {current}")),
            (Ok(_), Err(previous)) => Some(format!("previous quorums: {previous}")),
            (Err(current), Err(previous)) => Some(format!(
                "current quorums: {current}; previous quorums: {previous}"
            )),
        }
    }
}

type SharedRefresh = Shared<BoxFuture<'static, Arc<FetchedQuorums>>>;

struct LatestRefresh {
    refresh: SharedRefresh,
    generation: u64,
    started: Instant,
    finished: Arc<AtomicBool>,
}

/// The latest refresh and the number of refreshes started so far.
#[derive(Default)]
struct State {
    latest: Option<LatestRefresh>,
    generations: u64,
}

/// What a miss does about the latest refresh.
#[derive(Debug, PartialEq, Eq)]
enum Choice {
    Join,
    Start,
    /// Wait this long, then decide again.
    Wait(Duration),
}

/// The latest refresh, as a miss deciding what to do sees it.
#[derive(Clone, Copy)]
struct Seen {
    generation: u64,
    started: Instant,
    finished: bool,
}

/// Decide what a miss does about the latest refresh. With `newer_than`, only a
/// refresh of a higher generation will do.
fn choose(latest: Option<Seen>, now: Instant, newer_than: Option<u64>) -> Choice {
    let Some(latest) = latest else {
        return Choice::Start;
    };
    let age = now.saturating_duration_since(latest.started);
    let running = !latest.finished && age < JOIN_WINDOW;
    let recent = age < MIN_GAP;
    match newer_than {
        None if running || recent => Choice::Join,
        None => Choice::Start,
        Some(seen) if latest.generation > seen && (running || recent) => Choice::Join,
        Some(_) if recent => Choice::Wait(MIN_GAP - age),
        Some(_) => Choice::Start,
    }
}

/// The latest refresh of a provider and its clones.
#[derive(Default)]
pub(crate) struct QuorumRefreshes {
    state: Mutex<State>,
}

impl QuorumRefreshes {
    /// The generation of the latest refresh, or 0 if none has started.
    pub(crate) fn generation(&self) -> u64 {
        self.lock().generations
    }

    /// The refresh a miss waits on, and its generation: the latest one if it
    /// is running or started less than [`MIN_GAP`] ago, otherwise a new one.
    /// `fetch` builds the refresh when one has to start; it must do no work
    /// until it is first polled, because it is built under the lock.
    pub(crate) fn latest_or_start(
        &self,
        fetch: impl FnOnce() -> BoxFuture<'static, FetchedQuorums>,
    ) -> (SharedRefresh, u64) {
        let mut state = self.lock();
        let now = Instant::now();
        match (choose(state.seen(), now, None), state.latest.as_ref()) {
            (Choice::Join, Some(latest)) => (latest.refresh.clone(), latest.generation),
            _ => state.publish(now, fetch()),
        }
    }

    /// A refresh of a generation above `seen`: the latest one if it is, or a
    /// new one. Starting one before [`MIN_GAP`] has passed since the latest
    /// started is refused with how long to wait.
    pub(crate) fn newer_than(
        &self,
        seen: u64,
        fetch: impl FnOnce() -> BoxFuture<'static, FetchedQuorums>,
    ) -> Result<SharedRefresh, Duration> {
        let mut state = self.lock();
        let now = Instant::now();
        match (choose(state.seen(), now, Some(seen)), state.latest.as_ref()) {
            (Choice::Join, Some(latest)) => Ok(latest.refresh.clone()),
            (Choice::Wait(wait), _) => Err(wait),
            _ => Ok(state.publish(now, fetch()).0),
        }
    }

    /// Start a refresh regardless of the latest one, and publish it so misses
    /// that come in while it runs join it.
    pub(crate) fn start(&self, fetch: BoxFuture<'static, FetchedQuorums>) -> SharedRefresh {
        self.lock().publish(Instant::now(), fetch).0
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        // Nothing done under the lock can leave the state half-updated.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Make the latest refresh look `by` older, as if that much time had passed.
    #[cfg(test)]
    pub(crate) fn age_latest(&self, by: Duration) {
        if let Some(latest) = self.lock().latest.as_mut() {
            latest.started -= by;
        }
    }
}

impl State {
    fn seen(&self) -> Option<Seen> {
        self.latest.as_ref().map(|latest| Seen {
            generation: latest.generation,
            started: latest.started,
            finished: latest.finished.load(Ordering::Acquire),
        })
    }

    fn publish(
        &mut self,
        now: Instant,
        fetch: BoxFuture<'static, FetchedQuorums>,
    ) -> (SharedRefresh, u64) {
        self.generations += 1;
        let finished = Arc::new(AtomicBool::new(false));
        let done = Arc::clone(&finished);
        let refresh = async move {
            let fetched = fetch.await;
            done.store(true, Ordering::Release);
            Arc::new(fetched)
        }
        .boxed()
        .shared();
        self.latest = Some(LatestRefresh {
            refresh: refresh.clone(),
            generation: self.generations,
            started: now,
            finished,
        });
        (refresh, self.generations)
    }
}

/// Run a refresh as a `Send` future that never panics.
///
/// A panic inside a shared future would be raised again in every waiter, so it
/// becomes two failed lists instead.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn contained<F, Fut>(fetch: F) -> FetchedQuorums
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = FetchedQuorums> + Send + 'static,
{
    match std::panic::AssertUnwindSafe(async move { fetch().await })
        .catch_unwind()
        .await
    {
        Ok(fetched) => fetched,
        Err(payload) => {
            let message = payload
                .downcast_ref::<&str>()
                .map(|message| message.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            tracing::error!(%message, "quorum list refresh panicked");
            FetchedQuorums::failed(&format!("quorum refresh panicked: {message}"))
        }
    }
}

/// Run a refresh as a `Send` future.
///
/// Browser fetches are not `Send`, so the refresh is built and run on the
/// local executor and only its result crosses back. A panic traps on wasm32
/// rather than unwinding.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn contained<F, Fut>(fetch: F) -> FetchedQuorums
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = FetchedQuorums> + 'static,
{
    on_local_executor(fetch).await.unwrap_or_else(|| {
        tracing::warn!("quorum list refresh task was dropped");
        FetchedQuorums::failed("quorum refresh task was dropped")
    })
}

/// Wait for `duration`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

/// Wait for `duration`. Browser timers are not `Send`, so the timer runs on
/// the local executor.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn sleep(duration: Duration) {
    on_local_executor(move || gloo_timers::future::sleep(duration)).await;
}

/// Run a future that is not `Send` on the local executor and return its
/// output through a `Send` future, as `rs-dapi-client` does for its wasm
/// transport. `None` if the task is dropped before it finishes, which happens
/// only when the executor goes away.
#[cfg(target_arch = "wasm32")]
async fn on_local_executor<F, Fut>(make: F) -> Option<Fut::Output>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future + 'static,
    Fut::Output: Send + 'static,
{
    let (sender, receiver) = futures::channel::oneshot::channel();
    wasm_bindgen_futures::spawn_local(async move {
        let _ = sender.send(make().await);
    });
    receiver.await.ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(generation: u64, started: Instant, finished: bool) -> Option<Seen> {
        Some(Seen {
            generation,
            started,
            finished,
        })
    }

    fn after(base: Instant, millis: u64) -> Instant {
        base + Duration::from_millis(millis)
    }

    /// The rules that keep misses from refreshing more often than the quorum
    /// service can bear, while never judging a quorum absent from a refresh
    /// that started before the miss.
    #[test]
    fn should_choose_which_refresh_a_miss_waits_on() {
        let t0 = Instant::now();
        let wait = |millis| Choice::Wait(Duration::from_millis(millis));
        let cases = [
            // (latest, now, newer_than, expected, why)
            (None, t0, None, Choice::Start, "nothing to join"),
            (
                seen(1, t0, false),
                after(t0, 500),
                None,
                Choice::Join,
                "running refresh",
            ),
            (
                seen(1, t0, false),
                after(t0, 5_900),
                None,
                Choice::Join,
                "running within the join window",
            ),
            (
                seen(1, t0, false),
                after(t0, 6_000),
                None,
                Choice::Start,
                "abandoned refresh is not resumed",
            ),
            (
                seen(1, t0, true),
                after(t0, 900),
                None,
                Choice::Join,
                "finished within the minimum gap",
            ),
            (
                seen(1, t0, true),
                after(t0, 1_000),
                None,
                Choice::Start,
                "finished refresh is not reused past the gap",
            ),
            (
                seen(2, t0, true),
                after(t0, 500),
                Some(1),
                Choice::Join,
                "newer refresh, finished within the gap",
            ),
            (
                seen(2, t0, false),
                after(t0, 3_000),
                Some(1),
                Choice::Join,
                "newer refresh, still running",
            ),
            (
                seen(2, t0, true),
                after(t0, 3_000),
                Some(1),
                Choice::Start,
                "newer refresh, failed or not, is not reused past the gap",
            ),
            (
                seen(1, t0, true),
                after(t0, 400),
                Some(1),
                wait(600),
                "the miss saw this refresh: wait out the gap",
            ),
            (
                seen(1, t0, false),
                after(t0, 400),
                Some(1),
                wait(600),
                "the miss saw this running refresh: wait out the gap",
            ),
            (
                seen(1, t0, true),
                after(t0, 1_500),
                Some(1),
                Choice::Start,
                "the miss saw this refresh, the gap has passed",
            ),
            (
                seen(2, t0, false),
                after(t0, 6_000),
                Some(1),
                Choice::Start,
                "newer refresh, abandoned",
            ),
        ];
        for (latest, now, newer_than, expected, why) in cases {
            assert_eq!(choose(latest, now, newer_than), expected, "{why}");
        }
    }

    /// Clones of a provider share their refreshes: a running one is joined
    /// instead of started again, and a miss that saw a refresh is only served
    /// by a newer one.
    #[tokio::test]
    async fn should_publish_a_refresh_for_later_misses_to_join() {
        let refreshes = QuorumRefreshes::default();
        let built = std::sync::atomic::AtomicUsize::new(0);
        let fetch = || {
            built.fetch_add(1, Ordering::SeqCst);
            async { FetchedQuorums::failed("unused") }.boxed()
        };

        assert_eq!(refreshes.generation(), 0);
        let (first, first_generation) = refreshes.latest_or_start(fetch);
        let (second, second_generation) = refreshes.latest_or_start(fetch);
        assert_eq!(built.load(Ordering::SeqCst), 1);
        assert_eq!((first_generation, second_generation), (1, 1));
        assert!(Arc::ptr_eq(&first.await, &second.await));

        // A miss that saw generation 1 must wait out the gap, then gets a new
        // refresh.
        let seen = refreshes.generation();
        assert!(matches!(refreshes.newer_than(seen, fetch), Err(wait) if wait <= MIN_GAP));
        refreshes.age_latest(MIN_GAP);
        let _newer = refreshes
            .newer_than(seen, fetch)
            .expect("a new refresh past the gap");
        assert_eq!(refreshes.generation(), 2);
        assert_eq!(built.load(Ordering::SeqCst), 2);
    }

    /// A refresh that is still running past the gap is joined, not doubled;
    /// once it has finished, the gap applies.
    #[tokio::test]
    async fn should_join_a_running_refresh_past_the_gap_but_not_a_finished_one() {
        let refreshes = QuorumRefreshes::default();
        let built = std::sync::atomic::AtomicUsize::new(0);
        let (release, gate) = futures::channel::oneshot::channel::<()>();
        let mut gate = Some(gate);
        let mut fetch = || {
            built.fetch_add(1, Ordering::SeqCst);
            let gate = gate.take();
            async move {
                if let Some(gate) = gate {
                    let _ = gate.await;
                }
                FetchedQuorums::failed("unused")
            }
            .boxed()
        };

        let (running, _) = refreshes.latest_or_start(&mut fetch);
        let mut running = Box::pin(running);
        assert!(futures::poll!(&mut running).is_pending());
        refreshes.age_latest(2 * MIN_GAP);
        let _ = refreshes.latest_or_start(&mut fetch);
        assert_eq!(
            built.load(Ordering::SeqCst),
            1,
            "a running refresh is joined"
        );

        release.send(()).expect("release the refresh");
        running.await;
        let _ = refreshes.latest_or_start(&mut fetch);
        assert_eq!(
            built.load(Ordering::SeqCst),
            2,
            "a finished refresh is not reused past the gap"
        );
    }

    /// A panicking refresh must not take down every request waiting on it.
    #[tokio::test]
    async fn should_turn_a_panicking_refresh_into_failed_lists() {
        let fetched = contained(|| {
            std::future::ready(()).map(|()| -> FetchedQuorums { panic!("refresh bug") })
        })
        .await;
        assert!(fetched.current.is_err() && fetched.previous.is_err());
        assert!(fetched
            .incomplete()
            .is_some_and(|reason| reason.contains("panicked: refresh bug")));
    }
}
