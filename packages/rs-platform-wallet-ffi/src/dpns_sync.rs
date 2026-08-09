//! FFI bindings for `PlatformWalletManager`'s recurring DPNS
//! username-marketplace sync coordinator.
//!
//! Sibling of [`crate::dashpay_sync`] and shaped identically: lifecycle
//! controls (`start` / `stop` / `is_running` / `is_syncing` /
//! `last_sync_unix_seconds` / `set_interval` / `sync_now`). The sweep is
//! **wallet-driven, not registry-driven** (see
//! [`DpnsSyncManager`](platform_wallet::manager::dpns_sync::DpnsSyncManager)),
//! so there is no per-identity registry surface here — every registered
//! wallet is swept on every pass. It is a separate coordinator from the
//! DashPay one because marketplace state changes are rare: this loop
//! defaults to 60s against DashPay's 15s.
//!
//! `sync_now` surfaces the per-pass success / error counts and
//! completion timestamp through out-params; all three are optional —
//! pass null to ignore any of them. For a single wallet's delta (names
//! tracked / added / departed / re-priced) use the per-wallet
//! [`platform_wallet_dpns_marketplace_sync`](crate::dpns_marketplace::platform_wallet_dpns_marketplace_sync)
//! instead.
//!
//! Not auto-started. The host lifecycle calls
//! [`platform_wallet_manager_dpns_sync_start`] once the wallets are
//! registered and the SDK is connected; the on-demand `sync_now` entry
//! point stays available for pull-to-refresh.

use std::time::Duration;

use crate::error::*;
use crate::handle::*;
use crate::runtime::{block_on_worker, runtime};
use crate::{check_ptr, unwrap_option_or_return};

/// Start the recurring DPNS marketplace sync loop in the background.
/// Idempotent — calling while already running is a no-op.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_dpns_sync_start(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        let _entered = runtime().enter();
        manager.dpns_sync_arc().start();
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Stop the recurring DPNS marketplace sync loop if it is running.
///
/// Cancel-only: a pass already inside `sync_now` keeps running to
/// completion. Manager shutdown uses the Rust-side `quiesce` barrier;
/// the host does not need to.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_dpns_sync_stop(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager.dpns_sync().stop();
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Whether the recurring DPNS marketplace sync background loop is
/// running.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_dpns_sync_is_running(
    handle: Handle,
    out_running: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_running);
    // Define the out-slot before the stale-handle early return below can
    // fire, so the caller never reads uninitialized stack contents.
    *out_running = false;

    let option =
        PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| manager.dpns_sync().is_running());
    let running = unwrap_option_or_return!(option);
    *out_running = running;
    PlatformWalletFFIResult::ok()
}

/// Whether a DPNS marketplace sync pass is currently in flight.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_dpns_sync_is_syncing(
    handle: Handle,
    out_syncing: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_syncing);
    *out_syncing = false;

    let option =
        PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| manager.dpns_sync().is_syncing());
    let syncing = unwrap_option_or_return!(option);
    *out_syncing = syncing;
    PlatformWalletFFIResult::ok()
}

/// Unix seconds of the last completed DPNS marketplace sync pass, or 0
/// if no pass has ever completed.
///
/// The watermark is global (one last-sync per manager, not per-wallet),
/// matching the wallet-driven sweep.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_dpns_sync_last_sync_unix_seconds(
    handle: Handle,
    out_last_sync_unix: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_last_sync_unix);
    *out_last_sync_unix = 0;

    let option = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(handle, |manager| manager.dpns_sync().last_sync_unix_seconds());
    let value = unwrap_option_or_return!(option);
    *out_last_sync_unix = value.unwrap_or(0);
    PlatformWalletFFIResult::ok()
}

/// Set the background DPNS marketplace sync interval in seconds.
///
/// Clamped to a minimum of 1s on the Rust side; the running loop picks
/// up the new interval on its next sleep.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_dpns_sync_set_interval(
    handle: Handle,
    interval_seconds: u64,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager
            .dpns_sync()
            .set_interval(Duration::from_secs(interval_seconds));
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Run one DPNS marketplace sync pass across every registered wallet.
///
/// Synchronous from the FFI caller's point of view — blocks the calling
/// thread until the pass completes. If a pass is already in flight (e.g.
/// fired by the background loop), the underlying manager skips and
/// returns an empty summary immediately; this function then reports
/// `*out_success_count == 0`, `*out_error_count == 0`, and
/// `*out_sync_unix_seconds == 0` (the "no pass ran" sentinel). Check
/// `is_syncing` if the caller needs to distinguish "skipped" from
/// "swept zero wallets".
///
/// All three out-params are optional — pass null to ignore any of them:
///   * `out_success_count`: wallets whose marketplace sync succeeded.
///   * `out_error_count`: wallets whose marketplace sync failed (logged
///     Rust-side, non-fatal to the rest of the pass).
///   * `out_sync_unix_seconds`: Unix seconds the pass completed, or `0`
///     if no pass ran.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_dpns_sync_sync_now(
    handle: Handle,
    out_success_count: *mut usize,
    out_error_count: *mut usize,
    out_sync_unix_seconds: *mut u64,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        let mgr = manager.dpns_sync_arc();
        // `block_on_worker`, NOT `runtime().block_on`: the pass verifies
        // GroveDB document-query proofs whose recursion blows the ~512 KB
        // stack of the iOS calling thread. The worker dispatch moves the
        // compute onto the runtime's 8 MB-stack threads (see runtime.rs).
        block_on_worker(async move { mgr.sync_now().await })
    });
    let summary = unwrap_option_or_return!(option);

    if !out_success_count.is_null() {
        *out_success_count = summary.success_count();
    }
    if !out_error_count.is_null() {
        *out_error_count = summary.error_count();
    }
    if !out_sync_unix_seconds.is_null() {
        *out_sync_unix_seconds = summary.sync_unix_seconds;
    }
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every DPNS-sync entry point must reject an unknown `Handle` with
    /// `NotFound` rather than dereferencing a stale slot — the
    /// `unwrap_option_or_return!` contract every other coordinator's FFI
    /// upholds. Pins the stale-handle path for all seven calls.
    #[test]
    fn unknown_handle_returns_not_found() {
        let bogus: Handle = 0xDEAD_BEEF;

        let r = unsafe { platform_wallet_manager_dpns_sync_start(bogus) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);

        let r = unsafe { platform_wallet_manager_dpns_sync_stop(bogus) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);

        let mut running = true;
        let r = unsafe { platform_wallet_manager_dpns_sync_is_running(bogus, &mut running) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);
        assert!(!running);

        let mut syncing = true;
        let r = unsafe { platform_wallet_manager_dpns_sync_is_syncing(bogus, &mut syncing) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);
        assert!(!syncing);

        let mut last = 123u64;
        let r =
            unsafe { platform_wallet_manager_dpns_sync_last_sync_unix_seconds(bogus, &mut last) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);
        assert_eq!(last, 0);

        let r = unsafe { platform_wallet_manager_dpns_sync_set_interval(bogus, 30) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);

        let mut ok = 7usize;
        let mut err = 7usize;
        let mut ts = 7u64;
        let r =
            unsafe { platform_wallet_manager_dpns_sync_sync_now(bogus, &mut ok, &mut err, &mut ts) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);
    }

    /// Null out-pointers on the reader entry points must be rejected with
    /// `ErrorNullPointer` (the `check_ptr!` contract) before the handle is
    /// even looked up.
    #[test]
    fn null_required_out_pointers_are_rejected() {
        let bogus: Handle = 1;

        let r = unsafe { platform_wallet_manager_dpns_sync_is_running(bogus, std::ptr::null_mut()) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        let r = unsafe { platform_wallet_manager_dpns_sync_is_syncing(bogus, std::ptr::null_mut()) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        let r = unsafe {
            platform_wallet_manager_dpns_sync_last_sync_unix_seconds(bogus, std::ptr::null_mut())
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }
}
