//! Shared JNI plumbing: JVM caching, panic guards, exception throwing.

use dash_network::ffi::FFINetwork;
use jni::objects::JThrowable;
use jni::{JNIEnv, JavaVM};
use platform_wallet_ffi::error::{
    platform_wallet_ffi_result_free, PlatformWalletFFIResult, PlatformWalletFFIResultCode,
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

/// Android's generic crash-recovery APIs must never authorize consumption of
/// a bearer invitation voucher. That authority belongs exclusively to the
/// separate invitation-reclaim flow, even if a caller bypasses the Kotlin
/// wrapper and invokes JNI directly.
pub(crate) fn generic_asset_lock_recovery_allowed(consume_invitation: bool) -> bool {
    !consume_invitation
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
    let message = if result.message.is_null() {
        format!("platform-wallet error (code {})", result.code as i32)
    } else {
        // SAFETY: non-null message is a valid CString produced by the FFI.
        unsafe { CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned()
    };
    // Diagnostic breadcrumb (warn-level so it provably reaches logcat): the raw
    // platform-wallet code and the offset code Kotlin will see. The message
    // itself is NOT logged — it is an unbounded native string that can carry
    // caller-supplied text, query shapes or contract internals, and a device log
    // is readable by any process holding the log permission and is captured in
    // bug reports. The caller still receives it on the exception, so nothing is
    // lost; the two codes are enough to line a report up against either side of
    // the mapping.
    log::warn!(
        "{}",
        platform_wallet_error_breadcrumb(result.code as i32, &message)
    );
    throw_sdk_exception(env, result.code as i32 + PWFFI_CODE_OFFSET, &message);
    // SAFETY: `result` is a fresh PlatformWalletFFIResult; free its message.
    unsafe { platform_wallet_ffi_result_free(&mut result) };
    true
}

/// The breadcrumb recorded when a platform-wallet result is converted into a
/// Kotlin exception.
///
/// Records the raw platform-wallet code and the offset code the caller will see,
/// and deliberately renders neither the message nor anything derived from it.
/// The message is an unbounded native string that can carry caller-supplied
/// text, query shapes or contract internals; a device log is readable by any
/// process holding the log permission and is captured in bug reports. The
/// caller still receives the message on the exception itself, so keeping both
/// codes is enough to line a report up against either side of the mapping
/// without carrying anything unbounded.
pub(crate) fn platform_wallet_error_breadcrumb(
    platform_wallet_code: i32,
    _message: &str,
) -> String {
    format!(
        "take_pwffi_error: platform_wallet_code={} thrown_code={}",
        platform_wallet_code,
        platform_wallet_code + PWFFI_CODE_OFFSET
    )
}

/// The breadcrumb recorded when an exception is thrown to Kotlin.
///
/// Same reasoning as [`platform_wallet_error_breadcrumb`]: the message reaches
/// the caller on the exception, so the log records which error was raised
/// rather than what it said.
pub(crate) fn thrown_exception_breadcrumb(code: i32, _message: &str) -> String {
    format!("throw_sdk_exception: code={code}")
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
    // Diagnostic breadcrumb (warn-level so it provably reaches logcat): every
    // native→Kotlin error conversion is visible even when the Kotlin caller
    // contains the exception into a status line. Only the code is recorded —
    // see [`platform_wallet_error_breadcrumb`] for why the message is not.
    log::warn!("{}", thrown_exception_breadcrumb(code, message));
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

#[cfg(test)]
mod tests {
    use super::{
        generic_asset_lock_recovery_allowed, net_from_ord, platform_wallet_error_breadcrumb,
        thrown_exception_breadcrumb, PWFFI_CODE_OFFSET,
    };
    use dash_network::ffi::FFINetwork;

    /// A message shaped like the worst thing a native error can carry: a marker
    /// standing in for caller-supplied or contract-internal text, and an
    /// embedded newline that would forge an additional log line.
    const HOSTILE_MESSAGE: &str =
        "failed for ownerId 5Dc…\nFORGED WARN line s3cr3t-marker-do-not-log";
    const MARKER: &str = "s3cr3t-marker-do-not-log";

    /// The breadcrumb that accompanies every native→Kotlin error conversion
    /// records the two codes and nothing from the message.
    ///
    /// The message is unbounded and can carry caller-supplied text, query shapes
    /// or contract internals; the caller still receives it on the exception, so
    /// nothing is lost by keeping it out of a device log.
    #[test]
    fn the_platform_wallet_error_breadcrumb_records_codes_and_never_the_message() {
        let line = platform_wallet_error_breadcrumb(6, HOSTILE_MESSAGE);

        assert!(
            !line.contains(MARKER),
            "the message body must never reach the log: {line}"
        );
        assert!(
            !line.contains('\n'),
            "an embedded newline would let an error body forge further log lines: {line}"
        );
        assert!(
            line.contains("platform_wallet_code=6"),
            "the raw platform-wallet code must be recorded: {line}"
        );
        assert!(
            line.contains(&format!("thrown_code={}", 6 + PWFFI_CODE_OFFSET)),
            "the offset code the caller will see must be recorded so a report can \
             be lined up against either side of the mapping: {line}"
        );
    }

    /// Same contract on the throw path, which every JNI export reaches.
    #[test]
    fn the_thrown_exception_breadcrumb_records_the_code_and_never_the_message() {
        let line = thrown_exception_breadcrumb(1042, HOSTILE_MESSAGE);

        assert!(
            !line.contains(MARKER),
            "the message body must never reach the log: {line}"
        );
        assert!(!line.contains('\n'), "no forged log lines: {line}");
        assert_eq!(
            line, "throw_sdk_exception: code=1042",
            "the breadcrumb is the stage label plus the numeric code, nothing else"
        );
    }

    #[test]
    fn generic_asset_lock_recovery_rejects_invitation_authority() {
        assert!(generic_asset_lock_recovery_allowed(false));
        assert!(!generic_asset_lock_recovery_allowed(true));
    }

    #[test]
    fn net_from_ord_matches_kotlin_ffi_values() {
        assert_eq!(net_from_ord(0), FFINetwork::Mainnet);
        assert_eq!(net_from_ord(1), FFINetwork::Testnet);
        assert_eq!(net_from_ord(2), FFINetwork::Devnet);
        assert_eq!(net_from_ord(3), FFINetwork::Regtest);
        assert_eq!(net_from_ord(-1), FFINetwork::Testnet, "unknown → Testnet");
    }
}
