//! Handle management, balance, and network queries for CoreWallet.

use crate::error::*;
use crate::handle::*;
use crate::{check_ptr, unwrap_option_or_return};

/// Destroy a CoreWallet handle.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_destroy(handle: Handle) -> PlatformWalletFFIResult {
    CORE_WALLET_STORAGE.remove(handle);
    PlatformWalletFFIResult::ok()
}

/// Get lock-free balance (spendable, unconfirmed, immature, locked).
///
/// These are atomic reads — no lock contention.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_get_balance(
    handle: Handle,
    out_confirmed: *mut u64,
    out_unconfirmed: *mut u64,
    out_immature: *mut u64,
    out_locked: *mut u64,
) -> PlatformWalletFFIResult {
    let option = CORE_WALLET_STORAGE.with_item(handle, |wallet| {
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

/// Get the network this wallet operates on.
///
/// Returns: 0 = Mainnet, 1 = Testnet, 2 = Devnet, 3 = Regtest.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_get_network(
    handle: Handle,
    out_network: *mut u32,
) -> PlatformWalletFFIResult {
    check_ptr!(out_network);

    let option = CORE_WALLET_STORAGE.with_item(handle, |wallet| match wallet.network() {
        key_wallet::Network::Mainnet => 0u32,
        key_wallet::Network::Testnet => 1,
        key_wallet::Network::Devnet => 2,
        key_wallet::Network::Regtest => 3,
    });
    *out_network = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}
