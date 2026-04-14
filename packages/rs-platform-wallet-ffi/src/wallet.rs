//! FFI bindings for PlatformWallet (sub-wallet access, balance, persistence).

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;

/// Get the wallet ID (32 bytes).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_id(
    handle: Handle,
    out_wallet_id: *mut [u8; 32],
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_wallet_id.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    PLATFORM_WALLET_STORAGE
        .with_item(handle, |wallet| {
            *out_wallet_id = wallet.wallet_id();
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Get lock-free balance (spendable, unconfirmed, immature, locked).
///
/// These are atomic reads — no lock contention.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_balance(
    handle: Handle,
    out_confirmed: *mut u64,
    out_unconfirmed: *mut u64,
    out_immature: *mut u64,
    out_locked: *mut u64,
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    PLATFORM_WALLET_STORAGE
        .with_item(handle, |wallet| {
            let balance = wallet.balance();
            if !out_confirmed.is_null() {
                *out_confirmed = balance.confirmed();
            }
            if !out_unconfirmed.is_null() {
                *out_unconfirmed = balance.unconfirmed();
            }
            if !out_immature.is_null() {
                *out_immature = balance.immature();
            }
            if !out_locked.is_null() {
                *out_locked = balance.locked();
            }
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Get a PlatformAddressWallet handle from a PlatformWallet.
///
/// The returned handle is a clone (cheap — all Arc internals).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_platform(
    handle: Handle,
    out_platform_handle: *mut Handle,
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_platform_handle.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    PLATFORM_WALLET_STORAGE
        .with_item(handle, |wallet| {
            let platform_wallet = wallet.platform().clone();
            let platform_handle = PLATFORM_ADDRESS_WALLET_STORAGE.insert(platform_wallet);
            *out_platform_handle = platform_handle;
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Get an AssetLockManager handle from a PlatformWallet.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_asset_locks(
    handle: Handle,
    out_asset_lock_handle: *mut Handle,
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_asset_lock_handle.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    PLATFORM_WALLET_STORAGE
        .with_item(handle, |wallet| {
            let asset_locks = std::sync::Arc::clone(wallet.asset_locks());
            let asset_lock_handle = ASSET_LOCK_MANAGER_STORAGE.insert(asset_locks);
            *out_asset_lock_handle = asset_lock_handle;
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Get a CoreWallet handle from a PlatformWallet.
///
/// The returned handle is a clone (cheap — all Arc internals).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_core(
    handle: Handle,
    out_core_handle: *mut Handle,
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_core_handle.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    PLATFORM_WALLET_STORAGE
        .with_item(handle, |wallet| {
            let core_wallet = wallet.core().clone();
            let core_handle = CORE_WALLET_STORAGE.insert(core_wallet);
            *out_core_handle = core_handle;
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Flush all queued changesets to the storage backend.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_flush_persist(
    handle: Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    PLATFORM_WALLET_STORAGE
        .with_item(handle, |wallet| match wallet.flush_persist() {
            Ok(()) => PlatformWalletFFIResult::Success,
            Err(e) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorWalletOperation,
                        e.to_string(),
                    );
                }
                PlatformWalletFFIResult::ErrorWalletOperation
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Load persisted state and apply it to the in-memory wallet.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_load_and_apply_persisted(
    handle: Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    PLATFORM_WALLET_STORAGE
        .with_item(handle, |wallet| {
            match runtime().block_on(wallet.load_and_apply_persisted()) {
                Ok(()) => PlatformWalletFFIResult::Success,
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            e.to_string(),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Destroy a PlatformWallet handle.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_destroy(
    handle: Handle,
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    PLATFORM_WALLET_STORAGE.remove(handle);
    PlatformWalletFFIResult::Success
}
