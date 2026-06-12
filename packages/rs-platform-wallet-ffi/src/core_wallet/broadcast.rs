//! FFI bindings for CoreWallet transaction building and broadcasting.

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use rs_sdk_ffi::MnemonicResolverCoreSigner;
use rs_sdk_ffi::MnemonicResolverHandle;
use std::os::raw::c_char;
use std::str::FromStr;

/// Broadcast a signed transaction to the network.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_broadcast_transaction(
    handle: Handle,
    tx_bytes: *const u8,
    tx_bytes_len: usize,
    out_txid: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(tx_bytes);
    check_ptr!(out_txid);

    let bytes = std::slice::from_raw_parts(tx_bytes, tx_bytes_len);
    let tx: dashcore::Transaction =
        unwrap_result_or_return!(dashcore::consensus::deserialize(bytes));

    let option = CORE_WALLET_STORAGE.with_item(handle, |wallet| {
        runtime().block_on(wallet.broadcast_transaction(&tx))
    });
    let result = unwrap_option_or_return!(option);
    let txid = unwrap_result_or_return!(result);
    let c_str = unwrap_result_or_return!(std::ffi::CString::new(txid.to_string()));
    *out_txid = c_str.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Build, sign, and broadcast a payment to the given addresses via
/// an external mnemonic-resolver-backed signer.
///
/// The Swift caller supplies a [`MnemonicResolverHandle`] — the same
/// vtable shape used by
/// [`crate::dash_sdk_sign_with_mnemonic_resolver_and_path`] — which
/// the FFI wraps in a [`MnemonicResolverCoreSigner`] for the
/// lifetime of this call. Private keys never cross the FFI as raw
/// bytes; every signature is produced inside the signer's atomic
/// derive-and-sign step.
///
/// # Safety
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle`. Ownership is retained by the
///   caller — this function does NOT destroy it.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn core_wallet_send_to_addresses(
    handle: Handle,
    account_type: u32,
    account_index: u32,
    addresses: *const *const c_char,
    amounts: *const u64,
    count: usize,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_tx_bytes: *mut *mut u8,
    out_tx_len: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(core_signer_handle);
    if count > 0 {
        check_ptr!(addresses);
        check_ptr!(amounts);
    }
    check_ptr!(out_tx_bytes);
    check_ptr!(out_tx_len);

    let mut outputs = Vec::with_capacity(count);
    let addr_ptrs = std::slice::from_raw_parts(addresses, count);
    let amount_slice = std::slice::from_raw_parts(amounts, count);

    for i in 0..count {
        let c_str = unwrap_result_or_return!(std::ffi::CStr::from_ptr(addr_ptrs[i]).to_str());

        let addr = unwrap_result_or_return!(dashcore::Address::from_str(c_str)).assume_checked();

        outputs.push((addr, amount_slice[i]));
    }

    use key_wallet::account::account_type::StandardAccountType;
    let std_account_type = match account_type {
        0 => StandardAccountType::BIP44Account,
        1 => StandardAccountType::BIP32Account,
        _ => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("Unknown account type: {account_type}"),
            );
        }
    };

    let signer_addr = core_signer_handle as usize;

    let option = CORE_WALLET_STORAGE.with_item(handle, |wallet| {
        let wallet_id = wallet.wallet_id();
        let network = wallet.network();
        // SAFETY: the resolver handle is pinned alive for the duration
        // of this FFI call (see fn-level safety doc). The
        // `MnemonicResolverCoreSigner` lives on this stack frame and
        // is dropped before the function returns.
        let signer = unsafe {
            MnemonicResolverCoreSigner::new(
                signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        runtime().block_on(wallet.send_to_addresses(
            std_account_type,
            account_index,
            outputs,
            &signer,
        ))
    });
    let result = unwrap_option_or_return!(option);
    let tx = unwrap_result_or_return!(result);
    let serialized = dashcore::consensus::serialize(&tx);
    let len = serialized.len();
    let boxed = serialized.into_boxed_slice();
    *out_tx_bytes = Box::into_raw(boxed) as *mut u8;
    *out_tx_len = len;
    PlatformWalletFFIResult::ok()
}

/// Sweep the entire spendable balance of the wallet's CoinJoin account
/// (BIP44 purpose 4') to `dest_address` across one or more transactions,
/// leaving no change so the account is fully emptied.
///
/// CoinJoin "mixed coins" live on a dedicated account that
/// [`core_wallet_send_to_addresses`] cannot reach (it only handles
/// standard BIP44/BIP32 accounts). Used by the DashSync → SwiftDashSDK
/// migration to recover a user's mixed coins (no longer supported) into
/// their spendable balance. Uses the same external mnemonic-resolver
/// signer model as [`core_wallet_send_to_addresses`].
///
/// The sweep is split into one or more transactions because a heavy mixer's
/// UTXO set can exceed a single transaction's relay-size limit. On success,
/// `out_txids` receives a heap buffer of `*out_count` consecutive 32-byte
/// transaction ids in **wire order** (Dash Core's internal orientation — the
/// reverse of the hex shown in block explorers), in chunk order. Free the
/// buffer with [`core_wallet_free_tx_bytes`], passing `*out_count * 32` as the
/// length. The serialized transactions themselves are not returned — the
/// caller only needs the ids (to group the resulting withdrawals).
///
/// # Safety
/// - `dest_address` must be a valid NUL-terminated C string.
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle`. Ownership is retained by the caller.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_sweep_coinjoin(
    handle: Handle,
    account_index: u32,
    dest_address: *const c_char,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_txids: *mut *mut u8,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(dest_address);
    check_ptr!(core_signer_handle);
    check_ptr!(out_txids);
    check_ptr!(out_count);

    let dest_str = unwrap_result_or_return!(std::ffi::CStr::from_ptr(dest_address).to_str());
    // Parse without committing to a network; the destination is validated
    // against the wallet's own network inside the storage closure below so a
    // wrong-network address fails before any tx is built. This is the all-funds
    // sweep, so the destination script must be spendable on this chain.
    let dest_unchecked = unwrap_result_or_return!(dashcore::Address::from_str(dest_str));

    let signer_addr = core_signer_handle as usize;

    let option = CORE_WALLET_STORAGE.with_item(handle, |wallet| {
        let wallet_id = wallet.wallet_id();
        let network = wallet.network();
        // Reject a wrong-network destination before building/broadcasting the
        // all-funds sweep — the resulting script would be unspendable on this
        // chain and the sweep is irreversible. Mirrors the withdrawal FFI.
        let dest = dest_unchecked
            .require_network(network)
            .map_err(|e| platform_wallet::PlatformWalletError::AddressOperation(e.to_string()))?;
        // SAFETY: the resolver handle is pinned alive for the duration of
        // this FFI call (see fn-level safety doc). The
        // `MnemonicResolverCoreSigner` lives on this stack frame and is
        // dropped before the function returns.
        let signer = unsafe {
            MnemonicResolverCoreSigner::new(
                signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        runtime().block_on(wallet.sweep_coinjoin_to_address(account_index, dest, &signer))
    });
    let result = unwrap_option_or_return!(option);
    let txs = unwrap_result_or_return!(result);

    // Emit the chunks' txids as a contiguous `count * 32` byte buffer in wire
    // order (`Txid::as_byte_array`) — the orientation the app records and
    // groups withdrawals by. Free with `core_wallet_free_tx_bytes(ptr,
    // count * 32)`. `txs` is never empty here (the core sweep errors if no
    // transaction broadcast), so `out_count >= 1` on success.
    use dashcore::hashes::Hash; // brings `Txid::as_byte_array` into scope
    let count = txs.len();
    let mut buf: Vec<u8> = Vec::with_capacity(count * 32);
    for tx in &txs {
        let txid = tx.txid();
        buf.extend_from_slice(txid.as_byte_array());
    }
    let boxed = buf.into_boxed_slice();
    *out_txids = Box::into_raw(boxed) as *mut u8;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

/// Widen the wallet's CoinJoin account (BIP44 purpose 4') address gap limit
/// to `gap_limit` and generate the addresses so SPV watches the wider window.
///
/// Used by the DashSync → SwiftDashSDK migration's one-time CoinJoin recovery
/// scan. CoinJoin mixed coins are scattered with holes wider than the default
/// gap (30), so for wallets that used CoinJoin the app widens the gap (to match
/// DashSync's 400) before starting SPV, then reverts once the coins are swept.
/// The widened limit is in-memory only and not persisted.
///
/// On success, `out_highest_index` receives the pool's highest generated
/// address index after generation. Idempotent: re-running is a cheap no-op
/// once the window is covered.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_set_coinjoin_gap_limit(
    handle: Handle,
    account_index: u32,
    gap_limit: u32,
    out_highest_index: *mut u32,
) -> PlatformWalletFFIResult {
    check_ptr!(out_highest_index);

    let option = CORE_WALLET_STORAGE.with_item(handle, |wallet| {
        runtime().block_on(wallet.set_coinjoin_gap_limit(account_index, gap_limit))
    });
    let result = unwrap_option_or_return!(option);
    let highest = unwrap_result_or_return!(result);
    *out_highest_index = highest;
    PlatformWalletFFIResult::ok()
}

/// Free a heap buffer returned by this crate's CoreWallet FFI:
/// - the serialized transaction bytes from `core_wallet_send_to_addresses`
///   (pass the returned `out_tx_len` as `len`), or
/// - the contiguous `count * 32` byte txid buffer from
///   `core_wallet_sweep_coinjoin` (pass `out_count * 32` as `len`).
#[no_mangle]
pub unsafe extern "C" fn core_wallet_free_tx_bytes(bytes: *mut u8, len: usize) {
    if !bytes.is_null() && len > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(bytes, len));
    }
}
