//! One in-flight future shared by every caller that asks while it runs.
//!
//! [`SingleFlight::join_or_start`] starts a future, or joins the one running. Every caller
//! polls the shared future itself, so none depends on another caller being polled, and no
//! runtime or spawn is needed. A finished future is never joined: the next caller starts a
//! fresh one.
//!
//! Use it when:
//! - several concurrent callers need the same result (a probe, a refresh, a fetch) and only
//!   one request should go out;
//! - callers would otherwise hold an async mutex across an await while waiting for another
//!   caller's request, which futurelocks once the holder stops being polled.
//!
//! Do not use it for:
//! - keyed requests: it holds one slot, so different requests would join each other;
//! - work that must outlive the caller that started it: it is aborted when that caller is
//!   dropped.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Mutex, PoisonError, TryLockError};
use std::task::{Context, Poll};

use futures::future::{abortable, AbortHandle, Abortable, Aborted, FutureExt, Shared};

/// The future shared by the callers of a [`SingleFlight`].
type Running<Fut> = Shared<Abortable<Fut>>;

/// Why a joined future ended without an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the call that started this shared work was dropped before the work finished")]
pub struct StarterDropped;

/// At most one running future, shared by every caller that asks while it runs.
///
/// Use it so concurrent callers that need the same result send one request and none waits on
/// a caller that is not polled; not for keyed requests or work that must outlive its starter.
/// See the [module docs](self).
pub struct SingleFlight<Fut: Future> {
    running: Mutex<Option<Running<Fut>>>,
}

impl<Fut: Future> Default for SingleFlight<Fut> {
    fn default() -> Self {
        Self {
            running: Mutex::new(None),
        }
    }
}

impl<Fut: Future> fmt::Debug for SingleFlight<Fut> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("SingleFlight");
        // Never blocks, as `start` runs with the slot locked.
        match self.running.try_lock() {
            Ok(running) => debug.field("running", &running.is_some()),
            Err(TryLockError::Poisoned(running)) => {
                debug.field("running", &running.into_inner().is_some())
            }
            Err(TryLockError::WouldBlock) => debug.field("running", &format_args!("<locked>")),
        };
        debug.finish()
    }
}

impl<Fut> SingleFlight<Fut>
where
    Fut: Future,
    Fut::Output: Clone,
{
    /// An empty slot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Join the unfinished future running, or start the one `start` gives.
    ///
    /// `start` is called only when starting, with the slot locked, so it should only build
    /// the future and must not call [`Self::join_or_start`] on this `SingleFlight`. Awaiting a
    /// [`Joined`] future gives [`StarterDropped`] when the caller that started it is dropped
    /// before it finished; awaiting the [`Started`] one never does.
    ///
    /// ```
    /// use dash_async::single_flight::{Flight, SingleFlight};
    /// use futures::future::{BoxFuture, FutureExt};
    ///
    /// let refresh = SingleFlight::<BoxFuture<'static, u32>>::new();
    /// let answer = futures::executor::block_on(async {
    ///     match refresh.join_or_start(|| async { 42 }.boxed()) {
    ///         Flight::Started(started) => started.await,
    ///         Flight::Joined(joined) => joined.await,
    ///     }
    /// });
    /// assert_eq!(answer, Ok(42));
    /// ```
    #[must_use = "dropping a started flight aborts it"]
    pub fn join_or_start(&self, start: impl FnOnce() -> Fut) -> Flight<'_, Fut> {
        let mut running = self.running.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(future) = running.as_ref().filter(|future| future.peek().is_none()) {
            return Flight::Joined(Joined {
                future: future.clone(),
            });
        }
        let (future, abort) = abortable(start());
        let future = future.shared();
        *running = Some(future.clone());
        Flight::Started(Started {
            flight: self,
            future,
            abort,
        })
    }
}

/// What [`SingleFlight::join_or_start`] gives a caller.
#[must_use = "dropping a started flight aborts it"]
pub enum Flight<'a, Fut: Future> {
    /// The future another caller started.
    Joined(Joined<Fut>),
    /// The future this caller started.
    Started(Started<'a, Fut>),
}

impl<Fut: Future> fmt::Debug for Flight<'_, Fut> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Joined(joined) => f.debug_tuple("Joined").field(joined).finish(),
            Self::Started(started) => f.debug_tuple("Started").field(started).finish(),
        }
    }
}

/// A future another caller started; awaiting it polls it too.
#[must_use = "a joined future does nothing unless polled"]
pub struct Joined<Fut: Future> {
    future: Running<Fut>,
}

impl<Fut: Future> fmt::Debug for Joined<Fut> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Joined")
            .field("future", &self.future)
            .finish()
    }
}

impl<Fut> Future for Joined<Fut>
where
    Fut: Future,
    Fut::Output: Clone,
{
    type Output = Result<Fut::Output, StarterDropped>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        poll_running(&mut self.future, cx)
    }
}

/// The future this caller started; dropping it aborts the future for every caller and frees
/// the slot. Awaiting it never gives [`StarterDropped`].
#[must_use = "dropping a started flight aborts it"]
pub struct Started<'a, Fut: Future> {
    flight: &'a SingleFlight<Fut>,
    future: Running<Fut>,
    abort: AbortHandle,
}

impl<Fut: Future> fmt::Debug for Started<'_, Fut> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Started")
            .field("future", &self.future)
            .finish_non_exhaustive()
    }
}

impl<Fut> Future for Started<'_, Fut>
where
    Fut: Future,
    Fut::Output: Clone,
{
    type Output = Result<Fut::Output, StarterDropped>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        poll_running(&mut self.future, cx)
    }
}

impl<Fut: Future> Drop for Started<'_, Fut> {
    fn drop(&mut self) {
        self.abort.abort();
        let mut running = self
            .flight
            .running
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        // A newer future may hold the slot once this one finished.
        if running
            .as_ref()
            .is_some_and(|future| future.ptr_eq(&self.future))
        {
            *running = None;
        }
    }
}

/// Poll the shared future, an abort read as its starter dropped.
fn poll_running<Fut>(
    future: &mut Running<Fut>,
    cx: &mut Context<'_>,
) -> Poll<Result<Fut::Output, StarterDropped>>
where
    Fut: Future,
    Fut::Output: Clone,
{
    future
        .poll_unpin(cx)
        .map(|output| output.map_err(|Aborted| StarterDropped))
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::{pending, BoxFuture, LocalBoxFuture};
    use futures::task::noop_waker_ref;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    type Flights = SingleFlight<BoxFuture<'static, u32>>;

    fn started<'a, Fut: Future>(flight: Flight<'a, Fut>) -> Started<'a, Fut> {
        match flight {
            Flight::Started(started) => started,
            Flight::Joined(_) => panic!("expected to start a future, joined one"),
        }
    }

    fn joined<Fut: Future>(flight: Flight<'_, Fut>) -> Joined<Fut> {
        match flight {
            Flight::Joined(joined) => joined,
            Flight::Started(_) => panic!("expected to join a future, started one"),
        }
    }

    #[test]
    fn should_let_a_joiner_drive_a_future_whose_starter_is_not_polled() {
        let flights = Flights::new();
        let _starter = started(flights.join_or_start(|| async { 7 }.boxed()));
        let joiner = joined(flights.join_or_start(|| async { 8 }.boxed()));

        assert_eq!(futures::executor::block_on(joiner), Ok(7));
    }

    #[test]
    fn should_abort_and_clear_when_the_starter_is_dropped() {
        let flights = Flights::new();
        let starter = started(flights.join_or_start(|| pending().boxed()));
        let mut joiner = joined(flights.join_or_start(|| async { 8 }.boxed()));
        let mut cx = Context::from_waker(noop_waker_ref());
        assert!(Pin::new(&mut joiner).poll(&mut cx).is_pending());

        drop(starter);

        assert_eq!(
            Pin::new(&mut joiner).poll(&mut cx),
            Poll::Ready(Err(StarterDropped))
        );
        let fresh = started(flights.join_or_start(|| async { 9 }.boxed()));
        assert_eq!(futures::executor::block_on(fresh), Ok(9));
    }

    #[test]
    fn should_not_join_a_finished_future() {
        let flights = Flights::new();
        let _finished = started(flights.join_or_start(|| async { 7 }.boxed()));
        let joiner = joined(flights.join_or_start(|| async { 8 }.boxed()));
        assert_eq!(futures::executor::block_on(joiner), Ok(7));

        let fresh = started(flights.join_or_start(|| async { 9 }.boxed()));

        assert_eq!(futures::executor::block_on(fresh), Ok(9));
    }

    #[test]
    fn should_keep_a_newer_future_when_an_older_starter_is_dropped() {
        let flights = Flights::new();
        let finished = started(flights.join_or_start(|| async { 7 }.boxed()));
        let joiner = joined(flights.join_or_start(|| async { 8 }.boxed()));
        assert_eq!(futures::executor::block_on(joiner), Ok(7));
        let _newer = started(flights.join_or_start(|| pending().boxed()));

        drop(finished);

        let _joiner = joined(flights.join_or_start(|| async { 9 }.boxed()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn should_run_once_for_concurrent_callers() {
        const CALLERS: usize = 8;
        let flights = Arc::new(Flights::new());
        let runs = Arc::new(AtomicUsize::new(0));
        let asked = Arc::new(tokio::sync::Barrier::new(CALLERS + 1));
        let (release, released) = tokio::sync::watch::channel(false);

        let callers: Vec<_> = (0..CALLERS)
            .map(|_| {
                let (flights, runs, asked) = (flights.clone(), runs.clone(), asked.clone());
                let mut released = released.clone();
                tokio::spawn(async move {
                    let flight = flights.join_or_start(|| {
                        runs.fetch_add(1, Ordering::SeqCst);
                        async move {
                            let _ = released.wait_for(|released| *released).await;
                            42
                        }
                        .boxed()
                    });
                    asked.wait().await;
                    match flight {
                        Flight::Joined(joined) => joined.await,
                        Flight::Started(started) => started.await,
                    }
                })
            })
            .collect();
        asked.wait().await;
        release.send_replace(true);

        for caller in callers {
            assert_eq!(caller.await.expect("caller task panicked"), Ok(42));
        }
        assert_eq!(runs.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn should_format_a_single_flight_while_start_runs() {
        let flights = Arc::new(Flights::new());
        let (sent, received) = std::sync::mpsc::channel();
        let formatter = {
            let flights = flights.clone();
            std::thread::spawn(move || {
                let flight = flights.join_or_start(|| {
                    let _ = sent.send(format!("{flights:?}"));
                    async { 7 }.boxed()
                });
                drop(flight);
            })
        };

        let formatted = received
            .recv_timeout(Duration::from_secs(5))
            .expect("formatting inside start blocked");

        assert_eq!(formatted, "SingleFlight { running: <locked> }");
        formatter.join().expect("formatting thread panicked");
    }

    #[test]
    fn should_format_flights_of_futures_that_are_not_debug() {
        let flights = Flights::new();
        let starter = flights.join_or_start(|| pending().boxed());
        let joiner = flights.join_or_start(|| async { 8 }.boxed());

        assert!(format!("{starter:?}").starts_with("Started("));
        assert!(format!("{joiner:?}").starts_with("Joined("));
        assert_eq!(format!("{flights:?}"), "SingleFlight { running: true }");
    }

    #[test]
    fn should_share_a_future_that_is_not_send() {
        let flights = SingleFlight::<LocalBoxFuture<'static, Rc<u32>>>::new();
        let _starter = started(flights.join_or_start(|| async { Rc::new(7) }.boxed_local()));
        let joiner = joined(flights.join_or_start(|| async { Rc::new(8) }.boxed_local()));

        assert_eq!(futures::executor::block_on(joiner), Ok(Rc::new(7)));
    }
}
