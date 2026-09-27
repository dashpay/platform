//! Shared JNI plumbing: JVM caching, panic guards, exception throwing.

use dash_network::ffi::FFINetwork;
use jni::objects::{JThrowable, JValue};
use jni::sys::jint;
use jni::{JNIEnv, JavaVM};
use platform_wallet_ffi::error::{
    platform_wallet_ffi_result_free, PlatformWalletFFIConsensusErrorKind, PlatformWalletFFIResult,
    PlatformWalletFFIResultCode,
};
use std::ffi::CStr;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::OnceLock;

/// Offset added to every `PlatformWalletFFIResultCode` before it is thrown
/// as a `DashSDKException` code — see [`take_pwffi_error`].
///
/// platform-wallet-ffi's result codes (`PlatformWalletFFIResultCode`, 0–20 +
/// 98/99) occupy the SAME small-integer range as rs-sdk-ffi's
/// `DashSDKErrorCode` (1–10), which Kotlin's `DashSdkError.fromNative`
/// interprets. Throwing a raw platform-wallet code would collide: e.g.
/// `ErrorWalletOperation` (6) would surface to Kotlin as `CryptoError` (6),
/// and the retry-semantics-bearing codes (`ErrorShieldedNoRecordedAnchor`
/// = 19 retryable, `ErrorTransactionBroadcastUnconfirmed` = 20 do-NOT-retry)
/// would flatten into the `else -> InternalError` bucket, losing their
/// contract. Shifting into a dedicated `>= 1000` namespace lets
/// `DashSdkError.fromNative` route these to a distinct `PlatformWallet`
/// subtree (by subtracting the offset) while the native rs-sdk-ffi codes
/// stay in 1–10. Must stay in lockstep with
/// `DashSdkError.PLATFORM_WALLET_CODE_OFFSET` on the Kotlin side.
pub const PWFFI_CODE_OFFSET: i32 = 1000;

/// FFINetwork ordinal → enum (0=Mainnet, 2=Devnet, 3=Regtest, else
/// Testnet). Must stay in lockstep with Kotlin's `Network.ffiValue`.
///
/// This is the single shared mapping for every JNI module:
/// `rs_sdk_ffi::FFINetwork` and `platform_wallet_ffi::FFINetwork` are both
/// re-exports of this same `dash_network::ffi::FFINetwork`, so the one
/// helper serves callers regardless of which FFI crate they talk to.
pub fn net_from_ord(ord: i32) -> FFINetwork {
    match ord {
        0 => FFINetwork::Mainnet,
        2 => FFINetwork::Devnet,
        3 => FFINetwork::Regtest,
        _ => FFINetwork::Testnet,
    }
}

/// If `result` carries a non-`Success` code: throw `DashSDKException`,
/// free its message, and return `true` (the caller bails with its
/// default). On `Success` frees nothing (message is null) and returns
/// `false`.
///
/// The thrown exception code is the `PlatformWalletFFIResultCode` value
/// shifted by [`PWFFI_CODE_OFFSET`] so it never collides with the native
/// rs-sdk-ffi `DashSDKErrorCode` range Kotlin's `DashSdkError.fromNative`
/// also decodes. This is the single shared mapping used by every JNI module
/// that calls a platform-wallet-ffi entry point (mirrors `results::take_error`
/// for the rs-sdk-ffi side).
pub fn take_pwffi_error(env: &mut JNIEnv, mut result: PlatformWalletFFIResult) -> bool {
    if result.code == PlatformWalletFFIResultCode::Success {
        return false;
    }
    throw_pwffi_result(env, &result);
    // SAFETY: `result` is a fresh PlatformWalletFFIResult; free its message.
    unsafe { platform_wallet_ffi_result_free(&mut result) };
    true
}

/// Throw the `DashSDKException` for a non-`Success` `result`, leaving its
/// message for the caller to free. The exception code is the result code
/// shifted by [`PWFFI_CODE_OFFSET`]; a result that carries a consensus
/// rejection (`consensus_code != 0`) hands its code and kind to the exception
/// as well, so Kotlin can branch on them instead of on the message.
pub fn throw_pwffi_result(env: &mut JNIEnv, result: &PlatformWalletFFIResult) {
    let message = if result.message.is_null() {
        format!("platform-wallet error (code {})", result.code as i32)
    } else {
        // SAFETY: non-null message is a valid CString produced by the FFI.
        unsafe { CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned()
    };
    let code = result.code as i32 + PWFFI_CODE_OFFSET;
    if result.consensus_code == 0 {
        throw_sdk_exception(env, code, &message);
    } else {
        throw_sdk_consensus_exception(
            env,
            code,
            &message,
            result.consensus_code,
            result.consensus_kind,
        );
    }
}

/// The process-wide JVM, cached in [`crate::JNI_OnLoad`]. Callback
/// trampolines use this to attach Tokio worker threads.
pub static JVM: OnceLock<JavaVM> = OnceLock::new();

/// JNI-internal name of the Kotlin exception type thrown for SDK errors.
pub const SDK_EXCEPTION_CLASS: &str = "org/dashfoundation/dashsdk/ffi/DashSDKException";

/// Throw `DashSDKException(code, message)`; falls back to a plain
/// `RuntimeException` if the class or constructor lookup fails (e.g. the
/// library is loaded outside the Kotlin SDK).
pub fn throw_sdk_exception(env: &mut JNIEnv, code: i32, message: &str) {
    throw_sdk_exception_with(env, code, message, "(ILjava/lang/String;)V", &[]);
}

/// Throw `DashSDKException(code, message, consensusCode, consensusKind)` for
/// a failure that is a consensus rejection. `consensus_kind` crosses as its
/// discriminant, which Kotlin's `ConsensusErrorKind.fromNative` decodes. Same
/// `RuntimeException` fallback as [`throw_sdk_exception`].
pub fn throw_sdk_consensus_exception(
    env: &mut JNIEnv,
    code: i32,
    message: &str,
    consensus_code: u32,
    consensus_kind: PlatformWalletFFIConsensusErrorKind,
) {
    // Consensus codes top out in the 40000s, far inside a `jint`; one that
    // somehow is not goes out as `jint::MAX` rather than wrapping negative.
    let consensus_code = jint::try_from(consensus_code).unwrap_or(jint::MAX);
    throw_sdk_exception_with(
        env,
        code,
        message,
        "(ILjava/lang/String;II)V",
        &[consensus_code, consensus_kind as jint],
    );
}

/// Construct `DashSDKException` through the constructor `signature` names,
/// passing `code`, `message` and then `extra`, and throw it.
fn throw_sdk_exception_with(
    env: &mut JNIEnv,
    code: i32,
    message: &str,
    signature: &str,
    extra: &[jint],
) {
    // If an exception is already pending we must not call further JNI
    // functions that would themselves throw.
    if env.exception_check().unwrap_or(false) {
        return;
    }
    let thrown = (|| -> jni::errors::Result<()> {
        let jmsg = env.new_string(message)?;
        let mut args: Vec<JValue> = vec![code.into(), (&jmsg).into()];
        args.extend(extra.iter().map(|value| JValue::from(*value)));
        let obj = env.new_object(SDK_EXCEPTION_CLASS, signature, &args)?;
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

#[cfg(test)]
mod tests {
    use super::net_from_ord;
    use dash_network::ffi::FFINetwork;

    #[test]
    fn net_from_ord_matches_kotlin_ffi_values() {
        assert_eq!(net_from_ord(0), FFINetwork::Mainnet);
        assert_eq!(net_from_ord(1), FFINetwork::Testnet);
        assert_eq!(net_from_ord(2), FFINetwork::Devnet);
        assert_eq!(net_from_ord(3), FFINetwork::Regtest);
        assert_eq!(net_from_ord(-1), FFINetwork::Testnet, "unknown → Testnet");
    }
}
