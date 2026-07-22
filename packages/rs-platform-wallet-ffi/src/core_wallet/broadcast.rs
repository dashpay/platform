//! FFI bindings for CoreWallet transaction broadcasting.

use super::transaction_builder::{CoreAccountTypeFFI, FFICoreTransaction};
use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use platform_wallet::PlatformWalletError;
use std::os::raw::c_char;

fn classify_broadcast_result(
    result: Result<dashcore::Txid, PlatformWalletError>,
    local_txid: dashcore::Txid,
) -> (Option<dashcore::Txid>, PlatformWalletFFIResult) {
    match result {
        Ok(_) => (Some(local_txid), PlatformWalletFFIResult::ok()),
        Err(error @ PlatformWalletError::TransactionBroadcast(_))
        | Err(error @ PlatformWalletError::TransactionBroadcastUnconfirmed(_)) => {
            (Some(local_txid), error.into())
        }
        Err(error) => (None, error.into()),
    }
}

/// Consume and broadcast an atomically finalized V2 transaction.
///
/// Success and `MaybeSent` both permanently consume the handle. A definitive
/// rejection also consumes it after releasing the reservation. This prevents
/// accidental rebroadcast through the same ownership token.
///
/// A handle whose wallet generation is no longer registered in the manager
/// (removed, or re-created under the same id) is refused with `NotFound` (98)
/// **before** the network is touched; the handle is consumed and its reservation
/// reconciled. This mirrors the deferred-token path's `WalletRemoved` → 98.
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
    // Same generation identity the registry-token path uses: reject a caller
    // handle that names a different wallet generation (e.g. a re-created wallet
    // under the same id) before acting through the embedded originating wallet.
    if !wallet.is_same_generation(&finalized.wallet) {
        runtime().block_on(finalized.wallet.abandon_transaction(&finalized.transaction));
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "transaction was finalized by a different wallet generation".to_string(),
        );
    }
    let local_txid = finalized.transaction.transaction().txid();

    // Hold this generation's lifecycle gate across BOTH the liveness check and
    // the send. The `is_same_generation` check above compares two HANDLES, so it
    // passes for a removed generation — both sides name the same removed wallet —
    // and nothing further down re-checks: `broadcast_finalized_transaction` goes
    // straight to the broadcaster with no manager lookup. Without this, two
    // retained handles push a deleted wallet's transaction onto the network,
    // where it can conflict with inputs a re-created generation has since
    // selected (`dashpay/platform#4185`).
    //
    // The gate makes this atomic rather than check-then-act: a teardown takes the
    // exclusive side, so it cannot interleave between the check and the send.
    // Scoped per generation, so this send — up to the broadcaster's timeout —
    // blocks only THIS wallet's teardown, never an unrelated wallet's.
    let (_lifecycle, wallet_is_live) = runtime().block_on(async {
        let gate = wallet.generation_payment_guard().await;
        let live = wallet.is_current_generation().await;
        (gate, live)
    });
    if !wallet_is_live {
        runtime().block_on(finalized.wallet.abandon_transaction(&finalized.transaction));
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            "wallet is no longer registered in the manager (removed or re-created); the \
             transaction was NOT broadcast and its reservation was reconciled"
                .to_string(),
        );
    }

    let result = runtime().block_on(
        finalized
            .wallet
            .broadcast_finalized_transaction(&finalized.transaction),
    );
    let (txid, ffi_result) = classify_broadcast_result(result, local_txid);
    let Some(txid) = txid else {
        return ffi_result;
    };
    let c_str = unwrap_result_or_return!(std::ffi::CString::new(txid.to_string()));
    *out_txid = c_str.into_raw();
    ffi_result
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
    // Same generation identity as the broadcast path / registry-token path.
    if !wallet.is_same_generation(&transaction.wallet) {
        runtime().block_on(
            transaction
                .wallet
                .abandon_transaction(&transaction.transaction),
        );
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "transaction was finalized by a different wallet generation".to_string(),
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
/// On accepted, rejected, and unknown network outcomes `out_txid` receives a
/// Rust-owned C string that the caller frees with
/// `platform_wallet_string_free`. Operational errors leave it null.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_broadcast_transaction(
    handle: Handle,
    tx: *const FFICoreTransaction,
    account_type: CoreAccountTypeFFI,
    account_index: u32,
    out_txid: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_txid);
    *out_txid = std::ptr::null_mut();
    check_ptr!(tx);

    let tx: dashcore::Transaction =
        unwrap_result_or_return!(dashcore::consensus::deserialize((*tx).bytes()));
    let local_txid = tx.txid();

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

    let (txid, ffi_result) = classify_broadcast_result(result, local_txid);
    let Some(txid) = txid else {
        return ffi_result;
    };
    let c_str = unwrap_result_or_return!(std::ffi::CString::new(txid.to_string()));
    *out_txid = c_str.into_raw();

    ffi_result
}

#[cfg(test)]
mod outcome_tests {
    use dashcore::hashes::Hash;

    use super::*;

    fn txid(byte: u8) -> dashcore::Txid {
        dashcore::Txid::from_byte_array([byte; 32])
    }

    #[test]
    fn network_outcomes_all_carry_a_txid() {
        let accepted = classify_broadcast_result(Ok(txid(1)), txid(9));
        assert_eq!(accepted.0, Some(txid(9)));
        assert_eq!(accepted.1.code, PlatformWalletFFIResultCode::Success);

        let rejected = classify_broadcast_result(
            Err(PlatformWalletError::TransactionBroadcast(
                "rejected".to_string(),
            )),
            txid(2),
        );
        assert_eq!(rejected.0, Some(txid(2)));
        assert_eq!(
            rejected.1.code,
            PlatformWalletFFIResultCode::ErrorTransactionBroadcastRejected
        );

        let unknown = classify_broadcast_result(
            Err(PlatformWalletError::TransactionBroadcastUnconfirmed(
                "timeout".to_string(),
            )),
            txid(3),
        );
        assert_eq!(unknown.0, Some(txid(3)));
        assert_eq!(
            unknown.1.code,
            PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed
        );
    }

    #[test]
    fn operational_error_does_not_carry_a_txid() {
        let outcome = classify_broadcast_result(
            Err(PlatformWalletError::TransactionBuild("invalid".to_string())),
            txid(4),
        );
        assert_eq!(outcome.0, None);
        assert_eq!(outcome.1.code, PlatformWalletFFIResultCode::ErrorUnknown);
    }
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

    /// Prove the funding reservation is *still held*: a fresh finalize of the
    /// same size cannot reselect the single fixture UTXO, so it fails at the
    /// build stage. Used to show an aged abandon/free skipped the by-outpoint
    /// release (leaving the input reserved for key-wallet's TTL).
    fn assert_still_reserved(core: &TestCore, signer: &WalletSigner, tag: u8) {
        let rebuild = runtime().block_on(core.finalize_transaction(
            TransactionBuilder::new().add_output(
                &Address::dummy(Network::Testnet, usize::from(tag)),
                1_000_000,
            ),
            AccountTypePreference::BIP44,
            0,
            signer,
        ));
        assert!(
            rebuild.is_err(),
            "aged abandon/free must skip the release, leaving the input reserved; \
             got {rebuild:?}"
        );
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

    /// The deinit/GC backstop (`core_wallet_signed_transaction_v2_free`) is the
    /// exact path shumkov flagged: a `FinalizedCoreTransaction` never broadcast
    /// or abandoned, freed by the host GC long after finalize. If the reservation
    /// has aged past the guard bound the free must **skip** the by-outpoint
    /// release — key-wallet's TTL may already have swept and re-reserved the
    /// outpoint, and releasing it would free that newer build's reservation. The
    /// handle is still torn down (the storage entry is removed) so a re-free is a
    /// safe no-op.
    #[test]
    fn aged_v2_free_skips_reservation_release() {
        let (core, signer) =
            runtime().block_on(funded_spv_core_wallet(StandardAccountType::BIP44Account));
        let transaction_handle = insert(&core, finalize(&core, &signer, 48));

        // Age the pinned handle past the guard bound (still below the TTL, so the
        // reservation is provably still held — only the software guard trips).
        runtime().block_on(platform_wallet::test_support::age_core_past_reservation_guard(&core));

        core_wallet_signed_transaction_v2_free(transaction_handle);

        // The aged free skipped the release: the input is still reserved.
        assert_still_reserved(&core, &signer, 49);
        // Handle is gone regardless — a re-free is a harmless no-op.
        core_wallet_signed_transaction_v2_free(transaction_handle);
    }

    /// The FFI broadcast/abandon *failure* paths (invalid or wrong-generation
    /// wallet handle) route their cleanup through `abandon_transaction`, so they
    /// inherit the same age guard: when the handle has aged out, the failure-path
    /// cleanup must skip the by-outpoint release rather than free a possibly
    /// re-reserved outpoint.
    #[test]
    fn aged_failure_path_abandon_skips_reservation_release() {
        let (origin, signer) =
            runtime().block_on(funded_spv_core_wallet(StandardAccountType::BIP44Account));
        let transaction_handle = insert(&origin, finalize(&origin, &signer, 50));

        runtime().block_on(platform_wallet::test_support::age_core_past_reservation_guard(&origin));

        // Invalid wallet handle → routes through abandon_transaction, then returns
        // ErrorInvalidHandle. The embedded aged reservation must be left alone.
        let invalid =
            unsafe { core_wallet_abandon_signed_transaction_v2(u64::MAX, transaction_handle) };
        assert_eq!(
            invalid.code,
            PlatformWalletFFIResultCode::ErrorInvalidHandle
        );
        assert_still_reserved(&origin, &signer, 51);
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
