//! Refreshes of the quorum lists that misses share.
//!
//! When proof verification meets a quorum the provider has not cached, the
//! SDK asks the provider for its key through
//! [`ContextProvider::fetch_quorum_public_key`](dash_context_provider::ContextProvider::fetch_quorum_public_key).
//! Many requests meet a new quorum at about the same time, and any node can
//! name a quorum that does not exist in every response it sends, so misses
//! share refreshes: a miss joins a refresh that is still running, and the
//! starts of two refreshes are at least [`MIN_GAP`] apart.

use crate::types::QuorumData;
use futures::future::{BoxFuture, FutureExt, Shared};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use web_time::Instant;

/// Timeout of each quorum list request. Proof verification waits on these
/// requests when it meets a quorum newer than the caches.
pub(crate) const QUORUM_LIST_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// How long a miss may join a refresh that has not finished. A refresh's
/// requests time out by then. A shared future only runs while it is polled,
/// so an unfinished refresh that old was abandoned by every waiter, and
/// resuming it would only report a timeout.
pub(crate) const JOIN_WINDOW: Duration = QUORUM_LIST_REQUEST_TIMEOUT;

/// Minimum time between the starts of two refreshes that misses ask for.
/// This bounds the load quorum hashes that do not exist can put on the quorum
/// service, whether or not the SDK bans the nodes that send them, and spaces
/// out attempts while the service fails.
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

    /// The listed quorum with this hash, and whether the current list has it.
    pub(crate) fn find(&self, quorum_hash: &[u8; 32]) -> Option<(&QuorumData, bool)> {
        fn listed<'a>(
            quorums: &'a Result<Vec<QuorumData>, String>,
            quorum_hash: &[u8; 32],
        ) -> Option<&'a QuorumData> {
            quorums.as_ref().ok()?.iter().find(|quorum| {
                hex::decode(&quorum.quorum_hash).ok().as_deref() == Some(quorum_hash.as_slice())
            })
        }
        listed(&self.current, quorum_hash)
            .map(|quorum| (quorum, true))
            .or_else(|| listed(&self.previous, quorum_hash).map(|quorum| (quorum, false)))
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

pub(crate) type SharedRefresh = Shared<BoxFuture<'static, Arc<FetchedQuorums>>>;

struct LatestRefresh {
    refresh: SharedRefresh,
    started: Instant,
    finished: Arc<AtomicBool>,
}

/// What a miss does about the latest refresh.
#[derive(Debug, PartialEq, Eq)]
enum Choice {
    Join,
    Start,
    RateLimited,
}

/// Decide what a miss does, given when the latest refresh started and whether
/// it has finished. With `started_after`, only a refresh that started at or
/// after that instant will do.
fn choose(latest: Option<(Instant, bool)>, now: Instant, started_after: Option<Instant>) -> Choice {
    let Some((started, finished)) = latest else {
        return Choice::Start;
    };
    let age = now.saturating_duration_since(started);
    let running = !finished && age < JOIN_WINDOW;
    match started_after {
        None if running || age < MIN_GAP => Choice::Join,
        None => Choice::Start,
        Some(after) if started >= after && (finished || running) => Choice::Join,
        Some(_) if age < MIN_GAP => Choice::RateLimited,
        Some(_) => Choice::Start,
    }
}

/// The latest refresh of a provider and its clones.
#[derive(Default)]
pub(crate) struct QuorumRefreshes {
    latest: Mutex<Option<LatestRefresh>>,
}

impl QuorumRefreshes {
    /// The refresh a miss waits on, and when it started, or `None` when a new
    /// refresh would start less than [`MIN_GAP`] after the latest one. With
    /// `started_after`, only a refresh that started at or after that instant
    /// is returned. `fetch` builds the refresh when one has to start; it must
    /// do no work until it is first polled, because it is built under the lock.
    pub(crate) fn refresh_for(
        &self,
        started_after: Option<Instant>,
        fetch: impl FnOnce() -> BoxFuture<'static, FetchedQuorums>,
    ) -> Option<(SharedRefresh, Instant)> {
        let mut latest = self.latest.lock().unwrap_or_else(PoisonError::into_inner);
        let now = Instant::now();
        let state = latest
            .as_ref()
            .map(|latest| (latest.started, latest.finished.load(Ordering::Acquire)));
        match choose(state, now, started_after) {
            Choice::RateLimited => return None,
            Choice::Join => {
                if let Some(latest) = latest.as_ref() {
                    return Some((latest.refresh.clone(), latest.started));
                }
            }
            Choice::Start => {}
        }
        Some(Self::publish(&mut latest, now, fetch()))
    }

    /// Start a refresh regardless of the latest one, and publish it so misses
    /// that come in while it runs join it.
    pub(crate) fn start(&self, fetch: BoxFuture<'static, FetchedQuorums>) -> SharedRefresh {
        let mut latest = self.latest.lock().unwrap_or_else(PoisonError::into_inner);
        Self::publish(&mut latest, Instant::now(), fetch).0
    }

    fn publish(
        latest: &mut Option<LatestRefresh>,
        now: Instant,
        fetch: BoxFuture<'static, FetchedQuorums>,
    ) -> (SharedRefresh, Instant) {
        let finished = Arc::new(AtomicBool::new(false));
        let done = Arc::clone(&finished);
        let refresh = async move {
            let fetched = fetch.await;
            done.store(true, Ordering::Release);
            Arc::new(fetched)
        }
        .boxed()
        .shared();
        *latest = Some(LatestRefresh {
            refresh: refresh.clone(),
            started: now,
            finished,
        });
        (refresh, now)
    }

    /// Make the latest refresh look `by` older, as if that much time had passed.
    #[cfg(test)]
    pub(crate) fn age_latest(&self, by: Duration) {
        if let Some(latest) = self.latest.lock().unwrap().as_mut() {
            latest.started -= by;
        }
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
    std::panic::AssertUnwindSafe(async move { fetch().await })
        .catch_unwind()
        .await
        .unwrap_or_else(|_| FetchedQuorums::failed("quorum refresh panicked"))
}

/// Run a refresh as a `Send` future.
///
/// Browser fetches are not `Send`, so the refresh is built and run on the
/// local executor and only its result crosses back, as `rs-dapi-client` does
/// for its wasm transport. A panic aborts on wasm32, so the dropped sender is
/// what a failed task leaves behind.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn contained<F, Fut>(fetch: F) -> FetchedQuorums
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = FetchedQuorums> + 'static,
{
    let (sender, receiver) = futures::channel::oneshot::channel();
    wasm_bindgen_futures::spawn_local(async move {
        let _ = sender.send(fetch().await);
    });
    receiver
        .await
        .unwrap_or_else(|_| FetchedQuorums::failed("quorum refresh task was dropped"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(base: Instant, secs: f64) -> Instant {
        base + Duration::from_secs_f64(secs)
    }

    /// The rules that keep misses from refreshing more often than the quorum
    /// service can bear, while never judging a quorum absent from a refresh
    /// that started before the miss.
    #[test]
    fn should_choose_which_refresh_a_miss_waits_on() {
        let t0 = Instant::now();
        let cases = [
            // (latest (started, finished), now, started_after, expected, why)
            (None, at(t0, 0.0), None, Choice::Start, "nothing to join"),
            (
                Some((t0, false)),
                at(t0, 0.5),
                None,
                Choice::Join,
                "running refresh",
            ),
            (
                Some((t0, false)),
                at(t0, 4.9),
                None,
                Choice::Join,
                "running within the join window",
            ),
            (
                Some((t0, false)),
                at(t0, 5.0),
                None,
                Choice::Start,
                "abandoned refresh is not resumed",
            ),
            (
                Some((t0, true)),
                at(t0, 0.9),
                None,
                Choice::Join,
                "finished within the minimum gap",
            ),
            (
                Some((t0, true)),
                at(t0, 1.0),
                None,
                Choice::Start,
                "finished refresh is not reused past the gap",
            ),
            (
                Some((t0, true)),
                at(t0, 0.5),
                Some(t0 - Duration::from_millis(100)),
                Choice::Join,
                "started after the miss",
            ),
            (
                Some((t0, false)),
                at(t0, 0.5),
                Some(t0),
                Choice::Join,
                "running, started at the miss",
            ),
            (
                Some((t0, true)),
                at(t0, 0.5),
                Some(at(t0, 0.1)),
                Choice::RateLimited,
                "older than the miss, within the gap",
            ),
            (
                Some((t0, true)),
                at(t0, 1.5),
                Some(at(t0, 0.1)),
                Choice::Start,
                "older than the miss, past the gap",
            ),
            (
                Some((t0, false)),
                at(t0, 6.0),
                Some(t0),
                Choice::Start,
                "started after the miss but abandoned",
            ),
        ];
        for (latest, now, started_after, expected, why) in cases {
            assert_eq!(choose(latest, now, started_after), expected, "{why}");
        }
    }

    /// Clones of a provider share their refreshes, and an unfinished one is
    /// joined instead of started again.
    #[tokio::test]
    async fn should_publish_a_refresh_for_later_misses_to_join() {
        let refreshes = QuorumRefreshes::default();
        let built = std::sync::atomic::AtomicUsize::new(0);
        let fetch = || {
            built.fetch_add(1, Ordering::SeqCst);
            async { FetchedQuorums::failed("unused") }.boxed()
        };

        let (first, first_started) = refreshes.refresh_for(None, fetch).expect("first refresh");
        let (second, second_started) = refreshes.refresh_for(None, fetch).expect("joined refresh");
        assert_eq!(built.load(Ordering::SeqCst), 1);
        assert_eq!(first_started, second_started);
        assert!(Arc::ptr_eq(&first.await, &second.await));

        // The finished refresh predates a miss made now; within the gap the
        // miss is rate-limited, past it a new refresh starts.
        let called = Instant::now();
        assert!(refreshes.refresh_for(Some(called), fetch).is_none());
        refreshes.age_latest(MIN_GAP);
        let (_, started) = refreshes
            .refresh_for(Some(called), fetch)
            .expect("a new refresh past the gap");
        assert!(started >= called);
        assert_eq!(built.load(Ordering::SeqCst), 2);
    }

    /// A panicking refresh must not take down every request waiting on it.
    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn should_turn_a_panicking_refresh_into_failed_lists() {
        let fetched = contained(|| {
            std::future::ready(()).map(|()| -> FetchedQuorums { panic!("refresh bug") })
        })
        .await;
        assert!(fetched.current.is_err() && fetched.previous.is_err());
        assert!(fetched
            .incomplete()
            .is_some_and(|reason| reason.contains("panicked")));
    }
}
