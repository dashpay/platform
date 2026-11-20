use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::sync::{Mutex, PoisonError};
use std::task::Poll;

use tokio::task::JoinHandle;

use super::PlatformWalletManager;
use crate::changeset::PlatformWalletPersistence;
use crate::error::PlatformWalletError;

#[derive(Default)]
struct PrefetchState {
    sealed: bool,
    task: Option<JoinHandle<()>>,
}

#[derive(Default)]
pub(super) struct ShieldedPrefetch {
    state: Mutex<PrefetchState>,
    drain: tokio::sync::Mutex<()>,
}

impl ShieldedPrefetch {
    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) -> bool {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.sealed {
            return false;
        }
        if state.task.as_ref().is_none_or(JoinHandle::is_finished) {
            state.task = Some(tokio::spawn(future));
        }
        true
    }

    /// SDK proof callbacks borrow their host context, so cancellation must be
    /// joined through any synchronous callback; a timeout cannot permit teardown.
    pub(super) async fn shutdown(&self) {
        {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            state.sealed = true;
            if let Some(task) = &state.task {
                task.abort();
            }
        }
        let _drain = self.drain.lock().await;
        poll_fn(|cx| {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(task) = state.task.as_mut() {
                match Pin::new(task).poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Err(error)) if !error.is_cancelled() => {
                        tracing::warn!(%error, "shielded prefetch task failed");
                    }
                    Poll::Ready(_) => {}
                }
            }
            state.task = None;
            Poll::Ready(())
        })
        .await;
    }
}

impl Drop for ShieldedPrefetch {
    fn drop(&mut self) {
        let state = self.state.get_mut().unwrap_or_else(PoisonError::into_inner);
        if let Some(task) = &state.task {
            task.abort();
        }
    }
}

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// Inject a callback-capable prefetch future for host teardown tests.
    #[cfg(feature = "test-utils")]
    pub fn spawn_shielded_prefetch_for_testing(
        &self,
        future: impl Future<Output = ()> + Send + 'static,
    ) -> bool {
        self.shielded_prefetch.spawn(future)
    }

    /// Queue a best-effort anchor warm-up on the current runtime. Busy coordinator
    /// configuration is skipped; shutdown permanently closes task admission.
    pub fn prefetch_shielded_spend_anchors(&self) -> Result<(), PlatformWalletError> {
        let Ok(slot) = self.shielded_coordinator.try_read() else {
            return Ok(());
        };
        let coordinator = slot.as_ref().cloned().ok_or_else(|| {
            PlatformWalletError::ShieldedSyncFailed("shielded support not configured".into())
        })?;
        if !self.shielded_prefetch.spawn(async move {
            if let Err(error) = coordinator.prefetch_recorded_anchors().await {
                tracing::debug!(%error, "shielded spend-anchor prefetch failed");
            }
        }) {
            return Err(PlatformWalletError::ShutdownIncomplete(
                "shielded spend-anchor prefetch admission is closed".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[tokio::test]
    async fn should_cancel_a_delayed_prefetch_and_seal_admission() {
        let tasks = ShieldedPrefetch::default();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel::<()>();
        assert!(tasks.spawn(async move {
            let _ = started_tx.send(());
            let _ = response_rx.await;
            panic!("a cancelled response must not enter the host callback");
        }));
        started_rx.await.unwrap();
        let (duplicate_tx, duplicate_rx) = tokio::sync::oneshot::channel();
        assert!(tasks.spawn(async move {
            let _ = duplicate_tx.send(());
        }));
        assert!(
            duplicate_rx.await.is_err(),
            "overlapping prefetch is coalesced"
        );
        tokio::time::timeout(Duration::from_secs(1), tasks.shutdown())
            .await
            .unwrap();
        assert!(response_tx.send(()).is_err());
        assert!(!tasks.spawn(async {}));
        tokio::time::timeout(Duration::from_secs(1), tasks.shutdown())
            .await
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_retain_a_prefetch_while_its_host_callback_is_active() {
        let tasks = ShieldedPrefetch::default();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = mpsc::channel();
        assert!(tasks.spawn(async move {
            let _ = entered_tx.send(());
            release_rx.recv().unwrap();
        }));
        entered_rx.await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), tasks.shutdown())
                .await
                .is_err()
        );
        assert!(tasks.state.lock().unwrap().task.is_some());
        assert!(!tasks.spawn(async {}));
        release_tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(1), tasks.shutdown())
            .await
            .unwrap();
        assert!(tasks.state.lock().unwrap().task.is_none());
    }
}
