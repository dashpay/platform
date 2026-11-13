//! JNI exports for SDK lifecycle: init, logging, version, create, destroy,
//! and the quorum mode and source of the SDK's context provider.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.SdkNative`.

use crate::results::{unwrap_handle, unwrap_void};
use crate::support::{guard, net_from_ord};
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jint, jlong, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use rs_sdk_ffi::{
    dash_sdk_create_adaptive, dash_sdk_create_trusted, dash_sdk_destroy, dash_sdk_enable_logging,
    dash_sdk_get_context_provider_source, dash_sdk_get_network, dash_sdk_init,
    dash_sdk_set_context_provider_mode, dash_sdk_version, DashSDKConfig, DashSDKResult, SDKHandle,
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
    create(
        &mut env,
        dash_sdk_create_trusted,
        SdkSettings {
            network,
            dapi_addresses,
            quorum_url,
            skip_asset_lock_proof_verification,
            request_retry_count,
            request_timeout_ms,
            platform_version,
        },
    )
}

/// Create an SDK instance with the adaptive context provider, in Auto mode
/// (`dash_sdk_create_adaptive`); returns the `SDKHandle` as jlong. The
/// arguments are `createTrusted`'s.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_SdkNative_createAdaptive(
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
    create(
        &mut env,
        dash_sdk_create_adaptive,
        SdkSettings {
            network,
            dapi_addresses,
            quorum_url,
            skip_asset_lock_proof_verification,
            request_retry_count,
            request_timeout_ms,
            platform_version,
        },
    )
}

/// The arguments every `SdkNative.create*` export takes, as Kotlin passes
/// them.
struct SdkSettings<'local> {
    network: jint,
    dapi_addresses: JString<'local>,
    quorum_url: JString<'local>,
    skip_asset_lock_proof_verification: jboolean,
    request_retry_count: jint,
    request_timeout_ms: jlong,
    platform_version: jint,
}

/// Build a `DashSDKConfig` from `settings` and create an SDK with
/// `create_sdk`, one of rs-sdk-ffi's `dash_sdk_create_*` entry points.
fn create(
    env: &mut JNIEnv,
    create_sdk: unsafe extern "C" fn(*const DashSDKConfig) -> DashSDKResult,
    settings: SdkSettings,
) -> jlong {
    guard(env, 0, |env| {
        let dapi = to_c_string(env, &settings.dapi_addresses);
        let quorum = to_c_string(env, &settings.quorum_url);
        let config = DashSDKConfig {
            network: net_from_ord(settings.network),
            dapi_addresses: dapi.as_ref().map_or(ptr::null(), |s| s.as_ptr()),
            skip_asset_lock_proof_verification: settings.skip_asset_lock_proof_verification
                == JNI_TRUE,
            request_retry_count: settings.request_retry_count.max(0) as u32,
            request_timeout_ms: settings.request_timeout_ms.max(0) as u64,
            quorum_url: quorum.as_ref().map_or(ptr::null(), |s| s.as_ptr()),
            // 0 = SDK default (auto-detect); non-zero pins a protocol version.
            platform_version: settings.platform_version.max(0) as u32,
        };
        // Config strings are borrowed by the call and copied immediately;
        // the CStrings drop after it returns, per the FFI lifetime contract.
        let result = unsafe { create_sdk(&config) };
        unsafe { unwrap_handle(env, result) }
    })
}

/// Set the quorum mode of the SDK's context provider
/// (`dash_sdk_set_context_provider_mode`): 0=Auto, 1=SPV, 2=Trusted. Throws
/// when the provider has no mode or the mode is unknown; a value outside a
/// byte reaches the FFI as one it refuses.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_SdkNative_setQuorumMode(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    mode: jint,
) {
    guard(&mut env, (), |env| {
        let mode = u8::try_from(mode).unwrap_or(u8::MAX);
        let result = unsafe { dash_sdk_set_context_provider_mode(handle as *mut SDKHandle, mode) };
        unsafe { unwrap_void(env, result) }
    })
}

/// The source the SDK's context provider asks first for quorum keys
/// (`dash_sdk_get_context_provider_source`): 0=Trusted, 1=SPV, 2=a provider
/// that does not say.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_SdkNative_quorumSource(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    guard(&mut env, 2, |_| unsafe {
        jint::from(dash_sdk_get_context_provider_source(
            handle as *const SDKHandle,
        ))
    })
}

/// Destroy an SDK handle previously returned by `createTrusted` or
/// `createAdaptive`.
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
// platform-wallet-ffi is anchored once its bridges land (persistence.rs,
// signer.rs, wallet_manager.rs).
#[allow(dead_code)]
fn _anchor() -> *const c_char {
    dash_sdk_version()
}
