//! FFI bindings for CoreWallet transaction broadcasting.

use super::transaction_builder::{CoreAccountTypeFFI, FFICoreTransaction};
use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use std::os::raw::c_char;

/// Consume and broadcast an atomically finalized V2 transaction.
///
/// Success and `MaybeSent` both permanently consume the handle. A definitive
/// rejection also consumes it after releasing the reservation. This prevents
/// accidental rebroadcast through the same ownership token.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_broadcast_signed_transaction_v2(
    handle: Handle,
    transaction_handle: Handle,
    out_txid: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_txid);
    *out_txid = std::ptr::null_mut();

    // Ownership crosses into this call. Consume first; every later validation
    // failure explicitly abandons through the embedded originating wallet.
    let finalized =
        unwrap_option_or_return!(CORE_SIGNED_TRANSACTION_V2_STORAGE.remove(transaction_handle));
    let Some(wallet) = CORE_WALLET_STORAGE.with_item(handle, Clone::clone) else {
        runtime().block_on(finalized.wallet.abandon_transaction(&finalized.transaction));
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "invalid core wallet handle".to_string(),
        );
    };
    if wallet.wallet_id() != finalized.wallet.wallet_id() {
        runtime().block_on(finalized.wallet.abandon_transaction(&finalized.transaction));
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "transaction was finalized by a different wallet".to_string(),
        );
    }
    let result = runtime().block_on(
        finalized
            .wallet
            .broadcast_finalized_transaction(&finalized.transaction),
    );
    let txid = unwrap_result_or_return!(result);
    let c_str = unwrap_result_or_return!(std::ffi::CString::new(txid.to_string()));
    *out_txid = c_str.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Consume a finalized transaction without broadcasting and release its input
/// reservation immediately. Repeating abandon/free for the same handle is a
/// safe invalid-handle error; no freed pointer is dereferenced.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_abandon_signed_transaction_v2(
    handle: Handle,
    transaction_handle: Handle,
) -> PlatformWalletFFIResult {
    let transaction =
        unwrap_option_or_return!(CORE_SIGNED_TRANSACTION_V2_STORAGE.remove(transaction_handle));
    let Some(wallet) = CORE_WALLET_STORAGE.with_item(handle, Clone::clone) else {
        runtime().block_on(
            transaction
                .wallet
                .abandon_transaction(&transaction.transaction),
        );
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "invalid core wallet handle".to_string(),
        );
    };
    if wallet.wallet_id() != transaction.wallet.wallet_id() {
        runtime().block_on(
            transaction
                .wallet
                .abandon_transaction(&transaction.transaction),
        );
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "transaction was finalized by a different wallet".to_string(),
        );
    }
    runtime().block_on(
        transaction
            .wallet
            .abandon_transaction(&transaction.transaction),
    );
    PlatformWalletFFIResult::ok()
}

/// Idempotent ownership cleanup for a V2 handle when no transient wallet handle
/// is available. It abandons the transaction and releases its reservation.
#[no_mangle]
pub extern "C" fn core_wallet_signed_transaction_v2_free(transaction_handle: Handle) {
    if let Some(transaction) = CORE_SIGNED_TRANSACTION_V2_STORAGE.remove(transaction_handle) {
        runtime().block_on(
            transaction
                .wallet
                .abandon_transaction(&transaction.transaction),
        );
    }
}

#[no_mangle]
pub unsafe extern "C" fn core_wallet_signed_transaction_v2_fee(
    transaction_handle: Handle,
    out_fee: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_fee);
    let fee =
        unwrap_option_or_return!(CORE_SIGNED_TRANSACTION_V2_STORAGE
            .with_item(transaction_handle, |tx| tx.transaction.fee()));
    *out_fee = fee;
    PlatformWalletFFIResult::ok()
}

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

#[cfg(test)]
mod tests {
    use std::ptr;

    use dashcore::{Address, Network};
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
    use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
    use platform_wallet::test_support::{funded_spv_core_wallet, WalletSigner};
    use platform_wallet::{CoreWallet, SignedCoreTransaction};

    use super::*;
    use crate::core_wallet::FFICoreSignedTransactionV2;

    type TestCore = CoreWallet<platform_wallet::broadcaster::SpvBroadcaster>;

    fn finalize(core: &TestCore, signer: &WalletSigner, tag: u8) -> SignedCoreTransaction {
        runtime()
            .block_on(core.finalize_transaction(
                TransactionBuilder::new().add_output(
                    &Address::dummy(Network::Testnet, usize::from(tag)),
                    1_000_000,
                ),
                AccountTypePreference::BIP44,
                0,
                signer,
            ))
            .expect("finalize test transaction")
    }

    fn insert(core: &TestCore, transaction: SignedCoreTransaction) -> Handle {
        CORE_SIGNED_TRANSACTION_V2_STORAGE.insert(FFICoreSignedTransactionV2 {
            wallet: core.clone(),
            transaction,
        })
    }

    fn assert_released(core: &TestCore, signer: &WalletSigner, tag: u8) {
        let retry = finalize(core, signer, tag);
        runtime().block_on(core.abandon_transaction(&retry));
    }

    #[test]
    fn double_free_is_safe_and_releases_reservation() {
        let (core, signer) =
            runtime().block_on(funded_spv_core_wallet(StandardAccountType::BIP44Account));
        let transaction_handle = insert(&core, finalize(&core, &signer, 40));

        core_wallet_signed_transaction_v2_free(transaction_handle);
        core_wallet_signed_transaction_v2_free(transaction_handle);

        assert_released(&core, &signer, 41);
    }

    #[test]
    fn invalid_or_wrong_wallet_consumes_and_releases() {
        let (origin, origin_signer) =
            runtime().block_on(funded_spv_core_wallet(StandardAccountType::BIP44Account));
        let invalid_transaction = insert(&origin, finalize(&origin, &origin_signer, 42));
        let invalid =
            unsafe { core_wallet_abandon_signed_transaction_v2(u64::MAX, invalid_transaction) };
        assert_eq!(
            invalid.code,
            PlatformWalletFFIResultCode::ErrorInvalidHandle
        );
        assert_released(&origin, &origin_signer, 43);

        let (other, _) =
            runtime().block_on(funded_spv_core_wallet(StandardAccountType::BIP44Account));
        let other_handle = CORE_WALLET_STORAGE.insert(other);
        let wrong_transaction = insert(&origin, finalize(&origin, &origin_signer, 44));
        let wrong =
            unsafe { core_wallet_abandon_signed_transaction_v2(other_handle, wrong_transaction) };
        assert_eq!(
            wrong.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert_released(&origin, &origin_signer, 45);
        CORE_WALLET_STORAGE.remove(other_handle);
    }

    #[test]
    fn abandon_then_free_or_broadcast_cannot_reconsume_handle() {
        let (core, signer) =
            runtime().block_on(funded_spv_core_wallet(StandardAccountType::BIP44Account));
        let core_handle = CORE_WALLET_STORAGE.insert(core.clone());
        let transaction_handle = insert(&core, finalize(&core, &signer, 46));

        let abandoned =
            unsafe { core_wallet_abandon_signed_transaction_v2(core_handle, transaction_handle) };
        assert_eq!(abandoned.code, PlatformWalletFFIResultCode::Success);
        core_wallet_signed_transaction_v2_free(transaction_handle);

        let mut txid = ptr::null_mut();
        let rebroadcast = unsafe {
            core_wallet_broadcast_signed_transaction_v2(core_handle, transaction_handle, &mut txid)
        };
        assert_eq!(rebroadcast.code, PlatformWalletFFIResultCode::NotFound);
        assert!(txid.is_null());
        assert_released(&core, &signer, 47);
        CORE_WALLET_STORAGE.remove(core_handle);
    }
}
