//! FFI callback-based implementation of PlatformEventHandler.

use crate::platform_address_sync::{
    PlatformAddressSyncMetricsFFI, PlatformAddressSyncWalletResultFFI,
};
use crate::shielded_types::ShieldedSyncWalletResultFFI;
use platform_wallet::events::{EventHandler, PlatformEventHandler, WalletEvent};
#[cfg(feature = "shielded")]
use platform_wallet::manager::shielded_sync::{ShieldedSyncPassSummary, WalletShieldedOutcome};
use platform_wallet::{PlatformAddressSyncSummary, WalletSyncOutcome};
use std::os::raw::{c_char, c_void};

/// C callback vtable for event handling.
///
/// All callbacks are optional (`Option<fn>`) — pass null for events you don't
/// care about. The default behavior is to ignore the event.
///
/// # ABI growth constraint (accepted limitation)
///
/// New callback slots are appended at the **end** of this struct, which
/// preserves the byte offsets of all pre-existing fields. That keeps the
/// ABI stable for callers that read individual fields, but it does NOT make
/// by-value struct growth safe: `platform_wallet_manager_create` consumes
/// the struct via `std::ptr::read`, which copies `size_of::<Self>()` bytes
/// from the caller's pointer. A caller compiled against an older, shorter
/// copy of the generated header allocates a smaller struct, so each appended
/// slot makes `ptr::read` over-read past that allocation.
///
/// This is accepted for now because the only consumer is the in-tree Swift
/// SDK, which is regenerated (cbindgen) and rebuilt in lockstep with this
/// crate — there is no out-of-tree consumer pinned to a stale header. A
/// proper fix (a leading `size`/`version` discriminator passed to
/// `platform_wallet_manager_create`, or per-callback registration
/// entrypoints) is tracked as a follow-up and is out of scope for the
/// sync-progress work that added these slots.
#[repr(C)]
pub struct EventHandlerCallbacks {
    /// Opaque context pointer passed to all callbacks.
    pub context: *mut c_void,
    /// Called on wallet events (balance update, transaction received, etc.).
    /// `event_json` contains a JSON-serialized representation of the event.
    pub on_wallet_event_fn: Option<
        unsafe extern "C" fn(context: *mut c_void, event_json: *const u8, event_json_len: usize),
    >,
    /// Called on fatal errors.
    pub on_error_fn: Option<unsafe extern "C" fn(context: *mut c_void, error_msg: *const c_char)>,
    /// Called when a platform-address sync pass completes.
    pub on_platform_address_sync_completed_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            results: *const PlatformAddressSyncWalletResultFFI,
            count: usize,
            sync_unix_seconds: u64,
        ),
    >,
    /// Called when a shielded sync pass completes (only emitted when
    /// the `shielded` feature is enabled in the FFI build). The
    /// callback slot is plumbed unconditionally so the C ABI is
    /// stable across feature toggles, but is only invoked when
    /// shielded support is compiled in.
    pub on_shielded_sync_completed_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            results: *const ShieldedSyncWalletResultFFI,
            count: usize,
            sync_unix_seconds: u64,
        ),
    >,
    /// Called once per chunk during a shielded sync pass (~every
    /// 2048 notes processed). Carries the cumulative count of
    /// encrypted notes scanned so far in the current pass plus the
    /// latest block height observed. Lets the host render a live
    /// progress counter / ProgressView during long cold syncs.
    /// Slot is plumbed unconditionally for C-ABI stability; only
    /// fires when the `shielded` feature is enabled in the FFI.
    pub on_shielded_sync_progress_fn: Option<
        unsafe extern "C" fn(context: *mut c_void, cumulative_scanned: u64, block_height: u64),
    >,
    /// Called once per committed batch during a shielded sync pass as
    /// decrypted commitments are appended to the local Orchard tree.
    /// This is the "checked / committed-to-tree" signal, distinct from
    /// `on_shielded_sync_progress_fn` (which counts *downloaded*
    /// notes). `leaves_committed` is the cumulative tree leaf count;
    /// `total_target` is the on-chain MMR total leaf count, with
    /// `total_target == 0` meaning the total is **indeterminate** (the
    /// count RPC was unavailable). Pairs with the download progress
    /// callback to drive a dual ProgressView during cold syncs. Slot
    /// is plumbed unconditionally for C-ABI stability; only fires when
    /// the `shielded` feature is enabled in the FFI. Appended at the
    /// end of the struct to preserve existing field offsets.
    pub on_shielded_tree_progress_fn: Option<
        unsafe extern "C" fn(context: *mut c_void, leaves_committed: u64, total_target: u64),
    >,
    /// Destructor for `context`, called by Rust **exactly once** when the
    /// last internal reference to this vtable drops — that is, when the
    /// manager and every background worker that can still dispatch an
    /// event have finished. Appended at the END so the struct layout
    /// stays stable.
    ///
    /// Setting this transfers ownership of `context` to Rust: the host
    /// hands over a strong reference (Swift `Unmanaged.passRetained`, JNI
    /// a boxed `GlobalRef`) and must NOT free the context itself. The
    /// callback may fire on any thread. `None` keeps the legacy borrowed
    /// contract where the host guarantees the context outlives the
    /// manager.
    pub release_fn: Option<unsafe extern "C" fn(context: *mut c_void)>,
}

// SAFETY: The context pointer is managed by the FFI caller who must ensure
// thread safety. All function pointers are inherently Send + Sync.
unsafe impl Send for EventHandlerCallbacks {}
unsafe impl Sync for EventHandlerCallbacks {}

/// Wrapper that implements `PlatformEventHandler` via FFI callbacks.
pub(crate) struct FFIEventHandler {
    callbacks: EventHandlerCallbacks,
}

impl FFIEventHandler {
    pub fn new(callbacks: EventHandlerCallbacks) -> Self {
        Self { callbacks }
    }
}

/// Releases the host callback context when the handler's last owner drops.
/// The handler is constructed exactly once per manager into the
/// `Arc<dyn PlatformEventHandler>` every event dispatcher clones, so this
/// `Drop` runs exactly once — after the manager and every worker that
/// could still fire an event (including a straggler that outlived a
/// non-clean shutdown) are done. See [`FFIPersister`]'s `Drop` for the
/// full ownership rationale.
///
/// [`FFIPersister`]: crate::persistence::FFIPersister
impl Drop for FFIEventHandler {
    fn drop(&mut self) {
        if let Some(release) = self.callbacks.release_fn {
            // SAFETY: `release_fn` was supplied together with `context` by
            // the host, which contracted for exactly one call on any
            // thread. This is the only call site and `Drop` runs once.
            unsafe { release(self.callbacks.context) };
        }
    }
}

// SAFETY: Same as EventHandlerCallbacks.
unsafe impl Send for FFIEventHandler {}
unsafe impl Sync for FFIEventHandler {}

impl EventHandler for FFIEventHandler {
    fn on_wallet_event(&self, event: &WalletEvent) {
        if let Some(cb) = self.callbacks.on_wallet_event_fn {
            // Use Debug formatting since WalletEvent doesn't implement Serialize.
            let debug_str = format!("{:?}", event);
            unsafe {
                cb(self.callbacks.context, debug_str.as_ptr(), debug_str.len());
            }
        }
    }

    fn on_error(&self, error: &str) {
        if let Some(cb) = self.callbacks.on_error_fn {
            if let Ok(c_str) = std::ffi::CString::new(error) {
                unsafe {
                    cb(self.callbacks.context, c_str.as_ptr());
                }
            }
        }
    }
}

impl PlatformEventHandler for FFIEventHandler {
    fn on_platform_address_sync_completed(&self, summary: &PlatformAddressSyncSummary) {
        let Some(cb) = self.callbacks.on_platform_address_sync_completed_fn else {
            return;
        };

        if summary.wallet_results.is_empty() {
            unsafe {
                cb(
                    self.callbacks.context,
                    std::ptr::null(),
                    0,
                    summary.sync_unix_seconds,
                );
            }
            return;
        }

        let mut owned_errors = Vec::new();
        let mut results = Vec::with_capacity(summary.wallet_results.len());
        for (&wallet_id, outcome) in &summary.wallet_results {
            match outcome {
                WalletSyncOutcome::Ok(result) => {
                    results.push(PlatformAddressSyncWalletResultFFI {
                        wallet_id,
                        success: true,
                        found_count: result.found.len(),
                        absent_count: result.absent.len(),
                        checkpoint_height: result.checkpoint_height,
                        new_sync_height: result.new_sync_height,
                        new_sync_timestamp: result.new_sync_timestamp,
                        last_known_recent_block: result.last_known_recent_block,
                        metrics: PlatformAddressSyncMetricsFFI::from(&result.metrics),
                        error_message: std::ptr::null(),
                    });
                }
                WalletSyncOutcome::Err(error) => {
                    let error_message = std::ffi::CString::new(error.as_str()).ok();
                    let error_ptr = error_message
                        .as_ref()
                        .map_or(std::ptr::null(), |message| message.as_ptr());
                    if let Some(error_message) = error_message {
                        owned_errors.push(error_message);
                    }

                    results.push(PlatformAddressSyncWalletResultFFI {
                        wallet_id,
                        success: false,
                        metrics: PlatformAddressSyncMetricsFFI::default(),
                        error_message: error_ptr,
                        ..PlatformAddressSyncWalletResultFFI::default()
                    });
                }
            }
        }

        unsafe {
            cb(
                self.callbacks.context,
                results.as_ptr(),
                results.len(),
                summary.sync_unix_seconds,
            );
        }
    }

    #[cfg(feature = "shielded")]
    fn on_shielded_sync_completed(&self, summary: &ShieldedSyncPassSummary) {
        let Some(cb) = self.callbacks.on_shielded_sync_completed_fn else {
            return;
        };

        if summary.wallet_results.is_empty() {
            unsafe {
                cb(
                    self.callbacks.context,
                    std::ptr::null(),
                    0,
                    summary.sync_unix_seconds,
                );
            }
            return;
        }

        let mut owned_errors = Vec::new();
        let mut results = Vec::with_capacity(summary.wallet_results.len());
        for (&wallet_id, outcome) in &summary.wallet_results {
            match outcome {
                WalletShieldedOutcome::Ok(result) => {
                    results.push(ShieldedSyncWalletResultFFI::ok(wallet_id, result));
                }
                WalletShieldedOutcome::Skipped => {
                    results.push(ShieldedSyncWalletResultFFI::skipped(wallet_id));
                }
                WalletShieldedOutcome::Err(error) => {
                    let error_message = std::ffi::CString::new(error.as_str()).ok();
                    let error_ptr = error_message
                        .as_ref()
                        .map_or(std::ptr::null(), |message| message.as_ptr());
                    if let Some(error_message) = error_message {
                        owned_errors.push(error_message);
                    }
                    results.push(ShieldedSyncWalletResultFFI::err(wallet_id, error_ptr));
                }
            }
        }

        unsafe {
            cb(
                self.callbacks.context,
                results.as_ptr(),
                results.len(),
                summary.sync_unix_seconds,
            );
        }
    }

    #[cfg(feature = "shielded")]
    fn on_shielded_sync_progress(&self, cumulative_scanned: u64, block_height: u64) {
        let Some(cb) = self.callbacks.on_shielded_sync_progress_fn else {
            return;
        };
        unsafe {
            cb(self.callbacks.context, cumulative_scanned, block_height);
        }
    }

    #[cfg(feature = "shielded")]
    fn on_shielded_tree_progress(&self, leaves_committed: u64, total_target: u64) {
        let Some(cb) = self.callbacks.on_shielded_tree_progress_fn else {
            return;
        };
        unsafe {
            cb(self.callbacks.context, leaves_committed, total_target);
        }
    }
}

#[cfg(test)]
mod release_tests {
    use super::*;

    /// Mirror of the persister-level test: the event context is released
    /// exactly once, only when the last `Arc<dyn PlatformEventHandler>`
    /// clone drops — so a straggling event dispatcher keeps the host
    /// handler alive rather than firing into freed memory.
    #[test]
    fn release_fires_once_when_the_last_arc_clone_drops() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        unsafe extern "C" fn count_release(context: *mut c_void) {
            if let Some(counter) = (context as *const AtomicUsize).as_ref() {
                counter.fetch_add(1, Ordering::SeqCst);
            }
        }

        let releases = Box::leak(Box::new(AtomicUsize::new(0)));
        let handler: Arc<dyn PlatformEventHandler> =
            Arc::new(FFIEventHandler::new(EventHandlerCallbacks {
                context: releases as *const AtomicUsize as *mut c_void,
                on_wallet_event_fn: None,
                on_error_fn: None,
                on_platform_address_sync_completed_fn: None,
                on_shielded_sync_completed_fn: None,
                on_shielded_sync_progress_fn: None,
                on_shielded_tree_progress_fn: None,
                release_fn: Some(count_release),
            }));
        let straggler = Arc::clone(&handler);

        drop(handler);
        assert_eq!(releases.load(Ordering::SeqCst), 0);

        drop(straggler);
        assert_eq!(releases.load(Ordering::SeqCst), 1);
    }
}
