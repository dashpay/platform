use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::{Mutex, Notify, OnceCell, oneshot};
use tokio::task::{AbortHandle, JoinError, JoinSet};

use crate::{DapiError, metrics};

/// Boxed worker future accepted by the worker manager task.
type WorkerTask = Pin<Box<dyn Future<Output = Result<(), DapiError>> + Send>>;

/// Guard that keeps worker metrics and counters balanced.
struct WorkerMetricsGuard {
    task_count: Arc<AtomicUsize>,
}

impl WorkerMetricsGuard {
    /// Increase the active worker metric and return a guard that will decrement on drop.
    fn new(task_count: Arc<AtomicUsize>) -> Self {
        metrics::workers_active_inc();
        task_count.fetch_add(1, Ordering::SeqCst);
        Self { task_count }
    }
}

impl Drop for WorkerMetricsGuard {
    /// Decrease the active worker metric when the guard leaves scope.
    fn drop(&mut self) {
        metrics::workers_active_dec();
        self.task_count.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Async worker pool for managing background tasks.
///
/// The pool uses a command pattern: [`Workers`] handles send spawn requests
/// to a [`WorkerManager`] task that owns a [`JoinSet`]. The manager continuously
/// drains completed tasks and returns [`AbortHandle`]s to callers via oneshot channels.

#[derive(Clone)]
pub struct Workers {
    inner: Arc<WorkersInner>,
}

/// Internal state shared with the worker manager task.
struct WorkersInner {
    sender: UnboundedSender<WorkerCommand>,
    task_count: Arc<AtomicUsize>,
}

/// Request sent to the manager describing a worker spawn operation.
enum WorkerCommand {
    Spawn {
        task: WorkerTask,
        response: oneshot::Sender<AbortHandle>,
    },
}

/// Debug implementation that reports the number of active workers.
impl Debug for Workers {
    /// Display the number of active worker tasks.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let workers = self.inner.task_count.load(Ordering::SeqCst) as i64;
        write!(f, "Workers {{ num_workers: {workers} }}")
    }
}

impl Workers {
    /// Create a new worker pool backed by a shared `JoinSet`.
    pub fn new() -> Self {
        let task_count = Arc::new(AtomicUsize::new(0));
        let (sender, receiver) = unbounded_channel();
        WorkerManager::spawn(receiver);
        Self {
            inner: Arc::new(WorkersInner { sender, task_count }),
        }
    }

    /// Spawn a new task into the join set while tracking metrics and error conversion.
    pub fn spawn<F, O, E>(&self, fut: F) -> WorkerTaskHandle
    where
        F: Future<Output = Result<O, E>> + Send + 'static,
        O: Send + 'static,
        E: Debug + Into<DapiError> + Send + 'static,
    {
        let task_count = self.inner.task_count.clone();
        let metrics_guard = WorkerMetricsGuard::new(task_count);
        let task = async move {
            let _metrics_guard = metrics_guard;
            match fut.await {
                Ok(_) => Ok(()),
                Err(e) => Err(e.into()),
            }
        };

        let (response_tx, response_rx) = oneshot::channel();
        let handle = WorkerTaskHandle::new(response_rx);

        if let Err(err) = self.inner.sender.send(WorkerCommand::Spawn {
            task: Box::pin(task),
            response: response_tx,
        }) {
            tracing::error!(error=?err, "Failed to dispatch worker task to manager");
            handle.notify_failure();
        }

        handle
    }
}

impl Default for Workers {
    /// Construct a new worker pool using the default configuration.
    fn default() -> Self {
        Self::new()
    }
}

/// Provides a lazy abort handle for a spawned worker task.
pub struct WorkerTaskHandle {
    inner: Arc<WorkerTaskHandleInner>,
}

/// Shared handshake state between worker handles and the manager.
struct WorkerTaskHandleInner {
    handle: OnceCell<Result<AbortHandle, ()>>,
    receiver: Mutex<Option<oneshot::Receiver<AbortHandle>>>,
    notify: Notify,
}

impl WorkerTaskHandle {
    /// Create a handle that waits for the manager to return an abort handle.
    fn new(receiver: oneshot::Receiver<AbortHandle>) -> Self {
        let inner = WorkerTaskHandleInner {
            handle: OnceCell::new(),
            receiver: Mutex::new(Some(receiver)),
            notify: Notify::new(),
        };
        Self {
            inner: Arc::new(inner),
        }
    }

    /// Notify any waiters that the spawn request could not be fulfilled.
    fn notify_failure(&self) {
        if self.inner.handle.set(Err(())).is_ok() {
            self.inner.notify.notify_waiters();
        }
    }

    /// Abort the background task once its handle becomes available.
    pub async fn abort(&self) {
        if let Some(handle) = self.get_handle().await {
            handle.abort();
        }
    }

    /// Fetch the abort handle from the manager, waiting if necessary.
    async fn get_handle(&self) -> Option<AbortHandle> {
        if let Some(result) = self.inner.handle.get() {
            return result.clone().ok();
        }

        if let Some(receiver) = self.take_receiver().await {
            let outcome = receiver.await.map_err(|_| ());
            match &outcome {
                Ok(handle) => {
                    let _ = self.inner.handle.set(Ok(handle.clone()));
                }
                Err(_) => {
                    let _ = self.inner.handle.set(Err(()));
                }
            }
            self.inner.notify.notify_waiters();
            return outcome.ok();
        }

        self.inner.notify.notified().await;
        self.inner
            .handle
            .get()
            .and_then(|result| result.clone().ok())
    }

    /// Remove the pending receiver so only one waiter consumes the response.
    async fn take_receiver(&self) -> Option<oneshot::Receiver<AbortHandle>> {
        let mut guard = self.inner.receiver.lock().await;
        guard.take()
    }
}

/// Task that owns the JoinSet and coordinates worker execution.
struct WorkerManager {
    receiver: UnboundedReceiver<WorkerCommand>,
}

impl WorkerManager {
    /// Start a background manager that processes worker commands.
    fn spawn(receiver: UnboundedReceiver<WorkerCommand>) {
        tokio::spawn(async move {
            Self { receiver }.run().await;
        });
    }

    /// Main event loop: accept work and join completed tasks.
    async fn run(mut self) {
        let mut join_set = JoinSet::new();

        loop {
            if join_set.is_empty() {
                match self.receiver.recv().await {
                    Some(WorkerCommand::Spawn { task, response }) => {
                        let abort_handle = join_set.spawn(task);
                        let _ = response.send(abort_handle);
                    }
                    None => break,
                }
            } else {
                tokio::select! {
                    cmd = self.receiver.recv() => {
                        match cmd {
                            Some(WorkerCommand::Spawn { task, response }) => {
                                let abort_handle = join_set.spawn(task);
                                let _ = response.send(abort_handle);
                            }
                            None => break,
                        }
                    }
                    join_result = join_set.join_next() => {
                        if let Some(result) = join_result {
                            Self::handle_result(result);
                        }
                    }
                }
            }
        }

        while let Some(result) = join_set.join_next().await {
            Self::handle_result(result);
        }
    }

    /// Handle task completion results, emitting appropriate logs.
    fn handle_result(result: Result<Result<(), DapiError>, JoinError>) {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                tracing::error!(error=?error, "Worker task exited with error");
            }
            Err(join_error) if join_error.is_cancelled() => {
                tracing::debug!("Worker task cancelled during shutdown");
            }
            Err(join_error) => {
                tracing::error!(error=?join_error, "Worker task panicked or failed to join");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use tokio::sync::{Notify, oneshot};
    use tokio::time::{Duration, sleep, timeout};

    struct DropGuard(Option<oneshot::Sender<()>>);

    impl Drop for DropGuard {
        fn drop(&mut self) {
            if let Some(tx) = self.0.take() {
                let _ = tx.send(());
            }
        }
    }

    async fn wait_for_active_count(workers: &Workers, expected: usize) {
        for _ in 0..50 {
            if workers.inner.task_count.load(Ordering::SeqCst) == expected {
                return;
            }
            sleep(Duration::from_millis(10)).await;
        }
        panic!(
            "active worker count did not reach {expected}, last value {}",
            workers.inner.task_count.load(Ordering::SeqCst)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn worker_executes_task_and_cleans_up() {
        let workers = Workers::new();
        let (tx, rx) = oneshot::channel();

        workers.spawn(async move {
            let _ = tx.send(());
            Ok::<(), DapiError>(())
        });

        timeout(Duration::from_secs(1), rx)
            .await
            .expect("worker did not run")
            .expect("worker task dropped sender");

        wait_for_active_count(&workers, 0).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn aborting_worker_cancels_future() {
        let workers = Workers::new();
        let (drop_tx, drop_rx) = oneshot::channel();
        let notify = Arc::new(Notify::new());
        let ready = Arc::new(Notify::new());
        let ready_wait = ready.notified();

        let worker_notify = notify.clone();
        let worker_ready = ready.clone();
        let handle = workers.spawn(async move {
            let _guard = DropGuard(Some(drop_tx));
            worker_ready.notify_one();
            worker_notify.notified().await;
            Ok::<(), DapiError>(())
        });

        timeout(Duration::from_secs(1), ready_wait)
            .await
            .expect("worker did not signal readiness");

        timeout(Duration::from_secs(1), handle.abort())
            .await
            .expect("abort timed out");

        timeout(Duration::from_secs(1), drop_rx)
            .await
            .expect("worker did not drop after abort")
            .expect("drop receiver cancelled");

        wait_for_active_count(&workers, 0).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn worker_error_still_clears_active_count() {
        let workers = Workers::new();
        let (tx, rx) = oneshot::channel();

        workers.spawn(async move {
            let _ = tx.send(());
            Err::<(), DapiError>(DapiError::Internal("boom".into()))
        });

        timeout(Duration::from_secs(1), rx)
            .await
            .expect("worker did not run")
            .expect("worker task dropped sender");

        wait_for_active_count(&workers, 0).await;
    }
}
