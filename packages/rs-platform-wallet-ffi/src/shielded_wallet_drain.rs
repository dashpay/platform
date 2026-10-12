//! Pooled Core-to-shielded Max with a stable, per-operation recovery identity.

use dashcore::hashes::Hash;
use dpp::address_funds::OrchardAddress;
use platform_wallet::wallet::asset_lock::{
    AssetLockFunding, AssetLockOperationContext, AssetLockOperationSnapshot,
    AssetLockOperationState,
};
use platform_wallet::wallet::shielded::wallet_drain::{
    WalletDrainPreview, WalletDrainPreviewStatus,
};
use platform_wallet::wallet::shielded::CachedOrchardProver;
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle};

use crate::check_ptr;
use crate::core_wallet_types::OutPointFFI;
use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};
use crate::handle::Handle;
use crate::runtime::block_on_worker;
use crate::shielded_send::{
    catch_funding_panic, catch_panic_to_code, resolve_wallet_and_coordinator,
};

use std::sync::Arc;

/// Read-only estimate. Candidate figures include in-flight reservations; the
/// execution builder is authoritative. Optional values have explicit flags.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct WalletDrainPreviewFFI {
    /// 0 = ready, 1 = insufficient funds, 2 = input limit may be exceeded.
    pub status: u8,
    pub candidate_input_count: u64,
    pub candidate_balance_duffs: u64,
    pub pool_fee_credits: u64,
    pub has_estimated_l1_fee_duffs: bool,
    pub estimated_l1_fee_duffs: u64,
    pub has_estimated_recipient_credits: bool,
    pub estimated_recipient_credits: u64,
}

impl From<WalletDrainPreview> for WalletDrainPreviewFFI {
    fn from(preview: WalletDrainPreview) -> Self {
        Self {
            status: match preview.status {
                WalletDrainPreviewStatus::Ready => 0,
                WalletDrainPreviewStatus::InsufficientFunds => 1,
                WalletDrainPreviewStatus::InputLimitMayBeExceeded => 2,
            },
            candidate_input_count: preview.candidate_input_count as u64,
            candidate_balance_duffs: preview.candidate_balance_duffs,
            pool_fee_credits: preview.pool_fee_credits,
            has_estimated_l1_fee_duffs: preview.estimated_l1_fee_duffs.is_some(),
            estimated_l1_fee_duffs: preview.estimated_l1_fee_duffs.unwrap_or(0),
            has_estimated_recipient_credits: preview.estimated_recipient_credits.is_some(),
            estimated_recipient_credits: preview.estimated_recipient_credits.unwrap_or(0),
        }
    }
}

/// Outcome of ONE UUID-scoped drain. Written even when execution returns an
/// error or catches a panic. An absent outpoint never licenses a fresh drain.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WalletDrainOutcomeFFI {
    /// 0 = not sent, 1 = pending/unknown, 2 = completed.
    pub state: u8,
    pub has_out_point: bool,
    pub out_point: OutPointFFI,
}

impl Default for WalletDrainOutcomeFFI {
    fn default() -> Self {
        Self {
            state: 1,
            has_out_point: false,
            out_point: OutPointFFI {
                txid: [0; 32],
                vout: 0,
            },
        }
    }
}

impl From<AssetLockOperationSnapshot> for WalletDrainOutcomeFFI {
    fn from(snapshot: AssetLockOperationSnapshot) -> Self {
        Self {
            state: match snapshot.state {
                AssetLockOperationState::NotSent => 0,
                AssetLockOperationState::Pending => 1,
                AssetLockOperationState::Completed => 2,
            },
            has_out_point: snapshot.out_point.is_some(),
            out_point: snapshot.out_point.map_or(
                OutPointFFI {
                    txid: [0; 32],
                    vout: 0,
                },
                |out_point| OutPointFFI {
                    txid: out_point.txid.to_byte_array(),
                    vout: out_point.vout,
                },
            ),
        }
    }
}

fn catch_wallet_drain_read_panic(
    operation: &str,
    body: impl FnOnce() -> PlatformWalletFFIResult,
) -> PlatformWalletFFIResult {
    catch_panic_to_code(
        operation,
        PlatformWalletFFIResultCode::ErrorWalletOperation,
        "This read did not submit a transaction; retain any earlier pending operation.",
        body,
    )
}

/// Read the context AFTER the guarded worker finishes, including when its
/// panic became a typed error. The context belongs only to this invocation.
fn wallet_drain_result(
    snapshot: impl FnOnce() -> AssetLockOperationSnapshot,
    body: impl FnOnce() -> PlatformWalletFFIResult,
) -> (PlatformWalletFFIResult, WalletDrainOutcomeFFI) {
    let result = catch_funding_panic("shielded wallet drain", body);
    (result, snapshot().into())
}

/// Estimate a pooled drain without deriving addresses, signing or reserving inputs.
///
/// # Safety
/// `wallet_id_bytes` points to 32 readable bytes; `out_preview` is writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_wallet_drain_preview(
    handle: Handle,
    wallet_id_bytes: *const u8,
    account_index: u32,
    out_preview: *mut WalletDrainPreviewFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_preview);
    *out_preview = WalletDrainPreviewFFI::default();
    check_ptr!(wallet_id_bytes);
    let mut wallet_id = [0; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    catch_wallet_drain_read_panic("shielded wallet drain preview", || {
        let (wallet, _) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
            Ok(value) => value,
            Err(error) => return error,
        };
        match block_on_worker(async move {
            wallet.shielded_wallet_drain_preview(account_index, 0).await
        }) {
            Ok(preview) => {
                *out_preview = preview.into();
                PlatformWalletFFIResult::ok()
            }
            Err(error) => error.into(),
        }
    })
}

/// Drain the permitted transparent accounts into one shielded recipient.
///
/// The caller persists its UUID before calling. Reusing a UUID must never
/// create another lock. The result's error and `out_outcome` are complementary:
/// a typed error does not erase the outpoint or authorize another fresh drain.
///
/// # Safety
/// Wallet ID, operation ID and recipient point to 32, 16 and 43 readable bytes,
/// respectively. `out_outcome` is writable. The caller keeps `core_signer_handle`
/// alive until this synchronous call returns, including worker/proving time.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_fund_from_wallet_drain(
    handle: Handle,
    wallet_id_bytes: *const u8,
    operation_id_bytes: *const u8,
    account_index: u32,
    recipient_raw_43: *const u8,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_outcome: *mut WalletDrainOutcomeFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_outcome);
    *out_outcome = WalletDrainOutcomeFFI::default();
    check_ptr!(wallet_id_bytes);
    check_ptr!(operation_id_bytes);
    check_ptr!(recipient_raw_43);
    check_ptr!(core_signer_handle);

    let mut wallet_id = [0; 32];
    let mut operation_id = [0; 16];
    let mut recipient_bytes = [0; 43];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    std::ptr::copy_nonoverlapping(operation_id_bytes, operation_id.as_mut_ptr(), 16);
    std::ptr::copy_nonoverlapping(recipient_raw_43, recipient_bytes.as_mut_ptr(), 43);

    let operation = Arc::new(AssetLockOperationContext::new(operation_id));
    let worker_operation = Arc::clone(&operation);
    let (result, outcome) = wallet_drain_result(
        || operation.snapshot(),
        || {
            let recipient = match OrchardAddress::from_raw_bytes(&recipient_bytes) {
                Ok(value) => value,
                Err(error) => {
                    return PlatformWalletFFIResult::err(
                        PlatformWalletFFIResultCode::ErrorInvalidParameter,
                        format!("invalid Orchard recipient address: {error}"),
                    )
                }
            };
            let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
                Ok(value) => value,
                Err(error) => return error,
            };
            wallet.register_wallet_drain_operation(Arc::clone(&worker_operation));
            let network = wallet.network();
            let core_signer_addr = core_signer_handle as usize;
            match block_on_worker(async move {
                // SAFETY: the caller pins the resolver for the complete FFI call.
                let signer = unsafe {
                    MnemonicResolverCoreSigner::new(
                        core_signer_addr as *mut MnemonicResolverHandle,
                        wallet_id,
                        network,
                    )
                };
                wallet
                    .shielded_fund_from_asset_lock(
                        &coordinator,
                        AssetLockFunding::DrainWalletBalance {
                            account_index,
                            minimum_lock_duffs: None,
                            operation: worker_operation,
                        },
                        vec![(recipient, None)],
                        &signer,
                        &CachedOrchardProver::new(),
                        None,
                        0,
                        None,
                        None,
                    )
                    .await
            }) {
                Ok(()) => PlatformWalletFFIResult::ok(),
                Err(error) => error.into(),
            }
        },
    );
    *out_outcome = outcome;
    result
}

/// Reconcile exact outputs from authenticated local scan data; never broadcasts.
///
/// # Safety
/// IDs point to 32 and 16 readable bytes. `out_outcome` is writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_wallet_drain_status(
    handle: Handle,
    wallet_id_bytes: *const u8,
    operation_id_bytes: *const u8,
    out_outcome: *mut WalletDrainOutcomeFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_outcome);
    *out_outcome = WalletDrainOutcomeFFI::default();
    check_ptr!(wallet_id_bytes);
    check_ptr!(operation_id_bytes);
    let mut wallet_id = [0; 32];
    let mut operation_id = [0; 16];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    std::ptr::copy_nonoverlapping(operation_id_bytes, operation_id.as_mut_ptr(), 16);
    catch_wallet_drain_read_panic("shielded wallet drain status", || {
        let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
            Ok(value) => value,
            Err(error) => return error,
        };
        match block_on_worker(async move {
            wallet
                .shielded_wallet_drain_operation_status(&coordinator, operation_id)
                .await
        }) {
            Ok(snapshot) => {
                *out_outcome = snapshot.into();
                PlatformWalletFFIResult::ok()
            }
            Err(error) => error.into(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_keep_the_exact_operation_outpoint_after_a_worker_panic() {
        let out_point = dashcore::OutPoint {
            txid: dashcore::Txid::from_byte_array([7; 32]),
            vout: 3,
        };
        let (mut result, outcome) = wallet_drain_result(
            || AssetLockOperationSnapshot {
                state: AssetLockOperationState::Pending,
                out_point: Some(out_point),
            },
            || panic!("worker failed after the lock was tracked"),
        );
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed
        );
        assert_eq!(outcome.state, 1);
        assert!(outcome.has_out_point);
        assert_eq!(outcome.out_point.txid, [7; 32]);
        assert_eq!(outcome.out_point.vout, 3);
        unsafe { crate::error::platform_wallet_ffi_result_free(&mut result) };
    }

    #[test]
    fn should_clear_outcome_to_pending_when_an_abi_argument_is_missing() {
        let mut outcome = WalletDrainOutcomeFFI {
            state: 2,
            has_out_point: true,
            out_point: OutPointFFI {
                txid: [7; 32],
                vout: 3,
            },
        };
        let mut result = unsafe {
            platform_wallet_manager_shielded_fund_from_wallet_drain(
                0,
                std::ptr::null(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null_mut(),
                &mut outcome,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert_eq!(outcome.state, 1);
        assert!(!outcome.has_out_point);
        assert_eq!(outcome.out_point.txid, [0; 32]);
        unsafe { crate::error::platform_wallet_ffi_result_free(&mut result) };
    }

    #[test]
    fn should_preserve_absent_estimates_for_an_input_limit_warning() {
        let ffi = WalletDrainPreviewFFI::from(WalletDrainPreview {
            status: WalletDrainPreviewStatus::InputLimitMayBeExceeded,
            candidate_input_count: 501,
            candidate_balance_duffs: 1_000_000,
            pool_fee_credits: 100_000,
            estimated_l1_fee_duffs: None,
            estimated_recipient_credits: None,
        });
        assert_eq!(ffi.status, 2);
        assert_eq!(ffi.candidate_input_count, 501);
        assert!(!ffi.has_estimated_l1_fee_duffs);
        assert!(!ffi.has_estimated_recipient_credits);
    }

    #[test]
    fn should_preserve_typed_input_cap_refusal() {
        let mut result: PlatformWalletFFIResult =
            platform_wallet::PlatformWalletError::AssetLockTooManyInputs {
                count: 501,
                max: 500,
            }
            .into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorAssetLockTooManyInputs
        );
        assert_eq!(result.code as i32, 59);
        unsafe { crate::error::platform_wallet_ffi_result_free(&mut result) };
    }
}
