//! JNI bridge for the mnemonic resolver — the synchronous
//! "fetch mnemonic for wallet_id" vtable Rust invokes whenever a
//! derivation needs the seed (`rs-sdk-ffi/src/mnemonic_resolver.rs`).
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.MnemonicNative` +
//! the `NativeMnemonicBridge` implementation (`MnemonicResolverAndPersister`),
//! which decrypts the Keystore-wrapped mnemonic on demand.

use crate::support::{guard, JVM};
use jni::objects::{GlobalRef, JClass, JObject};
use jni::sys::jlong;
use jni::JNIEnv;
use rs_sdk_ffi::{
    dash_sdk_mnemonic_resolver_create, dash_sdk_mnemonic_resolver_destroy, MnemonicResolverHandle,
};
use std::ffi::c_void;
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Result codes from `rs-sdk-ffi::mnemonic_resolver_result`.
const RESULT_OK: i32 = 0;
const RESULT_NOT_FOUND: i32 = 1;
const RESULT_BUFFER_TOO_SMALL: i32 = 2;
const RESULT_OTHER: i32 = 3;

/// Sentinel returns from `NativeMnemonicBridge.resolveMnemonicInto`
/// (keep in sync with the Kotlin companion constants).
const RESOLVE_NOT_FOUND: i32 = -1;
const RESOLVE_BUFFER_TOO_SMALL: i32 = -2;
const RESOLVE_OTHER: i32 = -3;

struct KotlinMnemonicCtx {
    bridge: GlobalRef,
}

/// The synchronous resolve trampoline. May fire on Tokio worker threads —
/// attaches as daemon and calls
/// `NativeMnemonicBridge.resolveMnemonicInto(byte[], byte[]): int` — the
/// out-buffer contract: Kotlin writes raw UTF-8 phrase bytes into a Java
/// buffer sized to the Rust caller's capacity, this trampoline copies
/// them straight into the Rust-owned buffer and then ZEROES the Java
/// buffer. No `java.lang.String` of the phrase ever exists (an immutable
/// String can't be scrubbed and would sit in the JVM heap, recoverable
/// from a heap dump, until — if ever — collected). Never unwinds.
unsafe extern "C" fn resolve_trampoline(
    ctx: *const c_void,
    wallet_id_bytes: *const u8,
    out_mnemonic_utf8: *mut c_char,
    out_capacity: usize,
    out_len: *mut usize,
) -> i32 {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if ctx.is_null() || wallet_id_bytes.is_null() || out_mnemonic_utf8.is_null() {
            return RESULT_OTHER;
        }
        let ctx = &*(ctx as *const KotlinMnemonicCtx);
        let Some(vm) = JVM.get() else {
            return RESULT_OTHER;
        };
        let Ok(mut env) = vm.attach_current_thread_as_daemon() else {
            return RESULT_OTHER;
        };

        // A zero-capacity out buffer cannot even hold the NUL terminator;
        // reject up front (the `usable = cap - 1` arithmetic below would
        // otherwise let an empty phrase write one byte out of bounds).
        if out_capacity == 0 {
            return RESULT_BUFFER_TOO_SMALL;
        }

        match env.with_local_frame(16, |env| -> Result<i32, jni::errors::Error> {
            let wallet_id = std::slice::from_raw_parts(wallet_id_bytes, 32);
            let jwallet_id = env.byte_array_from_slice(wallet_id)?;
            // Reserve one byte of the Rust capacity for the NUL terminator.
            let usable = out_capacity - 1;
            let jout = env.new_byte_array(usable as i32)?;
            let written = env
                .call_method(
                    ctx.bridge.as_obj(),
                    "resolveMnemonicInto",
                    "([B[B)I",
                    &[(&jwallet_id).into(), (&jout).into()],
                )?
                .i()?;

            let code = match written {
                RESOLVE_NOT_FOUND => RESULT_NOT_FOUND,
                RESOLVE_BUFFER_TOO_SMALL => RESULT_BUFFER_TOO_SMALL,
                RESOLVE_OTHER => RESULT_OTHER,
                len if len < 0 || len as usize > usable => RESULT_OTHER,
                len => {
                    let len = len as usize;
                    // Copy the phrase bytes straight from the Java buffer
                    // into the Rust-owned out buffer, then scrub the Java
                    // buffer — the only JVM-side plaintext copy.
                    // `.cast()`: c_char is i8 on x86_64 but u8 on
                    // aarch64-linux-android; jbyte is always i8.
                    let dst = std::slice::from_raw_parts_mut(out_mnemonic_utf8.cast::<i8>(), len);
                    env.get_byte_array_region(&jout, 0, dst)?;
                    *(out_mnemonic_utf8.add(len)) = 0;
                    *out_len = len;
                    RESULT_OK
                }
            };
            let zeros = vec![0i8; usable];
            env.set_byte_array_region(&jout, 0, &zeros)?;
            Ok(code)
        }) {
            Ok(code) => code,
            Err(_) => {
                let _ = env.exception_clear();
                RESULT_OTHER
            }
        }
    }));
    result.unwrap_or(RESULT_OTHER)
}

unsafe extern "C" fn destroy_trampoline(ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }
    // Dropping the GlobalRef needs a JNIEnv; attach if necessary.
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let ctx = Box::from_raw(ctx as *mut KotlinMnemonicCtx);
        if let Some(vm) = JVM.get() {
            // GlobalRef's Drop attaches internally via its cached JavaVM;
            // ensure the thread is attached so the delete succeeds.
            let _ = vm.attach_current_thread_as_daemon();
        }
        drop(ctx);
    }));
}

/// Create a mnemonic resolver handle backed by a Kotlin
/// `NativeMnemonicBridge`. Returns the `MnemonicResolverHandle` pointer
/// as jlong; release with [`Java_org_dashfoundation_dashsdk_ffi_MnemonicNative_destroyResolver`].
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MnemonicNative_createResolver(
    mut env: JNIEnv,
    _class: JClass,
    bridge: JObject,
) -> jlong {
    guard(&mut env, 0, |env| {
        let Ok(global) = env.new_global_ref(&bridge) else {
            return 0;
        };
        let ctx = Box::into_raw(Box::new(KotlinMnemonicCtx { bridge: global }));
        let handle = unsafe {
            dash_sdk_mnemonic_resolver_create(
                ctx as *mut c_void,
                resolve_trampoline,
                destroy_trampoline,
            )
        };
        handle as jlong
    })
}

/// Destroy a resolver from `createResolver`. Safe on 0. Runs the destroy
/// callback (dropping the Kotlin bridge GlobalRef) exactly once.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MnemonicNative_destroyResolver(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    guard(&mut env, (), |_| unsafe {
        dash_sdk_mnemonic_resolver_destroy(handle as *mut MnemonicResolverHandle)
    });
}

/// Generate a fresh BIP-39 mnemonic via key-wallet-ffi — the same
/// `mnemonic_generate_with_language` call `Mnemonic.generate()` uses in
/// `SwiftDashSDK/KeyWallet/Mnemonic.swift`. `wordCount` ∈ {12,15,18,21,24};
/// `language` is the FFILanguage ordinal (0 = English). Throws on failure.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MnemonicNative_generateMnemonic(
    mut env: JNIEnv,
    _class: JClass,
    word_count: jni::sys::jint,
    language: jni::sys::jint,
) -> jni::sys::jstring {
    guard(&mut env, std::ptr::null_mut(), |env| {
        let language = match language {
            1 => key_wallet_ffi::mnemonic::FFILanguage::ChineseSimplified,
            2 => key_wallet_ffi::mnemonic::FFILanguage::ChineseTraditional,
            3 => key_wallet_ffi::mnemonic::FFILanguage::Czech,
            4 => key_wallet_ffi::mnemonic::FFILanguage::French,
            5 => key_wallet_ffi::mnemonic::FFILanguage::Italian,
            _ if language > 5 => key_wallet_ffi::mnemonic::FFILanguage::English,
            _ => key_wallet_ffi::mnemonic::FFILanguage::English,
        };
        // Out-param error slot; the FFI writes code+message into it.
        let mut error = key_wallet_ffi::FFIError {
            code: key_wallet_ffi::FFIErrorCode::Success,
            message: std::ptr::null_mut(),
        };
        let phrase_ptr = unsafe {
            key_wallet_ffi::mnemonic::mnemonic_generate_with_language(
                word_count.max(0) as std::os::raw::c_uint,
                language,
                &mut error,
            )
        };
        if phrase_ptr.is_null() {
            let message = if error.message.is_null() {
                String::from("mnemonic generation failed")
            } else {
                unsafe { std::ffi::CStr::from_ptr(error.message) }
                    .to_string_lossy()
                    .into_owned()
            };
            unsafe { error.clean() };
            crate::support::throw_sdk_exception(env, error.code as i32, &message);
            return std::ptr::null_mut();
        }
        let mut phrase = unsafe { std::ffi::CStr::from_ptr(phrase_ptr) }
            .to_string_lossy()
            .into_owned();
        unsafe { key_wallet_ffi::mnemonic::mnemonic_free(phrase_ptr) };
        let jstr = env
            .new_string(&phrase)
            .map(|s| s.into_raw())
            .unwrap_or(std::ptr::null_mut());
        // Zero the intermediate copy of the phrase.
        unsafe { phrase.as_bytes_mut().iter_mut().for_each(|b| *b = 0) };
        jstr
    })
}
