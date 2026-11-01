//! C ABI over `platform_wallet::manager::txo_reconcile`: the schedule,
//! start gate, heal verdicts, spend verdicts and stale-page retry of the
//! host TXO-store reconcile (dashpay/platform#4638). Each export forwards
//! to one Rust operation; the decisions, thresholds, cadence, the page
//! budget and resume point, and the lock that keeps a spend verdict true
//! until it is written live in `platform-wallet`.

use std::panic::{catch_unwind, AssertUnwindSafe};

use platform_wallet::manager::accessors::OutpointClass;
use platform_wallet::manager::txo_reconcile::{
    txo_reconcile_retry_stale_page, TxoReconcileHeldVerdicts, TxoReconcileRead, TxoReconcileStart,
};

use crate::check_ptr;
use crate::core_wallet_types::{OutpointOwnershipQueryFFI, WalletUtxoEntryFFI};
use crate::error::{catch_panic_to_code, PlatformWalletFFIResult, PlatformWalletFFIResultCode};
use crate::handle::{Handle, PLATFORM_WALLET_MANAGER_STORAGE};
use crate::manager_diagnostics::{
    classify_outpoint_rows, invalid_manager_handle, release_wallet_utxo_entry,
    wallet_utxo_entry_ffi,
};
use crate::runtime::block_on_worker;
use crate::types::IdentifierArray;
use crate::unwrap_option_or_return;

/// What a host should know after a reconcile export panicked: nothing was
/// written to its store by this call.
const RECONCILE_PANIC_GUIDANCE: &str = "The reconcile wrote nothing to the store in this call; \
     stop the run and let the schedule run it again.";

/// Run a reconcile export's body with a panic turned into an
/// `ErrorWalletOperation` result instead of unwinding into the C caller —
/// JNI's panic guard sits on the far side of this ABI and never sees it.
/// Every claim, guard and allocation the body took is released while it
/// unwinds.
fn catch_reconcile_panic(
    operation: &str,
    body: impl FnOnce() -> PlatformWalletFFIResult,
) -> PlatformWalletFFIResult {
    catch_panic_to_code(
        operation,
        PlatformWalletFFIResultCode::ErrorWalletOperation,
        RECONCILE_PANIC_GUIDANCE,
        body,
    )
}

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

/// `*out_start` of `platform_wallet_manager_txo_reconcile_begin`: a new
/// cycle, at the heal pass's first page.
pub const TXO_RECONCILE_START_HEAL: u8 = 0;
/// The heal pass, where an earlier run stopped (the reconciler keeps the
/// cursor; the host passes none).
pub const TXO_RECONCILE_START_HEAL_RESUMED: u8 = 1;
/// Straight to the classify pass, after the store cursor
/// `*out_store_after` (empty: the first row).
pub const TXO_RECONCILE_START_CLASSIFY: u8 = 2;

/// The longest store cursor
/// `platform_wallet_txo_reconcile_classify_page_applied` keeps, and the
/// size of `platform_wallet_manager_txo_reconcile_begin`'s
/// `out_store_after` buffer.
///
/// A literal, so the generated C header carries it.
pub const TXO_RECONCILE_STORE_CURSOR_MAX: usize = 64;
const _: () = assert!(
    TXO_RECONCILE_STORE_CURSOR_MAX
        == platform_wallet::manager::txo_reconcile::TXO_RECONCILE_STORE_CURSOR_MAX
);

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
    catch_reconcile_panic("platform_wallet_manager_txo_reconcile_due_wallets", || {
        check_ptr!(out_wallet_ids);
        *out_wallet_ids = IdentifierArray::empty();
        // Looked up under the registry guard, waited on outside it — see
        // `platform_wallet_manager_sync_progress`.
        let option =
            PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |m| m.txo_reconciler());
        let reconciler = unwrap_option_or_return!(option);
        let due = block_on_worker(async move { reconciler.due_wallets().await });
        *out_wallet_ids = IdentifierArray::from_hashes(due);
        PlatformWalletFFIResult::ok()
    })
}

/// Claim `wallet_id` for one reconcile run and check that it may start.
/// `*out_gate` is [`TXO_RECONCILE_GATE_RUN`] or one of the
/// `TXO_RECONCILE_SKIP_*` reasons; on `RUN`, `*out_tip_height` is the scan
/// tip the run is measured against (the claim keeps it; the host only
/// reports it), and the claim holds until
/// `platform_wallet_manager_txo_reconcile_finish`. A skip holds no claim.
/// Either way the wallet's cadence restarts now, unless another run
/// already held it.
///
/// On `RUN`, `*out_start` (a `TXO_RECONCILE_START_*`) says where the run
/// starts, from where the wallet's last run stopped; for
/// [`TXO_RECONCILE_START_CLASSIFY`] the store cursor to start after is
/// written to `out_store_after` (`*out_store_after_len` bytes; 0 is the
/// store's first row), exactly as the host handed it to
/// `platform_wallet_txo_reconcile_classify_page_applied`. The run may read
/// a fixed budget of pages; see `platform_wallet_txo_reconcile_heal_page`.
///
/// # Safety
/// `wallet_id` points at 32 readable bytes; `out_store_after` is writable
/// for [`TXO_RECONCILE_STORE_CURSOR_MAX`] bytes; the other out-pointers are
/// writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_txo_reconcile_begin(
    manager_handle: Handle,
    wallet_id: *const u8,
    out_gate: *mut u8,
    out_tip_height: *mut u32,
    out_start: *mut u8,
    out_store_after: *mut u8,
    out_store_after_len: *mut usize,
) -> PlatformWalletFFIResult {
    catch_reconcile_panic("platform_wallet_manager_txo_reconcile_begin", || {
        check_ptr!(wallet_id);
        check_ptr!(out_gate);
        check_ptr!(out_tip_height);
        check_ptr!(out_start);
        check_ptr!(out_store_after);
        check_ptr!(out_store_after_len);
        *out_gate = TXO_RECONCILE_SKIP_WALLET_UNKNOWN;
        *out_tip_height = 0;
        *out_start = TXO_RECONCILE_START_HEAL;
        *out_store_after_len = 0;
        let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
        let option =
            PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |m| m.txo_reconciler());
        let reconciler = unwrap_option_or_return!(option);
        match block_on_worker(async move { reconciler.begin(&wid).await }) {
            Ok(run) => {
                *out_gate = TXO_RECONCILE_GATE_RUN;
                *out_tip_height = run.tip;
                *out_start = match run.start {
                    TxoReconcileStart::Heal { resumed: false } => TXO_RECONCILE_START_HEAL,
                    TxoReconcileStart::Heal { resumed: true } => TXO_RECONCILE_START_HEAL_RESUMED,
                    TxoReconcileStart::Classify { after } => {
                        // The reconciler keeps no cursor longer than this.
                        let len = after.len().min(TXO_RECONCILE_STORE_CURSOR_MAX);
                        std::ptr::copy_nonoverlapping(after.as_ptr(), out_store_after, len);
                        *out_store_after_len = len;
                        TXO_RECONCILE_START_CLASSIFY
                    }
                };
            }
            Err(skip) => *out_gate = skip.as_u8(),
        }
        PlatformWalletFFIResult::ok()
    })
}

/// Release the claim a `RUN` from
/// `platform_wallet_manager_txo_reconcile_begin` granted. `ran` is whether
/// the run went ahead (even if it stopped early), which restarts the
/// wallet's cadence. Also releases, as a backstop, an engine guard the
/// host failed to free — but this resolves the manager handle, so a host
/// frees each page's guard first
/// (`platform_wallet_txo_reconcile_held_verdicts_free`).
///
/// # Safety
/// `wallet_id` points at 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_txo_reconcile_finish(
    manager_handle: Handle,
    wallet_id: *const u8,
    ran: bool,
) -> PlatformWalletFFIResult {
    catch_reconcile_panic("platform_wallet_manager_txo_reconcile_finish", || {
        check_ptr!(wallet_id);
        let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
        let option = PLATFORM_WALLET_MANAGER_STORAGE
            .with_item(manager_handle, |m| m.txo_reconciler().finish(&wid, ran));
        unwrap_option_or_return!(option);
        PlatformWalletFFIResult::ok()
    })
}

/// One page of the wallet's UTXO inventory — the same walk and bounds as
/// `platform_wallet_wallet_utxos_page` — each row with its
/// `TXO_HEAL_VERDICT_*`, for the reconcile run the wallet holds a claim
/// for. The run keeps its own place: the page starts after the last page
/// reported applied (`platform_wallet_txo_reconcile_heal_page_applied`),
/// or where the wallet's last run stopped, so the caller passes no cursor;
/// a page read again before it is reported applied is the same page. Confirmations are counted to the run's own tip (the one
/// `platform_wallet_manager_txo_reconcile_begin` returned), or to the
/// wallet's processed height under this page's guard when that is lower;
/// the caller passes no height. Contact watch-only coins are never
/// offered. The rows are released by
/// `platform_wallet_txo_reconcile_heal_page_free`.
///
/// The engine's wallet-manager read lock stays held when this returns, in
/// `*out_held`, so no fork can drop a coin the page offers before the host
/// has written it: a row healed after such a fork would sit unspent in the
/// store for a coin the engine no longer owns. Release `*out_held` with
/// `platform_wallet_txo_reconcile_held_verdicts_free` once the page is
/// written, under the same rules as
/// `platform_wallet_txo_reconcile_classify_outpoints`' guard.
///
/// Each call takes one page of the run's budget. `*out_budget_spent` is
/// true, with nothing read and nothing held, once the run has used it:
/// the host stops the run, and the wallet's next run picks up where it
/// stopped.
///
/// Fails with `*out_held` null — nothing held — when the wallet holds no
/// run claim.
///
/// # Safety
/// `wallet_id` points at 32 readable bytes; the out-pointers are writable.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_txo_reconcile_heal_page(
    manager_handle: Handle,
    wallet_id: *const u8,
    limit: usize,
    out_rows: *mut *const TxoHealRowFFI,
    out_count: *mut usize,
    out_has_more: *mut bool,
    out_budget_spent: *mut bool,
    out_held: *mut *mut TxoReconcileHeldVerdictsFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_held);
    *out_held = std::ptr::null_mut();
    let result = catch_reconcile_panic("platform_wallet_txo_reconcile_heal_page", || {
        check_ptr!(wallet_id);
        check_ptr!(out_rows);
        check_ptr!(out_count);
        check_ptr!(out_has_more);
        check_ptr!(out_budget_spent);
        *out_rows = std::ptr::null();
        *out_count = 0;
        *out_has_more = false;
        *out_budget_spent = false;
        let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);

        // Resolved under the registry guard, waited on outside it, with no
        // guard held — see `platform_wallet_txo_reconcile_classify_outpoints`.
        let Some(reconciler) =
            PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |m| m.txo_reconciler())
        else {
            return invalid_manager_handle();
        };
        let read = block_on_worker(async move { reconciler.heal_page_held(&wid, limit).await });
        let ((rows, has_more), held) = match read {
            Some(TxoReconcileRead::Page(page, held)) => (page, held),
            Some(TxoReconcileRead::BudgetSpent) => {
                *out_budget_spent = true;
                return PlatformWalletFFIResult::ok();
            }
            None => {
                return PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorWalletOperation,
                    "the wallet holds no reconcile run claim".to_string(),
                );
            }
        };
        *out_held = Box::into_raw(Box::new(TxoReconcileHeldVerdictsFFI { _held: held }));
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
    });
    // A failure holds nothing, even one that struck after the guard was
    // handed out.
    if result.code != PlatformWalletFFIResultCode::Success {
        platform_wallet_txo_reconcile_held_verdicts_free(*out_held);
        *out_held = std::ptr::null_mut();
    }
    result
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
    // No result to carry a panic: it is contained here, at worst leaking
    // the page, rather than crossing the C ABI.
    let _ = catch_unwind(AssertUnwindSafe(|| {
        for row in std::slice::from_raw_parts(rows, count) {
            release_wallet_utxo_entry(&row.utxo);
        }
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(rows, count));
    }));
}

/// The engine guard one page's heal or spend verdicts are held under — see
/// `platform_wallet_txo_reconcile_heal_page` and
/// `platform_wallet_txo_reconcile_classify_outpoints`. Opaque; released by
/// `platform_wallet_txo_reconcile_held_verdicts_free`.
pub struct TxoReconcileHeldVerdictsFFI {
    _held: TxoReconcileHeldVerdicts,
}

/// Classify `count` store rows of `wallet_id` for the spend pass of the
/// reconcile run it holds a claim for — `platform_wallet_classify_outpoints`'s
/// answers and marshalling — and keep the engine's wallet-manager read
/// lock held when this returns, in `*out_held`, so no fork can drop a
/// spender these answers name before the host has written them. Only
/// `OUTPOINT_CLASS_KNOWN_UNCREDITED` marks a row spent.
///
/// Release `*out_held` with
/// `platform_wallet_txo_reconcile_held_verdicts_free` as soon as the page
/// is applied, and before the next call for this wallet or its
/// `platform_wallet_manager_txo_reconcile_finish`: those resolve the
/// manager handle, and the handle registry must never be waited for while
/// the guard is held (every other path takes the registry before the
/// engine lock). Finish and manager destroy release a guard the host
/// failed to, as a backstop. Call this outside the host's
/// persistence-callback exclusion and never from inside a persistence
/// callback: the engine calls those callbacks under its write lock. The
/// registry is read only to resolve the manager, with no guard held; the
/// wait for the new guard happens after it is released.
///
/// Classify every store page the run reads, an empty one (`count` 0)
/// too: each call takes one page of the run's budget and names the page
/// `platform_wallet_txo_reconcile_classify_page_applied` reports. Once the
/// run has used its budget, `*out_budget_spent` is true, every class is
/// `OUTPOINT_CLASS_UNKNOWN` and nothing is held.
///
/// Fails with `*out_held` null — nothing held — when the wallet holds no
/// run claim.
///
/// # Safety
/// As `platform_wallet_classify_outpoints`: `wallet_id` points at 32
/// readable bytes; `queries` points at `count` live entries whose script
/// pointers are valid for the call (or is anything when `count` is 0);
/// `out_classes` is writable for `count` bytes. `out_budget_spent` and
/// `out_held` are writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_txo_reconcile_classify_outpoints(
    manager_handle: Handle,
    wallet_id: *const u8,
    queries: *const OutpointOwnershipQueryFFI,
    count: usize,
    out_classes: *mut u8,
    out_budget_spent: *mut bool,
    out_held: *mut *mut TxoReconcileHeldVerdictsFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_held);
    check_ptr!(out_budget_spent);
    *out_held = std::ptr::null_mut();
    *out_budget_spent = false;
    let result = catch_reconcile_panic("platform_wallet_txo_reconcile_classify_outpoints", || {
        let classify = |wid: [u8; 32], owned: Vec<_>| {
            let reconciler = PLATFORM_WALLET_MANAGER_STORAGE
                .with_item(manager_handle, |m| m.txo_reconciler())
                .ok_or_else(invalid_manager_handle)?;
            let asked = owned.len();
            match block_on_worker(async move { reconciler.classify_held(&wid, &owned).await }) {
                Some(TxoReconcileRead::Page(classes, held)) => {
                    *out_held =
                        Box::into_raw(Box::new(TxoReconcileHeldVerdictsFFI { _held: held }));
                    Ok(classes)
                }
                Some(TxoReconcileRead::BudgetSpent) => {
                    *out_budget_spent = true;
                    Ok(vec![OutpointClass::Unknown; asked])
                }
                None => Err(PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorWalletOperation,
                    "the wallet holds no reconcile run claim".to_string(),
                )),
            }
        };
        if count == 0 {
            // `classify_outpoint_rows` answers an empty batch without a
            // classification; the run still reads it, as one page.
            check_ptr!(wallet_id);
            let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
            return match classify(wid, Vec::new()) {
                Ok(_) => PlatformWalletFFIResult::ok(),
                Err(error) => error,
            };
        }
        classify_outpoint_rows(wallet_id, queries, count, out_classes, classify)
    });
    // A failure holds nothing, even one that struck after the guard was
    // handed out.
    if result.code != PlatformWalletFFIResultCode::Success {
        platform_wallet_txo_reconcile_held_verdicts_free(*out_held);
        *out_held = std::ptr::null_mut();
    }
    result
}

/// Release the engine guard `platform_wallet_txo_reconcile_heal_page` or
/// `platform_wallet_txo_reconcile_classify_outpoints` handed out. Touches no handle registry and never blocks, so it can run
/// whatever else is queued on the registry or the engine. Null is a no-op.
///
/// # Safety
/// `held` is null or a pointer that call returned, released exactly once.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_txo_reconcile_held_verdicts_free(
    held: *mut TxoReconcileHeldVerdictsFFI,
) {
    if !held.is_null() {
        // No result to carry a panic: it is contained here rather than
        // crossing the C ABI.
        let _ = catch_unwind(AssertUnwindSafe(|| drop(Box::from_raw(held))));
    }
}

/// The host has applied the heal page it last read
/// (`platform_wallet_txo_reconcile_heal_page`), an empty one included: the
/// run's next heal page starts after it, and so does the wallet's next run
/// if this one stops here. `*out_may_continue` is whether the run has
/// budget left for another page. Call it after freeing the page's guard:
/// it resolves the manager handle.
///
/// # Safety
/// `wallet_id` points at 32 readable bytes; `out_may_continue` is writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_txo_reconcile_heal_page_applied(
    manager_handle: Handle,
    wallet_id: *const u8,
    out_may_continue: *mut bool,
) -> PlatformWalletFFIResult {
    catch_reconcile_panic("platform_wallet_txo_reconcile_heal_page_applied", || {
        check_ptr!(wallet_id);
        check_ptr!(out_may_continue);
        *out_may_continue = false;
        let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
        let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |m| {
            m.txo_reconciler().heal_page_applied(&wid)
        });
        *out_may_continue = unwrap_option_or_return!(option);
        PlatformWalletFFIResult::ok()
    })
}

/// The host has applied the store page it last classified
/// (`platform_wallet_txo_reconcile_classify_outpoints`). `is_last` says it
/// was the store's last page, which ends the cycle: the wallet's next run
/// starts again at the heal pass. Otherwise the next run starts at the
/// classify pass after the host's own store cursor `next_after`
/// (`next_after_len` bytes, at most [`TXO_RECONCILE_STORE_CURSOR_MAX`]; a
/// longer one is not kept), handed back by
/// `platform_wallet_manager_txo_reconcile_begin` as it was given.
/// `*out_may_continue` is whether the run has budget left for another
/// page. Call it after freeing the page's guard.
///
/// # Safety
/// `wallet_id` points at 32 readable bytes; `next_after` points at
/// `next_after_len` readable bytes (or is anything when that is 0, or when
/// `is_last`); `out_may_continue` is writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_txo_reconcile_classify_page_applied(
    manager_handle: Handle,
    wallet_id: *const u8,
    next_after: *const u8,
    next_after_len: usize,
    is_last: bool,
    out_may_continue: *mut bool,
) -> PlatformWalletFFIResult {
    catch_reconcile_panic(
        "platform_wallet_txo_reconcile_classify_page_applied",
        || {
            check_ptr!(wallet_id);
            check_ptr!(out_may_continue);
            *out_may_continue = false;
            let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
            let after: Option<Vec<u8>> = if is_last {
                None
            } else if next_after_len == 0 {
                Some(Vec::new())
            } else {
                check_ptr!(next_after);
                Some(std::slice::from_raw_parts(next_after, next_after_len).to_vec())
            };
            let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |m| {
                m.txo_reconciler()
                    .classify_page_applied(&wid, after.as_deref())
            });
            *out_may_continue = unwrap_option_or_return!(option);
            PlatformWalletFFIResult::ok()
        },
    )
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

    use crate::core_wallet_types::{OutPointFFI, OUTPOINT_CLASS_UNKNOWN};

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
        let mut start = 0xFFu8;
        let mut store_after = [0u8; TXO_RECONCILE_STORE_CURSOR_MAX];
        let mut store_after_len = 9usize;
        let result = unsafe {
            platform_wallet_manager_txo_reconcile_begin(
                Handle::MAX,
                wallet_id.as_ptr(),
                &mut gate,
                &mut tip,
                &mut start,
                store_after.as_mut_ptr(),
                &mut store_after_len,
            )
        };
        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(gate, TXO_RECONCILE_SKIP_WALLET_UNKNOWN);
        assert_eq!(tip, 0);
        assert_eq!(start, TXO_RECONCILE_START_HEAL);
        assert_eq!(store_after_len, 0);

        let mut may_continue = true;
        let result = unsafe {
            platform_wallet_txo_reconcile_heal_page_applied(
                Handle::MAX,
                wallet_id.as_ptr(),
                &mut may_continue,
            )
        };
        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(!may_continue);
        let mut may_continue = true;
        let result = unsafe {
            platform_wallet_txo_reconcile_classify_page_applied(
                Handle::MAX,
                wallet_id.as_ptr(),
                std::ptr::null(),
                0,
                true,
                &mut may_continue,
            )
        };
        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(!may_continue);

        let mut ids = IdentifierArray::empty();
        let result =
            unsafe { platform_wallet_manager_txo_reconcile_due_wallets(Handle::MAX, &mut ids) };
        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(ids.items.is_null());
        assert_eq!(ids.count, 0);

        let mut rows: *const TxoHealRowFFI = std::ptr::null();
        let mut count = 0usize;
        let mut has_more = true;
        let mut held: *mut TxoReconcileHeldVerdictsFFI = std::ptr::dangling_mut();
        let result = unsafe {
            platform_wallet_txo_reconcile_heal_page(
                Handle::MAX,
                wallet_id.as_ptr(),
                0,
                &mut rows,
                &mut count,
                &mut has_more,
                &mut false,
                &mut held,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        assert!(rows.is_null());
        assert_eq!(count, 0);
        assert!(!has_more);
        assert!(held.is_null(), "nothing is held on a failure");

        let script = [0x76u8, 0xa9, 0x14];
        let query = OutpointOwnershipQueryFFI {
            type_tag: 0,
            standard_tag: 0,
            index: 0,
            registration_index: 0,
            key_class: 0,
            user_identity_id: [0; 32],
            friend_identity_id: [0; 32],
            outpoint: OutPointFFI {
                txid: [0x22; 32],
                vout: 0,
            },
            script_pubkey: script.as_ptr(),
            script_pubkey_len: script.len(),
        };
        let mut classes = [0xFFu8; 1];
        let mut held: *mut TxoReconcileHeldVerdictsFFI = std::ptr::dangling_mut();
        let result = unsafe {
            platform_wallet_txo_reconcile_classify_outpoints(
                Handle::MAX,
                wallet_id.as_ptr(),
                &query,
                1,
                classes.as_mut_ptr(),
                &mut false,
                &mut held,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        assert_eq!(classes, [OUTPOINT_CLASS_UNKNOWN]);
        assert!(held.is_null(), "nothing is held on a failure");
        unsafe { platform_wallet_txo_reconcile_held_verdicts_free(held) };
    }

    /// The run's place and budget across the exports: an empty classify
    /// page still takes a page and holds a guard, an applied store cursor
    /// comes back from the next run's start, and a run that has used its
    /// budget is told so with nothing held.
    #[test]
    fn the_applied_exports_move_the_runs_place_and_the_budget_stops_it() {
        use platform_wallet::manager::txo_reconcile::{
            TxoReconcileStart, TXO_RECONCILE_PAGE_BUDGET,
        };

        use crate::test_fixtures::mock_manager;

        let handle = PLATFORM_WALLET_MANAGER_STORAGE.insert(mock_manager());
        let wallet_id = [0x6cu8; 32];
        let reconciler = PLATFORM_WALLET_MANAGER_STORAGE
            .with_item(handle, |m| m.txo_reconciler())
            .expect("manager");
        let classify_empty = || {
            let mut spent = true;
            let mut held: *mut TxoReconcileHeldVerdictsFFI = std::ptr::null_mut();
            let result = unsafe {
                platform_wallet_txo_reconcile_classify_outpoints(
                    handle,
                    wallet_id.as_ptr(),
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    &mut spent,
                    &mut held,
                )
            };
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            unsafe { platform_wallet_txo_reconcile_held_verdicts_free(held) };
            (spent, !held.is_null())
        };

        assert!(reconciler.claim_without_gate(&wallet_id, 0).is_some());
        assert_eq!(classify_empty(), (false, true), "a page, held");
        let cursor = [0x36u8; 36];
        let mut may_continue = false;
        let result = unsafe {
            platform_wallet_txo_reconcile_classify_page_applied(
                handle,
                wallet_id.as_ptr(),
                cursor.as_ptr(),
                cursor.len(),
                false,
                &mut may_continue,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(may_continue);
        reconciler.finish(&wallet_id, true);

        let run = reconciler
            .claim_without_gate(&wallet_id, 0)
            .expect("claimed");
        assert_eq!(
            run.start,
            TxoReconcileStart::Classify {
                after: cursor.to_vec()
            }
        );
        for _ in 1..TXO_RECONCILE_PAGE_BUDGET {
            classify_empty();
        }
        let mut may_continue = true;
        let result = unsafe {
            platform_wallet_txo_reconcile_heal_page_applied(
                handle,
                wallet_id.as_ptr(),
                &mut may_continue,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(may_continue, "one page left");
        classify_empty();
        assert_eq!(
            classify_empty(),
            (true, false),
            "budget spent, nothing held"
        );

        let mut rows: *const TxoHealRowFFI = std::ptr::null();
        let mut count = 0usize;
        let mut has_more = true;
        let mut spent = false;
        let mut held: *mut TxoReconcileHeldVerdictsFFI = std::ptr::null_mut();
        let result = unsafe {
            platform_wallet_txo_reconcile_heal_page(
                handle,
                wallet_id.as_ptr(),
                10,
                &mut rows,
                &mut count,
                &mut has_more,
                &mut spent,
                &mut held,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(spent);
        assert!(held.is_null() && rows.is_null() && !has_more);
        reconciler.finish(&wallet_id, true);
        let destroyed = unsafe { crate::manager::platform_wallet_manager_destroy(handle) };
        assert_eq!(destroyed.code, PlatformWalletFFIResultCode::Success);
    }

    /// A heal page holds the engine for the run that claimed the wallet, and
    /// for no one else.
    #[test]
    fn a_heal_page_is_held_only_under_a_run_claim() {
        use crate::test_fixtures::mock_manager;

        let handle = PLATFORM_WALLET_MANAGER_STORAGE.insert(mock_manager());
        let wallet_id = [0x6bu8; 32];
        let (reconciler, engine) = PLATFORM_WALLET_MANAGER_STORAGE
            .with_item(handle, |m| (m.txo_reconciler(), m.wallet_manager_arc()))
            .expect("manager");
        let heal_page = |held: &mut *mut TxoReconcileHeldVerdictsFFI| {
            let mut rows: *const TxoHealRowFFI = std::ptr::null();
            let mut count = 0usize;
            let mut has_more = true;
            let result = unsafe {
                platform_wallet_txo_reconcile_heal_page(
                    handle,
                    wallet_id.as_ptr(),
                    10,
                    &mut rows,
                    &mut count,
                    &mut has_more,
                    &mut false,
                    held,
                )
            };
            assert!(rows.is_null(), "the wallet holds no coins");
            assert_eq!(count, 0);
            assert!(!has_more);
            result
        };

        let mut held: *mut TxoReconcileHeldVerdictsFFI = std::ptr::dangling_mut();
        let mut result = heal_page(&mut held);
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "no run claim"
        );
        assert!(held.is_null());
        unsafe { crate::error::platform_wallet_ffi_result_free(&mut result) };

        assert!(reconciler.claim_without_gate(&wallet_id, 0).is_some());
        let result = heal_page(&mut held);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(!held.is_null());
        assert!(
            engine.try_write().is_err(),
            "the engine is held until the page is released"
        );
        unsafe { platform_wallet_txo_reconcile_held_verdicts_free(held) };
        assert!(engine.try_write().is_ok());

        reconciler.finish(&wallet_id, false);
        drop(PLATFORM_WALLET_MANAGER_STORAGE.remove(handle));
    }

    /// Set in the child process that runs [`held_verdicts_registry_scenario`].
    const HELD_VERDICTS_CHILD: &str = "PLATFORM_WALLET_FFI_HELD_VERDICTS_CHILD";
    const WAIT: std::time::Duration = std::time::Duration::from_secs(10);

    /// The lock cycle the per-page release must not join. A run holds one
    /// page's engine guard; an engine writer (a block, a fork) queues
    /// behind it; a native read that resolved the manager under the
    /// registry's read guard parks on the engine behind that writer; a
    /// registry writer (a manager create or destroy) queues behind that
    /// reader, and parking_lot then parks every new registry reader. A
    /// release that looked the manager up in the registry would wait on
    /// the registry writer, it on the reader, the reader on the engine
    /// writer, and that writer on the guard. Freeing the held verdicts
    /// touches no registry, so the guard goes, then the writer, the reader
    /// and the registry writer in turn.
    ///
    /// The scenario runs in a child process (this test binary, re-invoked
    /// on this test alone with [`HELD_VERDICTS_CHILD`] set), as
    /// `spv_start_registry_tests` does: a regression forms a real deadlock
    /// that would hold the process-wide registry and wedge every other
    /// registry-backed test. The parent bounds the child and kills it.
    #[test]
    fn freeing_held_verdicts_completes_with_a_registry_writer_queued_behind_the_engine() {
        if std::env::var_os(HELD_VERDICTS_CHILD).is_some() {
            held_verdicts_registry_scenario();
            return;
        }
        let test_name = format!(
            "{}::freeing_held_verdicts_completes_with_a_registry_writer_queued_behind_the_engine",
            module_path!()
                .split_once("::")
                .map(|(_, path)| path)
                .expect("test module path below the crate root")
        );
        let mut child = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args(["--exact", &test_name, "--nocapture"])
            .env(HELD_VERDICTS_CHILD, "1")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("spawn the scenario child");
        let deadline = std::time::Instant::now() + 2 * WAIT;
        let status = loop {
            if let Some(status) = child.try_wait().expect("child status") {
                break Some(status);
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                break None;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        let output = child.wait_with_output().expect("reap the scenario child");
        let transcript = format!(
            "--- child stdout ---\n{}\n--- child stderr ---\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        match status {
            Some(status) => assert!(
                status.success(),
                "the scenario failed in the child process ({status})\n{transcript}"
            ),
            None => {
                panic!("the scenario deadlocked: the child was killed\n{transcript}")
            }
        }
    }

    fn held_verdicts_registry_scenario() {
        use std::sync::mpsc;
        use std::sync::Arc;

        use crate::test_fixtures::mock_manager;

        /// Poll `condition` until it holds, failing after [`WAIT`].
        fn wait_for(what: &str, condition: impl Fn() -> bool) {
            let deadline = std::time::Instant::now() + WAIT;
            while !condition() {
                assert!(std::time::Instant::now() < deadline, "{what}");
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }

        let handle = PLATFORM_WALLET_MANAGER_STORAGE.insert(mock_manager());
        let wallet_id = [0x5au8; 32];
        let (reconciler, engine) = PLATFORM_WALLET_MANAGER_STORAGE
            .with_item(handle, |m| (m.txo_reconciler(), m.wallet_manager_arc()))
            .expect("manager");
        assert!(reconciler.claim_without_gate(&wallet_id, 0).is_some());

        // One page's verdicts, held.
        let script = [0x76u8, 0xa9, 0x14];
        let query = OutpointOwnershipQueryFFI {
            type_tag: 0,
            standard_tag: 0,
            index: 0,
            registration_index: 0,
            key_class: 0,
            user_identity_id: [0; 32],
            friend_identity_id: [0; 32],
            outpoint: OutPointFFI {
                txid: [0x22; 32],
                vout: 0,
            },
            script_pubkey: script.as_ptr(),
            script_pubkey_len: script.len(),
        };
        let mut classes = [0xFFu8; 1];
        let mut held: *mut TxoReconcileHeldVerdictsFFI = std::ptr::null_mut();
        let result = unsafe {
            platform_wallet_txo_reconcile_classify_outpoints(
                handle,
                wallet_id.as_ptr(),
                &query,
                1,
                classes.as_mut_ptr(),
                &mut false,
                &mut held,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(!held.is_null());

        // An engine writer queued behind the guard. With the guard held, a
        // fresh engine read is refused exactly while a writer waits.
        let writer_engine = Arc::clone(&engine);
        let writer = std::thread::spawn(move || drop(writer_engine.blocking_write()));
        wait_for("the engine writer never queued behind the guard", || {
            engine.try_read().is_err()
        });

        // A native read holding the registry's read guard, parked on the
        // engine behind that writer.
        let (reading_tx, reading_rx) = mpsc::channel::<()>();
        let reader = std::thread::spawn(move || {
            PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
                let engine = manager.wallet_manager_arc();
                reading_tx.send(()).expect("test observes the reader");
                drop(engine.blocking_read());
            })
        });
        reading_rx
            .recv_timeout(WAIT)
            .expect("the reader holds the registry's read guard");

        // A registry writer queued behind that reader.
        let other = mock_manager();
        let registry_writer =
            std::thread::spawn(move || PLATFORM_WALLET_MANAGER_STORAGE.insert(other));
        wait_for("the registry writer never queued behind the reader", || {
            PLATFORM_WALLET_MANAGER_STORAGE.try_read_refused()
        });

        // The per-page release.
        let held_address = held as usize;
        let (freed_tx, freed_rx) = mpsc::channel::<()>();
        std::thread::spawn(move || {
            unsafe {
                platform_wallet_txo_reconcile_held_verdicts_free(
                    held_address as *mut TxoReconcileHeldVerdictsFFI,
                )
            };
            freed_tx.send(()).expect("test observes the release");
        });
        freed_rx
            .recv_timeout(WAIT)
            .expect("the release completes without the registry");

        writer.join().expect("engine writer");
        assert!(reader.join().expect("registry reader").is_some());
        let other = registry_writer.join().expect("registry writer");

        reconciler.finish(&wallet_id, false);
        drop(PLATFORM_WALLET_MANAGER_STORAGE.remove(other));
        drop(PLATFORM_WALLET_MANAGER_STORAGE.remove(handle));
    }

    #[test]
    fn a_panic_in_a_reconcile_export_is_an_error_result() {
        let mut result = catch_reconcile_panic("platform_wallet_txo_reconcile_heal_page", || {
            panic!("worker panicked")
        });
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        let message = unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned();
        assert!(
            message.contains("platform_wallet_txo_reconcile_heal_page panicked: worker panicked"),
            "{message}"
        );
        unsafe { crate::error::platform_wallet_ffi_result_free(&mut result) };

        let result = catch_reconcile_panic("x", PlatformWalletFFIResult::ok);
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::Success,
            "results pass through"
        );
    }
}
