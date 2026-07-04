//! JNI bridge for the async signer vtable
//! (`rs-sdk-ffi/src/signer.rs::SignerVTable`).
//!
//! Flow (mirrors `KeychainSigner.swift`):
//! 1. Rust needs a signature → `sign_async` trampoline fires (Tokio thread).
//! 2. The trampoline parks `{completion_ctx, completion}` behind a token
//!    and calls `NativeSignerBridge.signAsync(pubkey, keyType, data, token)`,
//!    which returns immediately.
//! 3. Kotlin decrypts the private key (BiometricPrompt hop if the Keystore
//!    auth window expired), signs via the one-shot
//!    `dash_sdk_signer_create_from_private_key` + `dash_sdk_signer_sign`
//!    helpers (exactly like Swift — no crypto in the language layer),
//!    zeroes the buffer, then calls `completeSign(token, signature, error)`.
//! 4. `completeSign` reclaims the token and fires the Rust completion
//!    exactly once.
//!
//! `can_sign_with` stays synchronous: a fast Kotlin lookup.

use crate::results::{unwrap_binary, unwrap_handle};
use crate::support::{guard, JVM};
use jni::objects::{GlobalRef, JByteArray, JClass, JObject, JString};
use jni::sys::{jbyteArray, jint, jlong};
use jni::JNIEnv;
use rs_sdk_ffi::{
    dash_sdk_signer_create_from_private_key, dash_sdk_signer_create_with_ctx,
    dash_sdk_signer_destroy, dash_sdk_signer_sign, SignCompletionCallback, SignerHandle,
};
use std::ffi::{c_void, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

struct KotlinSignerCtx {
    bridge: GlobalRef,
}

/// Parked completion for one in-flight sign request. Reclaimed exactly
/// once by `completeSign`; `sign_async` contract violations (double
/// completion) are prevented on the Kotlin side by the single-shot token.
struct PendingSign {
    completion_ctx: *mut c_void,
    completion: SignCompletionCallback,
}

// SAFETY: the raw completion pointers are only touched by the thread that
// reclaims the Box in completeSign, exactly once.
unsafe impl Send for PendingSign {}

/// Fire a parked completion with an error message. Consumes the pending box.
unsafe fn complete_with_error(pending: Box<PendingSign>, message: &str) {
    let c_message = CString::new(message)
        .unwrap_or_else(|_| CString::new("signing failed").expect("static string"));
    (pending.completion)(pending.completion_ctx, ptr::null(), 0, c_message.as_ptr());
}

unsafe extern "C" fn sign_async_trampoline(
    signer: *const c_void,
    pubkey_bytes: *const u8,
    pubkey_len: usize,
    key_type: u8,
    data: *const u8,
    data_len: usize,
    completion_ctx: *mut c_void,
    completion: SignCompletionCallback,
) {
    let pending = Box::new(PendingSign {
        completion_ctx,
        completion,
    });
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        if signer.is_null() {
            return Err(pending);
        }
        let ctx = &*(signer as *const KotlinSignerCtx);
        let Some(vm) = JVM.get() else {
            return Err(pending);
        };
        let Ok(mut env) = vm.attach_current_thread_as_daemon() else {
            return Err(pending);
        };

        // Copy the borrowed buffers before returning (callback-window rule).
        let pubkey = std::slice::from_raw_parts(pubkey_bytes, pubkey_len);
        let payload = std::slice::from_raw_parts(data, data_len);
        if env.push_local_frame(16).is_err() {
            let _ = env.exception_clear();
            return Err(pending);
        }
        let (Ok(jpubkey), Ok(jdata)) = (
            env.byte_array_from_slice(pubkey),
            env.byte_array_from_slice(payload),
        ) else {
            let _ = env.exception_clear();
            let _ = env.pop_local_frame(&JObject::null());
            return Err(pending);
        };

        let token = Box::into_raw(pending) as jlong;
        let dispatched = env.call_method(
            ctx.bridge.as_obj(),
            "signAsync",
            "([BI[BJ)V",
            &[
                (&jpubkey).into(),
                (key_type as jint).into(),
                (&jdata).into(),
                token.into(),
            ],
        );
        if dispatched.is_err() {
            let _ = env.exception_clear();
            let _ = env.pop_local_frame(&JObject::null());
            // Reclaim the token we just leaked and fail the request.
            return Err(Box::from_raw(token as *mut PendingSign));
        }
        let _ = env.pop_local_frame(&JObject::null());
        Ok(())
    }));

    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(pending)) => complete_with_error(pending, "JNI signer dispatch failed"),
        // The pending box was moved into the closure; on panic before the
        // token leak it was dropped — nothing left to complete. Rust's
        // 5-minute completion timeout bounds the damage of this edge.
        Err(_) => {}
    }
}

unsafe extern "C" fn can_sign_trampoline(
    signer: *const c_void,
    pubkey_bytes: *const u8,
    pubkey_len: usize,
    key_type: u8,
) -> bool {
    catch_unwind(AssertUnwindSafe(|| {
        if signer.is_null() {
            return false;
        }
        let ctx = &*(signer as *const KotlinSignerCtx);
        let Some(vm) = JVM.get() else { return false };
        let Ok(mut env) = vm.attach_current_thread_as_daemon() else {
            return false;
        };
        let pubkey = std::slice::from_raw_parts(pubkey_bytes, pubkey_len);
        match env.with_local_frame(16, |env| {
            let jpubkey = env.byte_array_from_slice(pubkey)?;
            env.call_method(
                ctx.bridge.as_obj(),
                "canSignWith",
                "([BI)Z",
                &[(&jpubkey).into(), (key_type as jint).into()],
            )?
            .z()
        }) {
            Ok(can_sign) => can_sign,
            Err(_) => {
                let _ = env.exception_clear();
                false
            }
        }
    }))
    .unwrap_or(false)
}

unsafe extern "C" fn destroy_trampoline(signer: *mut c_void) {
    if signer.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if let Some(vm) = JVM.get() {
            let _ = vm.attach_current_thread_as_daemon();
        }
        drop(Box::from_raw(signer as *mut KotlinSignerCtx));
    }));
}

/// Create a `SignerHandle` backed by a Kotlin `NativeSignerBridge`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_SignerNative_createSigner(
    mut env: JNIEnv,
    _class: JClass,
    bridge: JObject,
) -> jlong {
    guard(&mut env, 0, |env| {
        let Ok(global) = env.new_global_ref(&bridge) else {
            return 0;
        };
        let ctx = Box::into_raw(Box::new(KotlinSignerCtx { bridge: global }));
        let handle = unsafe {
            dash_sdk_signer_create_with_ctx(
                ctx as *mut c_void,
                sign_async_trampoline,
                can_sign_trampoline,
                Some(destroy_trampoline),
            )
        };
        handle as jlong
    })
}

/// Destroy a signer from `createSigner`; drops the bridge GlobalRef.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_SignerNative_destroySigner(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    guard(&mut env, (), |_| unsafe {
        dash_sdk_signer_destroy(handle as *mut SignerHandle)
    });
}

/// Complete an in-flight sign request. Exactly one of
/// `signature`/`errorMessage` should be non-null; a null signature with a
/// null error is treated as a generic failure. The token is consumed —
/// calling twice with the same token is undefined and prevented on the
/// Kotlin side.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_SignerNative_completeSign(
    mut env: JNIEnv,
    _class: JClass,
    token: jlong,
    signature: JByteArray,
    error_message: JString,
) {
    guard(&mut env, (), |env| {
        if token == 0 {
            return;
        }
        let pending = unsafe { Box::from_raw(token as *mut PendingSign) };

        if !signature.is_null() {
            if let Ok(bytes) = env.convert_byte_array(&signature) {
                unsafe {
                    (pending.completion)(
                        pending.completion_ctx,
                        bytes.as_ptr(),
                        bytes.len(),
                        ptr::null(),
                    );
                }
                return;
            }
            let _ = env.exception_clear();
            unsafe { complete_with_error(pending, "signature marshalling failed") };
            return;
        }

        let message: String = if error_message.is_null() {
            String::from("signing failed")
        } else {
            env.get_string(&error_message)
                .map(|s| s.into())
                .unwrap_or_else(|_| {
                    let _ = env.exception_clear();
                    String::from("signing failed")
                })
        };
        unsafe { complete_with_error(pending, &message) };
    });
}

/// One-shot ECDSA sign helper: build a single-key signer from raw private
/// key bytes, sign `data`, destroy the signer — the exact FFI route
/// `KeychainSigner.swift` uses so no crypto runs in the language layer.
/// Zeroes nothing itself: the caller owns and zeroes the key array.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_SignerNative_signWithPrivateKey(
    mut env: JNIEnv,
    _class: JClass,
    private_key: JByteArray,
    network: jint,
    data: JByteArray,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let (Ok(key), Ok(payload)) = (
            env.convert_byte_array(&private_key),
            env.convert_byte_array(&data),
        ) else {
            let _ = env.exception_clear();
            crate::support::throw_sdk_exception(env, 1, "invalid signer arguments");
            return ptr::null_mut();
        };
        let mut key = key;

        let network = match network {
            0 => dash_network::ffi::FFINetwork::Mainnet,
            2 => dash_network::ffi::FFINetwork::Devnet,
            3 => dash_network::ffi::FFINetwork::Regtest,
            _ => dash_network::ffi::FFINetwork::Testnet,
        };

        let created =
            unsafe { dash_sdk_signer_create_from_private_key(key.as_ptr(), key.len(), network) };
        // Zero our copy of the key material as soon as the Rust side has
        // its own zeroizing buffer.
        key.iter_mut().for_each(|b| *b = 0);

        let signer = unsafe { unwrap_handle(env, created) };
        if signer == 0 {
            return ptr::null_mut();
        }

        let signed = unsafe {
            dash_sdk_signer_sign(signer as *mut SignerHandle, payload.as_ptr(), payload.len())
        };
        let result = unsafe { unwrap_binary(env, signed) }
            .map(|arr| arr.into_raw())
            .unwrap_or(ptr::null_mut());

        unsafe { dash_sdk_signer_destroy(signer as *mut SignerHandle) };
        result
    })
}
