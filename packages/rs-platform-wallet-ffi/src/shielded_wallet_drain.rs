//! Pooled Core-to-shielded Max with a stable, per-operation recovery identity.

use dpp::address_funds::OrchardAddress;
use platform_wallet::wallet::asset_lock::{
    AssetLockFunding, AssetLockOperationContext, AssetLockOperationSnapshot,
    AssetLockOperationState,
};
use platform_wallet::wallet::shielded::wallet_drain::{
    WalletDrainPreview, WalletDrainPreviewStatus,
};
use platform_wallet::wallet::shielded::wallet_drain_recovery::{
    wallet_drain_attempt_finished, wallet_drain_lock_listing, wallet_drain_marker_decision,
    wallet_drain_preview_allows_start, wallet_drain_recovery_action, wallet_drain_start_offered,
    wallet_drain_status_during_host_call, WalletDrainLockListing, WalletDrainMarkerDecision,
    WalletDrainRecoveryAction, WalletDrainStatusRead,
};
use platform_wallet::wallet::shielded::CachedOrchardProver;
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle};

use crate::check_ptr;
use crate::core_wallet_types::OutPointFFI;
use crate::error::{catch_panic_to_code, PlatformWalletFFIResult, PlatformWalletFFIResultCode};
use crate::handle::Handle;
use crate::persistence::{funding_type_from_u8, status_from_u8};
use crate::runtime::block_on_worker;
use crate::shielded_send::{catch_funding_panic, resolve_wallet_and_coordinator};
use crate::types::FFINetwork;

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
/// This export does not ask [`platform_wallet_wallet_drain_start_offered`]:
/// a host that starts a new drain asks it first and does not call this where
/// it answers `false` (mainnet, until dashpay/platform#5580).
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
    let mut registration = None;
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
            registration =
                Some(wallet.register_wallet_drain_operation(Arc::clone(&worker_operation)));
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
                        &CachedOrchardProver,
                        None,
                        0,
                        None,
                        None,
                        // No built-lock observer: the caller persisted the
                        // operation UUID before this call, the lock's row
                        // carries it, and `out_outcome` reports the outpoint.
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
    drop(registration);
    result
}

/// Read-only: the durable row, the live row and the active context. Never
/// touches the network.
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
        let (wallet, _) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
            Ok(value) => value,
            Err(error) => return error,
        };
        match block_on_worker(async move {
            wallet
                .shielded_wallet_drain_operation_status(operation_id)
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

// ── Recovery rules ────────────────────────────────────────────────────
//
// Pure projections of `platform_wallet::wallet::shielded::wallet_drain_recovery`,
// which holds the rules. A host gathers the facts it owns and does what these
// return; none of them reads a wallet or touches the network.

/// Decode the `state` byte of a [`WalletDrainOutcomeFFI`].
fn operation_state_from_u8(state: u8) -> Result<AssetLockOperationState, PlatformWalletFFIResult> {
    match state {
        0 => Ok(AssetLockOperationState::NotSent),
        1 => Ok(AssetLockOperationState::Pending),
        2 => Ok(AssetLockOperationState::Completed),
        other => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("unknown wallet drain outcome state {other}"),
        )),
    }
}

/// One status read as a host passes it: `read_succeeded` false is a failed
/// read, and `reported_state` is then ignored.
fn status_read_from_ffi(
    read_succeeded: bool,
    reported_state: u8,
) -> Result<WalletDrainStatusRead, PlatformWalletFFIResult> {
    if read_succeeded {
        operation_state_from_u8(reported_state).map(WalletDrainStatusRead::Reported)
    } else {
        Ok(WalletDrainStatusRead::Failed)
    }
}

/// Whether a host keeps the operation id it stored, after one status read:
/// `wallet_drain_marker_decision`.
///
/// `read_succeeded` says whether `platform_wallet_manager_shielded_wallet_drain_status`
/// returned success; `reported_state` is then its `state` and is ignored
/// otherwise. `attempt_overlapped_read` says whether one of the host's attempts
/// for this operation id ran or started while that read was in progress.
/// `out_keep` receives `true` to keep the stored id and `false` to remove it;
/// it is `true` whenever the call fails.
///
/// # Safety
/// `out_keep` is writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_wallet_drain_marker_decision(
    read_succeeded: bool,
    reported_state: u8,
    attempt_overlapped_read: bool,
    out_keep: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_keep);
    *out_keep = true;
    let read = match status_read_from_ffi(read_succeeded, reported_state) {
        Ok(read) => read,
        Err(error) => return error,
    };
    *out_keep = match wallet_drain_marker_decision(read, attempt_overlapped_read) {
        WalletDrainMarkerDecision::Keep => true,
        WalletDrainMarkerDecision::Forget => false,
    };
    PlatformWalletFFIResult::ok()
}

/// What a host offers for a stored operation id: `wallet_drain_recovery_action`.
///
/// `attempt_in_flight` says whether one of the host's attempts for the
/// operation is running. `has_read` is false before any status read has
/// finished; `read_succeeded` and `reported_state` then describe the last one
/// as in [`platform_wallet_wallet_drain_marker_decision`], and
/// `read_named_asset_lock` is its `has_out_point`. `out_action` receives
/// 0 = in progress, 1 = resume the named outpoint, 2 = check again (read the
/// status, send nothing); it is 2 whenever the call fails.
///
/// # Safety
/// `out_action` is writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_wallet_drain_recovery_action(
    attempt_in_flight: bool,
    has_read: bool,
    read_succeeded: bool,
    reported_state: u8,
    read_named_asset_lock: bool,
    out_action: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(out_action);
    *out_action = 2;
    let last_read = if has_read {
        match status_read_from_ffi(read_succeeded, reported_state) {
            Ok(read) => Some(read),
            Err(error) => return error,
        }
    } else {
        None
    };
    *out_action =
        match wallet_drain_recovery_action(attempt_in_flight, last_read, read_named_asset_lock) {
            WalletDrainRecoveryAction::InProgress => 0,
            WalletDrainRecoveryAction::Resume => 1,
            WalletDrainRecoveryAction::CheckAgain => 2,
        };
    PlatformWalletFFIResult::ok()
}

/// Whether a returned drain attempt needs nothing more, from the `state` of
/// its [`WalletDrainOutcomeFFI`]: `wallet_drain_attempt_finished`. An attempt
/// that is not finished is shown as failed, with the error its call returned.
/// `out_finished` is `false` whenever the call fails.
///
/// # Safety
/// `out_finished` is writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_wallet_drain_attempt_finished(
    state: u8,
    out_finished: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_finished);
    *out_finished = false;
    match operation_state_from_u8(state) {
        Ok(state) => {
            *out_finished = wallet_drain_attempt_finished(state);
            PlatformWalletFFIResult::ok()
        }
        Err(error) => error,
    }
}

/// Whether a host may start a drain after a preview with this
/// [`WalletDrainPreviewFFI::status`]: `wallet_drain_preview_allows_start`.
/// `out_allowed` is `false` whenever the call fails.
///
/// # Safety
/// `out_allowed` is writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_wallet_drain_preview_allows_start(
    preview_status: u8,
    out_allowed: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_allowed);
    *out_allowed = false;
    let status = match preview_status {
        0 => WalletDrainPreviewStatus::Ready,
        1 => WalletDrainPreviewStatus::InsufficientFunds,
        2 => WalletDrainPreviewStatus::InputLimitMayBeExceeded,
        other => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("unknown wallet drain preview status {other}"),
            )
        }
    };
    *out_allowed = wallet_drain_preview_allows_start(status);
    PlatformWalletFFIResult::ok()
}

/// Whether a host offers to start a new drain on `network`:
/// `wallet_drain_start_offered`. Recovering an existing drain is offered on
/// every network and does not ask this. `false` if the rule panics.
#[no_mangle]
pub extern "C" fn platform_wallet_wallet_drain_start_offered(network: FFINetwork) -> bool {
    std::panic::catch_unwind(|| wallet_drain_start_offered(network.into())).unwrap_or(false)
}

/// The state a host reports for an operation while one of its own drain
/// calls for the same id may not have reached the wallet yet:
/// `wallet_drain_status_during_host_call`.
///
/// `reported_state` is the `state` a successful
/// `platform_wallet_manager_shielded_wallet_drain_status` returned, and
/// `host_call_active` says whether the host has a
/// `platform_wallet_manager_shielded_fund_from_wallet_drain` call for the
/// same operation id that has not returned. `out_state` receives the state to
/// report, in the same encoding; it is 1 (pending) whenever the call fails.
///
/// # Safety
/// `out_state` is writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_wallet_drain_status_during_host_call(
    reported_state: u8,
    host_call_active: bool,
    out_state: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(out_state);
    *out_state = 1;
    match operation_state_from_u8(reported_state) {
        Ok(reported) => {
            *out_state = match wallet_drain_status_during_host_call(reported, host_call_active) {
                AssetLockOperationState::NotSent => 0,
                AssetLockOperationState::Pending => 1,
                AssetLockOperationState::Completed => 2,
            };
            PlatformWalletFFIResult::ok()
        }
        Err(error) => error,
    }
}

/// Where a host lists a tracked asset lock, as far as drains are concerned:
/// `wallet_drain_lock_listing`.
///
/// `funding_type` and `status` are the row's `AssetLockEntryFFI` bytes.
/// `has_operation_id` says whether the row carries an operation id, and
/// `host_holds_stored_id` whether the host holds a stored id equal to it.
/// `out_listing` receives 0 = not decided here (the host's own rule for
/// resumable locks applies), 1 = hidden (the stored operation's row owns it),
/// 2 = offered with the resumable locks; it is 0 whenever the call fails.
///
/// # Safety
/// `out_listing` is writable.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_wallet_drain_lock_listing(
    funding_type: u8,
    status: u8,
    has_operation_id: bool,
    host_holds_stored_id: bool,
    out_listing: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(out_listing);
    *out_listing = 0;
    let (funding_type, status) = match (funding_type_from_u8(funding_type), status_from_u8(status))
    {
        (Ok(funding_type), Ok(status)) => (funding_type, status),
        (Err(error), _) | (_, Err(error)) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                error.to_string(),
            )
        }
    };
    *out_listing = match wallet_drain_lock_listing(
        funding_type,
        status,
        has_operation_id,
        host_holds_stored_id,
    ) {
        WalletDrainLockListing::Unaffected => 0,
        WalletDrainLockListing::Hidden => 1,
        WalletDrainLockListing::Offered => 2,
    };
    PlatformWalletFFIResult::ok()
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
        assert_eq!(result.code as i32, 60);
        unsafe { crate::error::platform_wallet_ffi_result_free(&mut result) };
    }

    #[test]
    fn should_project_the_recovery_rules_and_fail_closed_on_an_unknown_state() {
        let mut keep = false;
        let mut result =
            unsafe { platform_wallet_wallet_drain_marker_decision(true, 2, true, &mut keep) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(!keep, "completed: forget");
        result = unsafe { platform_wallet_wallet_drain_marker_decision(true, 0, true, &mut keep) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(keep, "not sent, overlapped by an attempt: keep");
        // A failed read ignores the state byte, even an undeclared one.
        keep = false;
        result =
            unsafe { platform_wallet_wallet_drain_marker_decision(false, 9, false, &mut keep) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(keep);
        keep = false;
        result = unsafe { platform_wallet_wallet_drain_marker_decision(true, 9, false, &mut keep) };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert!(keep, "an undecodable state keeps the stored id");
        unsafe { crate::error::platform_wallet_ffi_result_free(&mut result) };

        let mut action = 9;
        result = unsafe {
            platform_wallet_wallet_drain_recovery_action(false, true, true, 1, true, &mut action)
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(action, 1, "pending with a named lock: resume");
        result = unsafe {
            platform_wallet_wallet_drain_recovery_action(false, false, true, 1, true, &mut action)
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(action, 2, "no read yet: check again");
        result = unsafe {
            platform_wallet_wallet_drain_recovery_action(true, false, false, 9, false, &mut action)
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(action, 0, "a running attempt wins");
        result = unsafe {
            platform_wallet_wallet_drain_recovery_action(false, true, true, 9, true, &mut action)
        };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert_eq!(action, 2, "an undecodable state never offers a spend");
        unsafe { crate::error::platform_wallet_ffi_result_free(&mut result) };

        let mut state = 9;
        result =
            unsafe { platform_wallet_wallet_drain_status_during_host_call(0, true, &mut state) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(state, 1, "not sent while a host call is active: pending");
        result =
            unsafe { platform_wallet_wallet_drain_status_during_host_call(0, false, &mut state) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(state, 0);
        result =
            unsafe { platform_wallet_wallet_drain_status_during_host_call(2, true, &mut state) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(state, 2, "the wallet's own answer stands");
        result =
            unsafe { platform_wallet_wallet_drain_status_during_host_call(9, true, &mut state) };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert_eq!(state, 1, "an undecodable state is reported as pending");
        unsafe { crate::error::platform_wallet_ffi_result_free(&mut result) };

        let mut finished = true;
        result = unsafe { platform_wallet_wallet_drain_attempt_finished(1, &mut finished) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(!finished);
        result = unsafe { platform_wallet_wallet_drain_attempt_finished(2, &mut finished) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(finished);

        let mut allowed = true;
        result = unsafe { platform_wallet_wallet_drain_preview_allows_start(1, &mut allowed) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(!allowed, "insufficient funds");
        result = unsafe { platform_wallet_wallet_drain_preview_allows_start(2, &mut allowed) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(allowed, "the input limit warning does not refuse");
        result = unsafe { platform_wallet_wallet_drain_preview_allows_start(3, &mut allowed) };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert!(!allowed);
        unsafe { crate::error::platform_wallet_ffi_result_free(&mut result) };

        assert!(!platform_wallet_wallet_drain_start_offered(
            FFINetwork::Mainnet
        ));
        assert!(platform_wallet_wallet_drain_start_offered(
            FFINetwork::Testnet
        ));

        let mut listing = 9;
        // Shielded top-up (5), Built (0), with an id the host holds no marker for.
        result =
            unsafe { platform_wallet_wallet_drain_lock_listing(5, 0, true, false, &mut listing) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(listing, 2);
        result =
            unsafe { platform_wallet_wallet_drain_lock_listing(5, 1, true, true, &mut listing) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(listing, 1);
        result =
            unsafe { platform_wallet_wallet_drain_lock_listing(5, 0, false, false, &mut listing) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(listing, 0);
        result =
            unsafe { platform_wallet_wallet_drain_lock_listing(5, 99, true, true, &mut listing) };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert_eq!(listing, 0);
        unsafe { crate::error::platform_wallet_ffi_result_free(&mut result) };
    }
}
