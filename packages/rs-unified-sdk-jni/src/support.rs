//! Shared JNI plumbing: JVM caching, panic guards, exception throwing.

use jni::objects::JThrowable;
use jni::{JNIEnv, JavaVM};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::OnceLock;

/// The process-wide JVM, cached in [`crate::JNI_OnLoad`]. Callback
/// trampolines use this to attach Tokio worker threads.
pub static JVM: OnceLock<JavaVM> = OnceLock::new();

/// JNI-internal name of the Kotlin exception type thrown for SDK errors.
pub const SDK_EXCEPTION_CLASS: &str = "org/dashfoundation/dashsdk/ffi/DashSDKException";

/// Throw `DashSDKException(code, message)`; falls back to a plain
/// `RuntimeException` if the class or constructor lookup fails (e.g. the
/// library is loaded outside the Kotlin SDK).
pub fn throw_sdk_exception(env: &mut JNIEnv, code: i32, message: &str) {
    // If an exception is already pending we must not call further JNI
    // functions that would themselves throw.
    if env.exception_check().unwrap_or(false) {
        return;
    }
    let thrown = (|| -> jni::errors::Result<()> {
        let jmsg = env.new_string(message)?;
        let obj = env.new_object(
            SDK_EXCEPTION_CLASS,
            "(ILjava/lang/String;)V",
            &[code.into(), (&jmsg).into()],
        )?;
        env.throw(JThrowable::from(obj))
    })();
    if thrown.is_err() {
        let _ = env.exception_clear();
        let _ = env.throw_new("java/lang/RuntimeException", message);
    }
}

/// Run an export body under `catch_unwind` so a Rust panic surfaces as a
/// Java `RuntimeException` instead of unwinding across the JNI boundary
/// (which is undefined behavior).
pub fn guard<T>(env: &mut JNIEnv, default: T, f: impl FnOnce(&mut JNIEnv) -> T) -> T {
    match catch_unwind(AssertUnwindSafe(|| f(env))) {
        Ok(value) => value,
        Err(panic) => {
            let msg = panic
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "Rust panic in dash_sdk_jni".to_string());
            if !env.exception_check().unwrap_or(false) {
                let _ = env.throw_new("java/lang/RuntimeException", &msg);
            }
            default
        }
    }
}
