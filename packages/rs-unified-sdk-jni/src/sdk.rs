//! JNI exports for SDK lifecycle: init, logging, version, create, destroy.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.SdkNative`.

use crate::results::unwrap_handle;
use crate::support::guard;
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jint, jlong, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use rs_sdk_ffi::{
    dash_sdk_create_trusted, dash_sdk_destroy, dash_sdk_enable_logging, dash_sdk_get_network,
    dash_sdk_init, dash_sdk_version, DashSDKConfig, SDKHandle,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// One-time Rust library initialization (`dash_sdk_init`).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_SdkNative_nativeInit(
    mut env: JNIEnv,
    _class: JClass,
) {
    guard(&mut env, (), |_| dash_sdk_init());
}

/// Enable console logging. Level: 0=Error … 4=Trace.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_SdkNative_enableLogging(
    mut env: JNIEnv,
    _class: JClass,
    level: jint,
) {
    guard(&mut env, (), |_| {
        dash_sdk_enable_logging(level.clamp(0, 4) as u8)
    });
}

/// SDK version string (static, never freed).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_SdkNative_version(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let version = unsafe { CStr::from_ptr(dash_sdk_version()) }.to_string_lossy();
        env.new_string(version)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Create a trusted-context SDK instance; returns the `SDKHandle` as jlong.
///
/// `network`: 0=Mainnet, 1=Testnet, 2=Devnet, 3=Regtest (FFINetwork values).
/// `dapiAddresses`/`quorumUrl` may be null (network defaults).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_SdkNative_createTrusted(
    mut env: JNIEnv,
    _class: JClass,
    network: jint,
    dapi_addresses: JString,
    quorum_url: JString,
    skip_asset_lock_proof_verification: jboolean,
    request_retry_count: jint,
    request_timeout_ms: jlong,
    platform_version: jint,
) -> jlong {
    guard(&mut env, 0, |env| {
        let dapi = to_c_string(env, &dapi_addresses);
        let quorum = to_c_string(env, &quorum_url);
        let config = DashSDKConfig {
            network: match network {
                0 => dash_network::ffi::FFINetwork::Mainnet,
                2 => dash_network::ffi::FFINetwork::Devnet,
                3 => dash_network::ffi::FFINetwork::Regtest,
                _ => dash_network::ffi::FFINetwork::Testnet,
            },
            dapi_addresses: dapi.as_ref().map_or(ptr::null(), |s| s.as_ptr()),
            skip_asset_lock_proof_verification: skip_asset_lock_proof_verification == JNI_TRUE,
            request_retry_count: request_retry_count.max(0) as u32,
            request_timeout_ms: request_timeout_ms.max(0) as u64,
            quorum_url: quorum.as_ref().map_or(ptr::null(), |s| s.as_ptr()),
            // 0 = SDK default (auto-detect); non-zero pins a protocol version.
            platform_version: platform_version.max(0) as u32,
        };
        // Config strings are borrowed by the call and copied immediately;
        // the CStrings drop after it returns, per the FFI lifetime contract.
        let result = unsafe { dash_sdk_create_trusted(&config) };
        unsafe { unwrap_handle(env, result) }
    })
}

/// Destroy an SDK handle previously returned by `createTrusted`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_SdkNative_destroy(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    guard(&mut env, (), |_| unsafe {
        dash_sdk_destroy(handle as *mut SDKHandle)
    });
}

/// Network of a live SDK handle, as an FFINetwork ordinal.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_SdkNative_getNetwork(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    guard(&mut env, 0, |_| unsafe {
        dash_sdk_get_network(handle as *const SDKHandle) as jint
    })
}

/// Route the global tracing subscriber to per-bucket files under
/// `sessionRoot` (platform-wallet-ffi). Returns false if a subscriber is
/// already installed or the path is unwritable.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_SdkNative_enableFileLogging(
    mut env: JNIEnv,
    _class: JClass,
    level: jint,
    session_root: JString,
) -> jboolean {
    guard(&mut env, JNI_FALSE, |env| {
        let Some(path) = to_c_string(env, &session_root) else {
            return JNI_FALSE;
        };
        let installed = unsafe {
            platform_wallet_ffi::platform_wallet_enable_file_logging(
                level.clamp(0, 4) as u8,
                path.as_ptr(),
            )
        };
        if installed {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    })
}

/// Whether this build of the native library includes shielded (Orchard) sync.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_SdkNative_hasShielded(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    if cfg!(feature = "shielded") {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

/// Convert a nullable `JString` to an owned `CString`, treating JNI failures
/// as null. The returned pointer feeds borrowed-string FFI config fields.
fn to_c_string(env: &mut JNIEnv, value: &JString) -> Option<CString> {
    if value.is_null() {
        return None;
    }
    let rust_string: String = env.get_string(value).ok()?.into();
    CString::new(rust_string).ok()
}

// Keep the raw C ABI of the dependent FFI crates reachable so their symbols
// stay exported from this cdylib even before the shim references every
// module. `dash_sdk_version`'s address is enough to anchor rs-sdk-ffi;
// platform-wallet-ffi and key-wallet-ffi are anchored once their bridges
// land (persistence.rs, signer.rs, wallet_manager.rs).
#[allow(dead_code)]
fn _anchor() -> *const c_char {
    dash_sdk_version()
}
