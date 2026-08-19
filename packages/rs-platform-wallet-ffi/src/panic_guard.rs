//! Panic containment for this crate's `extern "C"` boundary.
//!
//! # Why this exists
//!
//! Every entry point here is a `pub unsafe extern "C" fn`, and `extern "C"`
//! is a **non-unwind** ABI. rustc plants the abort shim in the *callee*, so a
//! panic that reaches the boundary calls `abort()` (SIGABRT) from inside this
//! library — it never becomes an unwind that the caller could observe. That
//! is why `rs-unified-sdk-jni`'s `support::guard` (a `catch_unwind` on the
//! Rust side of each JNI export) cannot intercept it: the process is already
//! gone before control would return to the shim.
//!
//! The workspace states the requirement explicitly for the Android profiles
//! (`Cargo.toml`, `[profile.release-android]`): *"a JNI library must never
//! abort the app process — panics are caught at the shim boundary and
//! rethrown as Java exceptions."* That only holds if a panic can actually
//! *reach* the shim, so this crate has to stop it one frame earlier, on its
//! own side of the boundary.
//!
//! # The contract
//!
//! A caught panic is converted into the crate's generic error result —
//! [`PlatformWalletFFIResultCode::ErrorWalletOperation`] (code 6) — with a
//! message that starts with [`FFI_PANIC_PREFIX`], carries the panic payload,
//! and names the guarded call site. The panic is also logged at `ERROR` on
//! the `platform_wallet_ffi::panic` target before it is converted, so a
//! swallowed panic still leaves a trace in logcat / the host log.
//!
//! **A panic is never a domain outcome.** It says nothing about whether a
//! transition reached the network, which is why it maps to the generic code
//! rather than to any of the typed broadcast codes: hosts must treat it as
//! "unknown outcome" and reconcile against chain state, exactly as they would
//! for any other `ErrorWalletOperation`.
//!
//! # iOS
//!
//! `[profile.release-ios]` and `[profile.dev-ios]` deliberately set
//! `panic = "abort"`. Under those profiles [`std::panic::catch_unwind`] still
//! compiles — it simply never observes an `Err`, because the panic aborts
//! before unwinding starts. Nothing in this module is `cfg`-gated on the
//! panic strategy: the same source builds under both, and the iOS carve-out
//! keeps its documented behavior.

use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe, Location};

use platform_wallet::PlatformWalletError;

use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};

/// Machine-recognizable marker at position 0 of every FFI message this
/// module synthesizes from a caught panic.
///
/// Hosts (and log greps) can use it to tell an internal panic apart from an
/// ordinary generic wallet-operation failure, both of which arrive as code 6.
pub(crate) const FFI_PANIC_PREFIX: &str = "Internal panic caught at the FFI boundary: ";

/// Render a `catch_unwind` / `JoinError` payload as a human-readable string.
///
/// `panic!("literal")` payloads are `&'static str`; formatted payloads are
/// `String`. Anything else (a custom `panic_any`) has no textual form, so it
/// is reported by type rather than dropped silently.
pub(crate) fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

/// Compose the FFI-visible message for a caught panic: the marker prefix, the
/// guarded call site, and the panic payload.
pub(crate) fn ffi_panic_message(location: &Location<'_>, detail: &str) -> String {
    format!(
        "{FFI_PANIC_PREFIX}{detail} (guarded call site: {}:{})",
        location.file(),
        location.line()
    )
}

/// A type that can represent "the work panicked" as a value, so a guard can
/// return instead of letting the unwind reach the `extern "C"` abort shim.
///
/// Deliberately implemented **only** for shapes whose fallback is
/// unambiguously an error signal: an FFI result, a `Result`, or `()`. It is
/// NOT implemented for bare value types (`u64`, `Vec<_>`, a balance struct,
/// …) — fabricating a plausible-looking value for those would convert a
/// crash into silent data corruption, which is worse than the abort this
/// module exists to prevent. A call site whose output is a bare value
/// therefore fails to compile against [`crate::runtime::block_on_worker`],
/// which is the intended signal to hand it a `Result` or to guard its entry
/// point explicitly.
pub(crate) trait FromCaughtPanic: Sized {
    /// Build the "work panicked" value. `message` is already composed by
    /// [`ffi_panic_message`] and carries the marker prefix.
    fn from_caught_panic(message: String) -> Self;
}

/// The error half of [`FromCaughtPanic`] for `Result<T, E>` outputs.
///
/// Each impl picks the most honest *generic* variant its type offers; none of
/// them reach for a variant that carries an outcome guarantee.
pub(crate) trait FromCaughtPanicError: Sized {
    fn from_caught_panic_error(message: String) -> Self;
}

impl FromCaughtPanic for () {
    /// The last-resort fallback, for fire-and-forget work whose future
    /// genuinely yields nothing (`abandon_transaction`, `release`, a stop
    /// signal). There is no value to carry a failure in, so the guard's
    /// `ERROR` log — payload plus guarded call site — is the whole record,
    /// and the entry point goes on to report whatever it was going to report.
    ///
    /// That makes a swallowed panic look like success to the host, so it is
    /// **only** acceptable where the call is already best-effort cleanup on a
    /// path that returns an error for its own reasons. Anything whose success
    /// the host acts on must use [`crate::runtime::FfiRuntime::try_block_on`]
    /// (or [`crate::runtime::try_block_on_worker`]) and report the `Err`.
    fn from_caught_panic(_message: String) -> Self {}
}

impl FromCaughtPanic for PlatformWalletFFIResult {
    fn from_caught_panic(message: String) -> Self {
        PlatformWalletFFIResult::err(PlatformWalletFFIResultCode::ErrorWalletOperation, message)
    }
}

impl<T, E: FromCaughtPanicError> FromCaughtPanic for Result<T, E> {
    fn from_caught_panic(message: String) -> Self {
        Err(E::from_caught_panic_error(message))
    }
}

impl FromCaughtPanicError for PlatformWalletError {
    fn from_caught_panic_error(message: String) -> Self {
        PlatformWalletError::InternalPanic(message)
    }
}

impl FromCaughtPanicError for dash_sdk::Error {
    fn from_caught_panic_error(message: String) -> Self {
        dash_sdk::Error::Generic(message)
    }
}

impl FromCaughtPanicError for anyhow::Error {
    fn from_caught_panic_error(message: String) -> Self {
        anyhow::Error::msg(message)
    }
}

impl FromCaughtPanicError for std::io::Error {
    fn from_caught_panic_error(message: String) -> Self {
        std::io::Error::other(message)
    }
}

impl FromCaughtPanicError for String {
    fn from_caught_panic_error(message: String) -> Self {
        message
    }
}

impl FromCaughtPanicError for Box<dyn std::error::Error + Send + Sync> {
    fn from_caught_panic_error(message: String) -> Self {
        message.into()
    }
}

impl FromCaughtPanicError for PlatformWalletFFIResult {
    fn from_caught_panic_error(message: String) -> Self {
        <Self as FromCaughtPanic>::from_caught_panic(message)
    }
}

/// Run `f`, converting a panic into `T`'s error representation instead of
/// letting it unwind into the `extern "C"` abort shim.
///
/// `#[track_caller]` so the reported location is the *guarded call site*
/// (the entry point's `block_on`, or the `guard_ffi` wrapping an entry-point
/// body) rather than a line inside this module.
///
/// `AssertUnwindSafe` is required and is sound here for the same reason it is
/// in the JNI shim: the guarded work owns FFI-local state, and every value
/// that outlives the guard is either dropped during the unwind or is a handle
/// whose interior locks (`parking_lot`) do not poison. A panic mid-mutation
/// can still leave wallet state stale, so the guard reports the failure and
/// leaves recovery to the host rather than pretending the work succeeded.
#[track_caller]
pub(crate) fn guard_ffi<T: FromCaughtPanic>(f: impl FnOnce() -> T) -> T {
    let location = Location::caller();
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(payload) => T::from_caught_panic(report_panic(
            location,
            &panic_payload_message(payload.as_ref()),
        )),
    }
}

/// Log a caught panic and compose its FFI-visible message.
///
/// Shared by [`guard_ffi`] and by [`crate::runtime::block_on_worker`]'s
/// `JoinError` arm (where tokio caught the panic for us and hands back the
/// payload rather than an unwind).
#[must_use]
pub(crate) fn report_panic(location: &Location<'_>, detail: &str) -> String {
    let message = ffi_panic_message(location, detail);
    tracing::error!(
        target: "platform_wallet_ffi::panic",
        panic = %detail,
        call_site = %format_args!("{}:{}", location.file(), location.line()),
        "caught panic below an FFI entry point; returning an error result instead of aborting"
    );
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_message_reads_str_and_string_panics() {
        let str_payload = catch_unwind(|| panic!("literal boom")).unwrap_err();
        assert_eq!(panic_payload_message(str_payload.as_ref()), "literal boom");

        let value = 7;
        let string_payload = catch_unwind(|| panic!("formatted boom {value}")).unwrap_err();
        assert_eq!(
            panic_payload_message(string_payload.as_ref()),
            "formatted boom 7"
        );

        let other_payload = catch_unwind(|| std::panic::panic_any(7u8)).unwrap_err();
        assert_eq!(
            panic_payload_message(other_payload.as_ref()),
            "<non-string panic payload>"
        );
    }

    #[test]
    fn guard_returns_generic_error_result_with_payload() {
        let result: PlatformWalletFFIResult = guard_ffi(|| panic!("guarded boom"));
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        let message = unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_str()
            .expect("message is UTF-8");
        assert!(
            message.starts_with(FFI_PANIC_PREFIX),
            "message must be recognizable as a panic: {message}"
        );
        assert!(
            message.contains("guarded boom"),
            "message must carry the panic payload: {message}"
        );
        assert!(
            message.contains("panic_guard.rs"),
            "message must name the guarded call site: {message}"
        );
    }

    #[test]
    fn guard_maps_result_outputs_to_the_typed_internal_panic() {
        let result: Result<u64, PlatformWalletError> = guard_ffi(|| panic!("result boom"));
        match result {
            Err(PlatformWalletError::InternalPanic(message)) => {
                assert!(message.starts_with(FFI_PANIC_PREFIX));
                assert!(message.contains("result boom"));
            }
            other => panic!("expected InternalPanic, got {other:?}"),
        }
    }

    #[test]
    fn guard_passes_values_through_untouched() {
        let ok: Result<u64, PlatformWalletError> = guard_ffi(|| Ok(42));
        assert_eq!(ok.expect("no panic"), 42);
    }
}
