//! JNI bridge for identity credit movements — transfer, withdraw, and
//! address-input top-up on the platform-wallet `IdentityWallet`.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.CreditsNative`,
//! driven by `org.dashfoundation.dashsdk.credits.IdentityCredits`.
//!
//! ## What lives here
//!
//! Each export is a thin marshaler over a SINGLE `platform-wallet-ffi`
//! entry point (no stitching, per `packages/kotlin-sdk/CLAUDE.md`):
//! - [`platform_wallet_transfer_credits_with_signer`] — identity → identity.
//! - [`platform_wallet_withdraw_credits_with_signer`] — identity → L1 address.
//! - [`platform_wallet_top_up_from_addresses_with_signer`] — top up an
//!   existing identity from Platform-address inputs.
//!
//! All three take a plain `SignerHandle` (the identity- or platform-address
//! signer the manager owns) — no asset lock is involved, so these are the
//! credit movements the B-M6 credits screens invoke directly.
//!
//! ## Result convention
//!
//! These entry points return [`PlatformWalletFFIResult`], so errors go
//! through [`take_pwffi_error`] — the same mapping `identity.rs` /
//! `wallet_manager.rs` use.

#![allow(clippy::missing_safety_doc)]

use crate::support::{guard, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::jlong;
use jni::JNIEnv;
use platform_wallet_ffi::error::{
    platform_wallet_ffi_result_free, PlatformWalletFFIResult, PlatformWalletFFIResultCode,
};
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::identity_registration::IdentityFundingInputFFI;
use platform_wallet_ffi::identity_transfer::PlatformAddressCreditOutputFFI;
use rs_sdk_ffi::SignerHandle;
use std::ffi::CString;

// ── Result → exception ────────────────────────────────────────────────

/// If `result` carries a non-`Success` code: throw `DashSDKException`,
/// free its message, and return `true` (the caller bails with its
/// default). Mirrors `identity::take_pwffi_error` exactly — kept as a
/// local copy so `credits.rs` stays self-contained (the shared helper is
/// private to each module).
fn take_pwffi_error(env: &mut JNIEnv, mut result: PlatformWalletFFIResult) -> bool {
    if result.code == PlatformWalletFFIResultCode::Success {
        return false;
    }
    let message = if result.message.is_null() {
        format!("platform-wallet error (code {})", result.code as i32)
    } else {
        // SAFETY: non-null message is a valid CString produced by the FFI.
        unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned()
    };
    throw_sdk_exception(env, result.code as i32, &message);
    // SAFETY: `result` is a fresh PlatformWalletFFIResult; free its message.
    unsafe { platform_wallet_ffi_result_free(&mut result) };
    true
}

/// Read a required 32-byte id from a Java `byte[]`; throws + returns None
/// on the wrong length or a JNI error.
fn read_id32(env: &mut JNIEnv, arr: &JByteArray, field: &str) -> Option<[u8; 32]> {
    let bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} byte[] was null/invalid"));
            return None;
        }
    };
    if bytes.len() != 32 {
        throw_sdk_exception(
            env,
            1,
            &format!("{field} must be 32 bytes, got {}", bytes.len()),
        );
        return None;
    }
    let mut id = [0u8; 32];
    id.copy_from_slice(&bytes);
    Some(id)
}

// ── Transfer (identity → identity) ─────────────────────────────────────

/// Transfer `amount` credits from `fromIdentityId` to `toIdentityId`,
/// signed by the identity's transfer-purpose key via `signerHandle`.
///
/// Wraps `platform_wallet_transfer_credits_with_signer`. No balance is
/// returned by the FFI (the sender's balance refreshes through the
/// persistence changeset); this returns void and throws on error.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_CreditsNative_transferCredits(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    from_identity_id: JByteArray,
    to_identity_id: JByteArray,
    amount: jlong,
    signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let Some(from_id) = read_id32(env, &from_identity_id, "fromIdentityId") else {
            return;
        };
        let Some(to_id) = read_id32(env, &to_identity_id, "toIdentityId") else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_transfer_credits_with_signer(
                wallet_handle as Handle,
                from_id.as_ptr(),
                to_id.as_ptr(),
                amount.max(0) as u64,
                signer_handle as *mut SignerHandle,
            )
        };
        take_pwffi_error(env, result);
    })
}

// ── Withdraw (identity → L1 Dash address) ──────────────────────────────

/// Withdraw `amount` credits from `identityId` to the Base58Check Dash
/// address `toAddress`, signed via `signerHandle`. The Rust side
/// validates the address against the wallet's network. The L1 payout is
/// pooled and broadcast asynchronously (no txid is returned); this
/// returns void and throws on error.
///
/// Wraps `platform_wallet_withdraw_credits_with_signer`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_CreditsNative_withdrawCredits(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    amount: jlong,
    to_address: JString,
    signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return;
        };
        let addr_str: String = match env.get_string(&to_address) {
            Ok(s) => s.into(),
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "toAddress string was null/invalid");
                return;
            }
        };
        let c_addr = match CString::new(addr_str) {
            Ok(c) => c,
            Err(_) => {
                throw_sdk_exception(env, 1, "toAddress contained an interior NUL");
                return;
            }
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_withdraw_credits_with_signer(
                wallet_handle as Handle,
                id.as_ptr(),
                amount.max(0) as u64,
                c_addr.as_ptr(),
                signer_handle as *mut SignerHandle,
            )
        };
        take_pwffi_error(env, result);
    })
}

// ── Top up (existing identity ← Platform-address inputs) ───────────────

/// Top up an existing identity's credit balance from one or more
/// Platform-address inputs, signed per-address via `signerHandle` (the
/// platform-address signer). Returns the post-transition credit balance.
///
/// `inputsBlob` layout (big-endian): `u32 rowCount` then per row
/// `u8 addressType (0 P2PKH / 1 P2SH), u8[20] hash, u64 credits`.
///
/// Wraps `platform_wallet_top_up_from_addresses_with_signer`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_CreditsNative_topUpFromAddresses(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    inputs_blob: JByteArray,
    signer_handle: jlong,
) -> jlong {
    guard(&mut env, 0i64, |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return 0;
        };
        let Some(inputs) = decode_inputs_blob(env, &inputs_blob) else {
            return 0;
        };
        if inputs.is_empty() {
            throw_sdk_exception(env, 1, "inputsBlob contained no funding inputs");
            return 0;
        }

        let mut out_balance: u64 = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_top_up_from_addresses_with_signer(
                wallet_handle as Handle,
                &id as *const [u8; 32],
                inputs.as_ptr(),
                inputs.len(),
                signer_handle as *mut SignerHandle,
                &mut out_balance as *mut u64,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        out_balance as i64
    })
}

/// Decode the top-up inputs BLOB into owned [`IdentityFundingInputFFI`]
/// rows. Throws + returns None on a malformed blob. The struct is
/// `Copy`, so the returned `Vec` fully owns its rows (no borrowed
/// pointers escape).
fn decode_inputs_blob(
    env: &mut JNIEnv,
    arr: &JByteArray,
) -> Option<Vec<IdentityFundingInputFFI>> {
    let bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, "inputsBlob was null/invalid");
            return None;
        }
    };
    let mut cursor = 0usize;
    let read = |cursor: &mut usize, n: usize| -> Option<&[u8]> {
        if *cursor + n > bytes.len() {
            return None;
        }
        let s = &bytes[*cursor..*cursor + n];
        *cursor += n;
        Some(s)
    };
    let Some(count_bytes) = read(&mut cursor, 4) else {
        throw_sdk_exception(env, 1, "inputsBlob truncated (row count)");
        return None;
    };
    let count = u32::from_be_bytes(count_bytes.try_into().ok()?) as usize;
    let mut rows = Vec::with_capacity(count);
    for i in 0..count {
        let Some(type_byte) = read(&mut cursor, 1) else {
            throw_sdk_exception(env, 1, &format!("inputsBlob truncated at row {i} type"));
            return None;
        };
        let address_type = type_byte[0];
        let Some(hash_bytes) = read(&mut cursor, 20) else {
            throw_sdk_exception(env, 1, &format!("inputsBlob truncated at row {i} hash"));
            return None;
        };
        let mut hash = [0u8; 20];
        hash.copy_from_slice(hash_bytes);
        let Some(credits_bytes) = read(&mut cursor, 8) else {
            throw_sdk_exception(env, 1, &format!("inputsBlob truncated at row {i} credits"));
            return None;
        };
        let credits = u64::from_be_bytes(credits_bytes.try_into().ok()?);
        rows.push(IdentityFundingInputFFI {
            address_type,
            hash,
            credits,
        });
    }
    Some(rows)
}

// `PlatformAddressCreditOutputFFI` is imported for parity with the
// transfer-to-addresses shape; the credits screens use identity→identity
// and identity→L1 movements, so it is not yet marshalled here.
#[allow(dead_code)]
type _TransferToAddressesShape = PlatformAddressCreditOutputFFI;
