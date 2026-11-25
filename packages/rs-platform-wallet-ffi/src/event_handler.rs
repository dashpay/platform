//! FFI callback-based implementation of PlatformEventHandler.

#[cfg(feature = "shielded")]
use crate::error::PlatformWalletFFIResultCode;
use crate::platform_address_sync::{
    PlatformAddressSyncMetricsFFI, PlatformAddressSyncWalletResultFFI,
};
use crate::shielded_types::{ShieldedSyncWalletOutcomeFFI, ShieldedSyncWalletResultFFI};
use platform_wallet::events::{EventHandler, PlatformEventHandler, WalletEvent};
use platform_wallet::manager::dpns_sync::{DpnsSyncPassSummary, WalletDpnsSyncOutcome};
#[cfg(feature = "shielded")]
use platform_wallet::manager::shielded_sync::{ShieldedSyncPassSummary, WalletShieldedOutcome};
#[cfg(feature = "shielded")]
use platform_wallet::PlatformWalletError;
use platform_wallet::{PlatformAddressSyncSummary, WalletSyncOutcome};
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

pub type ShieldedSyncOutcomesFn = unsafe extern "C" fn(
    context: *mut c_void,
    results: *const ShieldedSyncWalletOutcomeFFI,
    count: usize,
    sync_unix_seconds: u64,
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
    /// Typed successor of `EventHandlerCallbacks::on_shielded_sync_completed_fn`:
    /// each element adds the failure's result code. When set, Rust calls it
    /// instead of the legacy slot. Fires only when the `shielded` feature is
    /// enabled in the FFI.
    pub on_shielded_sync_outcomes_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            results: *const ShieldedSyncWalletOutcomeFFI,
            count: usize,
            sync_unix_seconds: u64,
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
            on_shielded_sync_outcomes_fn: None,
        }
    }
}

/// The [`EventHandlerCallbacksExtension`] slots negotiated at manager creation.
#[derive(Clone, Copy, Default)]
pub(crate) struct EventExtensionCallbacks {
    pub(crate) dpns_sync_completed: Option<DpnsMarketplaceSyncCompletedFn>,
    #[cfg_attr(
        not(any(feature = "shielded", test)),
        expect(dead_code, reason = "shielded sync events need the `shielded` feature")
    )]
    pub(crate) shielded_sync_outcomes: Option<ShieldedSyncOutcomesFn>,
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
    extensions: EventExtensionCallbacks,
}

impl FFIEventHandler {
    pub fn new(callbacks: EventHandlerCallbacks, extensions: EventExtensionCallbacks) -> Self {
        Self {
            callbacks,
            extensions,
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
    fn on_dpns_marketplace_sync_completed(&self, summary: &DpnsSyncPassSummary) {
        let Some(callback) = self.extensions.dpns_sync_completed else {
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
        let typed = self.extensions.shielded_sync_outcomes;
        let legacy = self.callbacks.on_shielded_sync_completed_fn;
        if typed.is_none() && legacy.is_none() {
            return;
        }

        let mut owned_errors = Vec::new();
        let mut outcomes = Vec::with_capacity(summary.wallet_results.len());
        for (&wallet_id, outcome) in &summary.wallet_results {
            // Conversion can reject a wallet-wide sum even when each account
            // balance was valid. Report it through the existing error outcome
            // and retain its message for the entire callback invocation.
            let result = match outcome {
                WalletShieldedOutcome::Ok(result) => {
                    ShieldedSyncWalletResultFFI::ok(wallet_id, result).map_err(|e| {
                        (
                            PlatformWalletFFIResultCode::ErrorWalletOperation,
                            e.to_string(),
                        )
                    })
                }
                WalletShieldedOutcome::Skipped => {
                    Ok(ShieldedSyncWalletResultFFI::skipped(wallet_id))
                }
                WalletShieldedOutcome::Err(error) => Err((
                    PlatformWalletFFIResultCode::ErrorWalletOperation,
                    error.clone(),
                )),
                WalletShieldedOutcome::TreeInconsistent {
                    local_leaf_count,
                    chain_leaf_count,
                } => Err((
                    PlatformWalletFFIResultCode::ErrorShieldedTreeInconsistent,
                    PlatformWalletError::ShieldedTreeInconsistent {
                        local_leaf_count: *local_leaf_count,
                        chain_leaf_count: *chain_leaf_count,
                    }
                    .to_string(),
                )),
            };
            outcomes.push(match result {
                Ok(result) => ShieldedSyncWalletOutcomeFFI {
                    result,
                    error_code: PlatformWalletFFIResultCode::Success,
                },
                Err((error_code, error)) => {
                    let error_message = std::ffi::CString::new(error).ok();
                    let error_ptr = error_message
                        .as_ref()
                        .map_or(std::ptr::null(), |message| message.as_ptr());
                    if let Some(error_message) = error_message {
                        owned_errors.push(error_message);
                    }
                    ShieldedSyncWalletOutcomeFFI {
                        result: ShieldedSyncWalletResultFFI::err(wallet_id, error_ptr),
                        error_code,
                    }
                }
            });
        }

        // An empty pass is reported with a NULL array, never a dangling one.
        fn array_ptr<T>(items: &[T]) -> *const T {
            if items.is_empty() {
                std::ptr::null()
            } else {
                items.as_ptr()
            }
        }
        if let Some(cb) = typed {
            unsafe {
                cb(
                    self.callbacks.context,
                    array_ptr(&outcomes),
                    outcomes.len(),
                    summary.sync_unix_seconds,
                );
            }
        } else if let Some(cb) = legacy {
            let results: Vec<ShieldedSyncWalletResultFFI> =
                outcomes.iter().map(|outcome| outcome.result).collect();
            unsafe {
                cb(
                    self.callbacks.context,
                    array_ptr(&results),
                    results.len(),
                    summary.sync_unix_seconds,
                );
            }
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

    /// A host compiled against an older header indexes the legacy
    /// completion array with the original element stride; every field it
    /// reads must stay where that header put it.
    #[test]
    fn should_keep_the_legacy_shielded_result_layout() {
        #[repr(C)]
        struct PublishedShieldedSyncWalletResultFFI {
            wallet_id: [u8; 32],
            success: bool,
            skipped: bool,
            cooldown_skip: bool,
            new_notes: u32,
            total_scanned: u64,
            newly_spent: u32,
            balance: u64,
            error_message: *const c_char,
        }
        type Published = PublishedShieldedSyncWalletResultFFI;
        type Current = ShieldedSyncWalletResultFFI;
        assert_eq!(
            std::mem::size_of::<Current>(),
            std::mem::size_of::<Published>()
        );
        assert_eq!(
            std::mem::align_of::<Current>(),
            std::mem::align_of::<Published>()
        );
        assert_eq!(
            std::mem::offset_of!(Current, balance),
            std::mem::offset_of!(Published, balance)
        );
        assert_eq!(
            std::mem::offset_of!(Current, error_message),
            std::mem::offset_of!(Published, error_message)
        );
    }

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
            EventExtensionCallbacks::default(),
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

    /// Background sync is the path hosts actually run, so an inconsistent
    /// tree must reach them as a result code, not only as message text.
    /// A host registering both slots gets each pass once, typed.
    #[cfg(feature = "shielded")]
    #[test]
    fn should_report_typed_error_codes_through_the_shielded_sync_callback() {
        use platform_wallet::wallet::shielded::ShieldedSyncSummary;
        use std::collections::BTreeMap;
        use std::ffi::CStr;

        type Received = Vec<([u8; 32], PlatformWalletFFIResultCode, Option<String>)>;
        unsafe extern "C" fn capture(
            context: *mut c_void,
            outcomes: *const ShieldedSyncWalletOutcomeFFI,
            count: usize,
            _: u64,
        ) {
            let received = &mut *(context as *mut Received);
            for outcome in std::slice::from_raw_parts(outcomes, count) {
                let result = &outcome.result;
                received.push((
                    result.wallet_id,
                    outcome.error_code,
                    (!result.error_message.is_null()).then(|| {
                        CStr::from_ptr(result.error_message)
                            .to_string_lossy()
                            .into_owned()
                    }),
                ));
            }
        }
        // Recorded so the equality below fails if the legacy slot also fires.
        unsafe extern "C" fn legacy(
            context: *mut c_void,
            _: *const ShieldedSyncWalletResultFFI,
            _: usize,
            _: u64,
        ) {
            let received = &mut *(context as *mut Received);
            received.push(([0; 32], PlatformWalletFFIResultCode::Success, None));
        }
        let mut received = Received::new();
        let handler = FFIEventHandler::new(
            EventHandlerCallbacks {
                context: &mut received as *mut Received as *mut c_void,
                on_wallet_event_fn: None,
                on_error_fn: None,
                on_platform_address_sync_completed_fn: None,
                on_shielded_sync_completed_fn: Some(legacy),
                on_shielded_sync_progress_fn: None,
                on_shielded_tree_progress_fn: None,
                release_fn: None,
            },
            EventExtensionCallbacks {
                shielded_sync_outcomes: Some(capture),
                ..EventExtensionCallbacks::default()
            },
        );
        handler.on_shielded_sync_completed(&ShieldedSyncPassSummary {
            wallet_results: BTreeMap::from([
                (
                    [1; 32],
                    WalletShieldedOutcome::Ok(ShieldedSyncSummary::default()),
                ),
                ([2; 32], WalletShieldedOutcome::Skipped),
                (
                    [3; 32],
                    WalletShieldedOutcome::Err("transport failed".to_string()),
                ),
                (
                    [4; 32],
                    WalletShieldedOutcome::TreeInconsistent {
                        local_leaf_count: 3,
                        chain_leaf_count: 2,
                    },
                ),
            ]),
            sync_unix_seconds: 100,
        });
        let inconsistent = PlatformWalletError::ShieldedTreeInconsistent {
            local_leaf_count: 3,
            chain_leaf_count: 2,
        }
        .to_string();
        assert_eq!(
            received,
            vec![
                ([1; 32], PlatformWalletFFIResultCode::Success, None),
                ([2; 32], PlatformWalletFFIResultCode::Success, None),
                (
                    [3; 32],
                    PlatformWalletFFIResultCode::ErrorWalletOperation,
                    Some("transport failed".to_string())
                ),
                (
                    [4; 32],
                    PlatformWalletFFIResultCode::ErrorShieldedTreeInconsistent,
                    Some(inconsistent)
                ),
            ]
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
            EventExtensionCallbacks::default(),
        ));
        let straggler = Arc::clone(&handler);

        drop(handler);
        assert_eq!(releases.load(Ordering::SeqCst), 0);

        drop(straggler);
        assert_eq!(releases.load(Ordering::SeqCst), 1);
    }
}
