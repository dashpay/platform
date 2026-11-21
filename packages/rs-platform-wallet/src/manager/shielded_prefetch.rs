use std::any::Any;
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError};
use std::task::{Context, Poll, Waker};

use dash_async::WorkerStatus;
use tokio::runtime::Handle;
use tokio::sync::Notify;

use super::PlatformWalletManager;
use crate::changeset::PlatformWalletPersistence;
use crate::error::PlatformWalletError;

type Prefetch = Pin<Box<dyn Future<Output = ()> + Send>>;

#[derive(Default)]
struct Slot {
    future: Option<Prefetch>,
    task: Option<Waker>,
    failure: Option<WorkerStatus>,
}

/// Owns the prefetch future. The runtime task only polls it through `slot`,
/// so a host callback runs only while that lock is held. Teardown seals
/// admission, takes the lock and drops the future itself: once it holds the
/// lock no callback is running, and once the future is gone none can start.
/// It needs nothing from the runtime the task was spawned on.
#[derive(Default)]
struct Shared {
    sealed: AtomicBool,
    slot: Mutex<Slot>,
    released: Notify,
}

struct NotifyOnDrop<'a>(&'a Notify);

impl Drop for NotifyOnDrop<'_> {
    fn drop(&mut self) {
        self.0.notify_waiters();
    }
}

/// Fields drop in order, so waiters are notified after the lock is released,
/// including on unwind.
struct Held<'a> {
    slot: MutexGuard<'a, Slot>,
    _released: NotifyOnDrop<'a>,
}

/// Clears the slot if the runtime drops the driver task before it finishes,
/// so a later prefetch is not absorbed by a future nothing polls.
struct Driver {
    shared: Arc<Shared>,
    finished: bool,
}

impl Future for Driver {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let poll = self.shared.poll(cx);
        self.finished = poll.is_ready();
        poll
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        if !self.finished {
            self.shared.with_slot(Shared::finish);
        }
    }
}

impl Shared {
    fn with_slot<R>(&self, run: impl FnOnce(&mut Slot) -> R) -> R {
        self.release(
            self.slot.lock().unwrap_or_else(PoisonError::into_inner),
            run,
        )
    }

    fn try_with_slot<R>(&self, run: impl FnOnce(&mut Slot) -> R) -> Option<R> {
        let slot = match self.slot.try_lock() {
            Ok(slot) => slot,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => return None,
        };
        Some(self.release(slot, run))
    }

    fn release<R>(&self, slot: MutexGuard<'_, Slot>, run: impl FnOnce(&mut Slot) -> R) -> R {
        let mut held = Held {
            slot,
            _released: NotifyOnDrop(&self.released),
        };
        run(&mut held.slot)
    }

    fn poll(&self, cx: &mut Context<'_>) -> Poll<()> {
        self.with_slot(|slot| {
            if self.sealed.load(Ordering::SeqCst) {
                slot.future = None;
            }
            if let Some(future) = slot.future.as_mut() {
                match catch_unwind(AssertUnwindSafe(|| future.as_mut().poll(cx))) {
                    Ok(Poll::Pending) => {
                        slot.task = Some(cx.waker().clone());
                        return Poll::Pending;
                    }
                    Ok(Poll::Ready(())) => {}
                    Err(panic) => {
                        let message = panic_message(panic.as_ref());
                        tracing::warn!(message, "shielded prefetch task panicked");
                        slot.failure = Some(WorkerStatus::Panicked(message.to_owned()));
                    }
                }
            }
            slot.future = None;
            slot.task = None;
            Poll::Ready(())
        })
    }

    fn seal(&self) {
        self.sealed.store(true, Ordering::SeqCst);
    }

    fn finish(slot: &mut Slot) -> WorkerStatus {
        slot.future = None;
        if let Some(task) = slot.task.take() {
            task.wake();
        }
        slot.failure.clone().unwrap_or(WorkerStatus::Ok)
    }
}

fn panic_message(panic: &(dyn Any + Send)) -> &str {
    panic
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("non-string panic payload")
}

#[derive(Default)]
pub(super) struct ShieldedPrefetch {
    shared: Arc<Shared>,
}

impl ShieldedPrefetch {
    /// Returns `false` only once admission is sealed. A prefetch that is still
    /// in the slot absorbs the request.
    fn spawn(&self, runtime: &Handle, future: impl Future<Output = ()> + Send + 'static) -> bool {
        let shared = &self.shared;
        let mut spawned = false;
        let admitted = shared
            .try_with_slot(|slot| {
                if shared.sealed.load(Ordering::SeqCst) {
                    return false;
                }
                if slot.future.is_none() {
                    slot.future = Some(Box::pin(future));
                    spawned = true;
                }
                true
            })
            .unwrap_or_else(|| !shared.sealed.load(Ordering::SeqCst));
        if spawned {
            runtime.spawn(Driver {
                shared: Arc::clone(shared),
                finished: false,
            });
        }
        admitted
    }

    /// Seals admission and drops the prefetch future. A host callback that is
    /// already running holds the slot, so this stays pending until it returns;
    /// no timeout can permit freeing the borrowed context under it.
    pub(super) async fn shutdown(&self) -> WorkerStatus {
        self.shared.seal();
        loop {
            let released = self.shared.released.notified();
            tokio::pin!(released);
            released.as_mut().enable();
            if let Some(status) = self.shared.try_with_slot(Shared::finish) {
                return status;
            }
            released.await;
        }
    }
}

impl Drop for ShieldedPrefetch {
    fn drop(&mut self) {
        self.shared.seal();
        self.shared.with_slot(Shared::finish);
    }
}

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// Inject a callback-capable prefetch future for host teardown tests.
    #[cfg(feature = "test-utils")]
    pub fn spawn_shielded_prefetch_for_testing(
        &self,
        runtime: &Handle,
        future: impl Future<Output = ()> + Send + 'static,
    ) -> bool {
        self.shielded_prefetch.spawn(runtime, future)
    }

    /// Queue a best-effort anchor warm-up on `runtime`. Busy coordinator
    /// configuration is skipped; shutdown permanently closes task admission.
    /// Any Tokio runtime works: shutdown and drop cancel the warm-up without
    /// it.
    pub fn prefetch_shielded_spend_anchors(
        &self,
        runtime: &Handle,
    ) -> Result<(), PlatformWalletError> {
        let Ok(slot) = self.shielded_coordinator.try_read() else {
            return Ok(());
        };
        let coordinator = slot.as_ref().cloned().ok_or_else(|| {
            PlatformWalletError::ShieldedSyncFailed("shielded support not configured".into())
        })?;
        if !self.shielded_prefetch.spawn(runtime, async move {
            if let Err(error) = coordinator.prefetch_recorded_anchors().await {
                tracing::debug!(%error, "shielded spend-anchor prefetch failed");
            }
        }) {
            return Err(PlatformWalletError::ShutdownIncomplete(
                "shielded spend-anchor prefetch is closed: the manager is shut down".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::FutureExt;
    use std::sync::mpsc;
    use std::time::Duration;

    fn spawn(tasks: &ShieldedPrefetch, future: impl Future<Output = ()> + Send + 'static) -> bool {
        tasks.spawn(&Handle::current(), future)
    }

    fn holds_a_prefetch(tasks: &ShieldedPrefetch) -> bool {
        tasks
            .shared
            .try_with_slot(|slot| slot.future.is_some())
            .unwrap_or(true)
    }

    async fn settled(tasks: &ShieldedPrefetch) {
        while holds_a_prefetch(tasks) {
            tokio::task::yield_now().await;
        }
    }

    /// A prefetch parked inside a synchronous host callback, as a borrowed SDK
    /// context provider would hold it.
    struct ActiveCallback {
        release: mpsc::Sender<()>,
        exited: Arc<AtomicBool>,
    }

    async fn enter_callback(tasks: &ShieldedPrefetch) -> ActiveCallback {
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release, released) = mpsc::channel();
        let exited = Arc::new(AtomicBool::new(false));
        let callback_exited = Arc::clone(&exited);
        assert!(spawn(tasks, async move {
            let _ = entered_tx.send(());
            released.recv().unwrap();
            callback_exited.store(true, Ordering::SeqCst);
        }));
        entered_rx.await.unwrap();
        ActiveCallback { release, exited }
    }

    impl ActiveCallback {
        fn release_after(&self, delay: Duration) -> std::thread::JoinHandle<()> {
            let release = self.release.clone();
            std::thread::spawn(move || {
                std::thread::sleep(delay);
                release.send(()).unwrap();
            })
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_cancel_a_delayed_prefetch_and_seal_admission() {
        let tasks = ShieldedPrefetch::default();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel::<()>();
        assert!(spawn(&tasks, async move {
            let _ = started_tx.send(());
            let _ = response_rx.await;
            panic!("a cancelled response must not enter the host callback");
        }));
        started_rx.await.unwrap();
        let (duplicate_tx, duplicate_rx) = tokio::sync::oneshot::channel();
        assert!(spawn(&tasks, async move {
            let _ = duplicate_tx.send(());
        }));
        assert!(
            duplicate_rx.await.is_err(),
            "overlapping prefetch is coalesced"
        );
        assert_eq!(tasks.shutdown().await, WorkerStatus::Ok);
        assert!(response_tx.send(()).is_err());
        assert!(!spawn(&tasks, async {}));
        assert_eq!(tasks.shutdown().await, WorkerStatus::Ok);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_retain_a_prefetch_while_its_host_callback_is_active() {
        let tasks = ShieldedPrefetch::default();
        let callback = enter_callback(&tasks).await;
        assert!(
            tokio::time::timeout(Duration::from_millis(20), tasks.shutdown())
                .await
                .is_err()
        );
        assert!(!callback.exited.load(Ordering::SeqCst));
        assert!(!spawn(&tasks, async {}));
        callback.release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(1), tasks.shutdown())
            .await
            .unwrap();
        assert!(callback.exited.load(Ordering::SeqCst));
        assert!(!holds_a_prefetch(&tasks));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_report_panics_even_after_a_replacement_prefetch() {
        let tasks = ShieldedPrefetch::default();
        assert!(spawn(&tasks, async { panic!("prefetch panic") }));
        settled(&tasks).await;
        assert!(spawn(&tasks, async {}));
        assert!(matches!(tasks.shutdown().await, WorkerStatus::Panicked(_)));
        assert!(matches!(tasks.shutdown().await, WorkerStatus::Panicked(_)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_report_a_panic_when_the_replacing_caller_has_no_budget_left() {
        let tasks = ShieldedPrefetch::default();
        assert!(spawn(&tasks, async { panic!("prefetch panic") }));
        settled(&tasks).await;
        while tokio::task::coop::consume_budget().now_or_never().is_some() {}
        assert!(spawn(&tasks, async {}));
        assert!(matches!(tasks.shutdown().await, WorkerStatus::Panicked(_)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_join_an_active_prefetch_when_dropped() {
        let tasks = ShieldedPrefetch::default();
        let callback = enter_callback(&tasks).await;
        let releaser = callback.release_after(Duration::from_millis(50));
        drop(tasks);
        assert!(
            callback.exited.load(Ordering::SeqCst),
            "drop returned during an active callback"
        );
        releaser.join().unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_join_an_active_prefetch_when_dropped_inside_a_local_set() {
        let tasks = ShieldedPrefetch::default();
        let callback = enter_callback(&tasks).await;
        let releaser = callback.release_after(Duration::from_millis(50));
        let exited = Arc::clone(&callback.exited);
        tokio::task::LocalSet::new()
            .run_until(async move {
                tokio::task::spawn_local(async move {
                    drop(tasks);
                    assert!(
                        exited.load(Ordering::SeqCst),
                        "drop returned during an active callback"
                    );
                })
                .await
                .unwrap();
            })
            .await;
        releaser.join().unwrap();
    }

    #[test]
    fn should_admit_a_new_prefetch_after_its_runtime_drops_the_previous_one() {
        let tasks = ShieldedPrefetch::default();
        let stopped = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        assert!(tasks.spawn(stopped.handle(), std::future::pending()));
        assert!(holds_a_prefetch(&tasks));
        drop(stopped);
        assert!(!holds_a_prefetch(&tasks));

        let (ran_tx, ran_rx) = mpsc::channel();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        assert!(tasks.spawn(runtime.handle(), async move {
            ran_tx.send(()).unwrap();
        }));
        runtime.block_on(settled(&tasks));
        ran_rx.try_recv().unwrap();
    }

    #[tokio::test]
    async fn should_cancel_an_idle_prefetch_when_dropped_on_a_current_thread_runtime() {
        let tasks = ShieldedPrefetch::default();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel::<()>();
        assert!(spawn(&tasks, async move {
            let _ = started_tx.send(());
            let _ = response_rx.await;
            panic!("a cancelled response must not enter the host callback");
        }));
        started_rx.await.unwrap();
        drop(tasks);
        assert!(response_tx.send(()).is_err());
    }
}
