//! Consume `DashSDKResult` values inside Rust and convert them to JNI
//! values, discharging every memory-ownership rule (`*_free`) before
//! returning to Kotlin.
//!
//! The converter matrix grows with the bound surface; variants unused by the
//! current exports are kept (allow(dead_code)) because each new query module
//! picks them up.
#![allow(dead_code)]

use crate::support::throw_sdk_exception;
use jni::objects::{JByteArray, JString};
use jni::JNIEnv;
use rs_sdk_ffi::{
    dash_sdk_address_info_free, dash_sdk_address_info_map_free, dash_sdk_binary_data_free,
    dash_sdk_error_free, dash_sdk_identity_balance_map_free, dash_sdk_signature_free,
    dash_sdk_string_free, DashSDKAddressInfo, DashSDKAddressInfoMap, DashSDKBinaryData,
    DashSDKIdentityBalanceMap, DashSDKResult, DashSDKResultDataType, DashSDKSignature,
};
use std::ffi::{c_char, CStr};
use std::fmt::Write as _;

/// If `r` carries an error: throw `DashSDKException`, free the error, and
/// return `true` (the caller must bail out with its default value).
///
/// # Safety
/// `r.error`, when non-null, must be a valid pointer produced by the FFI
/// layer and not yet freed.
pub unsafe fn take_error(env: &mut JNIEnv, r: &DashSDKResult) -> bool {
    if r.error.is_null() {
        return false;
    }
    let err = &*r.error;
    let message = if err.message.is_null() {
        String::from("Unknown SDK error")
    } else {
        CStr::from_ptr(err.message).to_string_lossy().into_owned()
    };
    throw_sdk_exception(env, err.code as i32, &message);
    dash_sdk_error_free(r.error);
    true
}

/// Unwrap a result whose success payload is an opaque handle pointer
/// (`data_type` is a handle variant or `NoData` with a pointer payload,
/// e.g. `dash_sdk_create_trusted`). Returns the pointer as `jlong`,
/// or 0 after throwing.
///
/// # Safety
/// `r` must be a `DashSDKResult` freshly returned by an FFI function.
pub unsafe fn unwrap_handle(env: &mut JNIEnv, r: DashSDKResult) -> i64 {
    if take_error(env, &r) {
        return 0;
    }
    r.data as i64
}

/// Unwrap a result whose success payload is a Rust-owned C string.
/// Frees the string before returning. Returns `null` after throwing.
///
/// # Safety
/// `r` must be a `DashSDKResult` freshly returned by an FFI function whose
/// success payload is a `char*` allocated by the FFI layer.
pub unsafe fn unwrap_string<'l>(env: &mut JNIEnv<'l>, r: DashSDKResult) -> Option<JString<'l>> {
    if take_error(env, &r) {
        return None;
    }
    if r.data.is_null() {
        return None;
    }
    // `data_type` is advisory: `DashSDKResult::success()` tags every payload
    // `NoData` regardless of its real shape (43 FFI call sites use it, e.g.
    // `dash_sdk_data_contract_fetch_json` for a `char*`). The unwrap is chosen
    // by the caller per the FFI function it invoked, so accept the untyped
    // sentinel alongside the typed tag — a mismatch here would false-panic in
    // dev while release (no debug_assert) proceeds fine.
    debug_assert!(matches!(
        r.data_type,
        DashSDKResultDataType::String | DashSDKResultDataType::NoData
    ));
    let c_str = r.data as *mut c_char;
    let value = CStr::from_ptr(c_str).to_string_lossy().into_owned();
    dash_sdk_string_free(c_str);
    env.new_string(value).ok()
}

/// Unwrap a result whose success payload is `DashSDKBinaryData`. Copies the
/// bytes into a Java `byte[]` and frees the Rust buffer before returning.
/// Returns `null` after throwing.
///
/// # Safety
/// `r` must be a `DashSDKResult` freshly returned by an FFI function whose
/// success payload is a `DashSDKBinaryData` allocated by the FFI layer.
pub unsafe fn unwrap_binary<'l>(env: &mut JNIEnv<'l>, r: DashSDKResult) -> Option<JByteArray<'l>> {
    if take_error(env, &r) {
        return None;
    }
    if r.data.is_null() {
        return None;
    }
    // See `unwrap_string` — `data_type` is advisory (untyped `success()` tags
    // `NoData`), so accept it alongside the typed tag.
    debug_assert!(matches!(
        r.data_type,
        DashSDKResultDataType::BinaryData | DashSDKResultDataType::NoData
    ));
    let binary = r.data as *mut DashSDKBinaryData;
    let slice = std::slice::from_raw_parts((*binary).data, (*binary).len);
    let array = env.byte_array_from_slice(slice).ok();
    dash_sdk_binary_data_free(binary);
    array
}

/// Unwrap a result whose success payload is a `DashSDKSignature`. Copies the
/// signature bytes into a Java `byte[]` and frees the Rust struct before
/// returning. Returns `null` after throwing.
///
/// `dash_sdk_signer_sign` hands the signature back as a `DashSDKSignature`
/// struct pointer tagged `NoData` (not a `DashSDKBinaryData` tagged
/// `BinaryData`), so this must not go through [`unwrap_binary`]: the two
/// structs are layout-identical `{ *mut u8, usize }`, so a release build would
/// silently succeed, but a debug build trips `unwrap_binary`'s `BinaryData`
/// assertion — and `dash_sdk_binary_data_free` is the wrong deallocator.
///
/// # Safety
/// `r` must be a `DashSDKResult` freshly returned by an FFI function whose
/// success payload is a `DashSDKSignature` allocated by the FFI layer.
pub unsafe fn unwrap_signature<'l>(
    env: &mut JNIEnv<'l>,
    r: DashSDKResult,
) -> Option<JByteArray<'l>> {
    if take_error(env, &r) {
        return None;
    }
    if r.data.is_null() {
        return None;
    }
    let sig = r.data as *mut DashSDKSignature;
    let slice = std::slice::from_raw_parts((*sig).signature, (*sig).signature_len);
    let array = env.byte_array_from_slice(slice).ok();
    dash_sdk_signature_free(sig);
    array
}

/// Unwrap a result with no meaningful success payload; throws on error.
///
/// # Safety
/// `r` must be a `DashSDKResult` freshly returned by an FFI function.
pub unsafe fn unwrap_void(env: &mut JNIEnv, r: DashSDKResult) {
    if take_error(env, &r) {
        return;
    }
    debug_assert!(
        r.data.is_null(),
        "unwrap_void called on a result carrying data — memory would leak"
    );
}

/// Lowercase-hex-encode a byte slice into `out` (no allocation of an
/// intermediate `String` per byte).
fn push_hex(out: &mut String, bytes: &[u8]) {
    for b in bytes {
        // Infallible: writing to a String never errors.
        let _ = write!(out, "{:02x}", b);
    }
}

/// The rs-sdk-ffi sentinel for a Platform address / identity that was not
/// found: nonce and balance are both set to their type max.
const NONCE_NOT_FOUND: u32 = u32::MAX;
const BALANCE_NOT_FOUND: u64 = u64::MAX;

/// Serialize one address entry (address bytes + nonce + balance) as a JSON
/// object into `out`. Keys are stable: `addressHex`, `nonce`, `balance`,
/// `found`. `nonce`/`balance` are emitted verbatim (including the
/// not-found sentinels) so the Kotlin side can distinguish "found with max
/// value" only via `found`, which mirrors the Swift `isFound` flag.
///
/// # Safety
/// `address` must be null or point to `address_len` valid bytes.
unsafe fn write_address_json(
    out: &mut String,
    address: *const u8,
    address_len: usize,
    nonce: u32,
    balance: u64,
) {
    out.push_str("{\"addressHex\":\"");
    if !address.is_null() && address_len > 0 {
        let slice = std::slice::from_raw_parts(address, address_len);
        push_hex(out, slice);
    }
    let found = !(nonce == NONCE_NOT_FOUND && balance == BALANCE_NOT_FOUND);
    // Infallible writes into a String.
    let _ = write!(
        out,
        "\",\"nonce\":{},\"balance\":{},\"found\":{}}}",
        nonce, balance, found
    );
}

/// Unwrap a `DashSDKAddressInfo` payload (`data_type == AddressInfo`) into a
/// flat JSON object string and free the FFI struct. Returns `null` after
/// throwing, or `null` if the payload pointer was null. The address bytes
/// are hex-encoded (rather than base58/bech32m) to keep this crate free of
/// address-encoding dependencies; the Kotlin/UI layer owns human-readable
/// formatting, exactly as the Swift SDK re-encodes `addressHex` for display.
///
/// # Safety
/// `r` must be a `DashSDKResult` whose success payload is a
/// `DashSDKAddressInfo` allocated by the FFI layer.
pub unsafe fn unwrap_address_info<'l>(
    env: &mut JNIEnv<'l>,
    r: DashSDKResult,
) -> Option<JString<'l>> {
    if take_error(env, &r) {
        return None;
    }
    if r.data.is_null() {
        return None;
    }
    // `data_type` is advisory (untyped `success()` tags `NoData`); accept it.
    debug_assert!(matches!(
        r.data_type,
        DashSDKResultDataType::AddressInfo | DashSDKResultDataType::NoData
    ));
    let info = r.data as *mut DashSDKAddressInfo;
    let mut json = String::new();
    write_address_json(
        &mut json,
        (*info).address,
        (*info).address_len,
        (*info).nonce,
        (*info).balance,
    );
    dash_sdk_address_info_free(info);
    env.new_string(json).ok()
}

/// Unwrap a `DashSDKAddressInfoMap` payload (`data_type == AddressInfoMap`)
/// into a JSON array of the per-address objects produced by
/// [`write_address_json`], and free the FFI map. Returns `null` after
/// throwing, or an empty array `[]` for an empty/absent map.
///
/// # Safety
/// `r` must be a `DashSDKResult` whose success payload is a
/// `DashSDKAddressInfoMap` allocated by the FFI layer.
pub unsafe fn unwrap_address_info_map<'l>(
    env: &mut JNIEnv<'l>,
    r: DashSDKResult,
) -> Option<JString<'l>> {
    if take_error(env, &r) {
        return None;
    }
    let mut json = String::from("[");
    if !r.data.is_null() {
        // `data_type` is advisory (untyped `success()` tags `NoData`); accept it.
        debug_assert!(matches!(
            r.data_type,
            DashSDKResultDataType::AddressInfoMap | DashSDKResultDataType::NoData
        ));
        let map = r.data as *mut DashSDKAddressInfoMap;
        if !(*map).entries.is_null() && (*map).count > 0 {
            let entries = std::slice::from_raw_parts((*map).entries, (*map).count);
            for (i, entry) in entries.iter().enumerate() {
                if i > 0 {
                    json.push(',');
                }
                write_address_json(
                    &mut json,
                    entry.address,
                    entry.address_len,
                    entry.nonce,
                    entry.balance,
                );
            }
        }
        dash_sdk_address_info_map_free(map);
    }
    json.push(']');
    env.new_string(json).ok()
}

/// Unwrap a `DashSDKIdentityBalanceMap` payload
/// (`data_type == IdentityBalanceMap`) into a JSON array of
/// `{"identityIdHex":"<64 hex chars>","balance":<u64>,"found":<bool>}`
/// objects, and free the FFI map. Identity ids are hex-encoded for the
/// same dependency-free reason as addresses; the Kotlin layer re-encodes to
/// base58 for display. `found` is false when the balance is the
/// `u64::MAX` not-found sentinel.
///
/// # Safety
/// `r` must be a `DashSDKResult` whose success payload is a
/// `DashSDKIdentityBalanceMap` allocated by the FFI layer.
pub unsafe fn unwrap_identity_balance_map<'l>(
    env: &mut JNIEnv<'l>,
    r: DashSDKResult,
) -> Option<JString<'l>> {
    if take_error(env, &r) {
        return None;
    }
    let mut json = String::from("[");
    if !r.data.is_null() {
        // `data_type` is advisory (untyped `success()` tags `NoData`); accept it.
        debug_assert!(matches!(
            r.data_type,
            DashSDKResultDataType::IdentityBalanceMap | DashSDKResultDataType::NoData
        ));
        let map = r.data as *mut DashSDKIdentityBalanceMap;
        if !(*map).entries.is_null() && (*map).count > 0 {
            let entries = std::slice::from_raw_parts((*map).entries, (*map).count);
            for (i, entry) in entries.iter().enumerate() {
                if i > 0 {
                    json.push(',');
                }
                json.push_str("{\"identityIdHex\":\"");
                push_hex(&mut json, &entry.identity_id);
                let found = entry.balance != BALANCE_NOT_FOUND;
                let _ = write!(
                    &mut json,
                    "\",\"balance\":{},\"found\":{}}}",
                    entry.balance, found
                );
            }
        }
        dash_sdk_identity_balance_map_free(map);
    }
    json.push(']');
    env.new_string(json).ok()
}
