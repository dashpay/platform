//! JNI bridge for the platform-wallet token / DashPay / group-action
//! surface (`platform-wallet-ffi`'s `tokens`, `dashpay`, `dashpay_profile`
//! modules).
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.TokensNative`,
//! driven by `org.dashfoundation.dashsdk.tokens.{Tokens,Dashpay,Groups}`.
//!
//! ## Why platform-wallet-ffi (not rs-sdk-ffi)
//!
//! These mirror what the iOS SwiftExampleApp actually drives
//! (`ManagedPlatformWallet` in `packages/swift-sdk`). The
//! `platform_wallet_token_*` entry points take a **wallet handle** +
//! 32-byte `identity_id`/`token_contract_id` + `signing_key_id` + an
//! external `SignerHandle`, resolving the contract, signing key and
//! (for proof verification) an 8 MB worker stack Rust-side. They deliberately
//! do NOT take an `IdentityPublicKeyHandle` — the wallet resolves the key
//! from `signingKeyId` internally — which keeps this module free of the
//! `rs-sdk-ffi` public-key handle family the low-level `dash_sdk_token_*`
//! surface would require, and honours the CLAUDE.md "no policy-loop
//! orchestration in Kotlin" doctrine.
//!
//! ## Result convention
//!
//! Every entry point returns `PlatformWalletFFIResult` by value (consumed
//! inside Rust — it never crosses JNI). The shared
//! [`crate::support::take_pwffi_error`] maps a non-`Success` code to a
//! thrown `DashSDKException` (namespaced by
//! [`crate::support::PWFFI_CODE_OFFSET`]) and frees the message, mirroring
//! `results::take_error`.
//!
//! ## Handle / argument marshalling
//!
//! - Wallet handle + signer handle cross as `jlong` (the wallet handle is a
//!   `platform_wallet_ffi::handle::Handle` = `u64`; the signer handle is a
//!   `SignerHandle` pointer from `SignerNative.createSigner`).
//! - 32-byte ids cross as `byte[]` and are read into `[u8; 32]`.
//! - Group-action authorization is a flattened
//!   `(kind: u8, position: u16, actionId: byte[]?, actionIsProposer: bool)`
//!   tuple decoded Rust-side by `decode_group_info`.
//! - JSON out-params (`out_balances_json`, group queries) surface as the
//!   returned `jstring`; the Rust C string is freed before returning.

#![allow(clippy::missing_safety_doc)]

use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jboolean, jbyteArray, jint, jlong, jstring, JNI_TRUE};
use jni::JNIEnv;
use platform_wallet_ffi::dashpay_profile::DashPayProfileFFI;
use platform_wallet_ffi::error::{platform_wallet_ffi_result_free, PlatformWalletFFIResultCode};
use platform_wallet_ffi::handle::Handle;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use rs_sdk_ffi::{MnemonicResolverHandle, SignerHandle};

// ── Argument marshalling helpers ──────────────────────────────────────

/// Read a 32-byte id from a Java `byte[]`; throws + returns None on the
/// wrong length or a JNI error. `field` names the argument for the message.
fn read_id32(env: &mut JNIEnv, arr: &JByteArray, field: &str) -> Option<[u8; 32]> {
    if arr.is_null() {
        throw_sdk_exception(env, 1, &format!("{field} byte[] was null"));
        return None;
    }
    let bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} byte[] was invalid"));
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

/// Read an OPTIONAL 32-byte id from a nullable Java `byte[]`. Returns
/// `Ok(None)` for a JVM-null array, `Ok(Some(id))` for a valid 32-byte
/// array, and `Err(())` (after throwing) on a wrong length / JNI error.
fn read_opt_id32(env: &mut JNIEnv, arr: &JByteArray, field: &str) -> Result<Option<[u8; 32]>, ()> {
    if arr.is_null() {
        return Ok(None);
    }
    let bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} byte[] was invalid"));
            return Err(());
        }
    };
    if bytes.len() != 32 {
        throw_sdk_exception(
            env,
            1,
            &format!("{field} must be 32 bytes, got {}", bytes.len()),
        );
        return Err(());
    }
    let mut id = [0u8; 32];
    id.copy_from_slice(&bytes);
    Ok(Some(id))
}

/// Read an optional Java `String` into `Option<CString>`. JVM null →
/// `Ok(None)`. Returns `Err(())` (after throwing) on an interior NUL; a JNI
/// read error is treated as null.
fn read_cstring_opt(env: &mut JNIEnv, s: &JString) -> Result<Option<CString>, ()> {
    if s.is_null() {
        return Ok(None);
    }
    let owned: String = match env.get_string(s) {
        Ok(v) => v.into(),
        Err(_) => {
            let _ = env.exception_clear();
            return Ok(None);
        }
    };
    match CString::new(owned) {
        Ok(c) => Ok(Some(c)),
        Err(_) => {
            throw_sdk_exception(env, 1, "string argument contained an interior NUL");
            Err(())
        }
    }
}

/// A null `*const c_char` pointer for an absent optional CString.
fn opt_c_ptr(opt: &Option<CString>) -> *const c_char {
    opt.as_ref().map_or(ptr::null(), |s| s.as_ptr())
}

/// Bounds-check a `jint` token / group contract position into the `u16`
/// the FFI takes; throws + returns `None` outside `0..=65535` (a raw
/// `as u16` cast would silently truncate, and a negative would wrap).
/// `field` names the argument for the message.
fn checked_position(env: &mut JNIEnv, value: jint, field: &str) -> Option<u16> {
    if !(0..=jint::from(u16::MAX)).contains(&value) {
        throw_sdk_exception(env, 1, &format!("{field} must be in 0..=65535"));
        return None;
    }
    Some(value as u16)
}

/// Sign-check a `jint` signing key id into the `u32` the FFI takes;
/// throws and returns `None` when negative (a raw cast would bit-cast to
/// a bogus huge key id).
fn checked_signing_key_id(env: &mut JNIEnv, value: jint) -> Option<u32> {
    if value < 0 {
        throw_sdk_exception(env, 1, "signingKeyId must be non-negative");
        return None;
    }
    Some(value as u32)
}

/// Range-check a `jint` group-info kind tag into the `u8` the FFI takes;
/// throws + returns `None` outside `0..=2` (none / this-signer /
/// other-signer — the tags `group_info.rs` accepts). A raw `as u8` would
/// wrap a Kotlin `-1` to 255 and fail downstream with a less clear error.
fn checked_group_info_kind(env: &mut JNIEnv, value: jint) -> Option<u8> {
    if !(0..=2).contains(&value) {
        throw_sdk_exception(env, 1, "groupInfoKind must be 0, 1, or 2");
        return None;
    }
    Some(value as u8)
}

/// Consume a Rust-owned C string out-param produced by a platform-wallet
/// entry point (freed with `platform_wallet_string_free`), copy it into a
/// Java `String`, and free the Rust buffer. Null → null jstring.
///
/// # Safety
/// `c_str` must be null or a live `platform_wallet_string_free`-owned buffer.
unsafe fn consume_pw_string(env: &mut JNIEnv, c_str: *mut c_char) -> jstring {
    if c_str.is_null() {
        return ptr::null_mut();
    }
    let value = CStr::from_ptr(c_str).to_string_lossy().into_owned();
    platform_wallet_ffi::platform_wallet_string_free(c_str);
    env.new_string(value)
        .map(|s| s.into_raw())
        .unwrap_or(ptr::null_mut())
}

/// Consume a Rust-owned C string out-param produced by the group-query
/// entry points (freed with `platform_wallet_free_string`), copy it into a
/// Java `String`, and free the Rust buffer. Null → null jstring.
///
/// # Safety
/// `c_str` must be null or a live `platform_wallet_free_string`-owned buffer.
unsafe fn consume_pw_free_string(env: &mut JNIEnv, c_str: *mut c_char) -> jstring {
    if c_str.is_null() {
        return ptr::null_mut();
    }
    let value = CStr::from_ptr(c_str).to_string_lossy().into_owned();
    platform_wallet_ffi::platform_wallet_free_string(c_str);
    env.new_string(value)
        .map(|s| s.into_raw())
        .unwrap_or(ptr::null_mut())
}

// ── Token actions ─────────────────────────────────────────────────────
//
// Each of the 12 forms mirrors a `platform_wallet_token_*` entry point.
// The 8 group-gatable forms take the flattened group-info tuple; purchase,
// transfer and claim are single-signer only (no group tuple) matching the
// FFI signatures.

/// Mint `amount` of the token at `tokenPosition` on `tokenContractId`.
/// `issuedToIdentityId` may be null (mint to the transition owner).
/// Returns the post-mint balances JSON (`{"<recipientBase58>": "<new>"}`)
/// or null.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_tokenMint(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    token_contract_id: JByteArray,
    token_position: jint,
    issued_to_identity_id: JByteArray,
    amount: jlong,
    public_note: JString,
    group_info_kind: jint,
    group_info_position: jint,
    group_info_action_id: JByteArray,
    group_info_action_is_proposer: jboolean,
    signing_key_id: jint,
    signer_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        // Reject a non-positive amount at the boundary — a negative jlong
        // would otherwise bit-cast to a huge u64.
        if amount <= 0 {
            throw_sdk_exception(env, 1, "amount must be positive");
            return ptr::null_mut();
        }
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return ptr::null_mut();
        };
        let Some(contract) = read_id32(env, &token_contract_id, "tokenContractId") else {
            return ptr::null_mut();
        };
        let recipient = match read_opt_id32(env, &issued_to_identity_id, "issuedToIdentityId") {
            Ok(v) => v,
            Err(()) => return ptr::null_mut(),
        };
        let Ok(note) = read_cstring_opt(env, &public_note) else {
            return ptr::null_mut();
        };
        let action = match read_opt_id32(env, &group_info_action_id, "groupInfoActionId") {
            Ok(v) => v,
            Err(()) => return ptr::null_mut(),
        };
        let Some(token_position) = checked_position(env, token_position, "tokenPosition") else {
            return ptr::null_mut();
        };
        let Some(group_info_position) =
            checked_position(env, group_info_position, "groupInfoPosition")
        else {
            return ptr::null_mut();
        };
        let Some(signing_key_id) = checked_signing_key_id(env, signing_key_id) else {
            return ptr::null_mut();
        };
        let Some(group_info_kind) = checked_group_info_kind(env, group_info_kind) else {
            return ptr::null_mut();
        };
        let mut out_json: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_token_mint(
                wallet_handle as Handle,
                id.as_ptr(),
                contract.as_ptr(),
                token_position,
                recipient.as_ref().map_or(ptr::null(), |r| r.as_ptr()),
                amount as u64,
                opt_c_ptr(&note),
                group_info_kind,
                group_info_position,
                action.as_ref().map_or(ptr::null(), |a| a.as_ptr()),
                group_info_action_is_proposer == JNI_TRUE,
                signing_key_id,
                signer_handle as *mut SignerHandle,
                &mut out_json as *mut *mut c_char,
            )
        };
        if take_pwffi_error(env, result) {
            // On error nothing was written through out_json (FFI nulls it).
            return ptr::null_mut();
        }
        unsafe { consume_pw_string(env, out_json) }
    })
}

/// Burn `amount` of the token, debiting the caller. Returns the
/// post-burn balances JSON (`{"<ownerBase58>": "<remaining>"}`) or null.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_tokenBurn(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    token_contract_id: JByteArray,
    token_position: jint,
    amount: jlong,
    public_note: JString,
    group_info_kind: jint,
    group_info_position: jint,
    group_info_action_id: JByteArray,
    group_info_action_is_proposer: jboolean,
    signing_key_id: jint,
    signer_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        // Reject a non-positive amount at the boundary — a negative jlong
        // would otherwise bit-cast to a huge u64.
        if amount <= 0 {
            throw_sdk_exception(env, 1, "amount must be positive");
            return ptr::null_mut();
        }
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return ptr::null_mut();
        };
        let Some(contract) = read_id32(env, &token_contract_id, "tokenContractId") else {
            return ptr::null_mut();
        };
        let Ok(note) = read_cstring_opt(env, &public_note) else {
            return ptr::null_mut();
        };
        let action = match read_opt_id32(env, &group_info_action_id, "groupInfoActionId") {
            Ok(v) => v,
            Err(()) => return ptr::null_mut(),
        };
        let Some(token_position) = checked_position(env, token_position, "tokenPosition") else {
            return ptr::null_mut();
        };
        let Some(group_info_position) =
            checked_position(env, group_info_position, "groupInfoPosition")
        else {
            return ptr::null_mut();
        };
        let Some(signing_key_id) = checked_signing_key_id(env, signing_key_id) else {
            return ptr::null_mut();
        };
        let Some(group_info_kind) = checked_group_info_kind(env, group_info_kind) else {
            return ptr::null_mut();
        };
        let mut out_json: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_token_burn(
                wallet_handle as Handle,
                id.as_ptr(),
                contract.as_ptr(),
                token_position,
                amount as u64,
                opt_c_ptr(&note),
                group_info_kind,
                group_info_position,
                action.as_ref().map_or(ptr::null(), |a| a.as_ptr()),
                group_info_action_is_proposer == JNI_TRUE,
                signing_key_id,
                signer_handle as *mut SignerHandle,
                &mut out_json as *mut *mut c_char,
            )
        };
        if take_pwffi_error(env, result) {
            // On error nothing was written through out_json (FFI nulls it).
            return ptr::null_mut();
        }
        unsafe { consume_pw_string(env, out_json) }
    })
}

/// Transfer `amount` of the token to `recipientId`. Single-signer only.
/// Returns the post-transfer balances JSON, or null.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_tokenTransfer(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    token_contract_id: JByteArray,
    token_position: jint,
    recipient_id: JByteArray,
    amount: jlong,
    public_note: JString,
    signing_key_id: jint,
    signer_handle: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        // Reject a non-positive amount at the boundary — a negative jlong
        // would otherwise bit-cast to a huge u64.
        if amount <= 0 {
            throw_sdk_exception(env, 1, "amount must be positive");
            return ptr::null_mut();
        }
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return ptr::null_mut();
        };
        let Some(contract) = read_id32(env, &token_contract_id, "tokenContractId") else {
            return ptr::null_mut();
        };
        let Some(recipient) = read_id32(env, &recipient_id, "recipientId") else {
            return ptr::null_mut();
        };
        let Ok(note) = read_cstring_opt(env, &public_note) else {
            return ptr::null_mut();
        };
        let Some(token_position) = checked_position(env, token_position, "tokenPosition") else {
            return ptr::null_mut();
        };
        let Some(signing_key_id) = checked_signing_key_id(env, signing_key_id) else {
            return ptr::null_mut();
        };
        let mut out_json: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_token_transfer(
                wallet_handle as Handle,
                id.as_ptr(),
                contract.as_ptr(),
                token_position,
                recipient.as_ptr(),
                amount as u64,
                opt_c_ptr(&note),
                signing_key_id,
                signer_handle as *mut SignerHandle,
                &mut out_json as *mut *mut c_char,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        unsafe { consume_pw_string(env, out_json) }
    })
}

/// Freeze the entire balance of `frozenIdentityId` for the token.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_tokenFreeze(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    token_contract_id: JByteArray,
    token_position: jint,
    frozen_identity_id: JByteArray,
    public_note: JString,
    group_info_kind: jint,
    group_info_position: jint,
    group_info_action_id: JByteArray,
    group_info_action_is_proposer: jboolean,
    signing_key_id: jint,
    signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return;
        };
        let Some(contract) = read_id32(env, &token_contract_id, "tokenContractId") else {
            return;
        };
        let Some(frozen) = read_id32(env, &frozen_identity_id, "frozenIdentityId") else {
            return;
        };
        let Ok(note) = read_cstring_opt(env, &public_note) else {
            return;
        };
        let action = match read_opt_id32(env, &group_info_action_id, "groupInfoActionId") {
            Ok(v) => v,
            Err(()) => return,
        };
        let Some(token_position) = checked_position(env, token_position, "tokenPosition") else {
            return;
        };
        let Some(group_info_position) =
            checked_position(env, group_info_position, "groupInfoPosition")
        else {
            return;
        };
        let Some(signing_key_id) = checked_signing_key_id(env, signing_key_id) else {
            return;
        };
        let Some(group_info_kind) = checked_group_info_kind(env, group_info_kind) else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_token_freeze(
                wallet_handle as Handle,
                id.as_ptr(),
                contract.as_ptr(),
                token_position,
                frozen.as_ptr(),
                opt_c_ptr(&note),
                group_info_kind,
                group_info_position,
                action.as_ref().map_or(ptr::null(), |a| a.as_ptr()),
                group_info_action_is_proposer == JNI_TRUE,
                signing_key_id,
                signer_handle as *mut SignerHandle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Unfreeze the entire frozen balance of `frozenIdentityId`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_tokenUnfreeze(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    token_contract_id: JByteArray,
    token_position: jint,
    frozen_identity_id: JByteArray,
    public_note: JString,
    group_info_kind: jint,
    group_info_position: jint,
    group_info_action_id: JByteArray,
    group_info_action_is_proposer: jboolean,
    signing_key_id: jint,
    signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return;
        };
        let Some(contract) = read_id32(env, &token_contract_id, "tokenContractId") else {
            return;
        };
        let Some(frozen) = read_id32(env, &frozen_identity_id, "frozenIdentityId") else {
            return;
        };
        let Ok(note) = read_cstring_opt(env, &public_note) else {
            return;
        };
        let action = match read_opt_id32(env, &group_info_action_id, "groupInfoActionId") {
            Ok(v) => v,
            Err(()) => return,
        };
        let Some(token_position) = checked_position(env, token_position, "tokenPosition") else {
            return;
        };
        let Some(group_info_position) =
            checked_position(env, group_info_position, "groupInfoPosition")
        else {
            return;
        };
        let Some(signing_key_id) = checked_signing_key_id(env, signing_key_id) else {
            return;
        };
        let Some(group_info_kind) = checked_group_info_kind(env, group_info_kind) else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_token_unfreeze(
                wallet_handle as Handle,
                id.as_ptr(),
                contract.as_ptr(),
                token_position,
                frozen.as_ptr(),
                opt_c_ptr(&note),
                group_info_kind,
                group_info_position,
                action.as_ref().map_or(ptr::null(), |a| a.as_ptr()),
                group_info_action_is_proposer == JNI_TRUE,
                signing_key_id,
                signer_handle as *mut SignerHandle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Destroy the entire frozen balance of `frozenIdentityId`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_tokenDestroyFrozenFunds(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    token_contract_id: JByteArray,
    token_position: jint,
    frozen_identity_id: JByteArray,
    public_note: JString,
    group_info_kind: jint,
    group_info_position: jint,
    group_info_action_id: JByteArray,
    group_info_action_is_proposer: jboolean,
    signing_key_id: jint,
    signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return;
        };
        let Some(contract) = read_id32(env, &token_contract_id, "tokenContractId") else {
            return;
        };
        let Some(frozen) = read_id32(env, &frozen_identity_id, "frozenIdentityId") else {
            return;
        };
        let Ok(note) = read_cstring_opt(env, &public_note) else {
            return;
        };
        let action = match read_opt_id32(env, &group_info_action_id, "groupInfoActionId") {
            Ok(v) => v,
            Err(()) => return,
        };
        let Some(token_position) = checked_position(env, token_position, "tokenPosition") else {
            return;
        };
        let Some(group_info_position) =
            checked_position(env, group_info_position, "groupInfoPosition")
        else {
            return;
        };
        let Some(signing_key_id) = checked_signing_key_id(env, signing_key_id) else {
            return;
        };
        let Some(group_info_kind) = checked_group_info_kind(env, group_info_kind) else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_token_destroy_frozen_funds(
                wallet_handle as Handle,
                id.as_ptr(),
                contract.as_ptr(),
                token_position,
                frozen.as_ptr(),
                opt_c_ptr(&note),
                group_info_kind,
                group_info_position,
                action.as_ref().map_or(ptr::null(), |a| a.as_ptr()),
                group_info_action_is_proposer == JNI_TRUE,
                signing_key_id,
                signer_handle as *mut SignerHandle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Pause all operations for the token.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_tokenPause(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    token_contract_id: JByteArray,
    token_position: jint,
    public_note: JString,
    group_info_kind: jint,
    group_info_position: jint,
    group_info_action_id: JByteArray,
    group_info_action_is_proposer: jboolean,
    signing_key_id: jint,
    signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return;
        };
        let Some(contract) = read_id32(env, &token_contract_id, "tokenContractId") else {
            return;
        };
        let Ok(note) = read_cstring_opt(env, &public_note) else {
            return;
        };
        let action = match read_opt_id32(env, &group_info_action_id, "groupInfoActionId") {
            Ok(v) => v,
            Err(()) => return,
        };
        let Some(token_position) = checked_position(env, token_position, "tokenPosition") else {
            return;
        };
        let Some(group_info_position) =
            checked_position(env, group_info_position, "groupInfoPosition")
        else {
            return;
        };
        let Some(signing_key_id) = checked_signing_key_id(env, signing_key_id) else {
            return;
        };
        let Some(group_info_kind) = checked_group_info_kind(env, group_info_kind) else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_token_pause(
                wallet_handle as Handle,
                id.as_ptr(),
                contract.as_ptr(),
                token_position,
                opt_c_ptr(&note),
                group_info_kind,
                group_info_position,
                action.as_ref().map_or(ptr::null(), |a| a.as_ptr()),
                group_info_action_is_proposer == JNI_TRUE,
                signing_key_id,
                signer_handle as *mut SignerHandle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Resume all operations for the token.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_tokenResume(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    token_contract_id: JByteArray,
    token_position: jint,
    public_note: JString,
    group_info_kind: jint,
    group_info_position: jint,
    group_info_action_id: JByteArray,
    group_info_action_is_proposer: jboolean,
    signing_key_id: jint,
    signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return;
        };
        let Some(contract) = read_id32(env, &token_contract_id, "tokenContractId") else {
            return;
        };
        let Ok(note) = read_cstring_opt(env, &public_note) else {
            return;
        };
        let action = match read_opt_id32(env, &group_info_action_id, "groupInfoActionId") {
            Ok(v) => v,
            Err(()) => return,
        };
        let Some(token_position) = checked_position(env, token_position, "tokenPosition") else {
            return;
        };
        let Some(group_info_position) =
            checked_position(env, group_info_position, "groupInfoPosition")
        else {
            return;
        };
        let Some(signing_key_id) = checked_signing_key_id(env, signing_key_id) else {
            return;
        };
        let Some(group_info_kind) = checked_group_info_kind(env, group_info_kind) else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_token_resume(
                wallet_handle as Handle,
                id.as_ptr(),
                contract.as_ptr(),
                token_position,
                opt_c_ptr(&note),
                group_info_kind,
                group_info_position,
                action.as_ref().map_or(ptr::null(), |a| a.as_ptr()),
                group_info_action_is_proposer == JNI_TRUE,
                signing_key_id,
                signer_handle as *mut SignerHandle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Set the direct-purchase price per token. `pricePerToken == 0` clears
/// the schedule (disables direct purchase).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_tokenSetPrice(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    token_contract_id: JByteArray,
    token_position: jint,
    price_per_token: jlong,
    public_note: JString,
    group_info_kind: jint,
    group_info_position: jint,
    group_info_action_id: JByteArray,
    group_info_action_is_proposer: jboolean,
    signing_key_id: jint,
    signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        // Zero is legal (clears the schedule) but a negative price would
        // bit-cast to a huge u64 — reject it at the boundary.
        if price_per_token < 0 {
            throw_sdk_exception(env, 1, "pricePerToken must be non-negative");
            return;
        }
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return;
        };
        let Some(contract) = read_id32(env, &token_contract_id, "tokenContractId") else {
            return;
        };
        let Ok(note) = read_cstring_opt(env, &public_note) else {
            return;
        };
        let action = match read_opt_id32(env, &group_info_action_id, "groupInfoActionId") {
            Ok(v) => v,
            Err(()) => return,
        };
        let Some(token_position) = checked_position(env, token_position, "tokenPosition") else {
            return;
        };
        let Some(group_info_position) =
            checked_position(env, group_info_position, "groupInfoPosition")
        else {
            return;
        };
        let Some(signing_key_id) = checked_signing_key_id(env, signing_key_id) else {
            return;
        };
        let Some(group_info_kind) = checked_group_info_kind(env, group_info_kind) else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_token_set_price(
                wallet_handle as Handle,
                id.as_ptr(),
                contract.as_ptr(),
                token_position,
                price_per_token as u64,
                opt_c_ptr(&note),
                group_info_kind,
                group_info_position,
                action.as_ref().map_or(ptr::null(), |a| a.as_ptr()),
                group_info_action_is_proposer == JNI_TRUE,
                signing_key_id,
                signer_handle as *mut SignerHandle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Purchase `amount` of the token at the set direct-purchase price.
/// Single-signer only (never group-gated).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_tokenPurchase(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    token_contract_id: JByteArray,
    token_position: jint,
    amount: jlong,
    expected_total_cost: jlong,
    signing_key_id: jint,
    signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        // Reject sign errors at the boundary — negatives would bit-cast to
        // huge u64s. The expected total cost may legitimately be quoted as
        // zero, so only its sign is checked.
        if amount <= 0 {
            throw_sdk_exception(env, 1, "amount must be positive");
            return;
        }
        if expected_total_cost < 0 {
            throw_sdk_exception(env, 1, "expectedTotalCost must be non-negative");
            return;
        }
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return;
        };
        let Some(contract) = read_id32(env, &token_contract_id, "tokenContractId") else {
            return;
        };
        let Some(token_position) = checked_position(env, token_position, "tokenPosition") else {
            return;
        };
        let Some(signing_key_id) = checked_signing_key_id(env, signing_key_id) else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_token_purchase(
                wallet_handle as Handle,
                id.as_ptr(),
                contract.as_ptr(),
                token_position,
                amount as u64,
                expected_total_cost as u64,
                signing_key_id,
                signer_handle as *mut SignerHandle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Claim a distribution payout. `distributionType`: 0 = pre-programmed,
/// 1 = perpetual. Single-signer only.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_tokenClaim(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    token_contract_id: JByteArray,
    token_position: jint,
    distribution_type: jint,
    public_note: JString,
    signing_key_id: jint,
    signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return;
        };
        let Some(contract) = read_id32(env, &token_contract_id, "tokenContractId") else {
            return;
        };
        let Ok(note) = read_cstring_opt(env, &public_note) else {
            return;
        };
        let Some(token_position) = checked_position(env, token_position, "tokenPosition") else {
            return;
        };
        let Some(signing_key_id) = checked_signing_key_id(env, signing_key_id) else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_token_claim(
                wallet_handle as Handle,
                id.as_ptr(),
                contract.as_ptr(),
                token_position,
                distribution_type as u8,
                opt_c_ptr(&note),
                signing_key_id,
                signer_handle as *mut SignerHandle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Update the token configuration. `changeItemTag` selects the change
/// variant (0 = MaxSupply — the only tag wired by the FFI in this release);
/// `changeItemPayloadJson` is the per-tag JSON payload (e.g. for MaxSupply
/// `{"newMaxSupply":"<u64>"}` or `{"newMaxSupply":null}` to remove the cap).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_tokenUpdateConfig(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    token_contract_id: JByteArray,
    token_position: jint,
    change_item_tag: jint,
    change_item_payload_json: JString,
    public_note: JString,
    group_info_kind: jint,
    group_info_position: jint,
    group_info_action_id: JByteArray,
    group_info_action_is_proposer: jboolean,
    signing_key_id: jint,
    signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return;
        };
        let Some(contract) = read_id32(env, &token_contract_id, "tokenContractId") else {
            return;
        };
        // Payload is required by the FFI for the supported tags; a JVM-null
        // payload becomes a null C pointer the FFI rejects with a clear error.
        let Ok(payload) = read_cstring_opt(env, &change_item_payload_json) else {
            return;
        };
        let Ok(note) = read_cstring_opt(env, &public_note) else {
            return;
        };
        let action = match read_opt_id32(env, &group_info_action_id, "groupInfoActionId") {
            Ok(v) => v,
            Err(()) => return,
        };
        let Some(token_position) = checked_position(env, token_position, "tokenPosition") else {
            return;
        };
        let Some(group_info_position) =
            checked_position(env, group_info_position, "groupInfoPosition")
        else {
            return;
        };
        let Some(signing_key_id) = checked_signing_key_id(env, signing_key_id) else {
            return;
        };
        let Some(group_info_kind) = checked_group_info_kind(env, group_info_kind) else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_token_update_config(
                wallet_handle as Handle,
                id.as_ptr(),
                contract.as_ptr(),
                token_position,
                change_item_tag as u8,
                opt_c_ptr(&payload),
                opt_c_ptr(&note),
                group_info_kind,
                group_info_position,
                action.as_ref().map_or(ptr::null(), |a| a.as_ptr()),
                group_info_action_is_proposer == JNI_TRUE,
                signing_key_id,
                signer_handle as *mut SignerHandle,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

// ── Group-action queries ──────────────────────────────────────────────

/// List group-action proposals on `(tokenContractId, groupContractPosition)`
/// filtered by `status` (0 = active/pending, 1 = closed).
/// `startAtActionId` may be null. `limit == 0` uses the FFI default.
/// Returns the JSON array string described in
/// `platform_wallet_token_pending_group_actions`, or null.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_pendingGroupActions(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    token_contract_id: JByteArray,
    group_contract_position: jint,
    status: jint,
    start_at_action_id: JByteArray,
    limit: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(contract) = read_id32(env, &token_contract_id, "tokenContractId") else {
            return ptr::null_mut();
        };
        let start_at = match read_opt_id32(env, &start_at_action_id, "startAtActionId") {
            Ok(v) => v,
            Err(()) => return ptr::null_mut(),
        };
        let Some(group_contract_position) =
            checked_position(env, group_contract_position, "groupContractPosition")
        else {
            return ptr::null_mut();
        };
        let mut out_json: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_token_pending_group_actions(
                wallet_handle as Handle,
                contract.as_ptr(),
                group_contract_position,
                status as u8,
                start_at.as_ref().map_or(ptr::null(), |s| s.as_ptr()),
                limit.max(0) as u16,
                &mut out_json as *mut *mut c_char,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        unsafe { consume_pw_free_string(env, out_json) }
    })
}

/// List the signers of a specific group-action proposal. Returns the JSON
/// array `[{"identityId": "...", "power": <u32>}, ...]`, or null.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_groupActionSigners(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    token_contract_id: JByteArray,
    group_contract_position: jint,
    status: jint,
    action_id: JByteArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(contract) = read_id32(env, &token_contract_id, "tokenContractId") else {
            return ptr::null_mut();
        };
        let Some(action) = read_id32(env, &action_id, "actionId") else {
            return ptr::null_mut();
        };
        let Some(group_contract_position) =
            checked_position(env, group_contract_position, "groupContractPosition")
        else {
            return ptr::null_mut();
        };
        let mut out_json: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_token_group_action_signers(
                wallet_handle as Handle,
                contract.as_ptr(),
                group_contract_position,
                status as u8,
                action.as_ptr(),
                &mut out_json as *mut *mut c_char,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        unsafe { consume_pw_free_string(env, out_json) }
    })
}

// ── DashPay single calls ──────────────────────────────────────────────

/// Send a contact request to `recipientIdentityId`, signing the document
/// state-transition with `signerHandle`. `accountLabel` may be null.
/// `autoAcceptProof` may be null (no proof). `coreSignerHandle` is the
/// manager's `MnemonicResolverHandle` — the Rust side derives the
/// friendship xpub, the ECDH shared secret and the DIP-15
/// `accountReference` through it, so no resident seed is needed (mirror
/// of the Swift `sendContactRequest` wrapper, which pins a
/// `MnemonicResolver` for the call). Returns the created
/// `ContactRequest` handle as a jlong for follow-up reads; release with
/// [`Java_..._contactRequestDestroy`].
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_sendContactRequest(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    sender_identity_id: JByteArray,
    recipient_identity_id: JByteArray,
    account_label: JString,
    auto_accept_proof: JByteArray,
    signer_handle: jlong,
    core_signer_handle: jlong,
) -> jlong {
    guard(&mut env, 0, |env| {
        let Some(sender) = read_id32(env, &sender_identity_id, "senderIdentityId") else {
            return 0;
        };
        let Some(recipient) = read_id32(env, &recipient_identity_id, "recipientIdentityId") else {
            return 0;
        };
        let Ok(label) = read_cstring_opt(env, &account_label) else {
            return 0;
        };
        // Optional proof bytes (any length). JVM null → (null, 0).
        let proof: Option<Vec<u8>> = if auto_accept_proof.is_null() {
            None
        } else {
            match env.convert_byte_array(&auto_accept_proof) {
                Ok(b) => Some(b),
                Err(_) => {
                    let _ = env.exception_clear();
                    throw_sdk_exception(env, 1, "autoAcceptProof byte[] was invalid");
                    return 0;
                }
            }
        };
        let (proof_ptr, proof_len) = match proof.as_ref() {
            Some(v) => (v.as_ptr(), v.len()),
            None => (ptr::null(), 0),
        };
        let mut out_handle: Handle = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_send_contact_request_with_signer(
                wallet_handle as Handle,
                sender.as_ptr(),
                recipient.as_ptr(),
                opt_c_ptr(&label),
                proof_ptr,
                proof_len,
                signer_handle as *mut SignerHandle,
                core_signer_handle as *mut MnemonicResolverHandle,
                &mut out_handle as *mut Handle,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        out_handle as jlong
    })
}

/// Accept an incoming contact request (by its `ContactRequest` handle),
/// sending the reciprocal request via `signerHandle`. `coreSignerHandle`
/// is the manager's `MnemonicResolverHandle` — the reciprocal send and
/// the external-account registration source all key material through it
/// (mirror of the Swift `acceptContactRequest` wrapper). Returns the
/// resulting `EstablishedContact` handle as a jlong; release with
/// [`Java_..._establishedContactDestroy`].
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_acceptContactRequest(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    request_handle: jlong,
    signer_handle: jlong,
    core_signer_handle: jlong,
) -> jlong {
    guard(&mut env, 0, |env| {
        let mut out_handle: Handle = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_accept_contact_request_with_signer(
                wallet_handle as Handle,
                request_handle as Handle,
                signer_handle as *mut SignerHandle,
                core_signer_handle as *mut MnemonicResolverHandle,
                &mut out_handle as *mut Handle,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        out_handle as jlong
    })
}

/// Ignore a contact sender (per-sender mute, = block, reversible,
/// **local-only** — no on-chain artifact). Drops the sender's pending
/// incoming request and suppresses ALL of their requests (including
/// rotated ones) from future sync sweeps; persisted through the
/// changeset pipeline so it survives a relaunch. Replaces the removed
/// per-request `rejectContactRequest` (upstream swapped reject for
/// ignore semantics — mirror of the Swift `wallet.ignoreContactSender`).
/// Reverse with [`Java_..._unignoreContactSender`].
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_ignoreContactSender(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    our_identity_id: JByteArray,
    contact_identity_id: JByteArray,
) {
    guard(&mut env, (), |env| {
        let Some(our_id) = read_id32(env, &our_identity_id, "ourIdentityId") else {
            return;
        };
        let Some(contact_id) = read_id32(env, &contact_identity_id, "contactIdentityId") else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_ignore_contact_sender(
                wallet_handle as Handle,
                our_id.as_ptr(),
                contact_id.as_ptr(),
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Un-ignore a contact sender (reverse [`Java_..._ignoreContactSender`]).
/// Removes the sender from the ignore set AND rewinds the received
/// high-water cursor so the next sweep re-fetches their on-chain
/// requests. A no-op when the sender wasn't ignored.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_unignoreContactSender(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    our_identity_id: JByteArray,
    contact_identity_id: JByteArray,
) {
    guard(&mut env, (), |env| {
        let Some(our_id) = read_id32(env, &our_identity_id, "ourIdentityId") else {
            return;
        };
        let Some(contact_id) = read_id32(env, &contact_identity_id, "contactIdentityId") else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_unignore_contact_sender(
                wallet_handle as Handle,
                our_id.as_ptr(),
                contact_id.as_ptr(),
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Release a `ContactRequest` handle from [`sendContactRequest`]. Safe on 0.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_contactRequestDestroy(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    guard(&mut env, (), |_| {
        if handle != 0 {
            // Free the result message; an invalid handle is a benign no-op
            // (we don't throw on release), so we don't route through
            // take_pwffi_error.
            let mut result = platform_wallet_ffi::contact_request_destroy(handle as Handle);
            unsafe { platform_wallet_ffi_result_free(&mut result) };
        }
    })
}

/// Release an `EstablishedContact` handle from [`acceptContactRequest`].
/// Safe on 0.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_establishedContactDestroy(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    guard(&mut env, (), |_| {
        if handle != 0 {
            let mut result =
                unsafe { platform_wallet_ffi::established_contact_destroy(handle as Handle) };
            unsafe { platform_wallet_ffi_result_free(&mut result) };
        }
    })
}

/// Send a Dash payment from `fromIdentityId` to `toContactIdentityId`.
/// `amountDuffs` is in duffs (satoshi-equivalent), `memo` may be null.
/// `coreSignerHandle` is the manager's `MnemonicResolverHandle` — the
/// funding inputs are signed through it (the wallet seed is never made
/// resident; mirror of the Swift `sendPayment` wrapper, which returns
/// `(txid, feeDuffs)`). Returns a 40-byte packed `byte[]` —
/// `txid[32] || fee_duffs(u64 LE)` — or null on error. The fee is the
/// exact network fee (Σin − Σout) reported by the builder since
/// upstream #4095.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_sendDashPayPayment(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    from_identity_id: JByteArray,
    to_contact_identity_id: JByteArray,
    amount_duffs: jlong,
    memo: JString,
    core_signer_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        // Reject a non-positive amount at the boundary — a negative jlong
        // would otherwise bit-cast to a huge u64.
        if amount_duffs <= 0 {
            throw_sdk_exception(env, 1, "amountDuffs must be positive");
            return ptr::null_mut();
        }
        let Some(from_id) = read_id32(env, &from_identity_id, "fromIdentityId") else {
            return ptr::null_mut();
        };
        let Some(to_id) = read_id32(env, &to_contact_identity_id, "toContactIdentityId") else {
            return ptr::null_mut();
        };
        let Ok(memo_c) = read_cstring_opt(env, &memo) else {
            return ptr::null_mut();
        };
        let mut txid = [0u8; 32];
        let mut fee_duffs: u64 = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_send_dashpay_payment(
                wallet_handle as Handle,
                from_id.as_ptr(),
                to_id.as_ptr(),
                amount_duffs as u64, // sign-checked above
                opt_c_ptr(&memo_c),
                core_signer_handle as *mut MnemonicResolverHandle,
                &mut txid as *mut [u8; 32],
                &mut fee_duffs as *mut u64,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        // Pack txid[32] || fee(u64 LE) — the Kotlin wrapper splits it into
        // (txid, feeDuffs), mirroring Swift's (txid, feeDuffs) tuple.
        let mut packed = [0u8; 40];
        packed[..32].copy_from_slice(&txid);
        packed[32..].copy_from_slice(&fee_duffs.to_le_bytes());
        env.byte_array_from_slice(&packed)
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Fetch DashPay profile documents for every managed identity on the
/// wallet and refresh the local cache. Returns the number of profiles
/// synced.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_syncDashPayProfiles(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
) -> jint {
    guard(&mut env, 0, |env| {
        let mut count: u32 = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_sync_dashpay_profiles(
                wallet_handle as Handle,
                &mut count as *mut u32,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        count as jint
    })
}

/// Read the cached DashPay profile for `identityId` owned by the wallet.
/// Returns a JSON object string (fields: displayName, publicMessage,
/// avatarUrl, avatarHash hex, avatarFingerprint hex) or null when the
/// identity has no cached profile. Parsing happens Kotlin-side.
///
/// An identity the wallet does not manage is folded into the same `null`
/// result as a managed identity with no profile: the Kotlin `getProfile`
/// contract only promises "null when there is no cached profile", and both
/// cases mean exactly that. This matters because the Add Contact preview
/// probes `getProfile()` on a not-yet-managed recipient id; the shared FFI
/// maps that missing identity to a `NotFound` result, and surfacing it here
/// as an exception crashes the app. (The shared FFI keeps returning the error
/// for the Swift `getDashPayProfile`, which documents throwing for unknown
/// ids — only this JNI surface is lenient.)
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_getDashPayProfile(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return ptr::null_mut();
        };
        let mut profile = DashPayProfileFFI::empty();
        let mut has_profile = false;
        let mut result = unsafe {
            platform_wallet_ffi::platform_wallet_get_dashpay_profile(
                wallet_handle as Handle,
                id.as_ptr(),
                &mut profile as *mut DashPayProfileFFI,
                &mut has_profile as *mut bool,
            )
        };
        if result.code == PlatformWalletFFIResultCode::NotFound {
            // The wallet doesn't manage this identity (the FFI maps the missing
            // `ManagedIdentity` to the generic `NotFound` Option mapping) → no
            // cached profile → null, not a thrown exception. Free the error
            // message ourselves since we're bypassing take_pwffi_error.
            unsafe { platform_wallet_ffi_result_free(&mut result) };
            return ptr::null_mut();
        }
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        if !has_profile {
            // Nothing was allocated into the profile strings (empty()).
            return ptr::null_mut();
        }
        // Build a JSON object from the flat FFI struct, then free the
        // Rust-owned strings via the profile free routine.
        let json = profile_to_json(&profile);
        unsafe {
            platform_wallet_ffi::dashpay_profile_ffi_free(&mut profile as *mut DashPayProfileFFI)
        };
        env.new_string(json)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Render a [`DashPayProfileFFI`] as a compact JSON object string. Optional
/// scalars omit their key when absent; byte fields are lower-hex. Shared
/// with [`crate::dashpay`]'s profile readers so both surfaces emit the
/// same shape.
///
/// # Safety
/// `profile`'s string pointers, when non-null, must be valid C strings.
pub(crate) fn profile_to_json(profile: &DashPayProfileFFI) -> String {
    fn opt_cstr(ptr: *const c_char) -> Option<String> {
        if ptr.is_null() {
            None
        } else {
            // SAFETY: non-null pointer is a valid FFI-produced C string.
            Some(
                unsafe { CStr::from_ptr(ptr) }
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    }
    fn json_string(value: &str) -> String {
        // Minimal JSON string escaping for the fields we emit.
        let mut out = String::with_capacity(value.len() + 2);
        out.push('"');
        for c in value.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }

    let mut fields: Vec<String> = Vec::new();
    if let Some(name) = opt_cstr(profile.display_name) {
        fields.push(format!("\"displayName\":{}", json_string(&name)));
    }
    if let Some(msg) = opt_cstr(profile.public_message) {
        fields.push(format!("\"publicMessage\":{}", json_string(&msg)));
    }
    if let Some(url) = opt_cstr(profile.avatar_url) {
        fields.push(format!("\"avatarUrl\":{}", json_string(&url)));
    }
    if profile.avatar_hash_is_some {
        let hex: String = profile
            .avatar_hash
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        fields.push(format!("\"avatarHash\":{}", json_string(&hex)));
    }
    if profile.avatar_fingerprint_is_some {
        let hex: String = profile
            .avatar_fingerprint
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        fields.push(format!("\"avatarFingerprint\":{}", json_string(&hex)));
    }
    format!("{{{}}}", fields.join(","))
}

// ── DashPay contact-request enumeration (FriendsView hydration) ───────
//
// The `FriendsView` three-list hydration is: (1) sync incoming from the
// network, (2) snapshot the managed identity, (3) read three flat
// `IdentifierArray`s (incoming sender ids / sent recipient ids /
// established contact ids), then (4) Accept via the already-bridged
// `acceptContactRequest`, feeding it the incoming request handle looked up
// by sender id. These exports bridge exactly that minimal surface; every
// handle / array is freed with its paired destroy call, copying the ids
// into a flat Kotlin `byte[]` before returning.

/// Encode a freshly-returned `IdentifierArray` into a flat `byte[]` blob
/// (`u32 count` big-endian, then `count × 32` id bytes) and free the
/// Rust-owned array. The Kotlin side slices it back into 32-byte ids.
///
/// # Safety
/// `array` must be a valid `IdentifierArray` produced by an FFI enumerator
/// and not yet freed.
unsafe fn take_identifier_array_blob(
    env: &mut JNIEnv,
    mut array: platform_wallet_ffi::IdentifierArray,
) -> jbyteArray {
    let count = array.count;
    let mut blob = Vec::with_capacity(4 + count * 32);
    blob.extend_from_slice(&(count as u32).to_be_bytes());
    if !array.items.is_null() && count > 0 {
        let rows = std::slice::from_raw_parts(array.items, count);
        for row in rows {
            blob.extend_from_slice(row);
        }
    }
    platform_wallet_ffi::platform_wallet_identifier_array_free(
        &mut array as *mut platform_wallet_ffi::IdentifierArray,
    );
    env.byte_array_from_slice(&blob)
        .map(|a| a.into_raw())
        .unwrap_or(ptr::null_mut())
}

/// Sync incoming contact requests from Platform for every managed identity
/// on the wallet (bridges `platform_wallet_sync_contact_requests`). Applies
/// them to the wallet's in-memory state (the side effect that
/// [`getManagedIdentity`] and the id readers then observe); the returned
/// handle array is freed here (the underlying `ContactRequest` handles stay
/// in native storage). Blocking.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_syncContactRequests(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
) {
    guard(&mut env, (), |env| {
        let mut array = platform_wallet_ffi::ContactRequestHandleArray {
            handles: ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_sync_contact_requests(
                wallet_handle as Handle,
                &mut array as *mut platform_wallet_ffi::ContactRequestHandleArray,
            )
        };
        // Free the array regardless of outcome (the handles it references
        // remain valid in native storage; we only own the buffer).
        unsafe {
            platform_wallet_ffi::platform_wallet_contact_request_handle_array_free(
                &mut array as *mut platform_wallet_ffi::ContactRequestHandleArray,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Fetch the contact requests sent by `identityId` from Platform (bridges
/// `platform_wallet_fetch_sent_contact_requests`). Applies them to the
/// wallet's in-memory state; the returned handle array is freed here.
/// Blocking.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_fetchSentContactRequests(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
) {
    guard(&mut env, (), |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return;
        };
        let mut array = platform_wallet_ffi::ContactRequestHandleArray {
            handles: ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_fetch_sent_contact_requests(
                wallet_handle as Handle,
                id.as_ptr(),
                &mut array as *mut platform_wallet_ffi::ContactRequestHandleArray,
            )
        };
        unsafe {
            platform_wallet_ffi::platform_wallet_contact_request_handle_array_free(
                &mut array as *mut platform_wallet_ffi::ContactRequestHandleArray,
            )
        };
        let _ = take_pwffi_error(env, result);
    })
}

/// Take a fresh `ManagedIdentity` snapshot for `identityId` from the wallet
/// (bridges `platform_wallet_get_managed_identity`), returning its handle as
/// a jlong. The snapshot does NOT track later mutations — call again after a
/// sync to pick up fresh state. Release with [`managedIdentityDestroy`].
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_getManagedIdentity(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
) -> jlong {
    guard(&mut env, 0, |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return 0;
        };
        let mut out_handle: Handle = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_get_managed_identity(
                wallet_handle as Handle,
                id.as_ptr(),
                &mut out_handle as *mut Handle,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        out_handle as jlong
    })
}

/// Release a `ManagedIdentity` handle from [`getManagedIdentity`]. Safe on 0.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_managedIdentityDestroy(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    guard(&mut env, (), |_| {
        if handle != 0 {
            let mut result =
                unsafe { platform_wallet_ffi::managed_identity_destroy(handle as Handle) };
            unsafe { platform_wallet_ffi_result_free(&mut result) };
        }
    })
}

/// The 32-byte sender ids of the managed identity's incoming contact
/// requests, as a flat `byte[]` blob (`u32 count` + `count × 32`), freeing
/// the Rust-owned array (bridges
/// `managed_identity_get_incoming_contact_request_ids`).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_managedIdentityIncomingContactRequestIds(
    mut env: JNIEnv,
    _class: JClass,
    identity_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut array = platform_wallet_ffi::IdentifierArray {
            items: ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::managed_identity_get_incoming_contact_request_ids(
                identity_handle as Handle,
                &mut array as *mut platform_wallet_ffi::IdentifierArray,
            )
        };
        if take_pwffi_error(env, result) {
            // On error nothing was allocated; still normalize the (empty) array.
            unsafe {
                platform_wallet_ffi::platform_wallet_identifier_array_free(
                    &mut array as *mut platform_wallet_ffi::IdentifierArray,
                )
            };
            return ptr::null_mut();
        }
        unsafe { take_identifier_array_blob(env, array) }
    })
}

/// The 32-byte recipient ids of the managed identity's sent contact
/// requests, as a flat `byte[]` blob (bridges
/// `managed_identity_get_sent_contact_request_ids`).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_managedIdentitySentContactRequestIds(
    mut env: JNIEnv,
    _class: JClass,
    identity_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut array = platform_wallet_ffi::IdentifierArray {
            items: ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::managed_identity_get_sent_contact_request_ids(
                identity_handle as Handle,
                &mut array as *mut platform_wallet_ffi::IdentifierArray,
            )
        };
        if take_pwffi_error(env, result) {
            unsafe {
                platform_wallet_ffi::platform_wallet_identifier_array_free(
                    &mut array as *mut platform_wallet_ffi::IdentifierArray,
                )
            };
            return ptr::null_mut();
        }
        unsafe { take_identifier_array_blob(env, array) }
    })
}

/// The 32-byte contact ids of the managed identity's established contacts,
/// as a flat `byte[]` blob (bridges
/// `managed_identity_get_established_contact_ids`).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_managedIdentityEstablishedContactIds(
    mut env: JNIEnv,
    _class: JClass,
    identity_handle: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut array = platform_wallet_ffi::IdentifierArray {
            items: ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::managed_identity_get_established_contact_ids(
                identity_handle as Handle,
                &mut array as *mut platform_wallet_ffi::IdentifierArray,
            )
        };
        if take_pwffi_error(env, result) {
            unsafe {
                platform_wallet_ffi::platform_wallet_identifier_array_free(
                    &mut array as *mut platform_wallet_ffi::IdentifierArray,
                )
            };
            return ptr::null_mut();
        }
        unsafe { take_identifier_array_blob(env, array) }
    })
}

/// Look up the incoming `ContactRequest` handle for `senderId` on the managed
/// identity (bridges `managed_identity_get_incoming_contact_request`) — the
/// handle fed into [`acceptContactRequest`]. Returns the handle as a jlong,
/// or 0 when the request isn't in local state (`NotFound` is not thrown).
/// Release with [`contactRequestDestroy`].
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TokensNative_getIncomingContactRequest(
    mut env: JNIEnv,
    _class: JClass,
    identity_handle: jlong,
    sender_id: JByteArray,
) -> jlong {
    guard(&mut env, 0, |env| {
        let Some(id) = read_id32(env, &sender_id, "senderId") else {
            return 0;
        };
        let mut out_handle: Handle = 0;
        let mut result = unsafe {
            platform_wallet_ffi::managed_identity_get_incoming_contact_request(
                identity_handle as Handle,
                id.as_ptr(),
                &mut out_handle as *mut Handle,
            )
        };
        // NotFound is a benign "no such incoming request" — return 0 without
        // throwing, matching the Swift wrapper's nil.
        if result.code == PlatformWalletFFIResultCode::NotFound {
            unsafe { platform_wallet_ffi_result_free(&mut result) };
            return 0;
        }
        if take_pwffi_error(env, result) {
            return 0;
        }
        out_handle as jlong
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // profile_to_json is pure marshalling logic (no JNIEnv), so it is unit
    // testable on the JVM-free host. The natives above need a device.

    #[test]
    fn profile_json_omits_absent_fields() {
        let profile = DashPayProfileFFI::empty();
        assert_eq!(profile_to_json(&profile), "{}");
    }

    #[test]
    fn profile_json_hex_encodes_byte_fields() {
        let mut profile = DashPayProfileFFI::empty();
        profile.avatar_hash_is_some = true;
        profile.avatar_hash[0] = 0xAB;
        profile.avatar_hash[31] = 0x01;
        profile.avatar_fingerprint_is_some = true;
        profile.avatar_fingerprint = [0x10, 0x20, 0, 0, 0, 0, 0, 0xFF];
        let json = profile_to_json(&profile);
        // 32-byte hash: 0xAB … 0x01 → "ab" prefix, "01" suffix (64 hex chars).
        assert!(json.contains("\"avatarHash\":\"ab"), "got {json}");
        assert!(json.contains("01\""), "hash suffix; got {json}");
        // 8-byte fingerprint [0x10,0x20,0,0,0,0,0,0xFF] → 16 hex chars.
        assert!(
            json.contains("\"avatarFingerprint\":\"10200000000000ff\""),
            "got {json}"
        );
    }
}
