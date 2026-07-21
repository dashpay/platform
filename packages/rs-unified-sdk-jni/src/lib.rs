//! JNI bindings for the Dash Platform SDK (Android/Kotlin).
//!
//! This crate is the Android analog of the iOS umbrella framework: a thin
//! JNI shim that calls the `extern "C"` entry points of `rs-sdk-ffi`,
//! `platform-wallet-ffi` and `key-wallet-ffi` **as ordinary Rust functions**
//! (they are rlib dependencies), so `DashSDKResult` and every other
//! `#[repr(C)]` struct is consumed inside Rust and never crosses the JNI
//! boundary by value.
//!
//! Conventions (see packages/kotlin-sdk/CLAUDE.md):
//! - Opaque handles cross JNI as `jlong`.
//! - Errors surface as thrown `org.dashfoundation.dashsdk.ffi.DashSDKException`.
//! - Panics never unwind across JNI: every export body runs under
//!   [`support::guard`].
//! - Rust→Kotlin callbacks attach their (possibly Tokio) thread as a daemon.

mod credits;
mod dashpay;
mod events;
mod funding;
mod identity;
mod mnemonic;
mod persistence;
mod queries;
mod results;
mod sdk;
mod signer;
mod support;
mod tokens;
mod transactions;
mod wallet_manager;

use jni::sys::{jint, JNI_VERSION_1_6};
use jni::JavaVM;
use std::ffi::c_void;

/// Called by ART when `System.loadLibrary("dash_sdk_jni")` runs.
///
/// Caches the `JavaVM` for callback threads and installs the Android logger
/// so `log`/`tracing` output reaches logcat.
#[no_mangle]
pub extern "system" fn JNI_OnLoad(vm: JavaVM, _reserved: *mut c_void) -> jint {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("DashSDK"),
    );

    let _ = support::JVM.set(vm);
    JNI_VERSION_1_6
}
