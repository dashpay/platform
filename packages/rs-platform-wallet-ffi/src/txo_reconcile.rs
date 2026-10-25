//! C ABI over `platform_wallet::manager::txo_reconcile`: the schedule,
//! start gate, heal verdicts and stale-page retry of the host TXO-store
//! reconcile (dashpay/platform#4638). Each export forwards to one Rust
//! operation; the decisions, thresholds and cadence live in
//! `platform-wallet`. Which store rows to mark spent is
//! `platform_wallet_classify_outpoints`'s answer.

use platform_wallet::manager::accessors::WalletUtxoCursor;
use platform_wallet::manager::txo_reconcile::{txo_heal_page, txo_reconcile_retry_stale_page};

use crate::check_ptr;
use crate::core_wallet_types::{WalletUtxoCursorFFI, WalletUtxoEntryFFI};
use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};
use crate::handle::{Handle, PLATFORM_WALLET_MANAGER_STORAGE};
use crate::manager_diagnostics::{
    release_wallet_utxo_entry, wallet_utxo_cursor_from_ffi, wallet_utxo_entry_ffi,
};
use crate::runtime::block_on_worker;
use crate::types::IdentifierArray;
use crate::unwrap_option_or_return;

/// `platform_wallet_manager_txo_reconcile_begin`: the run may start.
pub const TXO_RECONCILE_GATE_RUN: u8 = 0;
/// `platform_wallet_manager_txo_reconcile_begin`: the manager does not hold
/// the wallet.
pub const TXO_RECONCILE_SKIP_WALLET_UNKNOWN: u8 = 1;
/// `platform_wallet_manager_txo_reconcile_begin`: a run for this wallet is
/// already in flight.
pub const TXO_RECONCILE_SKIP_ALREADY_RUNNING: u8 = 2;
/// `platform_wallet_manager_txo_reconcile_begin`: SPV is not running, or
/// not in its steady state.
pub const TXO_RECONCILE_SKIP_NOT_STEADY_STATE: u8 = 3;
/// `platform_wallet_manager_txo_reconcile_begin`: the progress carries no
/// scan tip.
pub const TXO_RECONCILE_SKIP_TIP_UNAVAILABLE: u8 = 4;
/// `platform_wallet_manager_txo_reconcile_begin`: the manager latched a
/// sync fault; rows are missing by design and a rescan is pending.
pub const TXO_RECONCILE_SKIP_SYNC_FAULT_DETECTED: u8 = 5;
/// `platform_wallet_manager_txo_reconcile_begin`: the wallet's scan
/// watermark trails the scan tip; it is still being scanned.
pub const TXO_RECONCILE_SKIP_WALLET_BEHIND_TIP: u8 = 6;

/// `TxoHealRowFFI::verdict`: no script or no address; the store could not
/// file the row.
pub const TXO_HEAL_VERDICT_INVALID: u8 = 0;
/// `TxoHealRowFFI::verdict`: not confirmed by the engine, or not deep
/// enough yet; it ages into a later run.
pub const TXO_HEAL_VERDICT_IMMATURE: u8 = 1;
/// `TxoHealRowFFI::verdict`: may be inserted unspent, if the store lacks
/// the outpoint and resolves the account that owns it.
pub const TXO_HEAL_VERDICT_ELIGIBLE: u8 = 2;

/// One row of a heal page: an inventory row and whether the store may heal
/// it in. Its `utxo` allocations are released by
/// `platform_wallet_txo_reconcile_heal_page_free`.
#[repr(C)]
#[derive(Debug)]
pub struct TxoHealRowFFI {
    pub utxo: WalletUtxoEntryFFI,
    /// One of the `TXO_HEAL_VERDICT_*` constants.
    pub verdict: u8,
}

/// The wallets whose reconcile is due now. Call on every SPV progress poll,
/// running or not: the poll is the schedule's clock, and a stopped client
/// arms the next steady-state edge. Reporting a wallet consumes nothing —
/// its cadence restarts when `platform_wallet_manager_txo_reconcile_begin`
/// claims it — so one the host skips is reported again on the next poll.
/// `*out_wallet_ids` is released by
/// `platform_wallet_identifier_array_free`.
///
/// # Safety
/// `out_wallet_ids` must be writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_txo_reconcile_due_wallets(
    manager_handle: Handle,
    out_wallet_ids: *mut IdentifierArray,
) -> PlatformWalletFFIResult {
    check_ptr!(out_wallet_ids);
    *out_wallet_ids = IdentifierArray::empty();
    // Looked up under the registry guard, waited on outside it — see
    // `platform_wallet_manager_sync_progress`.
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |m| m.txo_reconciler());
    let reconciler = unwrap_option_or_return!(option);
    let due = block_on_worker(async move { reconciler.due_wallets().await });
    *out_wallet_ids = IdentifierArray::from_hashes(due);
    PlatformWalletFFIResult::ok()
}

/// Claim `wallet_id` for one reconcile run and check that it may start.
/// `*out_gate` is [`TXO_RECONCILE_GATE_RUN`] or one of the
/// `TXO_RECONCILE_SKIP_*` reasons; on `RUN`, `*out_tip_height` is the scan
/// tip to pass to `platform_wallet_txo_reconcile_heal_page`, and the claim
/// holds until `platform_wallet_manager_txo_reconcile_finish`. A skip holds
/// no claim. Either way the wallet's cadence restarts now, unless another
/// run already held it.
///
/// # Safety
/// `wallet_id` points at 32 readable bytes; the out-pointers are writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_txo_reconcile_begin(
    manager_handle: Handle,
    wallet_id: *const u8,
    out_gate: *mut u8,
    out_tip_height: *mut u32,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(out_gate);
    check_ptr!(out_tip_height);
    *out_gate = TXO_RECONCILE_SKIP_WALLET_UNKNOWN;
    *out_tip_height = 0;
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |m| m.txo_reconciler());
    let reconciler = unwrap_option_or_return!(option);
    match block_on_worker(async move { reconciler.begin(&wid).await }) {
        Ok(tip_height) => {
            *out_gate = TXO_RECONCILE_GATE_RUN;
            *out_tip_height = tip_height;
        }
        Err(skip) => *out_gate = skip.as_u8(),
    }
    PlatformWalletFFIResult::ok()
}

/// Release the claim a `RUN` from
/// `platform_wallet_manager_txo_reconcile_begin` granted. `ran` is whether
/// the run went ahead (even if it stopped early), which restarts the
/// wallet's cadence.
///
/// # Safety
/// `wallet_id` points at 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_txo_reconcile_finish(
    manager_handle: Handle,
    wallet_id: *const u8,
    ran: bool,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let option = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |m| m.txo_reconciler().finish(&wid, ran));
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// One page of the wallet's UTXO inventory — the same walk, cursor and
/// bounds as `platform_wallet_wallet_utxos_page` — each row with its
/// `TXO_HEAL_VERDICT_*` at `tip_height` (the tip
/// `platform_wallet_manager_txo_reconcile_begin` returned). Contact
/// watch-only coins are never offered. The rows are released by
/// `platform_wallet_txo_reconcile_heal_page_free`.
///
/// # Safety
/// `wallet_id` points at 32 readable bytes; `cursor` is null or points at a
/// live `WalletUtxoCursorFFI`; the out-pointers are writable.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_txo_reconcile_heal_page(
    manager_handle: Handle,
    wallet_id: *const u8,
    cursor: *const WalletUtxoCursorFFI,
    limit: usize,
    tip_height: u32,
    out_rows: *mut *const TxoHealRowFFI,
    out_count: *mut usize,
    out_has_more: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(out_rows);
    check_ptr!(out_count);
    check_ptr!(out_has_more);
    *out_rows = std::ptr::null();
    *out_count = 0;
    *out_has_more = false;
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);

    let after: Option<WalletUtxoCursor> = if cursor.is_null() {
        None
    } else {
        match wallet_utxo_cursor_from_ffi(&*cursor) {
            Ok(after) => Some(after),
            Err(e) => {
                return PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorInvalidParameter,
                    format!("cursor: {e}"),
                );
            }
        }
    };

    // The wallet lock is taken outside the registry guard, as
    // `platform_wallet_wallet_utxos_page` does.
    let Some(wallet_manager) =
        PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |m| m.wallet_manager_arc())
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    let (rows, has_more) = {
        let wm = wallet_manager.blocking_read();
        txo_heal_page(&wm, &wid, after.as_ref(), limit, tip_height)
    };
    *out_has_more = has_more;
    if rows.is_empty() {
        return PlatformWalletFFIResult::ok();
    }
    let entries: Vec<TxoHealRowFFI> = rows
        .into_iter()
        .map(|(row, verdict)| TxoHealRowFFI {
            utxo: wallet_utxo_entry_ffi(row),
            verdict: verdict.as_u8(),
        })
        .collect();
    let count = entries.len();
    *out_rows = Box::into_raw(entries.into_boxed_slice()) as *const _;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

/// Release a page returned by `platform_wallet_txo_reconcile_heal_page`.
///
/// # Safety
/// `rows`/`count` must be exactly what one call returned, released once.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_txo_reconcile_heal_page_free(
    rows: *mut TxoHealRowFFI,
    count: usize,
) {
    if rows.is_null() || count == 0 {
        return;
    }
    for row in std::slice::from_raw_parts(rows, count) {
        release_wallet_utxo_entry(&row.utxo);
    }
    let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(rows, count));
}

/// Whether a reconcile run classifies a page again after it came back
/// stale for the `stale_in_a_row`-th time in a row (counting this one), or
/// stops incomplete.
#[no_mangle]
pub extern "C" fn platform_wallet_txo_reconcile_retry_stale_page(stale_in_a_row: u32) -> bool {
    txo_reconcile_retry_stale_page(stale_in_a_row)
}

#[cfg(test)]
mod tests {
    use super::*;

    use platform_wallet::manager::txo_reconcile::{TxoHealVerdict, TxoReconcileSkip};

    #[test]
    fn the_c_codes_match_the_rust_enums() {
        for (code, skip) in [
            (
                TXO_RECONCILE_SKIP_WALLET_UNKNOWN,
                TxoReconcileSkip::WalletUnknown,
            ),
            (
                TXO_RECONCILE_SKIP_ALREADY_RUNNING,
                TxoReconcileSkip::AlreadyRunning,
            ),
            (
                TXO_RECONCILE_SKIP_NOT_STEADY_STATE,
                TxoReconcileSkip::NotSteadyState,
            ),
            (
                TXO_RECONCILE_SKIP_TIP_UNAVAILABLE,
                TxoReconcileSkip::TipUnavailable,
            ),
            (
                TXO_RECONCILE_SKIP_SYNC_FAULT_DETECTED,
                TxoReconcileSkip::SyncFaultDetected,
            ),
            (
                TXO_RECONCILE_SKIP_WALLET_BEHIND_TIP,
                TxoReconcileSkip::WalletBehindTip,
            ),
        ] {
            assert_eq!(code, skip.as_u8(), "{skip:?}");
            assert_ne!(code, TXO_RECONCILE_GATE_RUN, "{skip:?}");
        }
        for (code, verdict) in [
            (TXO_HEAL_VERDICT_INVALID, TxoHealVerdict::Invalid),
            (TXO_HEAL_VERDICT_IMMATURE, TxoHealVerdict::Immature),
            (TXO_HEAL_VERDICT_ELIGIBLE, TxoHealVerdict::Eligible),
        ] {
            assert_eq!(code, verdict.as_u8(), "{verdict:?}");
        }
    }

    #[test]
    fn the_stale_page_export_forwards_the_retry_limit() {
        assert!(platform_wallet_txo_reconcile_retry_stale_page(5));
        assert!(!platform_wallet_txo_reconcile_retry_stale_page(6));
    }

    #[test]
    fn an_invalid_handle_is_an_error_and_leaves_the_out_params_initialized() {
        let wallet_id = [0u8; 32];
        let mut gate = 0xFFu8;
        let mut tip = 7u32;
        let result = unsafe {
            platform_wallet_manager_txo_reconcile_begin(
                Handle::MAX,
                wallet_id.as_ptr(),
                &mut gate,
                &mut tip,
            )
        };
        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(gate, TXO_RECONCILE_SKIP_WALLET_UNKNOWN);
        assert_eq!(tip, 0);

        let mut ids = IdentifierArray::empty();
        let result =
            unsafe { platform_wallet_manager_txo_reconcile_due_wallets(Handle::MAX, &mut ids) };
        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(ids.items.is_null());
        assert_eq!(ids.count, 0);

        let mut rows: *const TxoHealRowFFI = std::ptr::null();
        let mut count = 0usize;
        let mut has_more = true;
        let result = unsafe {
            platform_wallet_txo_reconcile_heal_page(
                Handle::MAX,
                wallet_id.as_ptr(),
                std::ptr::null(),
                0,
                0,
                &mut rows,
                &mut count,
                &mut has_more,
            )
        };
        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(rows.is_null());
        assert_eq!(count, 0);
        assert!(!has_more);
    }
}
