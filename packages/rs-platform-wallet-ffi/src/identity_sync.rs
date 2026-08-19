//! FFI bindings for `PlatformWalletManager`'s per-identity token state
//! sync coordinator.
//!
//! Mirrors the shape of [`crate::platform_address_sync`]: lifecycle
//! controls (`start` / `stop` / `is_running` / `is_syncing` /
//! `last_sync_unix_seconds` / `set_interval` / `sync_now`), plus a
//! flat snapshot read API for the per-identity cache (single
//! identity or whole-store) with a paired free helper.
//!
//! Error handling follows the same shape as the rest of this crate:
//! every entry point returns a `PlatformWalletFFIResult` carrying the
//! typed code + Rust-supplied detail; null / invalid inputs surface
//! through the `check_ptr!` and `unwrap_option_or_return!` macros.

use std::time::Duration;

use platform_wallet::{IdentityTokenSyncInfo, IdentityTokenSyncState};

use crate::error::*;
use crate::handle::*;
use crate::runtime::{runtime, try_block_on_worker};
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};

/// Flattened per-(identity, token) row mirroring
/// [`IdentityTokenSyncInfo`].
///
/// `identity_id` is replicated onto each row so the whole-store
/// snapshot can be a single flat array even though the in-memory
/// cache is keyed by identity.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IdentityTokenSyncInfoFFI {
    /// 32-byte identity that owns this row.
    pub identity_id: [u8; 32],
    /// 32-byte token id.
    pub token_id: [u8; 32],
    /// 32-byte data-contract id that issued the token. Currently a
    /// zero-filled placeholder until token → contract resolution is
    /// wired up on the watch registry.
    pub contract_id: [u8; 32],
    /// Latest balance reported by Platform.
    pub balance: u64,
    /// `IdentityContractNonce` this identity would use for the next
    /// state transition against `contract_id`. Same value across
    /// every row in this snapshot that shares an `(identity_id,
    /// contract_id)` tuple. `0` means "not fetched yet" until
    /// per-token contract resolution lands.
    pub identity_contract_nonce: u64,
}

impl IdentityTokenSyncInfoFFI {
    fn from_state_row(row: &IdentityTokenSyncState, info: &IdentityTokenSyncInfo) -> Self {
        Self {
            identity_id: *row.identity_id.as_bytes(),
            token_id: *info.token_id.as_bytes(),
            contract_id: *info.contract_id.as_bytes(),
            balance: info.balance,
            identity_contract_nonce: info.identity_contract_nonce,
        }
    }
}

/// Start the identity-token sync manager in the background.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_identity_sync_start(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        let _entered = runtime().enter();
        manager.identity_sync_arc().start();
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Stop the identity-token sync manager if it is running.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_identity_sync_stop(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager.identity_sync().stop();
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Whether the identity-token sync background loop is running.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_identity_sync_is_running(
    handle: Handle,
    out_running: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_running);

    let option = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(handle, |manager| manager.identity_sync().is_running());
    let running = unwrap_option_or_return!(option);
    *out_running = running;
    PlatformWalletFFIResult::ok()
}

/// Whether an identity-token sync pass is currently in flight.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_identity_sync_is_syncing(
    handle: Handle,
    out_syncing: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_syncing);

    let option = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(handle, |manager| manager.identity_sync().is_syncing());
    let syncing = unwrap_option_or_return!(option);
    *out_syncing = syncing;
    PlatformWalletFFIResult::ok()
}

/// Unix seconds of the last completed identity-token sync pass for
/// the given identity, or 0 if that identity has never been synced.
///
/// `identity_id_ptr` must point to a 32-byte identifier. The
/// last-sync timestamp is per-identity (not global) — different
/// identities may have different watermarks.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_identity_sync_last_sync_unix_seconds(
    handle: Handle,
    identity_id_ptr: *const u8,
    out_last_sync_unix: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(identity_id_ptr);
    check_ptr!(out_last_sync_unix);

    let mut id_bytes = [0u8; 32];
    std::ptr::copy_nonoverlapping(identity_id_ptr, id_bytes.as_mut_ptr(), 32);
    let identity_id = dpp::prelude::Identifier::from(id_bytes);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        let mgr = manager.identity_sync_arc();
        runtime().try_block_on(async move { mgr.last_sync_unix_for_identity(&identity_id).await })
    });
    let value = unwrap_result_or_return!(unwrap_option_or_return!(option));
    *out_last_sync_unix = value.unwrap_or(0);
    PlatformWalletFFIResult::ok()
}

/// Set the background identity-token sync interval in seconds.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_identity_sync_set_interval(
    handle: Handle,
    interval_seconds: u64,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager
            .identity_sync()
            .set_interval(Duration::from_secs(interval_seconds));
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Run one identity-token sync pass across all registered wallets.
///
/// Synchronous from the FFI caller's point of view — blocks the
/// calling thread until the pass completes. If a pass is already in
/// flight (e.g. fired by the background loop), returns `Success`
/// immediately without scheduling extra work; check `is_syncing` if
/// the caller needs to distinguish.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_identity_sync_sync_now(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        // `block_on_worker`, NOT `runtime().block_on`: the pass
        // verifies GroveDB proofs whose recursion blows the ~512 KB
        // stack of the host's dispatch/concurrency calling thread
        // (same SIGBUS as the shielded/dashpay Sync Now buttons).
        let mgr = manager.identity_sync_arc();
        try_block_on_worker(async move { mgr.sync_now().await })
    });
    unwrap_result_or_return!(unwrap_option_or_return!(option));
    PlatformWalletFFIResult::ok()
}

/// Snapshot the identity-token sync state for one identity.
///
/// On success:
///   * `*out_rows` points to a heap-owned array of
///     [`IdentityTokenSyncInfoFFI`] of length `*out_rows_count`,
///   * `*out_last_sync_unix` is the per-identity last-sync timestamp
///     (`0` if never synced).
///
/// If the identity has no cached state, `*out_rows` is set to null
/// and `*out_rows_count` to 0 (still `Success` — empty is not an
/// error).
///
/// Free `*out_rows` with
/// [`platform_wallet_manager_identity_sync_state_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_identity_sync_state_for_identity(
    handle: Handle,
    identity_id_ptr: *const u8,
    out_rows: *mut *mut IdentityTokenSyncInfoFFI,
    out_rows_count: *mut usize,
    out_last_sync_unix: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(identity_id_ptr);
    check_ptr!(out_rows);
    check_ptr!(out_rows_count);
    check_ptr!(out_last_sync_unix);

    let mut id_bytes = [0u8; 32];
    std::ptr::copy_nonoverlapping(identity_id_ptr, id_bytes.as_mut_ptr(), 32);
    let identity_id = dpp::prelude::Identifier::from(id_bytes);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        let mgr = manager.identity_sync_arc();
        runtime().try_block_on(async move { mgr.state_for_identity(&identity_id).await })
    });
    let row = unwrap_result_or_return!(unwrap_option_or_return!(option));
    match row {
        Some(state) => {
            let rows: Vec<IdentityTokenSyncInfoFFI> = state
                .tokens
                .iter()
                .map(|info| IdentityTokenSyncInfoFFI::from_state_row(&state, info))
                .collect();
            let len = rows.len();
            let boxed = rows.into_boxed_slice();
            *out_rows = Box::into_raw(boxed) as *mut IdentityTokenSyncInfoFFI;
            *out_rows_count = len;
            *out_last_sync_unix = state.last_sync_unix;
        }
        None => {
            *out_rows = std::ptr::null_mut();
            *out_rows_count = 0;
            *out_last_sync_unix = 0;
        }
    }
    PlatformWalletFFIResult::ok()
}

/// Snapshot the identity-token sync state for every cached identity
/// as a single flat array.
///
/// `identity_id` is replicated on every row so callers can group on
/// it directly; ordering follows the BTreeMap iteration order over
/// `(identity_id, token_id)`.
///
/// On success:
///   * `*out_rows` points to a heap-owned array of
///     [`IdentityTokenSyncInfoFFI`] of length `*out_rows_count`.
///
/// Free `*out_rows` with
/// [`platform_wallet_manager_identity_sync_state_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_identity_sync_state_all(
    handle: Handle,
    out_rows: *mut *mut IdentityTokenSyncInfoFFI,
    out_rows_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_rows);
    check_ptr!(out_rows_count);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        let mgr = manager.identity_sync_arc();
        runtime().try_block_on(async move { mgr.all_state().await })
    });
    let snapshot = unwrap_result_or_return!(unwrap_option_or_return!(option));
    let mut rows: Vec<IdentityTokenSyncInfoFFI> = Vec::new();
    for state in snapshot.values() {
        for info in &state.tokens {
            rows.push(IdentityTokenSyncInfoFFI::from_state_row(state, info));
        }
    }
    let len = rows.len();
    if len == 0 {
        *out_rows = std::ptr::null_mut();
        *out_rows_count = 0;
    } else {
        let boxed = rows.into_boxed_slice();
        *out_rows = Box::into_raw(boxed) as *mut IdentityTokenSyncInfoFFI;
        *out_rows_count = len;
    }
    PlatformWalletFFIResult::ok()
}

/// Free a heap-owned `IdentityTokenSyncInfoFFI` array returned by one
/// of the snapshot getters above. Safe to call with `(null, 0)` —
/// no-op.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_identity_sync_state_free(
    rows: *mut IdentityTokenSyncInfoFFI,
    count: usize,
) {
    if rows.is_null() || count == 0 {
        return;
    }
    let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(rows, count));
}

/// Decode a `*const u8` of length `count * 32` into a `Vec<Identifier>`.
///
/// `ptr` may be null when `count == 0`; otherwise it must point to a
/// contiguous buffer of `count * 32` bytes laid out as back-to-back
/// 32-byte identifiers. Used by the registry lifecycle calls below.
unsafe fn read_token_ids(ptr: *const u8, count: usize) -> Option<Vec<dpp::prelude::Identifier>> {
    if count == 0 {
        return Some(Vec::new());
    }
    if ptr.is_null() {
        return None;
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let mut buf = [0u8; 32];
        std::ptr::copy_nonoverlapping(ptr.add(i * 32), buf.as_mut_ptr(), 32);
        out.push(dpp::prelude::Identifier::from(buf));
    }
    Some(out)
}

/// Register an identity with the token-sync registry.
///
/// `identity_id_ptr` must point to a 32-byte identifier.
/// `token_ids_ptr` must point to `token_ids_count * 32` bytes (each
/// 32-byte chunk is one token id) — null is permitted only when
/// `token_ids_count == 0` (registers the identity with no watched
/// tokens). Idempotent: a second call replaces the row.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_identity_sync_register_identity(
    handle: Handle,
    identity_id_ptr: *const u8,
    token_ids_ptr: *const u8,
    token_ids_count: usize,
) -> PlatformWalletFFIResult {
    check_ptr!(identity_id_ptr);
    let mut id_bytes = [0u8; 32];
    std::ptr::copy_nonoverlapping(identity_id_ptr, id_bytes.as_mut_ptr(), 32);
    let identity_id = dpp::prelude::Identifier::from(id_bytes);

    let Some(token_ids) = read_token_ids(token_ids_ptr, token_ids_count) else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorNullPointer,
            "token_ids_ptr is null with non-zero count",
        );
    };

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        let mgr = manager.identity_sync_arc();
        runtime().try_block_on(async move { mgr.register_identity(identity_id, token_ids).await })
    });
    unwrap_result_or_return!(unwrap_option_or_return!(option));
    PlatformWalletFFIResult::ok()
}

/// Unregister an identity from the token-sync registry.
///
/// `identity_id_ptr` must point to a 32-byte identifier. Idempotent —
/// removing an unknown identity is a successful no-op.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_identity_sync_unregister_identity(
    handle: Handle,
    identity_id_ptr: *const u8,
) -> PlatformWalletFFIResult {
    check_ptr!(identity_id_ptr);
    let mut id_bytes = [0u8; 32];
    std::ptr::copy_nonoverlapping(identity_id_ptr, id_bytes.as_mut_ptr(), 32);
    let identity_id = dpp::prelude::Identifier::from(id_bytes);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        let mgr = manager.identity_sync_arc();
        runtime().try_block_on(async move { mgr.unregister_identity(&identity_id).await })
    });
    unwrap_result_or_return!(unwrap_option_or_return!(option));
    PlatformWalletFFIResult::ok()
}

/// Replace the watched-token list for an already-registered identity.
///
/// `identity_id_ptr` must point to a 32-byte identifier.
/// `token_ids_ptr` must point to `token_ids_count * 32` bytes (each
/// 32-byte chunk is one token id) — null is permitted only when
/// `token_ids_count == 0` (clears the watched-token list, keeping the
/// row registered with no tokens). No-op on an unregistered identity
/// (returns `Success`); call `register_identity` first if you need
/// promotion semantics.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_identity_sync_update_watched_tokens(
    handle: Handle,
    identity_id_ptr: *const u8,
    token_ids_ptr: *const u8,
    token_ids_count: usize,
) -> PlatformWalletFFIResult {
    check_ptr!(identity_id_ptr);
    let mut id_bytes = [0u8; 32];
    std::ptr::copy_nonoverlapping(identity_id_ptr, id_bytes.as_mut_ptr(), 32);
    let identity_id = dpp::prelude::Identifier::from(id_bytes);

    let Some(token_ids) = read_token_ids(token_ids_ptr, token_ids_count) else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorNullPointer,
            "token_ids_ptr is null with non-zero count",
        );
    };

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        let mgr = manager.identity_sync_arc();
        runtime()
            .try_block_on(async move { mgr.update_watched_tokens(identity_id, token_ids).await })
    });
    unwrap_result_or_return!(unwrap_option_or_return!(option));
    PlatformWalletFFIResult::ok()
}
