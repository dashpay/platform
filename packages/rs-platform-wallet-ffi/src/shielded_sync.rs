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

use crate::error::*;
use crate::handle::*;
use crate::runtime::{block_on_worker, runtime};
use crate::shielded_types::ShieldedSyncWalletResultFFI;
use crate::{check_ptr, unwrap_option_or_return};
use rs_sdk_ffi::MnemonicResolverHandle;

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

/// Stop the shielded sync manager and wait for any in-flight pass to
/// drain before returning. No-op if not running.
///
/// Uses `quiesce` rather than cancel-only stop, so on return: the loop
/// is cancelled, no new pass will start, and any in-flight pass has
/// fully drained — its **persistence callbacks have completed** (no
/// note/sync-state row can be written after this returns) and its
/// completion-event *dispatch* on the Rust side has run.
///
/// Caveat on host-observed events: a host that marshals the completion
/// callback onto its own executor (e.g. the Swift trampoline hops it to
/// the `@MainActor`) may still observe that final, already-dispatched
/// event land *after* this call returns — Rust controls when the event
/// is dispatched, not when the host's run loop applies it. The drain
/// guarantee above (no further persistence, no new pass) is the
/// load-bearing part; hosts that must ignore a trailing UI event should
/// gate their handler on their own post-stop/post-clear state (the
/// example app drops events while unbound).
///
/// **Bounded**: the drain waits at most the coordinator quiesce budget.
/// If the in-flight pass is wedged past that deadline this returns
/// `ErrorShutdownIncomplete` instead of a false success — the pass may
/// still fire persistence/completion callbacks, so the host must keep
/// its callback context alive and must not treat sync as stopped.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_sync_stop(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.shielded_sync().quiesce())
    });
    let drained = unwrap_option_or_return!(option);
    if !drained {
        // The in-flight pass did not drain within the quiesce budget —
        // it may still fire persistence / completion callbacks. Surface
        // that instead of a silent success so the host keeps its callback
        // context alive and does not treat sync as stopped.
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorShutdownIncomplete,
            "shielded sync pass did not drain within the quiesce budget; \
             a pass may still be running"
                .to_string(),
        );
    }
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

/// Bind the given wallet's Orchard accounts on the network-scoped
/// shielded coordinator — from persisted viewing keys when the host
/// persister has them, falling back to a mnemonic-resolver seed
/// derivation only when it doesn't.
///
/// `accounts_ptr` / `accounts_len` describe the ZIP-32 account
/// indices to bind. The slice must be non-empty and at most
/// `64` entries; pass a one-element `[0]` array for the
/// single-account default. Each entry produces an independent
/// viewing-key registration and bookkeeping `SubwalletId` inside
/// the store; the same commitment tree backs every account on the
/// network.
///
/// **The resolver does NOT fire on the common path.** When every
/// requested account has a persisted viewing key (written by the
/// first seed-backed bind via
/// `on_persist_shielded_viewing_keys_fn`), the bind completes from
/// those rows and the mnemonic is never touched. The resolver fires
/// exactly once only on the fallback (first bind after create /
/// import, or persistence predating viewing-key rows); the mnemonic
/// and the derived seed then live in `Zeroizing` buffers and are
/// scrubbed before this function returns. In every case only the
/// per-account FVK / IVK / OVK / default payment addresses survive
/// on the wallet — no `SpendAuthorizingKey` stays resident; spends
/// re-derive it per operation.
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

    // Look up the wallet + the network-scoped shielded coordinator
    // on the manager. The coordinator owns the single SQLite handle
    // *and* the per-network sync-coordination registry; we hand it
    // to the bind so the wallet reuses the shared store and
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

    // Seedless path first: rebind from viewing keys persisted by a
    // prior seed-backed bind. `Ok(false)` means at least one
    // requested account has no persisted row — only then is the
    // mnemonic resolved.
    match runtime()
        .block_on(wallet_arc.bind_shielded_from_persisted(accounts.as_slice(), &coordinator))
    {
        Ok(true) => return PlatformWalletFFIResult::ok(),
        Ok(false) => {}
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                format!("bind_shielded_from_persisted failed: {e}"),
            );
        }
    }

    // Fallback: resolve the mnemonic via the host callback and
    // derive from seed (which also persists the viewing keys so the
    // next launch takes the seedless path above).
    let seed = match crate::identity_keys_from_mnemonic::resolve_seed_from_resolver(
        mnemonic_resolver_handle,
        &wallet_id,
    ) {
        Ok(seed) => seed,
        Err(result) => return result,
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
/// fails. The host **must** check this before wiping its own
/// persistence: a silent failure would leave the shared tree
/// populated while the host drops its rows, and the next cold
/// resync would gate-skip every re-downloaded position against the
/// stale tree size.
///
/// Errors with `ErrorWalletOperation` when no shielded coordinator is
/// installed on this manager (the sync-loop stop still runs unconditionally
/// first). A Clear is only reachable behind a bound, shielded-enabled host
/// surface, so a missing coordinator means `configure_shielded` never ran on
/// THIS manager instance — a wiring fault that must surface (and make the host
/// fail closed) rather than report a phantom success while the on-disk tree is
/// left untouched.
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
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("clear_shielded failed: {e}"),
        );
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
