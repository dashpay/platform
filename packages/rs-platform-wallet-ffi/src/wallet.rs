//! FFI bindings for PlatformWallet (sub-wallet access, balance, persistence).

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};

/// Get the wallet ID (32 bytes).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_id(
    handle: Handle,
    out_wallet_id: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    check_ptr!(out_wallet_id);

    let option = PLATFORM_WALLET_STORAGE.with_item(handle, |wallet| wallet.wallet_id());
    *out_wallet_id = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
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
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_STORAGE.with_item(handle, |wallet| {
        let b = wallet.balance();
        (b.confirmed(), b.unconfirmed(), b.immature(), b.locked())
    });
    let (confirmed, unconfirmed, immature, locked) = unwrap_option_or_return!(option);
    if !out_confirmed.is_null() {
        *out_confirmed = confirmed;
    }
    if !out_unconfirmed.is_null() {
        *out_unconfirmed = unconfirmed;
    }
    if !out_immature.is_null() {
        *out_immature = immature;
    }
    if !out_locked.is_null() {
        *out_locked = locked;
    }
    PlatformWalletFFIResult::ok()
}

/// Get a PlatformAddressWallet handle from a PlatformWallet.
///
/// The returned handle is a clone (cheap — all Arc internals).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_platform(
    handle: Handle,
    out_platform_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_platform_handle);

    let option = PLATFORM_WALLET_STORAGE.with_item(handle, |wallet| wallet.platform().clone());
    let platform_wallet = unwrap_option_or_return!(option);
    *out_platform_handle = PLATFORM_ADDRESS_WALLET_STORAGE.insert(platform_wallet);
    PlatformWalletFFIResult::ok()
}

/// Get an AssetLockManager handle from a PlatformWallet.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_asset_locks(
    handle: Handle,
    out_asset_lock_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_asset_lock_handle);

    let option = PLATFORM_WALLET_STORAGE
        .with_item(handle, |wallet| std::sync::Arc::clone(wallet.asset_locks()));
    let asset_locks = unwrap_option_or_return!(option);
    *out_asset_lock_handle = ASSET_LOCK_MANAGER_STORAGE.insert(asset_locks);
    PlatformWalletFFIResult::ok()
}

/// Get a CoreWallet handle from a PlatformWallet.
///
/// The returned handle is a clone (cheap — all Arc internals).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_core(
    handle: Handle,
    out_core_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_core_handle);

    let option = PLATFORM_WALLET_STORAGE.with_item(handle, |wallet| wallet.core().clone());
    let core_wallet = unwrap_option_or_return!(option);
    *out_core_handle = CORE_WALLET_STORAGE.insert(core_wallet);
    PlatformWalletFFIResult::ok()
}

/// Flush all queued changesets to the storage backend.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_flush_persist(handle: Handle) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_STORAGE.with_item(handle, |wallet| wallet.flush_persist());
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

/// Load persisted state and apply it to the in-memory wallet.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_load_and_apply_persisted(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_STORAGE.with_item(handle, |wallet| {
        runtime().block_on(wallet.load_and_apply_persisted())
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
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
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(out_entries);
    check_ptr!(out_count);

    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        manager.account_balances_blocking(&wid)
    });
    let balances = unwrap_option_or_return!(option);

    let entries: Vec<crate::core_wallet_types::AccountBalanceEntryFFI> = balances
        .into_iter()
        .map(|row| {
            let tags = crate::core_wallet_types::account_type_to_tags(&row.account_type);
            crate::core_wallet_types::AccountBalanceEntryFFI {
                type_tag: tags.type_tag,
                standard_tag: tags.standard_tag,
                index: tags.index,
                registration_index: tags.registration_index,
                key_class: tags.key_class,
                user_identity_id: tags.user_identity_id,
                friend_identity_id: tags.friend_identity_id,
                confirmed: row.balance.confirmed(),
                unconfirmed: row.balance.unconfirmed(),
                immature: row.balance.immature(),
                locked: row.balance.locked(),
                keys_used: row.keys_used,
                keys_total: row.keys_total,
            }
        })
        .collect();
    let count = entries.len();

    if count == 0 {
        *out_entries = std::ptr::null();
        *out_count = 0;
        return PlatformWalletFFIResult::ok();
    }

    let boxed = entries.into_boxed_slice();
    *out_entries = Box::into_raw(boxed) as *const _;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

/// Free an array returned by [`platform_wallet_manager_get_account_balances`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_free_account_balances(
    entries: *mut crate::core_wallet_types::AccountBalanceEntryFFI,
    count: usize,
) {
    if !entries.is_null() && count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(entries, count));
    }
}

/// Destroy a PlatformWallet handle.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_destroy(handle: Handle) -> PlatformWalletFFIResult {
    PLATFORM_WALLET_STORAGE.remove(handle);
    PlatformWalletFFIResult::ok()
}
