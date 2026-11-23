//! JNI bridge for the pooled Core to shielded drain ("Max"): preview,
//! execution and status by a durable operation id, and the recovery rules a
//! host applies to what they report.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.WalletDrainNative`,
//! driven by the `PlatformWalletManager.shieldedWalletDrain*` wrappers and
//! `org.dashfoundation.dashsdk.wallet.WalletDrainRules`. Swift counterpart:
//! `PlatformWalletManagerWalletDrain.swift`.
//!
//! Each export is one call into `platform_wallet_ffi::shielded_wallet_drain`.
//! Nothing is decided here: the amounts, the inputs, the operation's state
//! and what a host does with it all come from Rust's wallet.
//!
//! ## The execution's two results
//!
//! `platform_wallet_manager_shielded_fund_from_wallet_drain` returns an error
//! AND writes an outcome, and the two are complementary: an error does not
//! erase the asset lock's outpoint or allow a second drain. A thrown
//! exception could carry only one of them, so the execution and the status
//! read return both in one packed array (see [`pack_outcome`]) and throw only
//! for an argument that could not be read.
//!
//! The whole module compiles only under the `shielded` cargo feature.

#![allow(clippy::missing_safety_doc)]
#![cfg(feature = "shielded")]

use crate::support::{
    guard, net_from_ord, take_pwffi_error, throw_sdk_exception, PWFFI_CODE_OFFSET,
};
use jni::objects::{JByteArray, JClass};
use jni::sys::{jboolean, jbyteArray, jint, jlong, jlongArray, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use platform_wallet_ffi::error::{
    platform_wallet_ffi_result_free, PlatformWalletFFIResult, PlatformWalletFFIResultCode,
};
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::shielded_wallet_drain::{
    platform_wallet_manager_shielded_fund_from_wallet_drain,
    platform_wallet_manager_shielded_wallet_drain_preview,
    platform_wallet_manager_shielded_wallet_drain_status,
    platform_wallet_wallet_drain_attempt_finished, platform_wallet_wallet_drain_lock_listing,
    platform_wallet_wallet_drain_marker_decision,
    platform_wallet_wallet_drain_preview_allows_start,
    platform_wallet_wallet_drain_recovery_action, platform_wallet_wallet_drain_start_offered,
    WalletDrainOutcomeFFI, WalletDrainPreviewFFI,
};
use rs_sdk_ffi::MnemonicResolverHandle;
use std::ffi::CStr;
use std::ptr;

/// Bytes of a packed outcome before its error message: state (1),
/// has-outpoint flag (1), txid (32), vout (4), error code (4), consensus
/// code (4), consensus kind (4).
pub(crate) const OUTCOME_HEADER_LEN: usize = 1 + 1 + 32 + 4 + 4 + 4 + 4;

/// Number of `long`s in a packed preview: status, candidate input count,
/// candidate balance, pool fee, has-L1-fee flag, L1 fee, has-recipient flag,
/// recipient credits.
pub(crate) const PREVIEW_LEN: usize = 8;

/// The error half of a packed outcome: `code` is 0 for success, otherwise
/// the result code shifted by [`PWFFI_CODE_OFFSET`], exactly what a thrown
/// `DashSDKException` would carry.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct OutcomeError {
    code: i32,
    consensus_code: u32,
    consensus_kind: i32,
    message: String,
}

/// Copy the error out of `result` and free its message.
fn take_outcome_error(mut result: PlatformWalletFFIResult) -> OutcomeError {
    if result.code == PlatformWalletFFIResultCode::Success {
        return OutcomeError::default();
    }
    let message = if result.message.is_null() {
        format!("platform-wallet error (code {})", result.code as i32)
    } else {
        // SAFETY: a non-null message is a valid C string produced by the FFI.
        unsafe { CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned()
    };
    let error = OutcomeError {
        code: result.code as i32 + PWFFI_CODE_OFFSET,
        consensus_code: result.consensus_code,
        consensus_kind: result.consensus_kind as i32,
        message,
    };
    // SAFETY: `result` is a fresh result this function owns.
    unsafe { platform_wallet_ffi_result_free(&mut result) };
    error
}

/// One outcome and the error returned beside it, in the layout
/// `WalletDrainOutcome.decode` reads: state, has-outpoint flag, the txid in
/// wire order, then vout, error code, consensus code and consensus kind as
/// little-endian 32-bit values, then the error message as UTF-8.
pub(crate) fn pack_outcome(outcome: &WalletDrainOutcomeFFI, error: &OutcomeError) -> Vec<u8> {
    let mut packed = Vec::with_capacity(OUTCOME_HEADER_LEN + error.message.len());
    packed.push(outcome.state);
    packed.push(u8::from(outcome.has_out_point));
    packed.extend_from_slice(&outcome.out_point.txid);
    packed.extend_from_slice(&outcome.out_point.vout.to_le_bytes());
    packed.extend_from_slice(&error.code.to_le_bytes());
    packed.extend_from_slice(&error.consensus_code.to_le_bytes());
    packed.extend_from_slice(&error.consensus_kind.to_le_bytes());
    packed.extend_from_slice(error.message.as_bytes());
    packed
}

/// One preview in the layout `WalletDrainPreview.decode` reads. The unsigned
/// 64-bit amounts cross as their two's-complement `long`.
pub(crate) fn pack_preview(preview: &WalletDrainPreviewFFI) -> [jlong; PREVIEW_LEN] {
    [
        jlong::from(preview.status),
        preview.candidate_input_count as jlong,
        preview.candidate_balance_duffs as jlong,
        preview.pool_fee_credits as jlong,
        jlong::from(preview.has_estimated_l1_fee_duffs),
        preview.estimated_l1_fee_duffs as jlong,
        jlong::from(preview.has_estimated_recipient_credits),
        preview.estimated_recipient_credits as jlong,
    ]
}

/// Read a required `N`-byte array; throws and returns `None` on JVM null, a
/// JNI error or the wrong length. `field` names the argument in the message.
fn read_fixed<const N: usize>(env: &mut JNIEnv, arr: &JByteArray, field: &str) -> Option<[u8; N]> {
    if arr.is_null() {
        throw_sdk_exception(env, 1, &format!("{field} byte[] was null"));
        return None;
    }
    let bytes = match env.convert_byte_array(arr) {
        Ok(bytes) => bytes,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} byte[] was invalid"));
            return None;
        }
    };
    match <[u8; N]>::try_from(bytes.as_slice()) {
        Ok(fixed) => Some(fixed),
        Err(_) => {
            throw_sdk_exception(
                env,
                1,
                &format!("{field} must be {N} bytes, got {}", bytes.len()),
            );
            None
        }
    }
}

fn to_jboolean(value: bool) -> jboolean {
    if value {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

/// A `Byte`-ranged argument as the `u8` the FFI takes; throws and returns
/// `None` outside `0..=255`. `field` names the argument in the message.
fn read_u8(env: &mut JNIEnv, value: jint, field: &str) -> Option<u8> {
    match u8::try_from(value) {
        Ok(value) => Some(value),
        Err(_) => {
            throw_sdk_exception(env, 1, &format!("{field} must be 0..=255, got {value}"));
            None
        }
    }
}

/// Estimate a pooled drain of `accountIndex` without reserving inputs,
/// deriving addresses or signing: bridges
/// `platform_wallet_manager_shielded_wallet_drain_preview`. Returns the
/// preview as [`PREVIEW_LEN`] longs (see [`pack_preview`]).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletDrainNative_preview(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    account_index: jint,
) -> jlongArray {
    guard(&mut env, ptr::null_mut(), |env| {
        if account_index < 0 {
            throw_sdk_exception(env, 1, "accountIndex must be non-negative");
            return ptr::null_mut();
        }
        let Some(wid) = read_fixed::<32>(env, &wallet_id, "walletId") else {
            return ptr::null_mut();
        };
        let mut preview = WalletDrainPreviewFFI::default();
        let result = unsafe {
            platform_wallet_manager_shielded_wallet_drain_preview(
                manager_handle as Handle,
                wid.as_ptr(),
                account_index as u32,
                &mut preview,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let Ok(array) = env.new_long_array(PREVIEW_LEN as jint) else {
            return ptr::null_mut();
        };
        if env
            .set_long_array_region(&array, 0, &pack_preview(&preview))
            .is_err()
        {
            return ptr::null_mut();
        }
        array.into_raw()
    })
}

/// Drain the permitted transparent accounts at `accountIndex` into one
/// Orchard recipient under `operationId` (16 bytes, persisted by the caller
/// before this call): bridges
/// `platform_wallet_manager_shielded_fund_from_wallet_drain`.
///
/// Returns the outcome and the error beside it in one packed array (see
/// [`pack_outcome`]); the two are complementary, so a failed drain is NOT
/// thrown. Throws only for an argument that could not be read, before any
/// Rust wallet code runs. Blocks for the asset-lock proof wait and the
/// Halo 2 proof.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletDrainNative_fundFromWalletDrain(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    operation_id: JByteArray,
    account_index: jint,
    recipient_raw43: JByteArray,
    core_signer_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        if account_index < 0 {
            throw_sdk_exception(env, 1, "accountIndex must be non-negative");
            return ptr::null_mut();
        }
        let Some(wid) = read_fixed::<32>(env, &wallet_id, "walletId") else {
            return ptr::null_mut();
        };
        let Some(operation) = read_fixed::<16>(env, &operation_id, "operationId") else {
            return ptr::null_mut();
        };
        let Some(recipient) = read_fixed::<43>(env, &recipient_raw43, "recipientRaw43") else {
            return ptr::null_mut();
        };
        let mut outcome = WalletDrainOutcomeFFI::default();
        let result = unsafe {
            platform_wallet_manager_shielded_fund_from_wallet_drain(
                manager_handle as Handle,
                wid.as_ptr(),
                operation.as_ptr(),
                account_index as u32,
                recipient.as_ptr(),
                core_signer_handle as *mut MnemonicResolverHandle,
                &mut outcome,
            )
        };
        let packed = pack_outcome(&outcome, &take_outcome_error(result));
        env.byte_array_from_slice(&packed)
            .map(|array| array.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Read one operation by its id, including after a restart: bridges
/// `platform_wallet_manager_shielded_wallet_drain_status`. Never touches the
/// network. Throws when the read fails, which is never evidence that nothing
/// was sent; otherwise returns the outcome packed as
/// [`Java_..._fundFromWalletDrain`] does, with no error.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletDrainNative_status(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    operation_id: JByteArray,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(wid) = read_fixed::<32>(env, &wallet_id, "walletId") else {
            return ptr::null_mut();
        };
        let Some(operation) = read_fixed::<16>(env, &operation_id, "operationId") else {
            return ptr::null_mut();
        };
        let mut outcome = WalletDrainOutcomeFFI::default();
        let result = unsafe {
            platform_wallet_manager_shielded_wallet_drain_status(
                manager_handle as Handle,
                wid.as_ptr(),
                operation.as_ptr(),
                &mut outcome,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let packed = pack_outcome(&outcome, &OutcomeError::default());
        env.byte_array_from_slice(&packed)
            .map(|array| array.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

// ── Recovery rules ────────────────────────────────────────────────────

/// Whether the host keeps its stored operation id after one status read:
/// bridges `platform_wallet_wallet_drain_marker_decision`. Returns `true`
/// to keep it.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletDrainNative_keepsStoredId(
    mut env: JNIEnv,
    _class: JClass,
    read_succeeded: jboolean,
    reported_state: jint,
    attempt_overlapped_read: jboolean,
) -> jboolean {
    guard(&mut env, JNI_TRUE, |env| {
        // A failed read carries no state; the FFI ignores the byte then.
        let state = if read_succeeded == JNI_FALSE {
            0
        } else {
            match read_u8(env, reported_state, "reportedState") {
                Some(state) => state,
                None => return JNI_TRUE,
            }
        };
        let mut keep = true;
        let result = unsafe {
            platform_wallet_wallet_drain_marker_decision(
                read_succeeded != JNI_FALSE,
                state,
                attempt_overlapped_read != JNI_FALSE,
                &mut keep,
            )
        };
        if take_pwffi_error(env, result) {
            return JNI_TRUE;
        }
        to_jboolean(keep)
    })
}

/// What the host offers for a stored operation id: bridges
/// `platform_wallet_wallet_drain_recovery_action`. Returns 0 = in progress,
/// 1 = resume the named outpoint, 2 = check again.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletDrainNative_recoveryAction(
    mut env: JNIEnv,
    _class: JClass,
    attempt_in_flight: jboolean,
    has_read: jboolean,
    read_succeeded: jboolean,
    reported_state: jint,
    read_named_asset_lock: jboolean,
) -> jint {
    const CHECK_AGAIN: jint = 2;
    guard(&mut env, CHECK_AGAIN, |env| {
        let reported = has_read != JNI_FALSE && read_succeeded != JNI_FALSE;
        let state = if reported {
            match read_u8(env, reported_state, "reportedState") {
                Some(state) => state,
                None => return CHECK_AGAIN,
            }
        } else {
            0
        };
        let mut action = CHECK_AGAIN as u8;
        let result = unsafe {
            platform_wallet_wallet_drain_recovery_action(
                attempt_in_flight != JNI_FALSE,
                has_read != JNI_FALSE,
                read_succeeded != JNI_FALSE,
                state,
                read_named_asset_lock != JNI_FALSE,
                &mut action,
            )
        };
        if take_pwffi_error(env, result) {
            return CHECK_AGAIN;
        }
        jint::from(action)
    })
}

/// Whether a returned drain attempt needs nothing more, from its outcome's
/// state: bridges `platform_wallet_wallet_drain_attempt_finished`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletDrainNative_attemptFinished(
    mut env: JNIEnv,
    _class: JClass,
    state: jint,
) -> jboolean {
    guard(&mut env, JNI_FALSE, |env| {
        let Some(state) = read_u8(env, state, "state") else {
            return JNI_FALSE;
        };
        let mut finished = false;
        let result = unsafe { platform_wallet_wallet_drain_attempt_finished(state, &mut finished) };
        if take_pwffi_error(env, result) {
            return JNI_FALSE;
        }
        to_jboolean(finished)
    })
}

/// Whether the host may start a drain after a preview with this status:
/// bridges `platform_wallet_wallet_drain_preview_allows_start`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletDrainNative_previewAllowsStart(
    mut env: JNIEnv,
    _class: JClass,
    preview_status: jint,
) -> jboolean {
    guard(&mut env, JNI_FALSE, |env| {
        let Some(status) = read_u8(env, preview_status, "previewStatus") else {
            return JNI_FALSE;
        };
        let mut allowed = false;
        let result =
            unsafe { platform_wallet_wallet_drain_preview_allows_start(status, &mut allowed) };
        if take_pwffi_error(env, result) {
            return JNI_FALSE;
        }
        to_jboolean(allowed)
    })
}

/// Whether the host offers to start a new drain on the network with this
/// `Network.ffiValue`: bridges `platform_wallet_wallet_drain_start_offered`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletDrainNative_startOffered(
    mut env: JNIEnv,
    _class: JClass,
    network: jint,
) -> jboolean {
    guard(&mut env, JNI_FALSE, |env| {
        // `net_from_ord` reads an unknown ordinal as testnet; refuse it here
        // instead, so an ordinal this library does not know is never offered
        // a drain as if it were a test network.
        if !(0..=3).contains(&network) {
            throw_sdk_exception(env, 1, &format!("unknown network ordinal {network}"));
            return JNI_FALSE;
        }
        to_jboolean(platform_wallet_wallet_drain_start_offered(net_from_ord(
            network,
        )))
    })
}

/// Where the host lists a tracked asset lock, as far as drains are
/// concerned: bridges `platform_wallet_wallet_drain_lock_listing`. Returns
/// 0 = not decided here, 1 = hidden, 2 = offered with the resumable locks.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletDrainNative_lockListing(
    mut env: JNIEnv,
    _class: JClass,
    funding_type: jint,
    status: jint,
    has_operation_id: jboolean,
    host_holds_stored_id: jboolean,
) -> jint {
    guard(&mut env, 0, |env| {
        let Some(funding_type) = read_u8(env, funding_type, "fundingType") else {
            return 0;
        };
        let Some(status) = read_u8(env, status, "status") else {
            return 0;
        };
        let mut listing = 0u8;
        let result = unsafe {
            platform_wallet_wallet_drain_lock_listing(
                funding_type,
                status,
                has_operation_id != JNI_FALSE,
                host_holds_stored_id != JNI_FALSE,
                &mut listing,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        jint::from(listing)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_wallet_ffi::OutPointFFI;

    /// The packed outcome is the only place the drain's error and its
    /// outpoint travel together, so its layout is pinned byte for byte:
    /// `WalletDrainOutcome.decode` reads these offsets.
    #[test]
    fn should_pack_an_outcome_with_its_outpoint_and_the_error_beside_it() {
        let outcome = WalletDrainOutcomeFFI {
            state: 1,
            has_out_point: true,
            out_point: OutPointFFI {
                txid: [7; 32],
                vout: 0x0102_0304,
            },
        };
        let error = OutcomeError {
            code: 1020,
            consensus_code: 40_001,
            consensus_kind: 4,
            message: "résumé".to_owned(),
        };
        let packed = pack_outcome(&outcome, &error);
        assert_eq!(
            packed.len(),
            OUTCOME_HEADER_LEN + "résumé".len(),
            "the message is UTF-8 bytes, not characters"
        );
        assert_eq!(packed[0], 1);
        assert_eq!(packed[1], 1);
        assert_eq!(&packed[2..34], &[7; 32]);
        assert_eq!(&packed[34..38], &[4, 3, 2, 1]);
        assert_eq!(&packed[38..42], &1020i32.to_le_bytes());
        assert_eq!(&packed[42..46], &40_001u32.to_le_bytes());
        assert_eq!(&packed[46..50], &4i32.to_le_bytes());
        assert_eq!(&packed[50..], "résumé".as_bytes());
    }

    #[test]
    fn should_pack_a_successful_outcome_as_the_bare_header() {
        let packed = pack_outcome(&WalletDrainOutcomeFFI::default(), &OutcomeError::default());
        assert_eq!(packed.len(), OUTCOME_HEADER_LEN);
        assert_eq!(packed[0], 1, "the default outcome is pending");
        assert_eq!(packed[1], 0);
        assert!(packed[2..].iter().all(|byte| *byte == 0));
    }

    /// A failed result's code is shifted into the platform-wallet range
    /// Kotlin decodes, exactly as a thrown exception's would be.
    #[test]
    fn should_shift_a_failed_result_code_like_a_thrown_exception() {
        let result = PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed,
            "unresolved",
        );
        let error = take_outcome_error(result);
        assert_eq!(
            error.code,
            PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed as i32
                + PWFFI_CODE_OFFSET
        );
        assert_eq!(error.message, "unresolved");
        assert_eq!(error.consensus_code, 0);
        assert_eq!(
            take_outcome_error(PlatformWalletFFIResult::ok()),
            OutcomeError::default()
        );
    }

    #[test]
    fn should_pack_a_preview_with_its_optional_estimates_flagged() {
        let packed = pack_preview(&WalletDrainPreviewFFI {
            status: 2,
            candidate_input_count: 101,
            candidate_balance_duffs: u64::MAX,
            pool_fee_credits: 100_000,
            has_estimated_l1_fee_duffs: false,
            estimated_l1_fee_duffs: 0,
            has_estimated_recipient_credits: true,
            estimated_recipient_credits: 42,
        });
        assert_eq!(packed, [2, 101, -1, 100_000, 0, 0, 1, 42]);
    }
}
