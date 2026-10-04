//! FFI callback-based implementation of PlatformEventHandler.

use crate::platform_address_sync::{
    PlatformAddressSyncMetricsFFI, PlatformAddressSyncWalletResultFFI,
};
use crate::shielded_types::ShieldedSyncWalletResultFFI;
use dashcore::Txid;
use platform_wallet::broadcast_probe::ProbeVerdict;
use platform_wallet::events::{EventHandler, PlatformEventHandler, WalletEvent};
use platform_wallet::manager::dpns_sync::{DpnsSyncPassSummary, WalletDpnsSyncOutcome};
#[cfg(feature = "shielded")]
use platform_wallet::manager::shielded_sync::{ShieldedSyncPassSummary, WalletShieldedOutcome};
use platform_wallet::{PlatformAddressSyncSummary, WalletSyncOutcome};
use std::ffi::CString;
use std::os::raw::{c_char, c_void};

/// Current layout version of [`EventHandlerCallbacksExtension`].
pub const PLATFORM_WALLET_EVENT_CALLBACKS_EXTENSION_VERSION: u32 = 1;

/// One wallet's owned DPNS marketplace sync result. All pointers are valid
/// only for the callback duration; managed-language bridges must copy them.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DpnsSyncWalletResultFFI {
    pub wallet_id: [u8; 32],
    pub success: bool,
    pub names_tracked: u32,
    pub names_added: u32,
    pub names_departed: u32,
    pub prices_changed: u32,
    pub error_message: *const c_char,
}

impl Default for DpnsSyncWalletResultFFI {
    fn default() -> Self {
        Self {
            wallet_id: [0; 32],
            success: false,
            names_tracked: 0,
            names_added: 0,
            names_departed: 0,
            prices_changed: 0,
            error_message: std::ptr::null(),
        }
    }
}

pub type DpnsMarketplaceSyncCompletedFn = unsafe extern "C" fn(
    context: *mut c_void,
    results: *const DpnsSyncWalletResultFFI,
    count: usize,
    sync_unix_seconds: u64,
);

/// Verdict codes for [`OutgoingTransactionProbedFn`].
pub const OUTGOING_PROBE_VERDICT_ACCEPTED: u8 = 0;
/// Reserved; not emitted by this version.
pub const OUTGOING_PROBE_VERDICT_DEAD: u8 = 1;
pub const OUTGOING_PROBE_VERDICT_UNRESOLVED: u8 = 2;
/// Not a verdict: drop whatever was kept for the send's earlier verdict. Sent
/// when the send settled or left the wallet, when its wallet was removed, for
/// every send when probing is turned off, and when nodes' report of it (or of
/// the send it builds on) in a block went unseen by the wallet for several
/// blocks — a fresh verdict follows then. It does not mean the send settled.
/// `reason` is null.
pub const OUTGOING_PROBE_VERDICT_CLEARED: u8 = 3;

/// A verdict on an unconfirmed send whose broadcast outcome was unknown.
/// `wallet_id` and `txid` point to 32 bytes each; `txid` is in wire
/// (internal) byte order — reverse it for the display hex. `verdict` is one of
/// the `OUTGOING_PROBE_VERDICT_*` codes; `reason` is null for `ACCEPTED` and
/// `CLEARED`, otherwise a NUL-terminated diagnostic string (never user-facing
/// copy). All pointers are valid only for the duration of the call.
pub type OutgoingTransactionProbedFn = unsafe extern "C" fn(
    context: *mut c_void,
    wallet_id: *const u8,
    txid: *const u8,
    verdict: u8,
    reason: *const c_char,
);

/// Size/version-tagged event extension. It shares the legacy event
/// vtable's context and destructor; only callback pointers are copied.
/// This avoids growing [`EventHandlerCallbacks`] and over-reading callers
/// compiled against an older generated header.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EventHandlerCallbacksExtension {
    pub struct_size: usize,
    pub version: u32,
    pub reserved: u32,
    /// Declared inline because cbindgen does not expand a named function-
    /// pointer alias inside `Option`; using the alias here emits an opaque
    /// `Option_*` field by value and produces an invalid C header.
    pub on_dpns_marketplace_sync_completed_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            results: *const DpnsSyncWalletResultFFI,
            count: usize,
            sync_unix_seconds: u64,
        ),
    >,
    /// Appended within version 1: a host whose `struct_size` stops before
    /// this slot simply never receives it. See [`OutgoingTransactionProbedFn`].
    pub on_outgoing_transaction_probed_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            txid: *const u8,
            verdict: u8,
            reason: *const c_char,
        ),
    >,
}

impl Default for EventHandlerCallbacksExtension {
    fn default() -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>(),
            version: PLATFORM_WALLET_EVENT_CALLBACKS_EXTENSION_VERSION,
            reserved: 0,
            on_dpns_marketplace_sync_completed_fn: None,
            on_outgoing_transaction_probed_fn: None,
        }
    }
}

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
/// Existing slots remain frozen for compatibility. New event kinds belong
/// in the size/version-tagged [`EventHandlerCallbacksExtension`] (or a later
/// extension version), never at the end of this legacy by-value vtable.
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
    /// callback may fire on any thread.
    ///
    /// **Required whenever `context` is non-null** —
    /// `platform_wallet_manager_create` rejects a context-carrying vtable
    /// without a destructor, because `destroy` returns without proving
    /// every worker joined and only ownership keeps a straggler's
    /// callbacks memory-safe. A context needing no cleanup takes a no-op
    /// `release_fn`; `None` is valid only alongside a null `context`.
    pub release_fn: Option<unsafe extern "C" fn(context: *mut c_void)>,
}

// SAFETY: The context pointer is managed by the FFI caller who must ensure
// thread safety. All function pointers are inherently Send + Sync.
unsafe impl Send for EventHandlerCallbacks {}
unsafe impl Sync for EventHandlerCallbacks {}

/// Wrapper that implements `PlatformEventHandler` via FFI callbacks.
pub(crate) struct FFIEventHandler {
    callbacks: EventHandlerCallbacks,
    dpns_sync_callback: Option<DpnsMarketplaceSyncCompletedFn>,
    outgoing_probe_callback: Option<OutgoingTransactionProbedFn>,
}

impl FFIEventHandler {
    pub fn new(
        callbacks: EventHandlerCallbacks,
        dpns_sync_callback: Option<DpnsMarketplaceSyncCompletedFn>,
        outgoing_probe_callback: Option<OutgoingTransactionProbedFn>,
    ) -> Self {
        Self {
            callbacks,
            dpns_sync_callback,
            outgoing_probe_callback,
        }
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
    fn on_outgoing_transaction_cleared(&self, wallet_id: &[u8; 32], txid: &Txid) {
        let Some(callback) = self.outgoing_probe_callback else {
            return;
        };
        let txid_bytes: &[u8] = txid.as_ref();
        unsafe {
            callback(
                self.callbacks.context,
                wallet_id.as_ptr(),
                txid_bytes.as_ptr(),
                OUTGOING_PROBE_VERDICT_CLEARED,
                std::ptr::null(),
            );
        }
    }

    fn on_outgoing_transaction_probed(
        &self,
        wallet_id: &[u8; 32],
        txid: &Txid,
        verdict: &ProbeVerdict,
    ) {
        let Some(callback) = self.outgoing_probe_callback else {
            return;
        };
        let (code, reason) = match verdict {
            // The host does not need to tell a mempool from a block: both mean
            // the payment is going through.
            ProbeVerdict::Accepted | ProbeVerdict::Mined => (OUTGOING_PROBE_VERDICT_ACCEPTED, None),
            ProbeVerdict::Unresolved { reason } => {
                (OUTGOING_PROBE_VERDICT_UNRESOLVED, Some(reason))
            }
        };
        // A reason is a node's text: strip interior NULs rather than lose it.
        let reason = reason.and_then(|reason| CString::new(reason.replace('\0', "")).ok());
        let txid_bytes: &[u8] = txid.as_ref();
        unsafe {
            callback(
                self.callbacks.context,
                wallet_id.as_ptr(),
                txid_bytes.as_ptr(),
                code,
                reason.as_ref().map_or(std::ptr::null(), |r| r.as_ptr()),
            );
        }
    }

    fn on_dpns_marketplace_sync_completed(&self, summary: &DpnsSyncPassSummary) {
        let Some(callback) = self.dpns_sync_callback else {
            return;
        };
        if summary.wallet_results.is_empty() {
            unsafe {
                callback(
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
                WalletDpnsSyncOutcome::Ok(wallet_summary) => {
                    results.push(DpnsSyncWalletResultFFI {
                        wallet_id,
                        success: true,
                        names_tracked: wallet_summary.names_tracked,
                        names_added: wallet_summary.names_added.len() as u32,
                        names_departed: wallet_summary.names_departed.len() as u32,
                        prices_changed: wallet_summary.prices_changed.len() as u32,
                        error_message: std::ptr::null(),
                    });
                }
                WalletDpnsSyncOutcome::Err(error) => {
                    let error_message = std::ffi::CString::new(error.as_str()).ok();
                    let error_ptr = error_message
                        .as_ref()
                        .map_or(std::ptr::null(), |message| message.as_ptr());
                    if let Some(error_message) = error_message {
                        owned_errors.push(error_message);
                    }
                    results.push(DpnsSyncWalletResultFFI {
                        wallet_id,
                        error_message: error_ptr,
                        ..DpnsSyncWalletResultFFI::default()
                    });
                }
            }
        }
        unsafe {
            callback(
                self.callbacks.context,
                results.as_ptr(),
                results.len(),
                summary.sync_unix_seconds,
            );
        }
    }

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
            // Conversion can reject a wallet-wide sum even when each account
            // balance was valid. Report it through the existing error outcome
            // and retain its message for the entire callback invocation.
            let result = match outcome {
                WalletShieldedOutcome::Ok(result) => {
                    ShieldedSyncWalletResultFFI::ok(wallet_id, result).map_err(|e| e.to_string())
                }
                WalletShieldedOutcome::Skipped => {
                    Ok(ShieldedSyncWalletResultFFI::skipped(wallet_id))
                }
                WalletShieldedOutcome::Err(error) => Err(error.clone()),
            };
            match result {
                Ok(result) => results.push(result),
                Err(error) => {
                    let error_message = std::ffi::CString::new(error).ok();
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

    #[cfg(feature = "shielded")]
    #[test]
    fn should_report_wallet_balance_overflow_through_the_sync_callback() {
        use platform_wallet::wallet::shielded::ShieldedSyncSummary;
        use std::collections::BTreeMap;
        use std::ffi::CStr;

        type Received = Vec<([u8; 32], bool, u64, Option<String>)>;
        unsafe extern "C" fn capture(
            context: *mut c_void,
            results: *const ShieldedSyncWalletResultFFI,
            count: usize,
            _: u64,
        ) {
            let received = &mut *(context as *mut Received);
            for result in std::slice::from_raw_parts(results, count) {
                received.push((
                    result.wallet_id,
                    result.success,
                    result.balance,
                    (!result.error_message.is_null()).then(|| {
                        CStr::from_ptr(result.error_message)
                            .to_string_lossy()
                            .into_owned()
                    }),
                ));
            }
        }
        let mut received = Received::new();
        let handler = FFIEventHandler::new(
            EventHandlerCallbacks {
                context: &mut received as *mut Received as *mut c_void,
                on_wallet_event_fn: None,
                on_error_fn: None,
                on_platform_address_sync_completed_fn: None,
                on_shielded_sync_completed_fn: Some(capture),
                on_shielded_sync_progress_fn: None,
                on_shielded_tree_progress_fn: None,
                release_fn: None,
            },
            None,
            None,
        );
        handler.on_shielded_sync_completed(&ShieldedSyncPassSummary {
            wallet_results: BTreeMap::from([
                (
                    [1; 32],
                    WalletShieldedOutcome::Ok(ShieldedSyncSummary {
                        balances: BTreeMap::from([(0, u64::MAX), (1, 1)]),
                        ..Default::default()
                    }),
                ),
                (
                    [2; 32],
                    WalletShieldedOutcome::Ok(ShieldedSyncSummary {
                        balances: BTreeMap::from([(0, u64::MAX - 1), (1, 1)]),
                        ..Default::default()
                    }),
                ),
                (
                    [3; 32],
                    WalletShieldedOutcome::Err("other wallet failed".to_string()),
                ),
            ]),
            sync_unix_seconds: 100,
        });
        assert_eq!(received.len(), 3);
        assert_eq!(received[0].0, [1; 32]);
        assert!(
            !received[0].1,
            "overflow must not publish a successful zero balance"
        );
        assert_eq!(received[0].2, 0);
        assert!(received[0]
            .3
            .as_ref()
            .unwrap()
            .contains("wallet-wide shielded balance exceeds u64"));
        assert_eq!(received[1], ([2; 32], true, u64::MAX, None));
        assert_eq!(
            received[2],
            ([3; 32], false, 0, Some("other wallet failed".to_string()))
        );
    }

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
        let handler: Arc<dyn PlatformEventHandler> = Arc::new(FFIEventHandler::new(
            EventHandlerCallbacks {
                context: releases as *const AtomicUsize as *mut c_void,
                on_wallet_event_fn: None,
                on_error_fn: None,
                on_platform_address_sync_completed_fn: None,
                on_shielded_sync_completed_fn: None,
                on_shielded_sync_progress_fn: None,
                on_shielded_tree_progress_fn: None,
                release_fn: Some(count_release),
            },
            None,
            None,
        ));
        let straggler = Arc::clone(&handler);

        drop(handler);
        assert_eq!(releases.load(Ordering::SeqCst), 0);

        drop(straggler);
        assert_eq!(releases.load(Ordering::SeqCst), 1);
    }
}

#[cfg(test)]
mod outgoing_probe_callback_tests {
    use std::ffi::CStr;
    use std::sync::Mutex;

    use dashcore::hashes::Hash;
    use platform_wallet::broadcast_probe::ProbeVerdict;

    use super::*;

    /// (wallet_id, txid, verdict, reason) of one call.
    type Call = ([u8; 32], [u8; 32], u8, Option<String>);

    static CALLS: Mutex<Vec<Call>> = Mutex::new(Vec::new());

    unsafe extern "C" fn record(
        _context: *mut c_void,
        wallet_id: *const u8,
        txid: *const u8,
        verdict: u8,
        reason: *const c_char,
    ) {
        let wallet_id = std::ptr::read(wallet_id as *const [u8; 32]);
        let txid = std::ptr::read(txid as *const [u8; 32]);
        let reason =
            (!reason.is_null()).then(|| CStr::from_ptr(reason).to_string_lossy().into_owned());
        CALLS
            .lock()
            .expect("calls")
            .push((wallet_id, txid, verdict, reason));
    }

    fn handler(probe: Option<OutgoingTransactionProbedFn>) -> FFIEventHandler {
        FFIEventHandler::new(
            EventHandlerCallbacks {
                context: std::ptr::null_mut(),
                on_wallet_event_fn: None,
                on_error_fn: None,
                on_platform_address_sync_completed_fn: None,
                on_shielded_sync_completed_fn: None,
                ..unsafe { std::mem::zeroed() }
            },
            None,
            probe,
        )
    }

    /// One test drives every case so the shared capture is never raced by a
    /// parallel test.
    #[test]
    fn should_deliver_each_verdict_with_its_code_wire_txid_and_reason() {
        CALLS.lock().expect("calls").clear();
        let wallet = [7u8; 32];
        let txid = dashcore::Txid::from_byte_array([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ]);
        let with_callback = handler(Some(record));

        with_callback.on_outgoing_transaction_probed(&wallet, &txid, &ProbeVerdict::Accepted);
        with_callback.on_outgoing_transaction_probed(
            &wallet,
            &txid,
            &ProbeVerdict::Unresolved {
                reason: "no quorum".to_string(),
            },
        );
        with_callback.on_outgoing_transaction_probed(
            &wallet,
            &txid,
            &ProbeVerdict::Unresolved {
                reason: "odd\0node".to_string(),
            },
        );
        with_callback.on_outgoing_transaction_cleared(&wallet, &txid);
        // A host without the slot receives nothing.
        handler(None).on_outgoing_transaction_probed(&wallet, &txid, &ProbeVerdict::Accepted);
        handler(None).on_outgoing_transaction_cleared(&wallet, &txid);

        let calls = CALLS.lock().expect("calls").clone();
        let wire: [u8; 32] = *txid.as_byte_array();
        assert_eq!(
            calls,
            vec![
                (wallet, wire, OUTGOING_PROBE_VERDICT_ACCEPTED, None),
                (
                    wallet,
                    wire,
                    OUTGOING_PROBE_VERDICT_UNRESOLVED,
                    Some("no quorum".to_string())
                ),
                (
                    wallet,
                    wire,
                    OUTGOING_PROBE_VERDICT_UNRESOLVED,
                    Some("oddnode".to_string())
                ),
                (wallet, wire, OUTGOING_PROBE_VERDICT_CLEARED, None),
            ]
        );
    }
}
