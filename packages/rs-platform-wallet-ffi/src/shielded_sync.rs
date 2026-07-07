//! FFI bindings for `PlatformWalletManager`'s shielded sync
//! coordinator + the host-driven `bind_shielded` entry point.
//!
//! Mirror of [`platform_address_sync`](crate::platform_address_sync)
//! for the Orchard/ZK path. The whole module is feature-gated behind
//! `shielded`; builds without the feature emit none of these symbols
//! and the upstream [`ShieldedSyncManager`] doesn't exist.
//!
//! [`ShieldedSyncManager`]: platform_wallet::manager::shielded_sync::ShieldedSyncManager

use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::time::Duration;

use platform_wallet::wallet::shielded::ShieldedSyncSummary;

use zeroize::Zeroizing;

use crate::error::*;
use crate::handle::*;
use crate::identity_keys_from_mnemonic::parse_mnemonic_any_language;
use crate::runtime::{block_on_worker, runtime};
use crate::shielded_types::ShieldedSyncWalletResultFFI;
use crate::{check_ptr, unwrap_option_or_return};
use rs_sdk_ffi::{
    mnemonic_resolver_result, MnemonicResolverHandle, MNEMONIC_RESOLVER_BUFFER_CAPACITY,
};

impl ShieldedSyncWalletResultFFI {
    pub(crate) fn ok(wallet_id: [u8; 32], summary: &ShieldedSyncSummary) -> Self {
        // Multi-account on the Rust side; flattened to wallet-level
        // sums here. Hosts that want per-account detail call
        // `platform_wallet_manager_shielded_balance(account)`.
        let new_notes = u32::try_from(summary.notes_result.total_new_notes()).unwrap_or(u32::MAX);
        let newly_spent = u32::try_from(summary.total_newly_spent()).unwrap_or(u32::MAX);
        Self {
            wallet_id,
            success: true,
            skipped: false,
            cooldown_skip: summary.is_cooldown_skip,
            new_notes,
            total_scanned: summary.notes_result.total_scanned,
            newly_spent,
            balance: summary.balance_total(),
            error_message: std::ptr::null(),
        }
    }
}

// ---------------------------------------------------------------------------
// Shielded sync coordinator FFI
// ---------------------------------------------------------------------------

/// Start the shielded sync manager in the background.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_start(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        let _entered = runtime().enter();
        manager.shielded_sync_arc().start();
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Stop the shielded sync manager. No-op if not running.
///
/// **Cancel-only**: signals the loop and returns immediately, matching
/// `platform_address_sync_stop` / `identity_sync_stop`. An in-flight
/// pass is cancelled mid-flight at its next `.await`; a parked prior-
/// generation orphan is **not** joined here. Never returns
/// `ErrorShutdownIncomplete` — the join-and-orphan-liveness gate that
/// prevents a host UAF lives on `platform_wallet_manager_destroy` and
/// `platform_wallet_manager_shielded_clear`, which are the host's
/// contract points for "safe to free the callback context".
///
/// Caveat: a host marshalling events onto its own executor (e.g. Swift
/// hops to `@MainActor`) may still observe an already-dispatched event
/// land after this returns; gate the handler on post-stop state if
/// trailing events must be dropped (the example app does so).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_stop(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        // Cancel-only by design: a second `AtomicFlagGuard` on `quiescing`
        // here would race a continuously-held gate in `shielded_clear`.
        manager.shielded_sync().stop();
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Whether the shielded sync background loop is running.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_is_running(
    handle: Handle,
    out_running: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_running);

    let option = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(handle, |manager| manager.shielded_sync().is_running());
    *out_running = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Whether a shielded sync pass is currently in flight.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_is_syncing(
    handle: Handle,
    out_syncing: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_syncing);

    let option = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(handle, |manager| manager.shielded_sync().is_syncing());
    *out_syncing = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Unix seconds of the last completed shielded sync pass, or 0 if
/// none has ever completed.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_last_sync_unix_seconds(
    handle: Handle,
    out_last_sync_unix: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_last_sync_unix);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager
            .shielded_sync()
            .last_sync_unix_seconds()
            .unwrap_or(0)
    });
    *out_last_sync_unix = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Set the background shielded sync interval in seconds.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_set_interval(
    handle: Handle,
    interval_seconds: u64,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager
            .shielded_sync()
            .set_interval(Duration::from_secs(interval_seconds));
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Run one shielded sync pass across all registered wallets.
///
/// This is the user-initiated entry point (the host's "Sync Now"
/// button), so `force=true` is passed through to bypass the
/// per-wallet caught-up cooldown: a user who just sent a
/// transaction and taps the button should see the resulting
/// note immediately, not wait out the cooldown. The background
/// loop in `ShieldedSyncManager::start()` uses `force=false`
/// and honors the cooldown.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_sync_now(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        let mgr = manager.shielded_sync_arc();
        // `block_on_worker`, NOT `runtime().block_on`: the host calls
        // this from a dispatch/concurrency thread with ~512 KB of
        // stack, and polling the whole notes-sync future there blows
        // it (SIGBUS "Thread stack size exceeded" observed on-device
        // and on-sim 2026-07-07 from the Sync Now button). The worker
        // dispatch moves the compute onto the runtime's 8 MB-stack
        // threads (see runtime.rs) — same fix as dashpay_sync.
        block_on_worker(async move { mgr.sync_now(true).await });
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Bind shielded
// ---------------------------------------------------------------------------

/// Derive Orchard keys for the given wallet from the host-supplied
/// mnemonic resolver and register the resulting accounts on the
/// network-scoped shielded coordinator.
///
/// `accounts_ptr` / `accounts_len` describe the ZIP-32 account
/// indices to derive. The slice must be non-empty and at most
/// `64` entries; pass a one-element `[0]` array for the
/// single-account default. Each entry produces an independent
/// [`OrchardKeySet`] and bookkeeping `SubwalletId` inside the
/// store; the same commitment tree backs every account on the
/// network.
///
/// The resolver fires exactly once per call. The mnemonic and the
/// derived seed live in `Zeroizing` buffers and are scrubbed
/// before this function returns; only the per-account FVK / IVK /
/// OVK / default payment addresses survive on the wallet.
///
/// **Prerequisite**: the host must have already called
/// [`platform_wallet_manager_configure_shielded`] with the
/// per-network SQLite path before invoking this function — the
/// shared commitment-tree handle is opened there, not here.
/// Calling `bind_shielded` before `configure_shielded` returns
/// `ErrorWalletOperation`.
///
/// Idempotent: a second call replaces the previously-bound
/// shielded wallet on the same `wallet_id`.
///
/// # Safety
/// - `wallet_id_bytes` must point at 32 readable bytes.
/// - `accounts_ptr` must point at `accounts_len` readable `u32`s.
/// - `mnemonic_resolver_handle` must come from
///   [`crate::dash_sdk_mnemonic_resolver_create`].
///
/// [`OrchardKeySet`]: platform_wallet::wallet::shielded::OrchardKeySet
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_bind_shielded(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    accounts_ptr: *const u32,
    accounts_len: usize,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(accounts_ptr);
    if accounts_len == 0 || accounts_len > 64 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("accounts_len must be in 1..=64, got {accounts_len}"),
        );
    }
    let accounts: Vec<u32> = std::slice::from_raw_parts(accounts_ptr, accounts_len).to_vec();

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);

    // Resolve mnemonic via the host callback.
    let mut mnemonic_buf: Zeroizing<[u8; MNEMONIC_RESOLVER_BUFFER_CAPACITY]> =
        Zeroizing::new([0u8; MNEMONIC_RESOLVER_BUFFER_CAPACITY]);
    let mut mnemonic_len: usize = 0;

    let resolver = &*mnemonic_resolver_handle;
    let resolver_vtable = &*resolver.vtable;
    let rc = (resolver_vtable.resolve)(
        resolver.ctx as *const std::os::raw::c_void,
        wallet_id_bytes,
        mnemonic_buf.as_mut_ptr() as *mut c_char,
        MNEMONIC_RESOLVER_BUFFER_CAPACITY,
        &mut mnemonic_len,
    );

    match rc {
        x if x == mnemonic_resolver_result::SUCCESS => {}
        x if x == mnemonic_resolver_result::NOT_FOUND => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "mnemonic missing for wallet",
            );
        }
        x if x == mnemonic_resolver_result::BUFFER_TOO_SMALL => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "mnemonic resolver buffer too small",
            );
        }
        _ => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "mnemonic resolver failed",
            );
        }
    }
    if mnemonic_len == 0 || mnemonic_len > MNEMONIC_RESOLVER_BUFFER_CAPACITY {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "mnemonic resolver returned empty buffer",
        );
    }

    // Parse and derive seed. Both intermediate forms live in
    // `Zeroizing` so they're scrubbed when this function exits.
    let mnemonic_str = match std::str::from_utf8(&mnemonic_buf[..mnemonic_len]) {
        Ok(s) => s,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                format!("mnemonic is not valid UTF-8: {e}"),
            );
        }
    };
    let mnemonic = match parse_mnemonic_any_language(mnemonic_str) {
        Ok(m) => m,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                format!("invalid mnemonic: {e}"),
            );
        }
    };
    let seed: Zeroizing<[u8; 64]> = Zeroizing::new(mnemonic.to_seed(""));
    drop(mnemonic);

    // Look up the wallet + the network-scoped shielded coordinator
    // on the manager. The coordinator owns the single SQLite handle
    // *and* the per-network sync-coordination registry; we hand it
    // to `bind_shielded` so the wallet reuses the shared store and
    // self-registers its viewing keys for the coordinator-driven
    // sync loop.
    let lookup = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(async {
            let wallet = manager.get_wallet(&wallet_id).await;
            let coordinator = manager.shielded_coordinator().await;
            (wallet, coordinator)
        })
    });
    let (wallet_arc, coordinator) = unwrap_option_or_return!(lookup);
    let wallet_arc = match wallet_arc {
        Some(w) => w,
        None => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                format!("wallet not found: {}", hex::encode(wallet_id)),
            );
        }
    };
    let coordinator = match coordinator {
        Some(c) => c,
        None => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "shielded support not configured — call platform_wallet_manager_configure_shielded first",
            );
        }
    };

    if let Err(e) = runtime().block_on(wallet_arc.bind_shielded(
        seed.as_ref(),
        accounts.as_slice(),
        &coordinator,
    )) {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("bind_shielded failed: {e}"),
        );
    }

    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Configure shielded (network-scoped)
// ---------------------------------------------------------------------------

/// Configure the network-scoped shielded coordinator for this
/// manager. Opens (or creates) the per-network commitment-tree
/// SQLite file at `db_path_cstr` and installs a coordinator that
/// every subsequent `platform_wallet_manager_bind_shielded` call
/// reuses — one SQLite handle per network manager, regardless of
/// how many wallets bind shielded.
///
/// Must be called **before** any `bind_shielded` on this manager.
/// Calling it again with the same path is a no-op (idempotent).
/// Calling it again with a different path returns
/// `ErrorWalletOperation`: the SQLite handle is opened once and
/// can't be repointed mid-flight.
///
/// # Safety
/// - `db_path_cstr` must be a valid NUL-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_configure_shielded(
    handle: Handle,
    db_path_cstr: *const c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(db_path_cstr);
    let db_path = match CStr::from_ptr(db_path_cstr).to_str() {
        Ok(s) => PathBuf::from(s),
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                format!("db_path is not valid UTF-8: {e}"),
            );
        }
    };

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.configure_shielded(&db_path))
    });
    let result = unwrap_option_or_return!(option);
    if let Err(e) = result {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("configure_shielded failed: {e}"),
        );
    }
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Clear shielded state (Rust side)
// ---------------------------------------------------------------------------

/// Reset the Rust-side shielded state on this manager: stop the
/// background sync loop, drop every wallet registration on the
/// network-scoped coordinator, and reset the caught-up cooldown
/// stamp.
///
/// The SQLite commitment-tree file stays on disk but its contents
/// are reset to empty — Clear semantics are "wipe my shielded
/// state and cold-resync from index 0 on the shared tree". The
/// host is responsible for wiping its own per-wallet persistence
/// layer (e.g. SwiftData rows) since Rust can't reach into iOS /
/// Android persistence; after that, the next
/// [`platform_wallet_manager_bind_shielded`] call repopulates the
/// coordinator's registries and the next sync pass re-saves notes
/// via the changeset path.
///
/// Returns `ErrorWalletOperation` if the Rust-side store reset
/// fails, or `ErrorShutdownIncomplete` if the in-flight sync pass
/// did not drain cleanly first (in which case the store is left
/// intact). The host **must** check this before wiping its own
/// persistence: a silent failure would leave the shared tree
/// populated while the host drops its rows, and the next cold
/// resync would gate-skip every re-downloaded position against the
/// stale tree size.
///
/// Idempotent: calling Clear when shielded support has never
/// been configured (no coordinator installed) is still a
/// successful no-op on the coordinator side. The sync-loop stop
/// is unconditional.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_clear(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        // Single library call: `clear_shielded` quiesces the sync
        // manager (cancel + drain the in-flight pass, incl. persister
        // fan-out, so nothing re-persists after Clear) and then clears
        // the coordinator registries + resets the shared store. Keeping
        // the quiesce+clear sequencing in the library (not stitched
        // here) follows the FFI's "resolve handle, call one function,
        // marshal result" contract.
        runtime().block_on(manager.clear_shielded())
    });
    let result = unwrap_option_or_return!(option);
    if let Err(e) = result {
        // A non-clean / timed-out quiesce aborts the clear *before* the store
        // is touched: surface it as ErrorShutdownIncomplete (symmetric with
        // destroy / shielded_sync_stop) so the host defers freeing its
        // callback context and does NOT commit its own persistence wipe — the
        // store was intentionally left intact. Every other clear failure is a
        // store-reset error → ErrorWalletOperation, as before.
        let code = match &e {
            platform_wallet::PlatformWalletError::ShieldedShutdownIncomplete { .. } => {
                PlatformWalletFFIResultCode::ErrorShutdownIncomplete
            }
            _ => PlatformWalletFFIResultCode::ErrorWalletOperation,
        };
        return PlatformWalletFFIResult::err(code, format!("clear_shielded failed: {e}"));
    }
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Default Orchard payment address
// ---------------------------------------------------------------------------

/// Read the default Orchard payment address for `account` on the
/// bound shielded sub-wallet of `wallet_id`. The host receives 43
/// raw bytes (recipient + diversifier) and applies its own
/// bech32m encoding.
///
/// `*out_present` is set to `true` and 43 bytes are written to
/// `out_bytes_43` when `account` is bound. `*out_present` is set
/// to `false` when the wallet is known but the shielded
/// sub-wallet hasn't been bound, or `account` isn't bound on it.
/// An unknown wallet returns `ErrorWalletOperation`.
///
/// # Safety
/// - `wallet_id_bytes` must point at 32 readable bytes.
/// - `out_bytes_43` must point at 43 writable bytes.
/// - `out_present` must be writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_default_address(
    handle: Handle,
    wallet_id_bytes: *const u8,
    account: u32,
    out_bytes_43: *mut u8,
    out_present: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(out_bytes_43);
    check_ptr!(out_present);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);

    enum Outcome {
        WalletMissing,
        Unbound,
        Bound([u8; 43]),
    }

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(async {
            match manager.get_wallet(&wallet_id).await {
                None => Outcome::WalletMissing,
                Some(w) => match w.shielded_default_address(account).await {
                    Some(bytes) => Outcome::Bound(bytes),
                    None => Outcome::Unbound,
                },
            }
        })
    });
    let outcome = unwrap_option_or_return!(option);

    match outcome {
        Outcome::WalletMissing => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("wallet not found: {}", hex::encode(wallet_id)),
        ),
        Outcome::Unbound => {
            *out_present = false;
            PlatformWalletFFIResult::ok()
        }
        Outcome::Bound(bytes) => {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes_43, 43);
            *out_present = true;
            PlatformWalletFFIResult::ok()
        }
    }
}

// ---------------------------------------------------------------------------
// Per-wallet sync_now
// ---------------------------------------------------------------------------

/// Run a shielded sync on a single wallet on demand.
///
/// Does not set the manager's global `is_syncing` flag — gate on
/// [`platform_wallet_manager_shielded_sync_is_syncing`] yourself if
/// you want to avoid concurrent passes. Returns an error if the
/// wallet doesn't exist or the sync itself fails. Wallets with no
/// bound shielded sub-wallet succeed silently with no observable
/// state change.
///
/// # Safety
/// - `wallet_id_bytes` must point at 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_wallet(
    handle: Handle,
    wallet_id_bytes: *const u8,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        // Per-wallet sync_wallet is exclusively a user-initiated
        // entry point — same `force=true` reasoning and same
        // `block_on_worker` stack-size requirement as
        // `platform_wallet_manager_shielded_sync_sync_now`.
        let mgr = manager.shielded_sync_arc();
        block_on_worker(async move { mgr.sync_wallet(&wallet_id, true).await })
    });
    let result = unwrap_option_or_return!(option);
    match result {
        Ok(_) => PlatformWalletFFIResult::ok(),
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("shielded sync failed: {e}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        platform_wallet_manager_shielded_sync_is_syncing,
        platform_wallet_manager_shielded_sync_start, platform_wallet_manager_shielded_sync_stop,
    };
    use crate::error::PlatformWalletFFIResultCode;
    use crate::event_handler::{EventHandlerCallbacks, FFIEventHandler};
    use crate::handle::{Handle, PLATFORM_WALLET_MANAGER_STORAGE};
    use crate::manager::platform_wallet_manager_destroy;
    use crate::persistence::{FFIPersister, PersistenceCallbacks};
    use crate::runtime::runtime;
    use crate::shielded_types::ShieldedSyncWalletResultFFI;

    use platform_wallet::{PlatformEventHandler, PlatformWalletManager};

    use std::os::raw::c_void;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    /// How long the slow shielded-completed callback parks the in-flight
    /// pass. Comfortably longer than any plausible cancel latency, so the
    /// `stop` promptness bound below can never race the drain.
    const IN_FLIGHT_PASS_MILLIS: u64 = 1000;

    /// Shared state the slow completion callback reaches through the FFI
    /// `context` pointer: `started` flips when a pass callback enters,
    /// `completed` flips only after it has parked for the full duration —
    /// i.e. after the pass has actually drained.
    struct SlowPassState {
        started: AtomicBool,
        completed: AtomicBool,
    }

    /// Slow `on_shielded_sync_completed` callback. Fires (with null/0
    /// results) even for the empty pass an unconfigured coordinator
    /// produces, so it holds `is_syncing` across a real sleep without
    /// needing a bound wallet. Reads `context` as a `SlowPassState`.
    ///
    /// # Safety
    /// `context` must point at a live `SlowPassState` (this crate's tests
    /// keep the box alive until after `destroy` has joined every worker).
    unsafe extern "C" fn slow_shielded_completed(
        context: *mut c_void,
        _results: *const ShieldedSyncWalletResultFFI,
        _count: usize,
        _sync_unix_seconds: u64,
    ) {
        let state = &*(context as *const SlowPassState);
        state.started.store(true, Ordering::Release);
        std::thread::sleep(Duration::from_millis(IN_FLIGHT_PASS_MILLIS));
        state.completed.store(true, Ordering::Release);
    }

    fn event_callbacks(context: *mut c_void, slow: bool) -> EventHandlerCallbacks {
        EventHandlerCallbacks {
            context,
            on_wallet_event_fn: None,
            on_error_fn: None,
            on_platform_address_sync_completed_fn: None,
            on_shielded_sync_completed_fn: slow.then_some(slow_shielded_completed),
            on_shielded_sync_progress_fn: None,
            on_shielded_tree_progress_fn: None,
        }
    }

    /// Insert a manager (mock SDK + no-op persister + the given handler)
    /// into the FFI handle storage and return its handle. Enters the FFI
    /// runtime for the event-adapter spawn `new` performs, mirroring
    /// `platform_wallet_manager_create`.
    fn insert_manager(callbacks: EventHandlerCallbacks) -> Handle {
        let sdk = std::sync::Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = std::sync::Arc::new(FFIPersister::new(PersistenceCallbacks::default()));
        let handler: std::sync::Arc<dyn PlatformEventHandler> =
            std::sync::Arc::new(FFIEventHandler::new(callbacks));
        let _entered = runtime().enter();
        let manager = PlatformWalletManager::new(sdk, persister, handler);
        PLATFORM_WALLET_MANAGER_STORAGE.insert(manager)
    }

    fn is_syncing(handle: Handle) -> bool {
        let mut out = false;
        let result = unsafe { platform_wallet_manager_shielded_sync_is_syncing(handle, &mut out) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        out
    }

    /// `platform_wallet_manager_shielded_sync_stop` is cancel-only: it
    /// signals the loop and returns promptly *without* joining the
    /// in-flight pass, and `platform_wallet_manager_destroy` is where the
    /// drain is actually observed.
    ///
    /// A slow completion callback parks the shielded pass in flight
    /// (`is_syncing` held). We assert `stop` returns while that pass is
    /// still parked (`completed == false`) and well under the park time —
    /// so it cannot have `block_on`'d the quiesce. We then assert `destroy`
    /// reports a clean `ShutdownReport` and only returns once the parked
    /// pass has drained (`completed == true`), proving `destroy` is the
    /// real join point.
    #[test]
    fn shielded_sync_stop_is_cancel_only_and_destroy_is_the_join_point() {
        let state = Box::new(SlowPassState {
            started: AtomicBool::new(false),
            completed: AtomicBool::new(false),
        });
        let state_ptr = &*state as *const SlowPassState as *mut c_void;
        let handle = insert_manager(event_callbacks(state_ptr, true));

        // Start the background loop; its first pass fires immediately and
        // dispatches the slow completion callback, parking `is_syncing`.
        let started = unsafe { platform_wallet_manager_shielded_sync_start(handle) };
        assert_eq!(started.code, PlatformWalletFFIResultCode::Success);

        // Wait (bounded) for the pass callback to be genuinely in flight.
        let deadline = Instant::now() + Duration::from_secs(5);
        while !state.started.load(Ordering::Acquire) {
            assert!(
                Instant::now() < deadline,
                "shielded pass callback never entered — nothing to cancel"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(is_syncing(handle), "precondition: a pass is in flight");

        // stop() must return promptly and cancel-only: the parked pass has
        // NOT drained (`completed == false`) when it returns.
        let t0 = Instant::now();
        let stopped = unsafe { platform_wallet_manager_shielded_sync_stop(handle) };
        let stop_elapsed = t0.elapsed();
        assert_eq!(stopped.code, PlatformWalletFFIResultCode::Success);
        assert!(
            !state.completed.load(Ordering::Acquire),
            "stop() returned before the in-flight pass drained — it must be cancel-only"
        );
        assert!(
            stop_elapsed < Duration::from_millis(IN_FLIGHT_PASS_MILLIS / 2),
            "stop() must not block on the drain; took {stop_elapsed:?}"
        );

        // destroy() is the real join point: it drains + joins, so it only
        // returns once the parked pass has completed, and reports clean.
        let destroyed = unsafe { platform_wallet_manager_destroy(handle) };
        assert_eq!(
            destroyed.code,
            PlatformWalletFFIResultCode::Success,
            "destroy must report a clean ShutdownReport once every worker joins"
        );
        assert!(
            state.completed.load(Ordering::Acquire),
            "destroy must observe the in-flight pass drain — it is the join point, not stop()"
        );

        // `state` outlived every worker: destroy joined them, so no callback
        // can still be reading `state_ptr` as the box drops here.
    }

    /// `stop` on a coordinator that was never started is a prompt,
    /// successful no-op, and `destroy` on the idle manager reports clean.
    #[test]
    fn shielded_sync_stop_on_idle_coordinator_is_ok_noop() {
        let handle = insert_manager(event_callbacks(std::ptr::null_mut(), false));

        let stopped = unsafe { platform_wallet_manager_shielded_sync_stop(handle) };
        assert_eq!(stopped.code, PlatformWalletFFIResultCode::Success);
        assert!(!is_syncing(handle), "no pass runs on a never-started loop");

        let destroyed = unsafe { platform_wallet_manager_destroy(handle) };
        assert_eq!(destroyed.code, PlatformWalletFFIResultCode::Success);
    }
}
