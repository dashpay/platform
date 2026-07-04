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
    dash_sdk_binary_data_free, dash_sdk_error_free, dash_sdk_string_free, DashSDKBinaryData,
    DashSDKResult, DashSDKResultDataType,
};
use std::ffi::{c_char, CStr};

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
    debug_assert!(matches!(r.data_type, DashSDKResultDataType::String));
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
    debug_assert!(matches!(r.data_type, DashSDKResultDataType::BinaryData));
    let binary = r.data as *mut DashSDKBinaryData;
    let slice = std::slice::from_raw_parts((*binary).data, (*binary).len);
    let array = env.byte_array_from_slice(slice).ok();
    dash_sdk_binary_data_free(binary);
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
