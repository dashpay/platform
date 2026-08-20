//! FFI bindings for `PlatformWalletManager`'s recurring DashPay
//! (contact-request + profile) sync coordinator.
//!
//! Mirrors the shape of [`crate::identity_sync`] /
//! [`crate::platform_address_sync`]: lifecycle controls (`start` /
//! `stop` / `is_running` / `is_syncing` / `last_sync_unix_seconds` /
//! `set_interval` / `sync_now`). Unlike the identity-token coordinator
//! the DashPay sweep is **wallet-driven, not registry-driven** (see
//! [`DashPaySyncManager`](platform_wallet::manager::dashpay_sync::DashPaySyncManager)),
//! so there is no per-identity registry surface here — every registered
//! wallet is swept on every pass.
//!
//! `sync_now` differs from the identity/shielded `sync_now` in one way:
//! the underlying [`DashPaySyncManager::sync_now`] returns a
//! [`DashPaySyncSummary`], so this entry point surfaces the per-pass
//! success / error counts and completion timestamp through out-params
//! (the host's "Sync Now" button can report "synced N wallets"). All
//! three out-params are optional — pass null to ignore any of them.
//!
//! Not auto-started — exactly like the sibling coordinators. The Swift
//! lifecycle calls [`platform_wallet_manager_dashpay_sync_start`] once
//! the wallets are registered and the SDK is connected; the on-demand
//! `sync_now` entry point stays available for pull-to-refresh.
//!
//! Error handling follows the same shape as the rest of this crate:
//! every entry point returns a `PlatformWalletFFIResult`; null `Handle`
//! lookups surface through `unwrap_option_or_return!` and out-pointer
//! validation through `check_ptr!`.

use std::time::Duration;

use crate::error::*;
use crate::handle::*;
use crate::runtime::{runtime, try_block_on_worker};
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};

/// Start the recurring DashPay sync loop in the background. Idempotent
/// — calling while already running is a no-op.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_dashpay_sync_start(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        // The loop's `tokio::spawn` needs a runtime in scope, so acquisition
        // is fallible here: with no runtime there is nothing to start the
        // DashPay loop on, and that has to be reported rather than
        // silently skipped.
        let _entered = runtime().checked()?.enter();
        manager.dashpay_sync_arc().start();
        Ok::<(), crate::panic_guard::FfiBoundaryError>(())
    });
    unwrap_result_or_return!(unwrap_option_or_return!(option));
    PlatformWalletFFIResult::ok()
}

/// Stop the recurring DashPay sync loop if it is running.
///
/// Cancel-only: a pass already inside `sync_now` keeps running to
/// completion. Manager shutdown uses the Rust-side `quiesce` barrier;
/// the host does not need to.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_dashpay_sync_stop(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager.dashpay_sync().stop();
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Whether the recurring DashPay sync background loop is running.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_dashpay_sync_is_running(
    handle: Handle,
    out_running: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_running);
    // Define the out-slot before the stale-handle early return below can fire,
    // so the caller never reads uninitialized stack contents.
    *out_running = false;

    let option = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(handle, |manager| manager.dashpay_sync().is_running());
    let running = unwrap_option_or_return!(option);
    *out_running = running;
    PlatformWalletFFIResult::ok()
}

/// Whether a DashPay sync pass is currently in flight.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_dashpay_sync_is_syncing(
    handle: Handle,
    out_syncing: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_syncing);
    *out_syncing = false;

    let option = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(handle, |manager| manager.dashpay_sync().is_syncing());
    let syncing = unwrap_option_or_return!(option);
    *out_syncing = syncing;
    PlatformWalletFFIResult::ok()
}

/// Unix seconds of the last completed DashPay sync pass, or 0 if no
/// pass has ever completed.
///
/// Unlike the identity-token coordinator this watermark is global (one
/// last-sync per manager, not per-identity), matching the
/// wallet-driven sweep.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_dashpay_sync_last_sync_unix_seconds(
    handle: Handle,
    out_last_sync_unix: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_last_sync_unix);
    *out_last_sync_unix = 0;

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager.dashpay_sync().last_sync_unix_seconds()
    });
    let value = unwrap_option_or_return!(option);
    *out_last_sync_unix = value.unwrap_or(0);
    PlatformWalletFFIResult::ok()
}

/// Set the background DashPay sync interval in seconds.
///
/// Clamped to a minimum of 1s on the Rust side; the running loop picks
/// up the new interval on its next sleep.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_dashpay_sync_set_interval(
    handle: Handle,
    interval_seconds: u64,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        manager
            .dashpay_sync()
            .set_interval(Duration::from_secs(interval_seconds));
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Run one DashPay sync pass across every registered wallet.
///
/// Synchronous from the FFI caller's point of view — blocks the
/// calling thread until the pass completes. If a pass is already in
/// flight (e.g. fired by the background loop), the underlying manager
/// skips and returns an empty summary immediately; this function then
/// reports `*out_success_count == 0`, `*out_error_count == 0`, and
/// `*out_sync_unix_seconds == 0` (the "no pass ran" sentinel). Check
/// `is_syncing` if the caller needs to distinguish "skipped" from
/// "swept zero wallets".
///
/// All three out-params are optional — pass null to ignore any of
/// them:
///   * `out_success_count`: wallets whose `dashpay_sync()` succeeded.
///   * `out_error_count`: wallets whose `dashpay_sync()` failed (logged
///     Rust-side, non-fatal to the rest of the pass).
///   * `out_sync_unix_seconds`: Unix seconds the pass completed, or `0`
///     if no pass ran.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_dashpay_sync_sync_now(
    handle: Handle,
    out_success_count: *mut usize,
    out_error_count: *mut usize,
    out_sync_unix_seconds: *mut u64,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        let mgr = manager.dashpay_sync_arc();
        // `block_on_worker`, NOT `runtime().block_on`: the sync pass
        // verifies GroveDB document-query proofs whose recursion blows
        // the ~512 KB stack of the iOS calling thread (SIGBUS observed
        // on-device 2026-06-12). The worker dispatch moves the compute
        // onto the runtime's 8 MB-stack threads (see runtime.rs).
        try_block_on_worker(async move { mgr.sync_now().await })
    });
    let summary = unwrap_result_or_return!(unwrap_option_or_return!(option));

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

    /// Every DashPay-sync entry point must reject an unknown `Handle`
    /// with `NotFound` rather than dereferencing a stale slot — the
    /// `unwrap_option_or_return!` contract every other coordinator's
    /// FFI upholds. Pins the null-handle path for all seven calls.
    #[test]
    fn unknown_handle_returns_not_found() {
        let bogus: Handle = 0xDEAD_BEEF;

        let r = unsafe { platform_wallet_manager_dashpay_sync_start(bogus) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);

        let r = unsafe { platform_wallet_manager_dashpay_sync_stop(bogus) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);

        let mut running = true;
        let r = unsafe { platform_wallet_manager_dashpay_sync_is_running(bogus, &mut running) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);

        let mut syncing = true;
        let r = unsafe { platform_wallet_manager_dashpay_sync_is_syncing(bogus, &mut syncing) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);

        let mut last = 123u64;
        let r = unsafe {
            platform_wallet_manager_dashpay_sync_last_sync_unix_seconds(bogus, &mut last)
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);

        let r = unsafe { platform_wallet_manager_dashpay_sync_set_interval(bogus, 30) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);

        let mut ok = 7usize;
        let mut err = 7usize;
        let mut ts = 7u64;
        let r = unsafe {
            platform_wallet_manager_dashpay_sync_sync_now(bogus, &mut ok, &mut err, &mut ts)
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);
    }

    /// Null out-pointers on the reader entry points must be rejected
    /// with `ErrorNullPointer` (the `check_ptr!` contract) before the
    /// handle is even looked up — guarding against a host that forgets
    /// to pass storage for a required scalar out-param.
    #[test]
    fn null_required_out_pointers_are_rejected() {
        let bogus: Handle = 1;

        let r =
            unsafe { platform_wallet_manager_dashpay_sync_is_running(bogus, std::ptr::null_mut()) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        let r =
            unsafe { platform_wallet_manager_dashpay_sync_is_syncing(bogus, std::ptr::null_mut()) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        let r = unsafe {
            platform_wallet_manager_dashpay_sync_last_sync_unix_seconds(bogus, std::ptr::null_mut())
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }
}
