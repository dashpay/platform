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
use crate::runtime::runtime;

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
/// manager or the manager is shutting down.
#[no_mangle]
pub extern "C" fn platform_wallet_manager_shielded_prefetch_spend_anchors(
    handle: Handle,
) -> PlatformWalletFFIResult {
    // Called from an event callback, this can run while the export that
    // dispatched the event holds the registry, hence the recursive read.
    let lookup = PLATFORM_WALLET_MANAGER_STORAGE.with_item_recursive(handle, |manager| {
        let _runtime = runtime().enter();
        manager.prefetch_shielded_spend_anchors()
    });
    match lookup {
        None => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            format!("invalid manager handle: {handle}"),
        ),
        Some(Err(error)) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            error.to_string(),
        ),
        Some(Ok(())) => PlatformWalletFFIResult::ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_handler::EventHandlerCallbacks;
    use crate::manager::platform_wallet_manager_destroy;
    use crate::persistence::{FFIPersister, PersistenceCallbacks};
    use crate::shielded_sync::platform_wallet_manager_shielded_sync_sync_now;
    use crate::shielded_types::ShieldedSyncWalletResultFFI;
    use crate::test_fixtures::{
        assert_scenario_passes_in_child, mock_manager, mock_manager_on, mock_manager_with_events,
    };
    use dash_context_provider::ContextProvider;
    use rs_sdk_ffi::{CallbackContextProvider, CallbackResult, ContextProviderCallbacks};
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
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

    #[test]
    fn should_reject_prefetch_after_manager_shutdown() {
        let directory = tempfile::tempdir().expect("shielded store directory");
        let manager = mock_manager();
        runtime()
            .block_on(manager.configure_shielded(directory.path().join("tree.sqlite")))
            .expect("configure shielded");
        let handle = PLATFORM_WALLET_MANAGER_STORAGE.insert(manager);
        assert_eq!(
            code_of(platform_wallet_manager_shielded_prefetch_spend_anchors(
                handle
            )),
            PlatformWalletFFIResultCode::Success
        );
        let report = PLATFORM_WALLET_MANAGER_STORAGE
            .with_item(handle, |manager| runtime().block_on(manager.shutdown()))
            .expect("registered manager");
        assert!(report.all_clean(), "{report:?}");

        assert_eq!(
            code_of(platform_wallet_manager_shielded_prefetch_spend_anchors(
                handle
            )),
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        drop(PLATFORM_WALLET_MANAGER_STORAGE.remove(handle));
    }

    struct BorrowedSdkContext {
        calls: AtomicUsize,
        exited: AtomicBool,
        entered: mpsc::Sender<()>,
        release: Mutex<mpsc::Receiver<()>>,
    }

    unsafe extern "C" fn activation_height(_: *mut c_void, height: *mut u32) -> CallbackResult {
        *height = 0;
        CallbackResult {
            success: true,
            error_code: 0,
            error_message: std::ptr::null(),
        }
    }

    unsafe extern "C" fn borrowed_quorum_key(
        context: *mut c_void,
        _: u32,
        _: *const u8,
        _: u32,
        public_key: *mut u8,
    ) -> CallbackResult {
        let context = &*(context as *const BorrowedSdkContext);
        context.calls.fetch_add(1, Ordering::SeqCst);
        let _ = context.entered.send(());
        let released = context
            .release
            .lock()
            .is_ok_and(|release| release.recv_timeout(WAIT).is_ok());
        std::ptr::write_bytes(public_key, 7, 48);
        context.exited.store(true, Ordering::SeqCst);
        CallbackResult {
            success: released,
            error_code: i32::from(!released),
            error_message: std::ptr::null(),
        }
    }

    fn sdk_borrowing(context: &BorrowedSdkContext) -> dash_sdk::Sdk {
        dash_sdk::SdkBuilder::new_mock()
            .with_network(dashcore::Network::Testnet)
            .with_context_provider(CallbackContextProvider::new(ContextProviderCallbacks {
                core_handle: (context as *const BorrowedSdkContext).cast_mut().cast(),
                get_platform_activation_height: activation_height,
                get_quorum_public_key: borrowed_quorum_key,
            }))
            .build()
            .expect("SDK with borrowed callback context")
    }

    #[test]
    fn should_join_an_active_borrowed_sdk_callback_before_ffi_destroy_returns() {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        // Reclaim only after a successful join so a failed regression assertion
        // cannot free the raw context beneath a still-running callback.
        let context: &BorrowedSdkContext = Box::leak(Box::new(BorrowedSdkContext {
            calls: AtomicUsize::new(0),
            exited: AtomicBool::new(false),
            entered: entered_tx,
            release: Mutex::new(release_rx),
        }));
        let sdk = sdk_borrowing(context);
        let manager = mock_manager_on(
            sdk.clone(),
            FFIPersister::new(PersistenceCallbacks::default()),
        );
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        {
            let _runtime = runtime().enter();
            assert!(manager.spawn_shielded_prefetch_for_testing(async move {
                response_rx.await.expect("delayed response");
                let key = sdk
                    .context_provider()
                    .expect("callback provider")
                    .get_quorum_public_key(1, [2; 32], 3)
                    .expect("host supplies the quorum key");
                assert_eq!(key, [7; 48]);
            }));
        }
        let handle = PLATFORM_WALLET_MANAGER_STORAGE.insert(manager);
        response_tx.send(()).expect("release delayed response");
        entered_rx.recv_timeout(WAIT).expect("SDK callback entered");
        let (destroyed_tx, destroyed_rx) = mpsc::channel();
        let destroy = std::thread::spawn(move || {
            let code = code_of(unsafe { platform_wallet_manager_destroy(handle) });
            destroyed_tx.send(code).expect("observe destroy result");
        });
        let deadline = std::time::Instant::now() + WAIT;
        while PLATFORM_WALLET_MANAGER_STORAGE
            .with_item(handle, |_| ())
            .is_some()
        {
            assert!(
                std::time::Instant::now() < deadline,
                "destroy never removed the handle"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let early = destroyed_rx.recv_timeout(Duration::from_millis(100));
        release_tx.send(()).expect("release synchronous callback");
        assert!(
            matches!(early, Err(mpsc::RecvTimeoutError::Timeout)),
            "destroy returned during the borrowed callback: {early:?}"
        );
        assert_eq!(
            destroyed_rx
                .recv_timeout(WAIT)
                .expect("destroy joined callback"),
            PlatformWalletFFIResultCode::Success
        );
        destroy.join().expect("destroy thread");
        assert!(context.exited.load(Ordering::SeqCst));
        assert_eq!(context.calls.load(Ordering::SeqCst), 1);
        unsafe {
            drop(Box::from_raw(
                (context as *const BorrowedSdkContext).cast_mut(),
            ))
        };
    }

    #[test]
    fn should_cancel_a_delayed_sdk_response_before_releasing_its_borrowed_context() {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (_release_tx, release_rx) = mpsc::channel();
        let context: &BorrowedSdkContext = Box::leak(Box::new(BorrowedSdkContext {
            calls: AtomicUsize::new(0),
            exited: AtomicBool::new(false),
            entered: entered_tx,
            release: Mutex::new(release_rx),
        }));
        let sdk = sdk_borrowing(context);
        let manager = mock_manager_on(
            sdk.clone(),
            FFIPersister::new(PersistenceCallbacks::default()),
        );
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        {
            let _runtime = runtime().enter();
            assert!(manager.spawn_shielded_prefetch_for_testing(async move {
                let _ = started_tx.send(());
                if response_rx.await.is_ok() {
                    let _ = sdk
                        .context_provider()
                        .expect("callback provider")
                        .get_quorum_public_key(1, [2; 32], 3);
                }
            }));
        }
        runtime()
            .block_on(started_rx)
            .expect("prefetch awaiting response");
        let handle = PLATFORM_WALLET_MANAGER_STORAGE.insert(manager);
        let (destroyed_tx, destroyed_rx) = mpsc::channel();
        let destroy = std::thread::spawn(move || {
            let code = code_of(unsafe { platform_wallet_manager_destroy(handle) });
            destroyed_tx.send(code).expect("observe destroy result");
        });
        assert_eq!(
            destroyed_rx
                .recv_timeout(WAIT)
                .expect("destroy cancelled the response"),
            PlatformWalletFFIResultCode::Success
        );
        destroy.join().expect("destroy thread");
        assert!(
            response_tx.send(()).is_err(),
            "destroy must drop the delayed response receiver"
        );
        assert!(entered_rx.try_recv().is_err());
        assert_eq!(context.calls.load(Ordering::SeqCst), 0);
        unsafe {
            drop(Box::from_raw(
                (context as *const BorrowedSdkContext).cast_mut(),
            ))
        };
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
