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

/// Query per-account balances from the in-memory `WalletManager`.
///
/// Returns an array of [`AccountBalanceEntryFFI`] — one per account
/// in the wallet's `ManagedAccountCollection`. The caller owns the
/// returned array and must free it via
/// [`platform_wallet_manager_free_account_balances`].
///
/// `out_entries` receives a pointer to the heap-allocated array;
/// `out_count` receives the element count.  Both are set to
/// null / 0 when the wallet is not found.
///
/// Reads the wallet manager lock via `blocking_read` — must not be
/// called from within a tokio async context.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_get_account_balances(
    manager_handle: Handle,
    wallet_id: *const u8,
    out_entries: *mut *const crate::core_wallet_types::AccountBalanceEntryFFI,
    out_count: *mut usize,
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if wallet_id.is_null() || out_entries.is_null() || out_count.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);

    PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |manager| {
            let balances = manager.account_balances_blocking(&wid);
            let entries: Vec<crate::core_wallet_types::AccountBalanceEntryFFI> = balances
                .into_iter()
                .map(|(account_type, balance)| {
                    let tags = crate::core_wallet_types::account_type_to_tags(&account_type);
                    crate::core_wallet_types::AccountBalanceEntryFFI {
                        type_tag: tags.type_tag,
                        standard_tag: tags.standard_tag,
                        index: tags.index,
                        registration_index: tags.registration_index,
                        key_class: tags.key_class,
                        user_identity_id: tags.user_identity_id,
                        friend_identity_id: tags.friend_identity_id,
                        confirmed: balance.confirmed(),
                        unconfirmed: balance.unconfirmed(),
                        immature: balance.immature(),
                        locked: balance.locked(),
                    }
                })
                .collect();
            let count = entries.len();
            if count == 0 {
                *out_entries = std::ptr::null();
                *out_count = 0;
                return PlatformWalletFFIResult::Success;
            }
            let boxed = entries.into_boxed_slice();
            *out_entries = Box::into_raw(boxed) as *const _;
            *out_count = count;
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            *out_entries = std::ptr::null();
            *out_count = 0;
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Free an array returned by [`platform_wallet_manager_get_account_balances`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_free_account_balances(
    entries: *mut crate::core_wallet_types::AccountBalanceEntryFFI,
    count: usize,
) {
    if !entries.is_null() && count > 0 {
        let _ = Box::from_raw(std::slice::from_raw_parts_mut(entries, count));
    }
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
