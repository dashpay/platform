//! Handle management, balance, and network queries for CoreWallet.

use crate::error::*;
use crate::handle::*;

/// Destroy a CoreWallet handle.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_destroy(
    handle: Handle,
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    CORE_WALLET_STORAGE.remove(handle);
    PlatformWalletFFIResult::Success
}

/// Get lock-free balance (spendable, unconfirmed, immature, locked).
///
/// These are atomic reads — no lock contention.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_get_balance(
    handle: Handle,
    out_spendable: *mut u64,
    out_unconfirmed: *mut u64,
    out_immature: *mut u64,
    out_locked: *mut u64,
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    CORE_WALLET_STORAGE
        .with_item(handle, |wallet| {
            let balance = wallet.balance();
            if !out_spendable.is_null() {
                *out_spendable = balance.spendable();
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

/// Get the network this wallet operates on.
///
/// Returns: 0 = Mainnet, 1 = Testnet, 2 = Devnet, 3 = Regtest.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_get_network(
    handle: Handle,
    out_network: *mut u32,
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_network.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    CORE_WALLET_STORAGE
        .with_item(handle, |wallet| {
            *out_network = match wallet.network() {
                key_wallet::Network::Mainnet => 0,
                key_wallet::Network::Testnet => 1,
                key_wallet::Network::Devnet => 2,
                key_wallet::Network::Regtest => 3,
                _ => 1, // fallback to testnet
            };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}
