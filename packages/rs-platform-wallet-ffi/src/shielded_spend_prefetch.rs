//! FFI entry point that prepares an upcoming shielded spend.
//!
//! Every shielded spend (transfer, unshield, withdraw, identity top-up /
//! create from the pool) must pick an anchor Platform has recorded, which
//! takes a round trip that downloads and proof-verifies Platform's recorded
//! anchor set. Hosts call
//! [`platform_wallet_manager_shielded_prefetch_spend_anchors`] when a send
//! screen opens so that round trip overlaps the user filling in the form
//! instead of delaying the send. Feature-gated behind `shielded`.

use crate::error::*;
use crate::handle::*;
use crate::runtime::{run_on_big_stack_thread, runtime};

/// Prefetch Platform's recorded shielded-anchor set for the next spend on
/// `handle`'s network, in the background.
///
/// Returns as soon as the fetch is queued; it runs on the FFI runtime. A
/// shielded send started within the next ~30 s, before any shielded sync
/// grows the commitment tree, reuses the prefetched set and skips that round
/// trip; otherwise the send fetches for itself exactly as without this call.
/// Cheap to call repeatedly (a no-op while a usable set is cached), and a
/// failed fetch is only logged — it never affects a later send. Safe to call
/// from any thread, including from inside an event callback.
///
/// Returns `ErrorInvalidHandle` for an unknown `handle` and
/// `ErrorWalletOperation` when shielded support isn't configured on the
/// manager.
#[no_mangle]
pub extern "C" fn platform_wallet_manager_shielded_prefetch_spend_anchors(
    handle: Handle,
) -> PlatformWalletFFIResult {
    // Called from an event callback, this can run while the export that
    // dispatched the event holds the registry, hence the recursive read.
    let lookup = PLATFORM_WALLET_MANAGER_STORAGE.with_item_recursive(handle, |manager| {
        let resolve = || runtime().block_on(manager.shielded_coordinator());
        // Event callbacks fire inside a runtime context (the sync loop
        // drives its passes with `Handle::block_on`), where a nested
        // `block_on` panics; resolve on a thread outside it instead.
        if tokio::runtime::Handle::try_current().is_ok() {
            run_on_big_stack_thread(resolve)
        } else {
            Ok(resolve())
        }
    });
    let coordinator = match lookup {
        None => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidHandle,
                format!("invalid manager handle: {handle}"),
            );
        }
        Some(Err(e)) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                format!("could not start the shielded spend-anchor prefetch: {e}"),
            );
        }
        Some(Ok(None)) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "shielded support not configured — call platform_wallet_manager_configure_shielded first",
            );
        }
        Some(Ok(Some(coordinator))) => coordinator,
    };
    runtime().spawn(async move {
        if let Err(e) = coordinator.prefetch_recorded_anchors().await {
            tracing::debug!(
                error = %e,
                "shielded spend-anchor prefetch failed; the next spend fetches for itself"
            );
        }
    });
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_handler::EventHandlerCallbacks;
    use crate::shielded_sync::platform_wallet_manager_shielded_sync_sync_now;
    use crate::shielded_types::ShieldedSyncWalletResultFFI;
    use crate::test_fixtures::{
        assert_scenario_passes_in_child, mock_manager, mock_manager_with_events,
    };
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{mpsc, Mutex};
    use std::time::Duration;

    const WAIT: Duration = Duration::from_secs(10);

    fn code_of(mut result: PlatformWalletFFIResult) -> PlatformWalletFFIResultCode {
        let code = result.code;
        unsafe { crate::platform_wallet_ffi_result_free(&mut result) };
        code
    }

    #[test]
    fn should_reject_an_unknown_manager_handle() {
        assert_eq!(
            code_of(platform_wallet_manager_shielded_prefetch_spend_anchors(
                NULL_HANDLE
            )),
            PlatformWalletFFIResultCode::ErrorInvalidHandle
        );
    }

    #[test]
    fn should_reject_a_manager_without_shielded_support() {
        let handle = PLATFORM_WALLET_MANAGER_STORAGE.insert(mock_manager());
        assert_eq!(
            code_of(platform_wallet_manager_shielded_prefetch_spend_anchors(
                handle
            )),
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        drop(PLATFORM_WALLET_MANAGER_STORAGE.remove(handle));
    }

    /// Event callbacks run inside a runtime context; the lookup must not
    /// nest `block_on` there.
    #[test]
    fn should_resolve_the_coordinator_from_inside_a_runtime_context() {
        let handle = PLATFORM_WALLET_MANAGER_STORAGE.insert(mock_manager());
        let code = runtime().block_on(async {
            code_of(platform_wallet_manager_shielded_prefetch_spend_anchors(
                handle,
            ))
        });
        assert_eq!(code, PlatformWalletFFIResultCode::ErrorWalletOperation);
        drop(PLATFORM_WALLET_MANAGER_STORAGE.remove(handle));
    }

    /// Set in the child process that runs [`completion_callback_scenario`].
    const COMPLETION_CALLBACK_CHILD: &str =
        "PLATFORM_WALLET_FFI_PREFETCH_FROM_COMPLETION_CALLBACK_CHILD";

    /// The lock cycle a call from a sync-completion callback must not form:
    /// `platform_wallet_manager_shielded_sync_sync_now` holds the registry's
    /// read guard until the pass, completion callback included, returns. A
    /// registry writer queued behind that guard makes parking_lot park every
    /// new plain reader, so a lookup that read the registry normally would
    /// wait on the writer, the writer on the sync, the sync on the callback.
    ///
    /// The scenario depends on the sync export holding that guard across the
    /// pass; if that changes, it no longer reproduces the cycle. It runs in a
    /// child process, as a regression is a real deadlock (see
    /// [`assert_scenario_passes_in_child`]).
    #[test]
    fn should_not_wedge_a_prefetch_from_a_sync_completion_callback_behind_a_queued_registry_writer()
    {
        if std::env::var_os(COMPLETION_CALLBACK_CHILD).is_some() {
            completion_callback_scenario();
            return;
        }
        assert_scenario_passes_in_child(
            module_path!(),
            "should_not_wedge_a_prefetch_from_a_sync_completion_callback_behind_a_queued_registry_writer",
            COMPLETION_CALLBACK_CHILD,
            WAIT,
        );
    }

    static SYNCING_HANDLE: AtomicU64 = AtomicU64::new(NULL_HANDLE);
    type CallbackReport = (PlatformWalletFFIResultCode, std::thread::JoinHandle<Handle>);
    static CALLBACK_REPORT: Mutex<Option<mpsc::Sender<CallbackReport>>> = Mutex::new(None);

    /// Queues a registry writer behind the dispatching sync's read guard,
    /// then calls the prefetch, as a host's completion handler would.
    unsafe extern "C" fn prefetch_on_sync_completed(
        _context: *mut std::ffi::c_void,
        _results: *const ShieldedSyncWalletResultFFI,
        _count: usize,
        _sync_unix_seconds: u64,
    ) {
        let other = mock_manager();
        let writer = std::thread::spawn(move || PLATFORM_WALLET_MANAGER_STORAGE.insert(other));
        // Well inside the parent's budget, so a writer that never queued is
        // reported as that rather than as a deadlock.
        let deadline = std::time::Instant::now() + WAIT / 2;
        while !PLATFORM_WALLET_MANAGER_STORAGE.try_read_refused() {
            assert!(
                std::time::Instant::now() < deadline,
                "the writer never queued behind the sync's guard"
            );
            std::thread::sleep(Duration::from_millis(5));
        }

        let code = code_of(platform_wallet_manager_shielded_prefetch_spend_anchors(
            SYNCING_HANDLE.load(Ordering::SeqCst),
        ));
        if let Some(report) = CALLBACK_REPORT.lock().unwrap().as_ref() {
            report
                .send((code, writer))
                .expect("test observes the callback");
        }
    }

    fn completion_callback_scenario() {
        let (report_tx, report_rx) = mpsc::channel();
        *CALLBACK_REPORT.lock().unwrap() = Some(report_tx);
        let handle = PLATFORM_WALLET_MANAGER_STORAGE.insert(mock_manager_with_events(
            EventHandlerCallbacks {
                context: std::ptr::null_mut(),
                on_wallet_event_fn: None,
                on_error_fn: None,
                on_platform_address_sync_completed_fn: None,
                on_shielded_sync_completed_fn: Some(prefetch_on_sync_completed),
                on_shielded_sync_progress_fn: None,
                on_shielded_tree_progress_fn: None,
                release_fn: None,
            },
        ));
        SYNCING_HANDLE.store(handle, Ordering::SeqCst);

        assert_eq!(
            code_of(unsafe { platform_wallet_manager_shielded_sync_sync_now(handle) }),
            PlatformWalletFFIResultCode::Success
        );
        let (code, writer) = report_rx
            .recv_timeout(WAIT)
            .expect("the pass dispatched its completion callback");
        assert_eq!(
            code,
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "the prefetch looked the manager up and found no shielded support"
        );
        let other = writer.join().expect("writer thread");

        drop(PLATFORM_WALLET_MANAGER_STORAGE.remove(other));
        drop(PLATFORM_WALLET_MANAGER_STORAGE.remove(handle));
    }
}
