use std::future::Future;
use std::sync::{Mutex, PoisonError};

use tokio::runtime::Handle;
use tokio::task::AbortHandle;

use super::PlatformWalletManager;
use crate::changeset::PlatformWalletPersistence;
use crate::error::PlatformWalletError;

#[derive(Default)]
struct State {
    sealed: bool,
    task: Option<AbortHandle>,
}

/// At most one background anchor warm-up. Teardown is seal-and-cancel: it
/// stops admission and aborts the task without waiting for it.
#[derive(Default)]
pub(super) struct ShieldedPrefetch {
    state: Mutex<State>,
}

impl ShieldedPrefetch {
    /// Returns `false` only once admission is sealed. A prefetch that is still
    /// running absorbs the request.
    fn spawn(&self, runtime: &Handle, future: impl Future<Output = ()> + Send + 'static) -> bool {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.sealed {
            return false;
        }
        if state.task.as_ref().is_none_or(AbortHandle::is_finished) {
            state.task = Some(runtime.spawn(future).abort_handle());
        }
        true
    }

    pub(super) fn seal(&self) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.sealed = true;
        if let Some(task) = state.task.take() {
            task.abort();
        }
    }
}

impl Drop for ShieldedPrefetch {
    fn drop(&mut self) {
        self.seal();
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
    ///
    /// Shutdown and drop cancel the warm-up without waiting for it, so an SDK
    /// context provider callback it already entered may still finish. A
    /// borrowed callback context must outlive all SDK work that can use it.
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
    use std::time::Duration;

    fn spawn(tasks: &ShieldedPrefetch, future: impl Future<Output = ()> + Send + 'static) -> bool {
        tasks.spawn(&Handle::current(), future)
    }

    async fn parked(tasks: &ShieldedPrefetch) -> tokio::sync::oneshot::Sender<()> {
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel::<()>();
        assert!(spawn(tasks, async move {
            let _ = started_tx.send(());
            let _ = response_rx.await;
            panic!("a cancelled response must not enter the host callback");
        }));
        started_rx.await.unwrap();
        response_tx
    }

    async fn cancelled(mut response: tokio::sync::oneshot::Sender<()>) {
        tokio::time::timeout(Duration::from_secs(5), response.closed())
            .await
            .expect("the parked prefetch was not cancelled");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_coalesce_an_overlapping_prefetch() {
        let tasks = ShieldedPrefetch::default();
        let response = parked(&tasks).await;
        let (duplicate_tx, duplicate_rx) = tokio::sync::oneshot::channel();
        assert!(spawn(&tasks, async move {
            let _ = duplicate_tx.send(());
        }));
        assert!(duplicate_rx.await.is_err());
        tasks.seal();
        cancelled(response).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_cancel_a_pending_prefetch_and_seal_admission() {
        let tasks = ShieldedPrefetch::default();
        let response = parked(&tasks).await;
        tasks.seal();
        cancelled(response).await;
        assert!(!spawn(&tasks, async {}));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_admit_a_new_prefetch_after_the_previous_one_finished() {
        let tasks = ShieldedPrefetch::default();
        let (first_tx, first_rx) = tokio::sync::oneshot::channel();
        assert!(spawn(&tasks, async move {
            let _ = first_tx.send(());
        }));
        first_rx.await.unwrap();
        loop {
            let (second_tx, second_rx) = tokio::sync::oneshot::channel();
            assert!(spawn(&tasks, async move {
                let _ = second_tx.send(());
            }));
            if second_rx.await.is_ok() {
                break;
            }
            tokio::task::yield_now().await;
        }
    }

    #[tokio::test]
    async fn should_cancel_a_pending_prefetch_when_dropped() {
        let tasks = ShieldedPrefetch::default();
        let response = parked(&tasks).await;
        drop(tasks);
        cancelled(response).await;
    }
}
