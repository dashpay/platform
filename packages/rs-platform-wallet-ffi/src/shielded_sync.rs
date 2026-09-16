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

use platform_wallet::wallet::shielded::{
    ShieldedBalanceSource, ShieldedLocalBalanceState, ShieldedSyncSummary,
};
use platform_wallet::PlatformWalletError;

use crate::error::*;
use crate::handle::*;
use crate::runtime::{block_on_worker, runtime};
use crate::shielded_types::{
    ShieldedBalanceSourceFFI, ShieldedLocalAccountBalanceFFI, ShieldedLocalBalanceSnapshotFFI,
    ShieldedLocalBalanceStatusFFI, ShieldedSyncWalletResultFFI,
};
use crate::{check_ptr, unwrap_option_or_return};
use rs_sdk_ffi::MnemonicResolverHandle;

impl ShieldedSyncWalletResultFFI {
    pub(crate) fn ok(
        wallet_id: [u8; 32],
        summary: &ShieldedSyncSummary,
    ) -> Result<Self, PlatformWalletError> {
        // Multi-account on the Rust side; flattened to wallet-level
        // sums here. Hosts that want per-account detail call
        // `platform_wallet_manager_local_shielded_balance_snapshot`.
        let balance = summary.balance_total()?;
        let new_notes = u32::try_from(summary.notes_result.total_new_notes()).unwrap_or(u32::MAX);
        let newly_spent = u32::try_from(summary.total_newly_spent()).unwrap_or(u32::MAX);
        Ok(Self {
            wallet_id,
            success: true,
            skipped: false,
            cooldown_skip: summary.is_cooldown_skip,
            new_notes,
            total_scanned: summary.notes_result.total_scanned,
            newly_spent,
            balance,
            error_message: std::ptr::null(),
        })
    }
}

impl From<ShieldedLocalBalanceState> for ShieldedLocalBalanceSnapshotFFI {
    fn from(state: ShieldedLocalBalanceState) -> Self {
        let snapshot = match state {
            ShieldedLocalBalanceState::Unbound => return Self::default(),
            ShieldedLocalBalanceState::RestoreIncomplete => {
                return Self {
                    status: ShieldedLocalBalanceStatusFFI::RestoreIncomplete,
                    ..Self::default()
                }
            }
            ShieldedLocalBalanceState::Ready(snapshot) => snapshot,
        };
        let accounts: Box<[_]> = snapshot
            .accounts
            .into_iter()
            .map(|(account_index, balance)| ShieldedLocalAccountBalanceFFI {
                account_index,
                spendable_credits: balance.spendable_credits,
                last_scanned_index: balance.last_scanned_index.unwrap_or(0),
                has_last_scanned_index: balance.last_scanned_index.is_some(),
                source: match balance.source {
                    ShieldedBalanceSource::NoHistory => ShieldedBalanceSourceFFI::NoHistory,
                    ShieldedBalanceSource::Restored => ShieldedBalanceSourceFFI::Restored,
                    ShieldedBalanceSource::ScannedThisSession => {
                        ShieldedBalanceSourceFFI::ScannedThisSession
                    }
                },
            })
            .collect();
        let accounts_count = accounts.len();
        Self {
            status: ShieldedLocalBalanceStatusFFI::Ready,
            accounts: if accounts_count == 0 {
                std::ptr::null()
            } else {
                Box::into_raw(accounts) as *const ShieldedLocalAccountBalanceFFI
            },
            accounts_count,
        }
    }
}

/// Read the bound wallet's local shielded balance, including pending-spend
/// reservations, without starting sync or resolving a mnemonic. Waits at most
/// 100 ms for the coordinator lifecycle/store locks. On contention, returns
/// ErrorWalletOperation with an empty output; retain the last balance and retry.
/// A store wait releases the lifecycle guard before parking, so unrelated
/// store-free binds can proceed. The account set is revalidated on reacquisition.
/// Check the returned result code before interpreting any output fields. Only
/// Success makes the status authoritative; an error's reset Unbound value is
/// allocation cleanup state, not a statement that the wallet is unbound.
/// Call it off the host's UI thread.
///
/// # Safety
/// `wallet_id_bytes` must point to 32 readable bytes; `out_snapshot` must be
/// writable and must not still own an earlier snapshot. Free the result with
/// `platform_wallet_manager_local_shielded_balance_snapshot_free`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_local_shielded_balance_snapshot(
    handle: Handle,
    wallet_id_bytes: *const u8,
    out_snapshot: *mut ShieldedLocalBalanceSnapshotFFI,
) -> PlatformWalletFFIResult {
    local_shielded_balance_snapshot_with_budget(
        handle,
        wallet_id_bytes,
        out_snapshot,
        Duration::from_millis(100),
    )
}

// Separate the budget from the exported ABI so the registry-lock regression
// can keep a snapshot parked longer than its independent writer/stop deadlines.
unsafe fn local_shielded_balance_snapshot_with_budget(
    handle: Handle,
    wallet_id_bytes: *const u8,
    out_snapshot: *mut ShieldedLocalBalanceSnapshotFFI,
    wait_budget: Duration,
) -> PlatformWalletFFIResult {
    check_ptr!(out_snapshot);
    *out_snapshot = ShieldedLocalBalanceSnapshotFFI::default();
    check_ptr!(wallet_id_bytes);
    let mut wallet_id = [0; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), wallet_id.len());
    let lookup = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(async {
            let wallet = manager.get_wallet(&wallet_id).await;
            let coordinator = manager.shielded_coordinator().await;
            (wallet, coordinator)
        })
    });
    let (wallet, coordinator) = unwrap_option_or_return!(lookup);
    // Only the owned Arcs cross this wait. Holding the global registry's read
    // guard while a scan owns the store lock can block an unrelated writer,
    // which then blocks the stop lookup behind it on the fair registry lock.
    let result = runtime().block_on(async {
        // A missing wallet is an API error, not a legitimate unbound state.
        let wallet = wallet.ok_or_else(|| {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                format!(
                    "local shielded balance snapshot failed: {}",
                    PlatformWalletError::WalletNotFound(hex::encode(wallet_id))
                ),
            )
        })?;
        let Some(coordinator) = coordinator else {
            return Ok(ShieldedLocalBalanceState::Unbound);
        };
        // Keep the wallet alive until the read completes. Coordinator
        // lifecycle locking serializes removal/Clear against its snapshot.
        tokio::time::timeout(
            wait_budget,
            coordinator.local_balance_snapshot(wallet.wallet_id()),
        )
        .await
        .map_err(|_| {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "local shielded balance snapshot busy; retry after sync releases its locks",
            )
        })?
        .map_err(|error| {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                format!("local shielded balance snapshot failed: {error}"),
            )
        })
    });
    match result {
        Ok(snapshot) => {
            *out_snapshot = snapshot.into();
            PlatformWalletFFIResult::ok()
        }
        Err(error) => error,
    }
}

/// Free the account array and reset the caller's snapshot. A null pointer is a
/// no-op; calling again on the same reset value is also safe. Resetting only
/// releases ownership; it does not establish the wallet's binding status.
///
/// # Safety
/// `snapshot` must be null or point to a live value returned by the matching
/// snapshot function, not a copied owner or an already-freed array.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_local_shielded_balance_snapshot_free(
    snapshot: *mut ShieldedLocalBalanceSnapshotFFI,
) {
    let Some(snapshot) = snapshot.as_mut() else {
        return;
    };
    if !snapshot.accounts.is_null() {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            snapshot.accounts as *mut ShieldedLocalAccountBalanceFFI,
            snapshot.accounts_count,
        )));
    }
    *snapshot = ShieldedLocalBalanceSnapshotFFI::default();
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
    // mnemonic resolved. Persisted-key load/restore errors propagate without
    // resolving the seed; fallback must not mask a persistence failure.
    match runtime()
        .block_on(wallet_arc.bind_shielded_from_persisted(accounts.as_slice(), &coordinator))
    {
        Ok(true) => return PlatformWalletFFIResult::ok(),
        Ok(false) => {}
        Err(e) => {
            return map_shielded_error(e, "bind_shielded_from_persisted");
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
        return map_shielded_error(e, "bind_shielded");
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
        return map_shielded_error(e, "configure_shielded");
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
        // A drain that did not complete is NOT an ordinary store failure:
        // it means callback-capable work may still be running, which the
        // host must be able to tell apart (it keeps its callback context
        // alive rather than just retrying the wipe). Route that one case
        // through the typed conversion and keep the generic mapping for
        // every other failure.
        if matches!(
            e,
            platform_wallet::PlatformWalletError::ShutdownIncomplete(_)
        ) {
            return PlatformWalletFFIResult::from(e);
        }
        return map_shielded_error(e, "clear_shielded");
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
        Err(e) => map_shielded_error(e, "shielded sync"),
    }
}

/// Recovery errors must remain typed when opening, binding or syncing durable state,
/// just as they do when submitting or explicitly abandoning a payment.
fn map_shielded_error(error: PlatformWalletError, operation: &str) -> PlatformWalletFFIResult {
    match error {
        error @ PlatformWalletError::ShieldedRecoveryCorrupted { .. }
        | error @ PlatformWalletError::ShieldedRecoveryKeysRequired { .. } => error.into(),
        other => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("{operation} failed: {other}"),
        ),
    }
}

#[cfg(test)]
mod recovery_error_tests {
    use super::*;

    #[test]
    fn should_preserve_recovery_errors_when_configuring_binding_or_syncing() {
        for (error, expected) in [
            (
                PlatformWalletError::ShieldedRecoveryCorrupted {
                    account_index: None,
                    reason: "invalid durable recovery envelope".into(),
                },
                PlatformWalletFFIResultCode::ErrorShieldedRecoveryCorrupted,
            ),
            (
                PlatformWalletError::ShieldedRecoveryKeysRequired {
                    account_index: 9,
                    reason: "account viewing keys required".into(),
                },
                PlatformWalletFFIResultCode::ErrorShieldedRecoveryKeysRequired,
            ),
        ] {
            let message = error.to_string();
            let result = map_shielded_error(error, "configure or bind");
            assert_eq!(result.code, expected);
            assert_eq!(
                unsafe { CStr::from_ptr(result.message) }.to_str().unwrap(),
                message
            );
        }
    }
}

#[cfg(test)]
mod local_balance_tests {
    use super::*;
    use crate::event_handler::{EventHandlerCallbacks, FFIEventHandler};
    use crate::persistence::{FFIPersister, PersistenceCallbacks};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use platform_wallet::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use platform_wallet::wallet::shielded::{
        OrchardKeySet, ShieldedLocalAccountBalance, ShieldedLocalBalanceSnapshot,
    };
    use platform_wallet::PlatformWalletManager;
    use std::collections::BTreeMap;
    use std::future::{poll_fn, Future};
    use std::sync::{mpsc, Arc};
    use std::task::Poll;
    use std::time::Instant;

    fn mock_manager() -> PlatformWalletManager<FFIPersister> {
        unsafe extern "C" fn begin(_: *mut std::ffi::c_void, _: *const u8) -> i32 {
            0
        }
        unsafe extern "C" fn end(_: *mut std::ffi::c_void, _: *const u8, _: bool) -> i32 {
            0
        }
        let persister = FFIPersister::new(PersistenceCallbacks {
            on_changeset_begin_fn: Some(begin),
            on_changeset_end_fn: Some(end),
            ..Default::default()
        });
        let events = FFIEventHandler::new(
            EventHandlerCallbacks {
                context: std::ptr::null_mut(),
                on_wallet_event_fn: None,
                on_error_fn: None,
                on_platform_address_sync_completed_fn: None,
                on_shielded_sync_completed_fn: None,
                on_shielded_sync_progress_fn: None,
                on_shielded_tree_progress_fn: None,
                release_fn: None,
            },
            None,
        );
        let _runtime_guard = runtime().enter();
        PlatformWalletManager::new(
            Arc::new(
                dash_sdk::SdkBuilder::new_mock()
                    .with_network(dashcore::Network::Testnet)
                    .build()
                    .expect("mock sdk"),
            ),
            Arc::new(persister),
            Arc::new(events),
        )
    }

    fn bound_manager() -> (
        Handle,
        [u8; 32],
        Arc<platform_wallet::wallet::shielded::NetworkShieldedCoordinator>,
        PathBuf,
    ) {
        let manager = mock_manager();
        let path = std::env::temp_dir().join(format!(
            "ffi-shielded-local-balance-{}-{}.sqlite",
            std::process::id(),
            next_handle()
        ));
        let (wallet_id, coordinator) = runtime().block_on(async {
            let wallet = manager
                .create_wallet_from_seed_bytes(
                    dashcore::Network::Testnet,
                    &[42; 64],
                    WalletAccountCreationOptions::Default,
                    Some(0),
                )
                .await
                .expect("create mock wallet");
            manager
                .configure_shielded(&path)
                .await
                .expect("configure shielded");
            let coordinator = manager.shielded_coordinator().await.expect("coordinator");
            let wallet_id = wallet.wallet_id();
            let views = OrchardKeySet::from_seed(&[42; 64], dashcore::Network::Testnet, 0)
                .expect("viewing keys")
                .viewing_keys();
            coordinator
                .register_wallet(
                    wallet_id,
                    BTreeMap::from([(0, views)]),
                    WalletPersister::new(wallet_id, Arc::new(NoPlatformPersistence)),
                )
                .await
                .expect("register test wallet");
            coordinator.mark_hydrated(wallet_id, true).await;
            (wallet_id, coordinator)
        });
        let handle = PLATFORM_WALLET_MANAGER_STORAGE.insert(manager);
        (handle, wallet_id, coordinator, path)
    }

    #[test]
    fn should_allow_registry_writer_and_stop_while_local_balance_snapshot_is_parked() {
        let (handle, wallet_id, coordinator, path) = bound_manager();
        let unrelated_handle = PLATFORM_WALLET_MANAGER_STORAGE.insert(mock_manager());
        // Model a scan owning the real store lock. Once the FFI snapshot has
        // cloned this coordinator, it cannot finish until this guard drops.
        let store_guard = runtime().block_on(coordinator.store().write());
        let coordinator_refs = Arc::strong_count(&coordinator);
        let snapshot_thread = std::thread::spawn(move || unsafe {
            let mut snapshot = ShieldedLocalBalanceSnapshotFFI::default();
            let mut result = local_shielded_balance_snapshot_with_budget(
                handle,
                wallet_id.as_ptr(),
                &mut snapshot,
                Duration::from_secs(10),
            );
            let outcome = (result.code, snapshot.status, snapshot.accounts_count);
            platform_wallet_manager_local_shielded_balance_snapshot_free(&mut snapshot);
            crate::platform_wallet_ffi_result_free(&mut result);
            outcome
        });
        // Observe the actual entrypoint's owned coordinator, not elapsed time
        // or the lifecycle mutex (which a contended snapshot now releases).
        // In the regressed with_item implementation this clone occurs while
        // holding the registry guard; the store writer prevents that call from
        // returning before the unrelated registry writer below is attempted.
        let deadline = Instant::now() + Duration::from_secs(5);
        let snapshot_entered = loop {
            let entered = Arc::strong_count(&coordinator) > coordinator_refs;
            if entered || Instant::now() >= deadline {
                break entered;
            }
            std::thread::yield_now();
        };

        let (writer_done_tx, writer_done_rx) = mpsc::channel();
        let writer_thread = std::thread::spawn(move || unsafe {
            // Destroying another manager takes the same global registry's
            // write lock. It must finish while the snapshot remains parked.
            let mut result = crate::manager::platform_wallet_manager_destroy(unrelated_handle);
            writer_done_tx.send(result.code).expect("writer receiver");
            crate::platform_wallet_ffi_result_free(&mut result);
        });
        let writer_result = writer_done_rx.recv_timeout(Duration::from_secs(2));
        let (stop_done_tx, stop_done_rx) = mpsc::channel();
        let stop_thread = std::thread::spawn(move || unsafe {
            let mut result = platform_wallet_manager_shielded_sync_stop(handle);
            stop_done_tx.send(result.code).expect("stop receiver");
            crate::platform_wallet_ffi_result_free(&mut result);
        });
        let stop_result = stop_done_rx.recv_timeout(Duration::from_secs(2));

        // Always release the scan and join workers before asserting, even
        // against the broken implementation, so a regression cannot leave
        // the process-global handle registry locked for the rest of the suite.
        drop(store_guard);
        let snapshot_result = snapshot_thread.join().expect("snapshot thread");
        writer_thread.join().expect("writer thread");
        stop_thread.join().expect("stop thread");
        unsafe {
            let mut result = crate::manager::platform_wallet_manager_destroy(handle);
            crate::platform_wallet_ffi_result_free(&mut result);
        }
        drop(coordinator);
        std::fs::remove_file(path).expect("remove test store");

        assert!(
            snapshot_entered,
            "snapshot never acquired its owned coordinator"
        );
        assert_eq!(writer_result, Ok(PlatformWalletFFIResultCode::Success));
        assert_eq!(stop_result, Ok(PlatformWalletFFIResultCode::Success));
        assert_eq!(
            snapshot_result,
            (
                PlatformWalletFFIResultCode::Success,
                ShieldedLocalBalanceStatusFFI::Ready,
                1
            )
        );
    }

    fn assert_snapshot_contention_is_bounded(hold_lifecycle: bool) {
        let (handle, wallet_id, coordinator, path) = bound_manager();
        let store_guard = runtime().block_on(coordinator.store().write());
        // A fresh registration checks durable pending rows under the store
        // lock while holding lifecycle. Poll it once to park that transaction
        // before any registry mutation, then test the exported read's deadline.
        let other_wallet = [0xf3; 32];
        let mut lifecycle_holder = Box::pin(coordinator.register_wallet(
            other_wallet,
            BTreeMap::new(),
            WalletPersister::new(other_wallet, Arc::new(NoPlatformPersistence)),
        ));
        if hold_lifecycle {
            let pending = runtime().block_on(poll_fn(|cx| {
                Poll::Ready(lifecycle_holder.as_mut().poll(cx).is_pending())
            }));
            assert!(pending, "store guard must park the lifecycle holder");
        }
        let (done_tx, done_rx) = mpsc::channel();
        let snapshot_thread = std::thread::spawn(move || unsafe {
            let mut snapshot = ShieldedLocalBalanceSnapshotFFI {
                status: ShieldedLocalBalanceStatusFFI::Ready,
                accounts: std::ptr::dangling(),
                accounts_count: 99,
            };
            let mut result = platform_wallet_manager_local_shielded_balance_snapshot(
                handle,
                wallet_id.as_ptr(),
                &mut snapshot,
            );
            let message = if result.message.is_null() {
                String::new()
            } else {
                CStr::from_ptr(result.message)
                    .to_string_lossy()
                    .into_owned()
            };
            let outcome = (
                result.code,
                snapshot.status,
                snapshot.accounts.is_null(),
                snapshot.accounts_count,
                message,
            );
            platform_wallet_manager_local_shielded_balance_snapshot_free(&mut snapshot);
            crate::platform_wallet_ffi_result_free(&mut result);
            done_tx.send(outcome).expect("snapshot receiver");
        });
        // Observe completion BEFORE releasing either guard. The generous
        // watchdog tolerates slow CI without turning the lock release into
        // what lets the read complete in a regressed implementation.
        let bounded_result = done_rx.recv_timeout(Duration::from_secs(2));
        drop(lifecycle_holder);
        drop(store_guard);
        snapshot_thread.join().expect("snapshot thread");
        let (retry_code, retry_status, retry_count) = unsafe {
            let mut snapshot = ShieldedLocalBalanceSnapshotFFI::default();
            let mut result = platform_wallet_manager_local_shielded_balance_snapshot(
                handle,
                wallet_id.as_ptr(),
                &mut snapshot,
            );
            let outcome = (result.code, snapshot.status, snapshot.accounts_count);
            platform_wallet_manager_local_shielded_balance_snapshot_free(&mut snapshot);
            crate::platform_wallet_ffi_result_free(&mut result);
            let mut destroy = crate::manager::platform_wallet_manager_destroy(handle);
            crate::platform_wallet_ffi_result_free(&mut destroy);
            outcome
        };
        drop(coordinator);
        std::fs::remove_file(path).expect("remove test store");
        let (code, status, empty, count, message) =
            bounded_result.expect("FFI read must finish while locks remain held");
        assert_eq!(code, PlatformWalletFFIResultCode::ErrorWalletOperation);
        assert_eq!(status, ShieldedLocalBalanceStatusFFI::Unbound);
        assert!(empty);
        assert_eq!(count, 0);
        assert!(message.contains("snapshot busy; retry"), "{message}");
        assert_eq!(retry_code, PlatformWalletFFIResultCode::Success);
        assert_eq!(retry_status, ShieldedLocalBalanceStatusFFI::Ready);
        assert_eq!(retry_count, 1);
    }

    #[test]
    fn should_bound_snapshot_store_lock_wait_and_allow_retry() {
        assert_snapshot_contention_is_bounded(false);
    }

    #[test]
    fn should_bound_snapshot_lifecycle_lock_wait_and_allow_retry() {
        assert_snapshot_contention_is_bounded(true);
    }

    #[test]
    fn should_reject_null_local_balance_output_pointer() {
        let wallet_id = [1; 32];
        let mut result = unsafe {
            platform_wallet_manager_local_shielded_balance_snapshot(
                NULL_HANDLE,
                wallet_id.as_ptr(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        unsafe { crate::platform_wallet_ffi_result_free(&mut result) };
    }

    #[test]
    fn should_reset_local_balance_output_before_rejecting_null_wallet_id() {
        // A non-owning sentinel, not an earlier allocated snapshot: the API
        // overwrites output but must not free a value supplied by the caller.
        let mut snapshot = ShieldedLocalBalanceSnapshotFFI {
            status: ShieldedLocalBalanceStatusFFI::Ready,
            accounts: std::ptr::dangling(),
            accounts_count: 99,
        };
        let mut result = unsafe {
            platform_wallet_manager_local_shielded_balance_snapshot(
                NULL_HANDLE,
                std::ptr::null(),
                &mut snapshot,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert_eq!(snapshot.status, ShieldedLocalBalanceStatusFFI::Unbound);
        assert!(snapshot.accounts.is_null());
        assert_eq!(snapshot.accounts_count, 0);
        unsafe {
            platform_wallet_manager_local_shielded_balance_snapshot_free(&mut snapshot);
            crate::platform_wallet_ffi_result_free(&mut result);
        }
    }

    #[test]
    fn local_balance_ffi_preserves_accounts_provenance_and_explicit_zero_then_frees() {
        let ready = ShieldedLocalBalanceSnapshot {
            accounts: BTreeMap::from([
                (
                    0,
                    ShieldedLocalAccountBalance {
                        spendable_credits: 12,
                        last_scanned_index: None,
                        source: ShieldedBalanceSource::Restored,
                    },
                ),
                (
                    2,
                    ShieldedLocalAccountBalance {
                        spendable_credits: 0,
                        last_scanned_index: Some(0),
                        source: ShieldedBalanceSource::ScannedThisSession,
                    },
                ),
                (3, ShieldedLocalAccountBalance::default()),
            ]),
        };
        let mut ffi =
            ShieldedLocalBalanceSnapshotFFI::from(ShieldedLocalBalanceState::Ready(ready));
        assert_eq!(ffi.status, ShieldedLocalBalanceStatusFFI::Ready);
        assert_eq!(ffi.accounts_count, 3);
        let accounts = unsafe { std::slice::from_raw_parts(ffi.accounts, ffi.accounts_count) };
        assert_eq!(accounts[0].account_index, 0);
        assert_eq!(accounts[0].spendable_credits, 12);
        assert!(!accounts[0].has_last_scanned_index);
        assert_eq!(accounts[0].source, ShieldedBalanceSourceFFI::Restored);
        assert_eq!(accounts[1].account_index, 2);
        assert!(accounts[1].has_last_scanned_index);
        assert_eq!(accounts[1].last_scanned_index, 0);
        assert_eq!(
            accounts[1].source,
            ShieldedBalanceSourceFFI::ScannedThisSession
        );
        assert_eq!(accounts[2].source, ShieldedBalanceSourceFFI::NoHistory);
        unsafe { platform_wallet_manager_local_shielded_balance_snapshot_free(&mut ffi) };
        assert!(ffi.accounts.is_null());
        assert_eq!(ffi.accounts_count, 0);
        assert_eq!(ffi.status, ShieldedLocalBalanceStatusFFI::Unbound);
        unsafe {
            platform_wallet_manager_local_shielded_balance_snapshot_free(&mut ffi);
            platform_wallet_manager_local_shielded_balance_snapshot_free(std::ptr::null_mut());
        }
    }

    #[test]
    fn local_balance_ffi_unavailable_states_never_allocate_numeric_payloads() {
        for state in [
            ShieldedLocalBalanceState::Unbound,
            ShieldedLocalBalanceState::RestoreIncomplete,
        ] {
            let mut ffi = ShieldedLocalBalanceSnapshotFFI::from(state.clone());
            assert_eq!(
                ffi.status,
                if state == ShieldedLocalBalanceState::Unbound {
                    ShieldedLocalBalanceStatusFFI::Unbound
                } else {
                    ShieldedLocalBalanceStatusFFI::RestoreIncomplete
                }
            );
            assert!(ffi.accounts.is_null());
            assert_eq!(ffi.accounts_count, 0);
            unsafe { platform_wallet_manager_local_shielded_balance_snapshot_free(&mut ffi) };
        }
    }

    #[test]
    fn local_balance_ffi_invalid_handle_initializes_output_and_returns_error() {
        let mut ffi = ShieldedLocalBalanceSnapshotFFI::default();
        let wallet_id = [1; 32];
        let mut result = unsafe {
            platform_wallet_manager_local_shielded_balance_snapshot(
                NULL_HANDLE,
                wallet_id.as_ptr(),
                &mut ffi,
            )
        };
        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(ffi.accounts.is_null());
        assert_eq!(ffi.accounts_count, 0);
        unsafe { crate::platform_wallet_ffi_result_free(&mut result) };
    }
}
