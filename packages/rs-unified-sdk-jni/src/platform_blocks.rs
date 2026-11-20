//! JNI exports for the block and state transition status reads (protocol version 15): one
//! export per `rs-sdk-ffi` function in `system/queries/blocks.rs` and
//! `system/queries/transition_status.rs`. rs-sdk sends the request, verifies the proofs and
//! says what is proved; rs-sdk-ffi writes the answer as JSON, which these exports hand to
//! Kotlin as a string. Nothing here reads the answer.
//!
//! Kotlin has no unsigned integers, so a height or count that is negative as a Kotlin `Long`
//! or `Int` is refused here with `InvalidParameter` (code 1) instead of being reinterpreted
//! as a huge unsigned value.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.PlatformBlocksNative`.

use crate::results::unwrap_string;
use crate::support::{guard, read_jstring, throw_sdk_exception};
use jni::objects::{JClass, JString};
use jni::sys::{jint, jlong, jstring};
use jni::JNIEnv;
use rs_sdk_ffi::{
    dash_sdk_system_get_blocks, dash_sdk_system_get_owner_transactions_in_block,
    dash_sdk_system_get_recent_state_transition_status,
    dash_sdk_system_get_state_transition_status_by_hash, DashSDKResult, SDKHandle,
};
use std::ffi::CString;
use std::ptr;

/// `value` as the unsigned integer the C function takes; throws `InvalidParameter` and
/// returns `None` when it is negative.
fn non_negative<T: TryFrom<i64>>(env: &mut JNIEnv, value: i64, field: &str) -> Option<T> {
    match T::try_from(value) {
        Ok(value) => Some(value),
        Err(_) => {
            throw_sdk_exception(
                env,
                1,
                &format!("{field} must not be negative, got {value}"),
            );
            None
        }
    }
}

/// The Java string `value` as a C string; throws `InvalidParameter` and returns `None` when it
/// is null, unreadable or holds a NUL.
fn required_c_string(env: &mut JNIEnv, value: &JString, field: &str) -> Option<CString> {
    if value.is_null() {
        throw_sdk_exception(env, 1, &format!("{field} was null"));
        return None;
    }
    let text: String = match read_jstring(env, value) {
        Ok(text) => text,
        Err(e) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} {e}"));
            return None;
        }
    };
    match CString::new(text) {
        Ok(text) => Some(text),
        Err(_) => {
            throw_sdk_exception(env, 1, &format!("{field} holds a NUL character"));
            None
        }
    }
}

/// The JSON string a successful `result` holds, as a Java string; null after throwing.
///
/// # Safety
/// `result` must come straight from one of the rs-sdk-ffi functions bound here, whose
/// success payload is a C string the FFI allocated.
unsafe fn json_string(env: &mut JNIEnv, result: DashSDKResult) -> jstring {
    unsafe { unwrap_string(env, result) }
        .map(|json| json.into_raw())
        .unwrap_or(ptr::null_mut())
}

/// Up to `count` consecutive blocks from `startHeight`, each with every state transition it
/// holds and its result, as the JSON `dash_sdk_system_get_blocks` writes.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_PlatformBlocksNative_getBlocks(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    start_height: jlong,
    count: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(start_height) = non_negative::<u64>(env, start_height, "startHeight") else {
            return ptr::null_mut();
        };
        let Some(count) = non_negative::<u32>(env, i64::from(count), "count") else {
            return ptr::null_mut();
        };
        let result =
            unsafe { dash_sdk_system_get_blocks(sdk as *const SDKHandle, start_height, count) };
        unsafe { json_string(env, result) }
    })
}

/// The state transitions the identity `ownerId` (base58) owns in the block at `height`, as the
/// JSON `dash_sdk_system_get_owner_transactions_in_block` writes.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_PlatformBlocksNative_getOwnerTransactionsInBlock(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    height: jlong,
    owner_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(height) = non_negative::<u64>(env, height, "height") else {
            return ptr::null_mut();
        };
        let Some(owner_id) = required_c_string(env, &owner_id, "ownerId") else {
            return ptr::null_mut();
        };
        let result = unsafe {
            dash_sdk_system_get_owner_transactions_in_block(
                sdk as *const SDKHandle,
                height,
                owner_id.as_ptr(),
            )
        };
        unsafe { json_string(env, result) }
    })
}

/// What one node says of the transition with this hash (64 hex characters), unproved, as the
/// JSON `dash_sdk_system_get_recent_state_transition_status` writes.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_PlatformBlocksNative_getRecentStateTransitionStatus(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    transition_hash: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(hash) = required_c_string(env, &transition_hash, "transitionHash") else {
            return ptr::null_mut();
        };
        let result = unsafe {
            dash_sdk_system_get_recent_state_transition_status(
                sdk as *const SDKHandle,
                hash.as_ptr(),
            )
        };
        unsafe { json_string(env, result) }
    })
}

/// The status of the transition with this hash (64 hex characters), proved by its block where
/// the network can prove it, as the JSON `dash_sdk_system_get_state_transition_status_by_hash`
/// writes.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_PlatformBlocksNative_getStateTransitionStatusByHash(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    transition_hash: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(hash) = required_c_string(env, &transition_hash, "transitionHash") else {
            return ptr::null_mut();
        };
        let result = unsafe {
            dash_sdk_system_get_state_transition_status_by_hash(
                sdk as *const SDKHandle,
                hash.as_ptr(),
            )
        };
        unsafe { json_string(env, result) }
    })
}
