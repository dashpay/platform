//! FFI bindings for CoreWallet transaction broadcasting.

use super::transaction_builder::{CoreAccountTypeFFI, FFICoreTransaction};
use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use std::os::raw::c_char;

/// Broadcast a transaction built by `core_wallet_tx_builder_build_signed`.
///
/// `account_type`/`account_index` identify the funding account handed to
/// `core_wallet_tx_builder_set_funding` when the transaction was built: on a
/// definitive broadcast rejection its UTXO reservation is released so an
/// immediate retry can reselect the inputs; an ambiguous failure keeps it.
/// `CoinJoin` funding has no standard-account reservation to reconcile and is
/// broadcast plainly.
///
/// # Safety
/// `handle` must be a valid core-wallet handle; `tx` must be a valid,
/// non-null pointer to an `FFICoreTransaction`; `out_txid` must be writable.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_broadcast_transaction(
    handle: Handle,
    tx: *const FFICoreTransaction,
    account_type: CoreAccountTypeFFI,
    account_index: u32,
    out_txid: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(tx);
    check_ptr!(out_txid);

    let tx: dashcore::Transaction =
        unwrap_result_or_return!(dashcore::consensus::deserialize((*tx).bytes()));

    let option = CORE_WALLET_STORAGE.with_item(handle, |wallet| {
        runtime().block_on(async {
            match account_type.as_standard_account_type() {
                Some(account_type) => {
                    wallet
                        .broadcast_transaction_releasing_reservation(
                            account_type,
                            account_index,
                            &tx,
                        )
                        .await
                }
                None => wallet.broadcast_transaction(&tx).await,
            }
        })
    });

    let result = unwrap_option_or_return!(option);

    let txid = unwrap_result_or_return!(result);
    let c_str = unwrap_result_or_return!(std::ffi::CString::new(txid.to_string()));
    *out_txid = c_str.into_raw();

    PlatformWalletFFIResult::ok()
}
