//! FFI bindings for a wallet's locked outpoints: the outpoints kept out of
//! coin selection.
//!
//! The wallet locks the collateral of a masternode registration once the
//! registration is in a block (a collateral the ProRegTx creates as its own
//! output, from any sighting), since spending the collateral would end the
//! registration.
//! These calls list the locks and lock or unlock an outpoint by hand; a lock
//! or unlock is persisted before the call returns (see
//! `PlatformWallet::lock_outpoint`).

use crate::check_ptr;
use crate::core_wallet_types::OutPointFFI;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// List the outpoints the wallet keeps out of coin selection, in outpoint
/// order. An entry may name a coin the wallet does not hold (yet).
///
/// The caller owns the array and frees it with
/// [`platform_wallet_locked_outpoints_free`]. `out_outpoints` / `out_count`
/// are set to null / 0 when nothing is locked.
///
/// # Safety
/// `out_outpoints` and `out_count` must be writable. `handle` must refer to
/// a live platform wallet for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_locked_outpoints(
    handle: Handle,
    out_outpoints: *mut *const OutPointFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_outpoints);
    check_ptr!(out_count);
    *out_outpoints = std::ptr::null();
    *out_count = 0;

    let option = PLATFORM_WALLET_STORAGE.with_item(handle, |wallet| {
        let wallet = wallet.clone();
        block_on_worker(async move { wallet.locked_outpoints().await })
    });
    let result = unwrap_option_or_return!(option);
    let outpoints: Vec<OutPointFFI> = unwrap_result_or_return!(result)
        .iter()
        .map(OutPointFFI::from)
        .collect();
    if outpoints.is_empty() {
        return PlatformWalletFFIResult::ok();
    }
    *out_count = outpoints.len();
    *out_outpoints = Box::into_raw(outpoints.into_boxed_slice()) as *const OutPointFFI;
    PlatformWalletFFIResult::ok()
}

/// Free an array returned by [`platform_wallet_locked_outpoints`].
///
/// # Safety
/// `outpoints` / `count` must be exactly what that call returned, freed once.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_locked_outpoints_free(
    outpoints: *mut OutPointFFI,
    count: usize,
) {
    if !outpoints.is_null() && count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(outpoints, count));
    }
}

/// Lock `outpoint`, so no send, asset lock or special-transaction fee spends
/// it until [`platform_wallet_unlock_outpoint`]. The wallet does not need to
/// hold the coin yet. `out_changed` is set to whether the outpoint was not
/// locked before.
///
/// The lock is persisted before this returns. On a persistence error the
/// wallet still holds the lock in memory, but it does not survive a restart.
///
/// # Safety
/// `outpoint` must be readable and `out_changed` writable. `handle` must
/// refer to a live platform wallet for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_lock_outpoint(
    handle: Handle,
    outpoint: *const OutPointFFI,
    out_changed: *mut bool,
) -> PlatformWalletFFIResult {
    set_outpoint_lock(handle, outpoint, out_changed, true)
}

/// Unlock `outpoint`, so coin selection may spend it again. Unlocking a
/// masternode collateral lets a send spend it, and spending it ends the
/// masternode registration. `out_changed` is set to whether the outpoint
/// was locked.
///
/// The unlock is persisted before this returns. On a persistence error the
/// outpoint is unlocked in memory but comes back locked after a restart.
///
/// # Safety
/// As [`platform_wallet_lock_outpoint`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_unlock_outpoint(
    handle: Handle,
    outpoint: *const OutPointFFI,
    out_changed: *mut bool,
) -> PlatformWalletFFIResult {
    set_outpoint_lock(handle, outpoint, out_changed, false)
}

unsafe fn set_outpoint_lock(
    handle: Handle,
    outpoint: *const OutPointFFI,
    out_changed: *mut bool,
    locked: bool,
) -> PlatformWalletFFIResult {
    check_ptr!(outpoint);
    check_ptr!(out_changed);
    *out_changed = false;
    let outpoint = dashcore::OutPoint::from(&*outpoint);

    let option = PLATFORM_WALLET_STORAGE.with_item(handle, |wallet| {
        let wallet = wallet.clone();
        block_on_worker(async move {
            if locked {
                wallet.lock_outpoint(outpoint).await
            } else {
                wallet.unlock_outpoint(outpoint).await
            }
        })
    });
    let result = unwrap_option_or_return!(option);
    *out_changed = unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::runtime;
    use platform_wallet::test_support::test_platform_wallet_manager;

    unsafe fn listed(handle: Handle) -> Vec<(u8, u32)> {
        let mut ptr: *const OutPointFFI = std::ptr::null();
        let mut count = 0usize;
        let result = platform_wallet_locked_outpoints(handle, &mut ptr, &mut count);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        if count == 0 {
            assert!(ptr.is_null());
            return Vec::new();
        }
        let entries = std::slice::from_raw_parts(ptr, count)
            .iter()
            .map(|outpoint| (outpoint.txid[0], outpoint.vout))
            .collect();
        platform_wallet_locked_outpoints_free(ptr as *mut OutPointFFI, count);
        entries
    }

    #[test]
    fn should_lock_list_and_unlock_an_outpoint_through_the_ffi() {
        let (manager, handle) = runtime().block_on(async {
            let (manager, wallet_id) = test_platform_wallet_manager().await;
            let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
            (manager, PLATFORM_WALLET_STORAGE.insert(wallet))
        });
        let outpoint = OutPointFFI {
            txid: [0x33; 32],
            vout: 2,
        };
        let mut changed = false;
        unsafe {
            assert!(listed(handle).is_empty());

            let result = platform_wallet_lock_outpoint(handle, &outpoint, &mut changed);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert!(changed, "the outpoint was not locked before");
            assert_eq!(listed(handle), vec![(0x33, 2)]);

            let result = platform_wallet_unlock_outpoint(handle, &outpoint, &mut changed);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert!(changed, "the outpoint was locked");
            assert!(listed(handle).is_empty());

            let result = platform_wallet_unlock_outpoint(handle, &outpoint, &mut changed);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert!(!changed, "unlocking twice changes nothing");
        }
        PLATFORM_WALLET_STORAGE.remove(handle);
        drop(manager);
    }
}
