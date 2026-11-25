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
use crate::identity_keys_from_mnemonic::resolve_seed_from_resolver;
use crate::runtime::{big_stack_spawn_failed, block_on_worker, run_on_big_stack_thread, runtime};
use crate::shielded_send::resolve_shielded_wallet;
use crate::shielded_types::{
    ShieldedBalanceSourceFFI, ShieldedLocalAccountBalanceFFI, ShieldedLocalBalanceSnapshotFFI,
    ShieldedLocalBalanceStatusFFI, ShieldedSyncWalletResultFFI,
};
use crate::types::read_identifier;
use crate::{check_ptr, unwrap_option_or_return};
use rs_sdk_ffi::MnemonicResolverHandle;

impl ShieldedSyncWalletResultFFI {
    pub(crate) fn ok(
        wallet_id: [u8; 32],
        summary: &ShieldedSyncSummary,
    ) -> Result<Self, PlatformWalletError> {
        // Multi-account on the Rust side; flattened to wallet-level
        // sums here. `balance` is the ordinary balance: identity accounts
        // are bound and scanned with the wallet, but a send from its
        // ordinary accounts cannot spend their notes, so the wallet leaves
        // them out. Hosts that want per-account detail call
        // `platform_wallet_manager_local_shielded_balance_snapshot`.
        let balance = summary.ordinary_balance_total()?;
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
/// The seed is the one of the mnemonic the resolver returns for
/// `wallet_id`, and it must be the seed the wallet was created from:
/// another is refused, nothing bound or stored, with
/// `ErrorWalletOperation` and a message naming "The supplied seed does
/// not belong to this wallet".
///
/// **Prerequisite**: the host must have already called
/// [`platform_wallet_manager_configure_shielded`] with the
/// per-network SQLite path before invoking this function — the
/// shared commitment-tree handle is opened there, not here.
/// Calling `bind_shielded` before `configure_shielded` returns
/// `ErrorWalletOperation`.
///
/// Idempotent: a second call replaces the previously-bound
/// shielded wallet on the same `wallet_id`. The wallet stays on the
/// manager's coordinator; a bind on another one is refused.
///
/// The mnemonic resolver is called on the calling thread. The persistence
/// callbacks of the bind are not: they run on the thread the bind is polled
/// on (see `PersistenceCallbacks`).
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

    // The wallet binds onto its manager's coordinator itself; this lookup
    // only refuses an unconfigured manager before the host is asked for the
    // mnemonic.
    let wallet_arc = match resolve_shielded_wallet(handle, &wallet_id) {
        Ok(wallet) => wallet,
        Err(result) => return result,
    };

    // Seedless path first: rebind from viewing keys persisted by a
    // prior seed-backed bind. `Ok(false)` means at least one
    // requested account has no persisted row — only then is the
    // mnemonic resolved. Persisted-key load/restore errors propagate without
    // resolving the seed; fallback must not mask a persistence failure.
    //
    // Both binds are polled on the big-stack thread, not the caller's, as
    // the wallet creates are (dashpay/platform#6020): the install restores
    // the wallet's private payments. The resolver between them stays on the
    // caller's thread.
    let from_persisted = run_on_big_stack_thread(|| {
        runtime().block_on(wallet_arc.bind_shielded_from_persisted(accounts.as_slice()))
    });
    match from_persisted {
        Ok(Ok(true)) => return PlatformWalletFFIResult::ok(),
        Ok(Ok(false)) => {}
        Ok(Err(e)) => {
            return map_shielded_error(e, "bind_shielded_from_persisted");
        }
        Err(e) => return big_stack_spawn_failed("shielded bind", e),
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

    let bound = run_on_big_stack_thread(|| {
        runtime().block_on(wallet_arc.bind_shielded(seed.as_ref(), accounts.as_slice()))
    });
    match bound {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return map_shielded_error(e, "bind_shielded"),
        Err(e) => return big_stack_spawn_failed("shielded bind", e),
    }

    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Identity shielded accounts
// ---------------------------------------------------------------------------

/// Post-panic guidance for exports that move no funds. Paired with
/// `ErrorWalletOperation` in [`catch_query_panic`].
const QUERY_PANIC_GUIDANCE: &str = "No funds were moved; the call may be retried.";

/// [`catch_panic_to_code`] for exports that spend nothing (identity shielded
/// account preparation). A panic there is a plain failure of the call, so it
/// maps to [`PlatformWalletFFIResultCode::ErrorWalletOperation`] and the host
/// may retry. An unwind that reaches the `extern "C"` frame aborts the
/// process before the JNI guard on the far side can translate it, so these
/// exports need the boundary as much as the spend path does.
///
/// The boundary only works where panics unwind: the Android and host/test
/// profiles. The iOS profiles (`dev-ios` / `release-ios`) build with
/// `panic = "abort"`, so on iOS a panic still ends the process before this
/// guard sees it.
pub(crate) fn catch_query_panic(
    operation: &str,
    body: impl FnOnce() -> PlatformWalletFFIResult,
) -> PlatformWalletFFIResult {
    catch_panic_to_code(
        operation,
        PlatformWalletFFIResultCode::ErrorWalletOperation,
        QUERY_PANIC_GUIDANCE,
        body,
    )
}

/// Prepare the dedicated shielded account of a wallet-owned identity and
/// write its default 43-byte raw Orchard address to `out_address`. Publishes
/// nothing and moves no funds.
///
/// Calls `PlatformWallet::prepare_shielded_identity_address`, which proves
/// the identity's slot with the seed, binds the identity's account next to
/// the wallet's bound accounts, persists its viewing key and records the
/// preparation for this session. A contact request sent afterwards carries
/// the sender's per-contact shielded address only once this preparation
/// stands. Requires a prior [`platform_wallet_manager_bind_shielded`] on this
/// manager. Repeated calls are idempotent.
///
/// The mnemonic is resolved through `mnemonic_resolver_handle` on every
/// call; it and the derived seed live in zeroizing buffers and are scrubbed
/// before this function returns.
///
/// `out_address` is zeroed on entry and written only on success. Error
/// codes a host sees:
/// - `ErrorIdentityNotFound`: the wallet holds no identity with that id.
/// - `ErrorShieldedRecoveryCorrupted` / `ErrorShieldedRecoveryKeysRequired`:
///   the bind step met unusable persisted shielded state.
/// - `ErrorWalletOperation`: every other failure, including a wallet that
///   was not bound, a seed that does not derive the identity's key, and an
///   unknown wallet. The message carries the wallet's reason.
///
/// In a build that unwinds (Android and the host tests), a panic inside the
/// call is returned as `ErrorWalletOperation`: nothing was moved and the call
/// may be retried. The iOS profiles build with `panic = "abort"`, so there a
/// panic ends the process; see [`catch_query_panic`].
///
/// # Safety
/// - `wallet_id_bytes` and `identity_id_bytes` must point at 32 readable bytes.
/// - `mnemonic_resolver_handle` must come from
///   [`crate::dash_sdk_mnemonic_resolver_create`].
/// - `out_address` must point at 43 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_prepare_shielded_identity_address(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    identity_id_bytes: *const u8,
    out_address: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(out_address);
    std::ptr::write_bytes(out_address, 0, 43);
    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(identity_id_bytes);
    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    let identity_id = match read_identifier(identity_id_bytes) {
        Ok(id) => id,
        Err(e) => return e.into(),
    };
    // Wallet, coordinator and seed resolution, the worker call and its
    // result mapping all sit inside the panic boundary: resolution blocks on
    // the runtime, which panics when the host calls from inside one. The raw
    // output write happens outside it and only on success, so a panic leaves
    // the zeroed buffer untouched.
    let mut address = [0u8; 43];
    let result = catch_query_panic("shielded identity address preparation", || {
        let wallet = match resolve_shielded_wallet(handle, &wallet_id) {
            Ok(value) => value,
            Err(e) => return e,
        };
        let seed = match resolve_seed_from_resolver(mnemonic_resolver_handle, &wallet_id) {
            Ok(seed) => seed,
            Err(e) => return e,
        };
        match block_on_worker(async move {
            wallet
                .prepare_shielded_identity_address(seed.as_ref(), &identity_id)
                .await
        }) {
            Ok(bytes) => {
                address = bytes;
                PlatformWalletFFIResult::ok()
            }
            // Typed, as the wallet reports it to every caller that names an
            // identity; everything else is a wallet-operation failure with the
            // wallet's reason, as for the bind above.
            Err(e @ PlatformWalletError::IdentityNotFound(..)) => e.into(),
            Err(e) => map_shielded_error(e, "prepare shielded identity address"),
        }
    });
    if result.code == PlatformWalletFFIResultCode::Success {
        std::ptr::copy_nonoverlapping(address.as_ptr(), out_address, 43);
    }
    result
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

    // Big-stack polling: configuring restores the held wallets' private
    // payments (see the bind above).
    let spawned = run_on_big_stack_thread(|| {
        PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
            runtime().block_on(manager.configure_shielded(&db_path))
        })
    });
    let option = match spawned {
        Ok(option) => option,
        Err(e) => return big_stack_spawn_failed("shielded configure", e),
    };
    let result = unwrap_option_or_return!(option);
    if let Err(e) = result {
        return map_shielded_error(e, "configure_shielded");
    }
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Clear shielded state (native and host)
// ---------------------------------------------------------------------------

/// Clear this manager's shielded state, native and host, in one call: stop
/// the background sync loop, drop every wallet registration on the
/// network-scoped coordinator, reset the shared commitment tree, the
/// per-subwallet notes and watermarks and the caught-up cooldown, and then
/// call the persistence vtable's `on_wipe_shielded_state_fn` for the
/// manager's network, before returning. No bind, account add, spend or sync
/// pass of this manager overlaps the reset or the host wipe. The host must
/// not delete shielded rows of its own around this call.
///
/// The SQLite commitment-tree file stays on disk but its contents are reset
/// to empty; the next [`platform_wallet_manager_bind_shielded`] call
/// repopulates the coordinator's registries and the next sync pass re-saves
/// notes via the changeset path.
///
/// Refused with nothing reset when the host did not wire and declare the
/// wipe slot (`PLATFORM_WALLET_PERSISTENCE_CAPABILITY_SHIELDED_STATE_WIPE`),
/// or when a sync pass or another holder Clear waits for does not finish
/// within the Clear budget (`ErrorShutdownIncomplete`-class result; retry).
/// A failed native reset returns an error before the host is called. A
/// failed host wipe after a successful reset returns an error and refuses
/// binds until a Clear succeeds.
///
/// Errors with `ErrorWalletOperation` when no shielded coordinator is
/// installed on this manager (the sync-loop stop still runs unconditionally
/// first). A Clear is only reachable behind a bound, shielded-enabled host
/// surface, so a missing coordinator means `configure_shielded` never ran on
/// THIS manager instance — a wiring fault that must surface (and make the host
/// fail closed) rather than report a phantom success while the on-disk tree is
/// left untouched.
///
/// Returns `ErrorShieldedOperationInProgress` (84) at once, without waiting,
/// while a shielded operation a caller started is in flight (a bind, an
/// account being added, a spend from the pool, a shield, a token pool
/// transfer or unshield, a DashPay profile write, a tip address preparation
/// or a shielded tip being sent, or another Clear). Nothing was changed: the
/// sync loop keeps running, no registration, note or tree leaf was touched,
/// and the wipe slot was not called. Call again once the operation has
/// finished.
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
        return map_clear_error(e);
    }
    PlatformWalletFFIResult::ok()
}

/// The result [`platform_wallet_manager_shielded_clear`] reports a failed
/// Clear with.
///
/// Two failures keep their own code, because a host acts on each
/// differently from a store failure. A drain that did not complete
/// (`ErrorShutdownIncomplete`) means callback-capable work may still be
/// running, so the host keeps its callback context alive instead of only
/// retrying the wipe. A Clear refused while a shielded operation is in flight
/// (`ErrorShieldedOperationInProgress`) changed nothing, the host's rows
/// included, so the host keeps everything as it was and offers the Clear
/// again. Every other failure keeps the generic mapping.
fn map_clear_error(error: PlatformWalletError) -> PlatformWalletFFIResult {
    if matches!(
        error,
        PlatformWalletError::ShutdownIncomplete(_)
            | PlatformWalletError::ShieldedOperationInProgress
    ) {
        return PlatformWalletFFIResult::from(error);
    }
    map_shielded_error(error, "clear_shielded")
}

// ---------------------------------------------------------------------------
// Ordinary shielded accounts
// ---------------------------------------------------------------------------

/// Read the ordinary (non-identity) shielded accounts bound on `wallet_id`,
/// in ascending order: the list a host passes to
/// [`platform_wallet_manager_bind_shielded`] to bind the wallet again after
/// [`platform_wallet_manager_shielded_clear`]. A Clear drops the
/// coordinator's registration, not the keys the wallet handle holds, so the
/// accounts read here are the ones bound before it. Identity accounts are
/// left out: a bind adds them itself and refuses them from a host.
///
/// On success up to `accounts_capacity` entries are written to
/// `out_accounts` and their number to `*out_count`. A wallet never bound on
/// this manager reports `*out_count = 0`. A bind accepts at most 64
/// accounts, so a 64-entry buffer always holds the list of a wallet bound
/// through this FFI. A list longer than `accounts_capacity` returns
/// `ErrorInvalidParameter` naming both numbers and writes nothing. An unknown
/// wallet returns `ErrorWalletOperation` and leaves both outputs untouched.
///
/// # Safety
/// - `wallet_id_bytes` must point at 32 readable bytes.
/// - `out_accounts` must point at `accounts_capacity` writable `u32`s.
/// - `out_count` must be writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_ordinary_accounts(
    handle: Handle,
    wallet_id_bytes: *const u8,
    out_accounts: *mut u32,
    accounts_capacity: usize,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(out_accounts);
    check_ptr!(out_count);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(async {
            match manager.get_wallet(&wallet_id).await {
                Some(wallet) => Some(wallet.shielded_ordinary_account_indices().await),
                None => None,
            }
        })
    });
    let Some(accounts) = unwrap_option_or_return!(option) else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("wallet not found: {}", hex::encode(wallet_id)),
        );
    };
    if accounts.len() > accounts_capacity {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!(
                "the wallet has {} ordinary shielded accounts, more than the buffer's capacity of {accounts_capacity}",
                accounts.len()
            ),
        );
    }
    std::ptr::copy_nonoverlapping(accounts.as_ptr(), out_accounts, accounts.len());
    *out_count = accounts.len();
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
// Fresh Orchard payment address + own-address check
// ---------------------------------------------------------------------------

/// Derive a fresh Orchard payment address for `account` on the bound
/// shielded sub-wallet of `wallet_id`, at a uniformly random diversifier
/// index (never index 0, the default address). The host receives 43 raw
/// bytes and applies its own bech32m encoding. Writes no state; notes
/// sent to the address are found by the regular scan.
///
/// `*out_present` is set to `true` and 43 bytes are written to
/// `out_bytes_43` when `account` is bound. `*out_present` is set to
/// `false` when the wallet is known but the shielded sub-wallet hasn't
/// been bound, or `account` isn't bound on it. An unknown wallet or a
/// CSPRNG failure returns `ErrorWalletOperation` and leaves both outputs
/// untouched.
///
/// # Safety
/// - `wallet_id_bytes` must point at 32 readable bytes.
/// - `out_bytes_43` must point at 43 writable bytes.
/// - `out_present` must be writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_fresh_address(
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
        RandomnessUnavailable,
        Unbound,
        Bound([u8; 43]),
    }

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(async {
            match manager.get_wallet(&wallet_id).await {
                None => Outcome::WalletMissing,
                Some(w) => match w.shielded_fresh_address(account).await {
                    Ok(Some(bytes)) => Outcome::Bound(bytes),
                    Ok(None) => Outcome::Unbound,
                    Err(_) => Outcome::RandomnessUnavailable,
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
        Outcome::RandomnessUnavailable => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "shielded fresh address: randomness unavailable",
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

/// Whether `address_bytes` is one of `account`'s own Orchard addresses on
/// the bound shielded sub-wallet of `wallet_id`, at any diversifier
/// index — the same test the scan classifies sends by.
///
/// `*out_present` is set to `true` and `*out_is_own` to the answer when
/// `account` is bound; an `address_len` other than 43 or an off-curve
/// address answers `false`, never an error. `*out_present` is set to
/// `false` when the wallet is known but the shielded sub-wallet hasn't
/// been bound, or `account` isn't bound on it. An unknown wallet returns
/// `ErrorWalletOperation`.
///
/// # Safety
/// - `wallet_id_bytes` must point at 32 readable bytes.
/// - `address_bytes` must point at `address_len` readable bytes.
/// - `out_is_own` and `out_present` must be writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_is_own_shielded_address(
    handle: Handle,
    wallet_id_bytes: *const u8,
    account: u32,
    address_bytes: *const u8,
    address_len: usize,
    out_is_own: *mut bool,
    out_present: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(address_bytes);
    check_ptr!(out_is_own);
    check_ptr!(out_present);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    let address = std::slice::from_raw_parts(address_bytes, address_len);

    enum Outcome {
        WalletMissing,
        Unbound,
        Bound(bool),
    }

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(async {
            match manager.get_wallet(&wallet_id).await {
                None => Outcome::WalletMissing,
                Some(w) => match w.is_own_shielded_address(account, address).await {
                    Some(is_own) => Outcome::Bound(is_own),
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
        Outcome::Bound(is_own) => {
            *out_is_own = is_own;
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
/// state change, and so does a pass whose scan a concurrent
/// [`platform_wallet_manager_shielded_sync_stop`] cut short: it
/// published nothing, and the next pass re-walks from the last
/// completed watermark.
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
pub(crate) fn map_shielded_error(
    error: PlatformWalletError,
    operation: &str,
) -> PlatformWalletFFIResult {
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
mod sync_result_tests {
    use super::*;
    use platform_wallet::shielded_identity_account_index;
    use std::collections::BTreeMap;

    #[test]
    fn should_report_only_ordinary_accounts_in_the_sync_result_balance() {
        let identity_account = shielded_identity_account_index(0).expect("identity account");
        let summary = ShieldedSyncSummary {
            balances: BTreeMap::from([(0, 100), (1, 50), (identity_account, 900)]),
            ..ShieldedSyncSummary::default()
        };
        let result = ShieldedSyncWalletResultFFI::ok([7; 32], &summary).expect("result");
        assert!(result.success);
        assert_eq!(result.balance, 150);
    }

    #[test]
    fn should_report_zero_for_a_wallet_holding_only_identity_account_notes() {
        let identity_account = shielded_identity_account_index(3).expect("identity account");
        let summary = ShieldedSyncSummary {
            balances: BTreeMap::from([(identity_account, u64::MAX)]),
            ..ShieldedSyncSummary::default()
        };
        let result = ShieldedSyncWalletResultFFI::ok([7; 32], &summary).expect("result");
        assert_eq!(result.balance, 0);
    }
}

#[cfg(test)]
mod recovery_error_tests {
    use super::*;

    /// The exported Clear reports a refusal during a shielded operation and
    /// an incomplete drain by their own codes, and any other failure as a
    /// wallet operation failure. Flattening the refusal to the generic code
    /// would lose the typed retry outcome in Swift and Kotlin.
    #[test]
    fn should_keep_the_typed_codes_of_a_refused_and_an_undrained_clear() {
        for (error, expected, raw) in [
            (
                PlatformWalletError::ShieldedOperationInProgress,
                PlatformWalletFFIResultCode::ErrorShieldedOperationInProgress,
                84,
            ),
            (
                PlatformWalletError::ShutdownIncomplete("a pass did not drain".into()),
                PlatformWalletFFIResultCode::ErrorShutdownIncomplete,
                27,
            ),
            (
                PlatformWalletError::ShieldedStoreError("purge failed".into()),
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                6,
            ),
        ] {
            let mut result = map_clear_error(error);
            assert_eq!(result.code, expected);
            assert_eq!(result.code as u32, raw);
            unsafe { platform_wallet_ffi_result_free(&mut result) };
        }
    }

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
    use crate::test_fixtures::mock_manager;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use platform_wallet::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use platform_wallet::wallet::shielded::{
        OrchardKeySet, ShieldedLocalAccountBalance, ShieldedLocalBalanceSnapshot,
    };
    use std::collections::BTreeMap;
    use std::future::{poll_fn, Future};
    use std::sync::{mpsc, Arc};
    use std::task::Poll;
    use std::time::Instant;

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

#[cfg(test)]
mod fresh_address_tests {
    use super::*;
    use crate::test_fixtures::mock_manager;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;

    /// A manager with one known wallet whose shielded sub-wallet is not bound.
    fn unbound_manager() -> (Handle, [u8; 32]) {
        let manager = mock_manager();
        let wallet_id = runtime().block_on(async {
            manager
                .create_wallet_from_seed_bytes(
                    dashcore::Network::Testnet,
                    &[42; 64],
                    WalletAccountCreationOptions::Default,
                    Some(0),
                )
                .await
                .expect("create mock wallet")
                .wallet_id()
        });
        (PLATFORM_WALLET_MANAGER_STORAGE.insert(manager), wallet_id)
    }

    /// A manager with one wallet whose shielded account 0 is bound from
    /// its seed, plus the store path to clean up.
    fn bound_manager() -> (Handle, [u8; 32], std::path::PathBuf) {
        unsafe extern "C" fn begin(_: *mut std::ffi::c_void, _: *const u8) -> i32 {
            0
        }
        unsafe extern "C" fn end(_: *mut std::ffi::c_void, _: *const u8, _: bool) -> i32 {
            0
        }
        // A seed-backed bind persists the viewing keys, so the persister
        // must declare it can hold them atomically.
        let manager = crate::test_fixtures::mock_manager_with(
            crate::persistence::FFIPersister::new_with_persistence_capabilities(
                crate::persistence::PersistenceCallbacks {
                    on_changeset_begin_fn: Some(begin),
                    on_changeset_end_fn: Some(end),
                    on_persist_shielded_viewing_keys_fn: Some(
                        crate::test_fixtures::noop_persist_viewing_keys,
                    ),
                    on_load_shielded_viewing_keys_fn: Some(
                        crate::test_fixtures::noop_load_viewing_keys,
                    ),
                    on_load_shielded_viewing_keys_free_fn: Some(
                        crate::test_fixtures::noop_free_viewing_keys,
                    ),
                    ..Default::default()
                },
                platform_wallet::changeset::PersistenceCapabilities::ATOMIC_CHANGESETS.union(
                    platform_wallet::changeset::PersistenceCapabilities::SHIELDED_VIEWING_KEYS,
                ),
            ),
        );
        let path = std::env::temp_dir().join(format!(
            "ffi-shielded-fresh-address-{}-{}.sqlite",
            std::process::id(),
            next_handle()
        ));
        let wallet_id = runtime().block_on(async {
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
            wallet
                .bind_shielded(&[42; 64], &[0])
                .await
                .expect("bind shielded");
            wallet.wallet_id()
        });
        (
            PLATFORM_WALLET_MANAGER_STORAGE.insert(manager),
            wallet_id,
            path,
        )
    }

    unsafe fn fresh_address(handle: Handle, wallet_id: &[u8; 32]) -> [u8; 43] {
        let mut out = [0u8; 43];
        let mut present = false;
        let mut r = platform_wallet_manager_shielded_fresh_address(
            handle,
            wallet_id.as_ptr(),
            0,
            out.as_mut_ptr(),
            &mut present,
        );
        assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
        assert!(present, "bound account has a fresh address");
        crate::platform_wallet_ffi_result_free(&mut r);
        out
    }

    unsafe fn is_own(handle: Handle, wallet_id: &[u8; 32], address: &[u8]) -> bool {
        let mut is_own = false;
        let mut present = false;
        let mut r = platform_wallet_manager_is_own_shielded_address(
            handle,
            wallet_id.as_ptr(),
            0,
            address.as_ptr(),
            address.len(),
            &mut is_own,
            &mut present,
        );
        assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
        assert!(present, "bound account can answer");
        crate::platform_wallet_ffi_result_free(&mut r);
        is_own
    }

    #[test]
    fn fresh_address_writes_a_new_address_on_a_bound_wallet() {
        let (handle, wallet_id, path) = bound_manager();
        unsafe {
            let mut default = [0u8; 43];
            let mut present = false;
            let mut r = platform_wallet_manager_shielded_default_address(
                handle,
                wallet_id.as_ptr(),
                0,
                default.as_mut_ptr(),
                &mut present,
            );
            assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
            assert!(present);
            crate::platform_wallet_ffi_result_free(&mut r);

            let first = fresh_address(handle, &wallet_id);
            let second = fresh_address(handle, &wallet_id);
            assert_ne!(first, default, "a fresh address is never the default");
            assert_ne!(first, second, "each call draws a new address");
            assert!(
                is_own(handle, &wallet_id, &first),
                "the 43 bytes decode to one of the wallet's own addresses"
            );
            assert_eq!(
                crate::platform_wallet_manager_destroy(handle).code,
                PlatformWalletFFIResultCode::Success
            );
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn is_own_answers_for_own_and_foreign_addresses_on_a_bound_wallet() {
        let (handle, wallet_id, path) = bound_manager();
        let foreign = platform_wallet::wallet::shielded::OrchardKeySet::from_seed(
            &[9; 64],
            dashcore::Network::Testnet,
            0,
        )
        .expect("foreign keys")
        .default_address
        .to_raw_address_bytes();
        unsafe {
            let fresh = fresh_address(handle, &wallet_id);
            assert!(is_own(handle, &wallet_id, &fresh), "own fresh address");
            assert!(!is_own(handle, &wallet_id, &foreign), "foreign address");
            let mut long = [0u8; 44];
            long[..43].copy_from_slice(&fresh);
            for len in [0, 42, 44] {
                assert!(
                    !is_own(handle, &wallet_id, &long[..len]),
                    "{len} bytes is malformed, so not own"
                );
            }
            assert_eq!(
                crate::platform_wallet_manager_destroy(handle).code,
                PlatformWalletFFIResultCode::Success
            );
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn fresh_address_reports_unbound_unknown_and_null() {
        let (handle, wallet_id) = unbound_manager();
        let mut out = [0xAAu8; 43];
        let mut present = true;
        unsafe {
            let mut r = platform_wallet_manager_shielded_fresh_address(
                handle,
                wallet_id.as_ptr(),
                0,
                out.as_mut_ptr(),
                &mut present,
            );
            assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
            assert!(!present, "unbound wallet has no fresh address");
            assert_eq!(out, [0xAAu8; 43], "buffer untouched when not present");
            crate::platform_wallet_ffi_result_free(&mut r);

            present = true;
            let mut r = platform_wallet_manager_shielded_fresh_address(
                handle,
                [7u8; 32].as_ptr(),
                0,
                out.as_mut_ptr(),
                &mut present,
            );
            assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorWalletOperation);
            assert!(present, "out_present untouched on error");
            crate::platform_wallet_ffi_result_free(&mut r);

            let mut r = platform_wallet_manager_shielded_fresh_address(
                handle,
                wallet_id.as_ptr(),
                0,
                std::ptr::null_mut(),
                &mut present,
            );
            assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            crate::platform_wallet_ffi_result_free(&mut r);
        }
    }

    #[test]
    fn is_own_reports_unbound_unknown_and_null() {
        let (handle, wallet_id) = unbound_manager();
        let address = [0u8; 43];
        let mut is_own = true;
        let mut present = true;
        unsafe {
            let mut r = platform_wallet_manager_is_own_shielded_address(
                handle,
                wallet_id.as_ptr(),
                0,
                address.as_ptr(),
                address.len(),
                &mut is_own,
                &mut present,
            );
            assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
            assert!(!present, "unbound wallet cannot answer");
            crate::platform_wallet_ffi_result_free(&mut r);

            present = true;
            let mut r = platform_wallet_manager_is_own_shielded_address(
                handle,
                [7u8; 32].as_ptr(),
                0,
                address.as_ptr(),
                address.len(),
                &mut is_own,
                &mut present,
            );
            assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorWalletOperation);
            assert!(present, "out_present untouched on error");
            crate::platform_wallet_ffi_result_free(&mut r);

            let mut r = platform_wallet_manager_is_own_shielded_address(
                handle,
                wallet_id.as_ptr(),
                0,
                std::ptr::null(),
                43,
                &mut is_own,
                &mut present,
            );
            assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            crate::platform_wallet_ffi_result_free(&mut r);
        }
    }
}

#[cfg(test)]
mod clear_tests {
    use super::*;
    use crate::persistence::{
        FFIPersister, PersistenceCallbacks, PersistenceExtensionCallbacks, WipeShieldedStateFn,
    };
    use crate::test_fixtures::mock_manager_with;
    use crate::types::FFINetwork;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use platform_wallet::changeset::PersistenceCapabilities;
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
    use std::sync::Mutex;

    /// The wipe callback's observations. One static per test process, so the
    /// tests below serialize on `SERIAL`.
    static WIPES: AtomicUsize = AtomicUsize::new(0);
    static WIPED_NETWORK: AtomicI32 = AtomicI32::new(-1);
    static WIPE_RC: AtomicI32 = AtomicI32::new(0);
    static SERIAL: Mutex<()> = Mutex::new(());

    unsafe extern "C" fn wipe(_: *mut c_void, network: FFINetwork) -> i32 {
        WIPES.fetch_add(1, Ordering::SeqCst);
        WIPED_NETWORK.store(network as i32, Ordering::SeqCst);
        WIPE_RC.load(Ordering::SeqCst)
    }

    unsafe extern "C" fn begin(_: *mut c_void, _: *const u8) -> i32 {
        0
    }

    unsafe extern "C" fn end(_: *mut c_void, _: *const u8, _: bool) -> i32 {
        0
    }

    /// A configured manager over a persister declaring the wipe capability,
    /// with the wipe slot wired when `wired`.
    fn configured(wired: bool) -> (Handle, PathBuf) {
        let extensions = PersistenceExtensionCallbacks {
            wipe_shielded_state: wired.then_some(wipe as WipeShieldedStateFn),
            ..Default::default()
        };
        let persister = FFIPersister::new_with_persistence_capabilities_and_extensions(
            PersistenceCallbacks {
                on_changeset_begin_fn: Some(begin),
                on_changeset_end_fn: Some(end),
                ..Default::default()
            },
            PersistenceCapabilities::ATOMIC_CHANGESETS
                .union(PersistenceCapabilities::SHIELDED_STATE_WIPE),
            extensions,
        );
        let manager = mock_manager_with(persister);
        let path = std::env::temp_dir().join(format!(
            "ffi-shielded-clear-{}-{}.sqlite",
            std::process::id(),
            next_handle()
        ));
        runtime().block_on(async {
            manager
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
        });
        (PLATFORM_WALLET_MANAGER_STORAGE.insert(manager), path)
    }

    fn clear_epoch(handle: Handle) -> u64 {
        PLATFORM_WALLET_MANAGER_STORAGE
            .with_item(handle, |manager| {
                runtime().block_on(async {
                    manager
                        .shielded_coordinator()
                        .await
                        .expect("configured")
                        .clear_epoch()
                        .await
                })
            })
            .expect("live handle")
    }

    fn finish(handle: Handle, path: PathBuf) {
        drop(PLATFORM_WALLET_MANAGER_STORAGE.remove(handle));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn should_call_the_wipe_once_with_the_managers_network_before_clear_returns() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        WIPES.store(0, Ordering::SeqCst);
        WIPE_RC.store(0, Ordering::SeqCst);
        let (handle, path) = configured(true);

        let result = unsafe { platform_wallet_manager_shielded_clear(handle) };

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(WIPES.load(Ordering::SeqCst), 1);
        assert_eq!(
            WIPED_NETWORK.load(Ordering::SeqCst),
            FFINetwork::Testnet as i32
        );
        finish(handle, path);
    }

    #[test]
    fn should_refuse_clear_and_reset_nothing_without_the_wipe_slot() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        WIPES.store(0, Ordering::SeqCst);
        let (handle, path) = configured(false);
        let epoch = clear_epoch(handle);

        let result = unsafe { platform_wallet_manager_shielded_clear(handle) };

        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(WIPES.load(Ordering::SeqCst), 0);
        assert_eq!(clear_epoch(handle), epoch, "nothing was reset");
        finish(handle, path);
    }

    #[test]
    fn should_fail_clear_when_the_host_wipe_fails() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        WIPES.store(0, Ordering::SeqCst);
        WIPE_RC.store(-1, Ordering::SeqCst);
        let (handle, path) = configured(true);

        let result = unsafe { platform_wallet_manager_shielded_clear(handle) };

        WIPE_RC.store(0, Ordering::SeqCst);
        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(WIPES.load(Ordering::SeqCst), 1);
        let retried = unsafe { platform_wallet_manager_shielded_clear(handle) };
        assert_eq!(retried.code, PlatformWalletFFIResultCode::Success);
        finish(handle, path);
    }
}

#[cfg(test)]
mod identity_account_tests {
    use super::*;
    use crate::manager::platform_wallet_manager_destroy;
    use crate::persistence::{
        FFIPersister, PersistenceCallbacks, PersistenceExtensionCallbacks, WipeShieldedStateFn,
    };
    use crate::shielded_tip::{
        platform_wallet_manager_shielded_identity_balance, ShieldedIdentityBalanceFFI,
        ShieldedIdentityBalanceStateFFI,
    };
    use crate::test_fixtures::{
        mock_manager, mock_manager_with, noop_free_viewing_keys, noop_load_viewing_keys,
        noop_persist_viewing_keys,
    };
    use crate::types::FFINetwork;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{Identity, IdentityPublicKey, IdentityV0, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::prelude::Identifier;
    use key_wallet::bip32::ExtendedPrivKey;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Mnemonic;
    use platform_wallet::changeset::PersistenceCapabilities;
    use platform_wallet::wallet::identity::network::derive_ecdsa_identity_auth_keypair_from_master;
    use platform_wallet::PlatformWallet;
    use platform_wallet::{shielded_identity_account_index, MASTER_KEY_INDEX};
    use rs_sdk_ffi::{mnemonic_resolver_result, MnemonicResolverVTable};
    use std::collections::BTreeMap;
    use std::ffi::c_void;
    use std::sync::Arc;

    /// The phrase the test resolver answers for every wallet id; the wallet
    /// under test is created from it, so the resolved seed belongs to it.
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon \
                                 abandon abandon abandon abandon about";

    unsafe extern "C" fn resolve_test_mnemonic(
        _ctx: *const c_void,
        _wallet_id: *const u8,
        out: *mut c_char,
        cap: usize,
        out_len: *mut usize,
    ) -> i32 {
        let phrase = TEST_MNEMONIC.as_bytes();
        assert!(cap >= phrase.len());
        std::ptr::copy_nonoverlapping(phrase.as_ptr(), out.cast::<u8>(), phrase.len());
        *out_len = phrase.len();
        mnemonic_resolver_result::SUCCESS
    }

    unsafe extern "C" fn destroy_test_resolver(_ctx: *mut c_void) {}

    /// A resolver over [`TEST_MNEMONIC`]. The vtable is boxed so the handle
    /// can point at it for as long as the pair lives.
    fn test_resolver() -> (Box<MnemonicResolverVTable>, MnemonicResolverHandle) {
        let mut vtable = Box::new(MnemonicResolverVTable {
            resolve: resolve_test_mnemonic,
            destroy: destroy_test_resolver,
        });
        let handle = MnemonicResolverHandle {
            ctx: std::ptr::null_mut(),
            vtable: &mut *vtable,
        };
        (vtable, handle)
    }

    fn message_of(result: &PlatformWalletFFIResult) -> String {
        assert!(!result.message.is_null(), "error result carries a message");
        unsafe { CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned()
    }

    /// A manager with a configured shielded store and one wallet created
    /// from [`TEST_MNEMONIC`] whose shielded sub-wallet is not bound.
    /// Returns the store path to clean up.
    fn configured_manager() -> (Handle, [u8; 32], std::path::PathBuf) {
        unsafe extern "C" fn begin(_: *mut c_void, _: *const u8) -> i32 {
            0
        }
        unsafe extern "C" fn end(_: *mut c_void, _: *const u8, _: bool) -> i32 {
            0
        }
        unsafe extern "C" fn wipe(_: *mut c_void, _: FFINetwork) -> i32 {
            0
        }
        // A seed-backed bind persists the viewing keys, so the persister
        // declares it can hold them atomically. A Clear wipes the host's
        // rows before it returns, so it declares and wires that too.
        let manager = mock_manager_with(
            FFIPersister::new_with_persistence_capabilities_and_extensions(
                PersistenceCallbacks {
                    on_changeset_begin_fn: Some(begin),
                    on_changeset_end_fn: Some(end),
                    on_persist_shielded_viewing_keys_fn: Some(noop_persist_viewing_keys),
                    on_load_shielded_viewing_keys_fn: Some(noop_load_viewing_keys),
                    on_load_shielded_viewing_keys_free_fn: Some(noop_free_viewing_keys),
                    ..Default::default()
                },
                PersistenceCapabilities::ATOMIC_CHANGESETS
                    .union(PersistenceCapabilities::SHIELDED_VIEWING_KEYS)
                    .union(PersistenceCapabilities::SHIELDED_STATE_WIPE),
                PersistenceExtensionCallbacks {
                    wipe_shielded_state: Some(wipe as WipeShieldedStateFn),
                    ..Default::default()
                },
            ),
        );
        let path = std::env::temp_dir().join(format!(
            "ffi-shielded-identity-account-{}-{}.sqlite",
            std::process::id(),
            next_handle()
        ));
        let wallet_id = runtime().block_on(async {
            manager
                .configure_shielded(&path)
                .await
                .expect("configure shielded");
            manager
                .create_wallet_from_mnemonic(
                    TEST_MNEMONIC,
                    dashcore::Network::Testnet,
                    WalletAccountCreationOptions::Default,
                    Some(0),
                )
                .await
                .expect("create wallet")
                .wallet_id()
        });
        (
            PLATFORM_WALLET_MANAGER_STORAGE.insert(manager),
            wallet_id,
            path,
        )
    }

    /// [`configured_manager`], bound on account 0 through the exported bind.
    fn bound_manager() -> (Handle, [u8; 32], std::path::PathBuf) {
        let (handle, wallet_id, path) = configured_manager();
        let (_vtable, mut resolver) = test_resolver();
        let accounts = [0u32];
        let mut r = unsafe {
            platform_wallet_manager_bind_shielded(
                handle,
                wallet_id.as_ptr(),
                &mut resolver,
                accounts.as_ptr(),
                accounts.len(),
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
        unsafe { platform_wallet_ffi_result_free(&mut r) };
        (handle, wallet_id, path)
    }

    fn destroy_manager(handle: Handle, path: std::path::PathBuf) {
        let mut destroyed = unsafe { platform_wallet_manager_destroy(handle) };
        assert_eq!(destroyed.code, PlatformWalletFFIResultCode::Success);
        unsafe { platform_wallet_ffi_result_free(&mut destroyed) };
        let _ = std::fs::remove_file(path);
    }

    /// Resolution blocks on the runtime, which panics when the host calls
    /// from inside one. The export contains it as a retryable failure and no
    /// address escapes.
    #[test]
    fn should_contain_a_preparation_panic_inside_a_runtime() {
        let (handle, wallet_id, path) = bound_manager();
        let (_vtable, mut resolver) = test_resolver();
        let identity_id = [0x21u8; 32];
        let mut out_address = [0xAAu8; 43];
        let mut r = runtime().block_on(async {
            unsafe {
                platform_wallet_manager_prepare_shielded_identity_address(
                    handle,
                    wallet_id.as_ptr(),
                    &mut resolver,
                    identity_id.as_ptr(),
                    out_address.as_mut_ptr(),
                )
            }
        });
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorWalletOperation);
        let message = message_of(&r);
        assert!(
            message.contains("shielded identity address preparation panicked")
                && message.contains("may be retried"),
            "{message}"
        );
        assert_eq!(out_address, [0u8; 43]);
        unsafe { platform_wallet_ffi_result_free(&mut r) };
        destroy_manager(handle, path);
    }

    /// The export reaches the wallet with the resolved seed and returns the
    /// wallet's typed refusal for an identity it does not hold, with the
    /// output left zeroed.
    #[test]
    fn should_refuse_preparing_an_identity_the_wallet_does_not_hold() {
        let (handle, wallet_id, path) = bound_manager();
        let (_vtable, mut resolver) = test_resolver();
        let identity_id = [0x21u8; 32];
        let mut out_address = [0xAAu8; 43];
        let mut r = unsafe {
            platform_wallet_manager_prepare_shielded_identity_address(
                handle,
                wallet_id.as_ptr(),
                &mut resolver,
                identity_id.as_ptr(),
                out_address.as_mut_ptr(),
            )
        };
        assert_eq!(
            r.code,
            PlatformWalletFFIResultCode::ErrorIdentityNotFound,
            "{}",
            message_of(&r)
        );
        assert_eq!(out_address, [0u8; 43]);
        unsafe { platform_wallet_ffi_result_free(&mut r) };
        destroy_manager(handle, path);
    }

    /// The identity whose MASTER authentication key is the one
    /// [`TEST_MNEMONIC`] derives at `identity_index`, which is what the
    /// seed proof checks.
    fn identity_at_slot(id: Identifier, identity_index: u32) -> Identity {
        let seed = Mnemonic::from_phrase(TEST_MNEMONIC)
            .expect("test mnemonic")
            .to_seed("");
        let master = ExtendedPrivKey::new_master(key_wallet::Network::Testnet, &seed[..])
            .expect("master key");
        let public_key = derive_ecdsa_identity_auth_keypair_from_master(
            &master,
            key_wallet::Network::Testnet,
            identity_index,
            MASTER_KEY_INDEX,
        )
        .expect("derive the master key")
        .public_key;
        Identity::V0(IdentityV0 {
            id,
            public_keys: BTreeMap::from([(
                0,
                IdentityPublicKey::V0(IdentityPublicKeyV0 {
                    id: 0,
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::MASTER,
                    contract_bounds: None,
                    key_type: KeyType::ECDSA_SECP256K1,
                    read_only: false,
                    data: BinaryData::new(public_key.to_vec()),
                    disabled_at: None,
                }),
            )]),
            balance: 0,
            revision: 0,
        })
    }

    /// Adds the identity `id` at `identity_index` to the manager's wallet,
    /// carrying the MASTER key the seed derives there, and returns the wallet.
    fn add_identity_at_slot(
        handle: Handle,
        wallet_id: [u8; 32],
        id: Identifier,
        identity_index: u32,
    ) -> Arc<PlatformWallet> {
        let wallet = PLATFORM_WALLET_MANAGER_STORAGE
            .with_item(handle, |manager| manager.get_wallet_blocking(&wallet_id))
            .flatten()
            .expect("the wallet");
        let identity = identity_at_slot(id, identity_index);
        runtime().block_on(async {
            let mut wm = wallet.wallet_manager().write().await;
            wm.get_wallet_info_mut(&wallet_id)
                .expect("the wallet's info")
                .identity_manager
                .add_identity(identity, identity_index, wallet_id, wallet.persister())
                .expect("add the identity");
        });
        wallet
    }

    /// Before the shielded store is configured the wallet still answers the
    /// identity balance read: `Unverified` for an identity it holds, and
    /// `ErrorIdentityNotFound` for one it does not. The export resolves the
    /// wallet alone and leaves both answers to it.
    #[test]
    fn should_let_the_wallet_answer_an_identity_balance_before_shielded_is_configured() {
        let manager = mock_manager();
        let wallet_id = runtime().block_on(async {
            manager
                .create_wallet_from_mnemonic(
                    TEST_MNEMONIC,
                    dashcore::Network::Testnet,
                    WalletAccountCreationOptions::Default,
                    Some(0),
                )
                .await
                .expect("create wallet")
                .wallet_id()
        });
        let handle = PLATFORM_WALLET_MANAGER_STORAGE.insert(manager);
        let held = Identifier::from([0x31; 32]);
        add_identity_at_slot(handle, wallet_id, held, 3);

        let read = |id: Identifier| {
            let mut balance = ShieldedIdentityBalanceFFI {
                state: ShieldedIdentityBalanceStateFFI::Available,
                account: 9,
                credits: 9,
            };
            let mut r = unsafe {
                platform_wallet_manager_shielded_identity_balance(
                    handle,
                    wallet_id.as_ptr(),
                    id.as_bytes().as_ptr(),
                    &mut balance,
                )
            };
            let code = r.code;
            unsafe { platform_wallet_ffi_result_free(&mut r) };
            (code, balance)
        };

        assert_eq!(
            read(held),
            (
                PlatformWalletFFIResultCode::Success,
                ShieldedIdentityBalanceFFI::default()
            ),
            "an identity the wallet holds is unverified, not an error"
        );
        let (unknown, balance) = read(Identifier::from([0x32; 32]));
        assert_eq!(unknown, PlatformWalletFFIResultCode::ErrorIdentityNotFound);
        assert_eq!(balance, ShieldedIdentityBalanceFFI::default());

        let mut destroyed = unsafe { platform_wallet_manager_destroy(handle) };
        assert_eq!(destroyed.code, PlatformWalletFFIResultCode::Success);
        unsafe { platform_wallet_ffi_result_free(&mut destroyed) };
    }

    /// Calls the export for `id` with a pre-filled buffer and returns the
    /// result code, its message and the buffer.
    fn prepare(
        handle: Handle,
        wallet_id: [u8; 32],
        id: Identifier,
    ) -> (PlatformWalletFFIResultCode, Option<String>, [u8; 43]) {
        let (_vtable, mut resolver) = test_resolver();
        let mut out_address = [0xAAu8; 43];
        let mut r = unsafe {
            platform_wallet_manager_prepare_shielded_identity_address(
                handle,
                wallet_id.as_ptr(),
                &mut resolver,
                id.as_bytes().as_ptr(),
                out_address.as_mut_ptr(),
            )
        };
        let message = (!r.message.is_null()).then(|| message_of(&r));
        let code = r.code;
        unsafe { platform_wallet_ffi_result_free(&mut r) };
        (code, message, out_address)
    }

    /// Preparing through the export binds the identity's account next to
    /// the ordinary one, returns that account's default address and records
    /// the preparation.
    #[test]
    fn should_prepare_the_identity_account_and_bind_it() {
        const IDENTITY_INDEX: u32 = 2;
        let (handle, wallet_id, path) = bound_manager();
        let id = Identifier::from([0x42; 32]);
        let wallet = add_identity_at_slot(handle, wallet_id, id, IDENTITY_INDEX);
        let account = shielded_identity_account_index(IDENTITY_INDEX).expect("identity account");
        let (code, message, out_address) = prepare(handle, wallet_id, id);
        assert_eq!(code, PlatformWalletFFIResultCode::Success, "{message:?}");
        let (default_address, prepared, bound) = runtime().block_on(async {
            (
                wallet.shielded_default_address(account).await,
                wallet.prepared_shielded_identity_account(&id).await,
                wallet.shielded_account_indices().await,
            )
        });
        assert_eq!(default_address, Some(out_address));
        assert_eq!(prepared, Some(account));
        assert_eq!(bound, vec![0, account]);
        drop(wallet);
        destroy_manager(handle, path);
    }

    #[test]
    fn should_return_the_same_address_when_prepared_twice() {
        const IDENTITY_INDEX: u32 = 4;
        let (handle, wallet_id, path) = bound_manager();
        let id = Identifier::from([0x44; 32]);
        let wallet = add_identity_at_slot(handle, wallet_id, id, IDENTITY_INDEX);
        let account = shielded_identity_account_index(IDENTITY_INDEX).expect("identity account");
        let (first_code, first_message, first) = prepare(handle, wallet_id, id);
        assert_eq!(
            first_code,
            PlatformWalletFFIResultCode::Success,
            "{first_message:?}"
        );
        let (second_code, second_message, second) = prepare(handle, wallet_id, id);
        assert_eq!(
            second_code,
            PlatformWalletFFIResultCode::Success,
            "{second_message:?}"
        );
        assert_eq!(first, second);
        let (prepared, bound) = runtime().block_on(async {
            (
                wallet.prepared_shielded_identity_account(&id).await,
                wallet.shielded_account_indices().await,
            )
        });
        assert_eq!(prepared, Some(account));
        assert_eq!(bound, vec![0, account]);
        drop(wallet);
        destroy_manager(handle, path);
    }

    /// The store is configured and the identity proves its slot, so the
    /// call reaches the wallet, which refuses it because the wallet's
    /// shielded sub-wallet was never bound (`ShieldedNotBound`). That reaches
    /// the host as a wallet-operation failure, not as an unknown error.
    #[test]
    fn should_refuse_preparation_on_an_unbound_wallet() {
        let (handle, wallet_id, path) = configured_manager();
        let id = Identifier::from([0x43; 32]);
        let wallet = add_identity_at_slot(handle, wallet_id, id, 3);
        let (code, message, out_address) = prepare(handle, wallet_id, id);
        assert_eq!(
            code,
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "{message:?}"
        );
        let message = message.expect("an error message");
        assert!(
            message.starts_with("prepare shielded identity address failed:")
                && message.contains("Shielded sub-wallet not bound"),
            "{message}"
        );
        assert_eq!(out_address, [0u8; 43]);
        let prepared = runtime().block_on(wallet.prepared_shielded_identity_account(&id));
        assert_eq!(prepared, None);
        drop(wallet);
        destroy_manager(handle, path);
    }

    #[test]
    fn should_map_a_query_panic_to_a_retryable_wallet_operation() {
        let mut result = catch_query_panic("shielded identity address preparation", || {
            panic!("tokio worker panicked");
        });
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        let message = message_of(&result);
        assert!(
            message.contains("shielded identity address preparation panicked")
                && message.contains("tokio worker panicked")
                && message.contains("may be retried"),
            "{message}"
        );
        unsafe { platform_wallet_ffi_result_free(&mut result) };
    }

    #[test]
    fn should_pass_a_query_result_through_when_nothing_panics() {
        let result = catch_query_panic(
            "shielded identity address preparation",
            PlatformWalletFFIResult::ok,
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(result.message.is_null());
    }

    #[test]
    fn should_zero_the_address_and_refuse_an_invalid_manager_handle() {
        let (_vtable, mut resolver) = test_resolver();
        let wallet_id = [0x11u8; 32];
        let identity_id = [0x21u8; 32];
        let mut out_address = [0xAAu8; 43];
        let mut r = unsafe {
            platform_wallet_manager_prepare_shielded_identity_address(
                NULL_HANDLE,
                wallet_id.as_ptr(),
                &mut resolver,
                identity_id.as_ptr(),
                out_address.as_mut_ptr(),
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        assert_eq!(out_address, [0u8; 43]);
        unsafe { platform_wallet_ffi_result_free(&mut r) };
    }

    #[test]
    fn should_zero_the_address_before_rejecting_a_null_argument() {
        let identity_id = [0x21u8; 32];
        let mut out_address = [0xAAu8; 43];
        let mut r = unsafe {
            platform_wallet_manager_prepare_shielded_identity_address(
                NULL_HANDLE,
                std::ptr::null(),
                std::ptr::null_mut(),
                identity_id.as_ptr(),
                out_address.as_mut_ptr(),
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert_eq!(out_address, [0u8; 43]);
        unsafe { platform_wallet_ffi_result_free(&mut r) };
    }

    /// Bind `accounts` on `wallet_id` through the exported bind.
    fn bind_through_the_export(handle: Handle, wallet_id: &[u8; 32], accounts: &[u32]) {
        let (_vtable, mut resolver) = test_resolver();
        let mut r = unsafe {
            platform_wallet_manager_bind_shielded(
                handle,
                wallet_id.as_ptr(),
                &mut resolver,
                accounts.as_ptr(),
                accounts.len(),
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
        unsafe { platform_wallet_ffi_result_free(&mut r) };
    }

    /// Read the ordinary accounts into a 64-entry buffer, the size a host
    /// passes, prefilled with a sentinel so untouched entries show.
    fn ordinary_accounts(
        handle: Handle,
        wallet_id: &[u8; 32],
        capacity: usize,
    ) -> (PlatformWalletFFIResult, [u32; 64], usize) {
        let mut out_accounts = [u32::MAX; 64];
        let mut out_count = usize::MAX;
        let r = unsafe {
            platform_wallet_manager_shielded_ordinary_accounts(
                handle,
                wallet_id.as_ptr(),
                out_accounts.as_mut_ptr(),
                capacity,
                &mut out_count,
            )
        };
        (r, out_accounts, out_count)
    }

    /// A Clear drops the registration, not the keys the wallet handle holds,
    /// so the export still reports the accounts bound before it, which the
    /// host then passes back to the bind.
    #[test]
    fn should_return_the_ordinary_accounts_after_clear_through_the_export() {
        let (handle, wallet_id, path) = configured_manager();
        bind_through_the_export(handle, &wallet_id, &[2, 1]);

        let mut cleared = unsafe { platform_wallet_manager_shielded_clear(handle) };
        assert_eq!(cleared.code, PlatformWalletFFIResultCode::Success);
        unsafe { platform_wallet_ffi_result_free(&mut cleared) };

        let (mut r, out_accounts, out_count) = ordinary_accounts(handle, &wallet_id, 64);
        assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(&out_accounts[..out_count], &[1, 2]);
        unsafe { platform_wallet_ffi_result_free(&mut r) };
        destroy_manager(handle, path);
    }

    #[test]
    fn should_return_no_accounts_for_an_unbound_wallet() {
        let (handle, wallet_id, path) = configured_manager();
        let (mut r, out_accounts, out_count) = ordinary_accounts(handle, &wallet_id, 64);
        assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out_count, 0);
        assert_eq!(out_accounts, [u32::MAX; 64]);
        unsafe { platform_wallet_ffi_result_free(&mut r) };
        destroy_manager(handle, path);
    }

    #[test]
    fn should_refuse_an_unknown_wallet() {
        let (handle, _wallet_id, path) = configured_manager();
        let (mut r, out_accounts, out_count) = ordinary_accounts(handle, &[0x77u8; 32], 64);
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorWalletOperation);
        assert!(message_of(&r).contains("wallet not found"));
        assert_eq!(out_count, usize::MAX);
        assert_eq!(out_accounts, [u32::MAX; 64]);
        unsafe { platform_wallet_ffi_result_free(&mut r) };
        destroy_manager(handle, path);
    }

    /// A list longer than the buffer is refused whole, not truncated: a
    /// truncated list bound again would drop the accounts left out.
    #[test]
    fn should_refuse_a_buffer_too_small_for_the_ordinary_accounts() {
        let (handle, wallet_id, path) = configured_manager();
        bind_through_the_export(handle, &wallet_id, &[0, 1, 2]);

        let (mut r, out_accounts, out_count) = ordinary_accounts(handle, &wallet_id, 2);
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorInvalidParameter);
        let message = message_of(&r);
        assert!(
            message.contains("has 3 ") && message.contains("capacity of 2"),
            "{message}"
        );
        assert_eq!(out_count, usize::MAX);
        assert_eq!(out_accounts, [u32::MAX; 64]);
        unsafe { platform_wallet_ffi_result_free(&mut r) };
        destroy_manager(handle, path);
    }
}
