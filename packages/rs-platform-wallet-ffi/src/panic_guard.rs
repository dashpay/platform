//! Keeping a panic in an FFI export body from unwinding into the C ABI.

use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};

/// Render a caught panic payload as a human-readable string.
fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

/// Run an FFI export body under [`std::panic::catch_unwind`], converting a panic into the
/// operation's contract-appropriate result `code` (with `guidance` appended to the message)
/// instead of letting it reach the `extern "C"` frame.
///
/// A Rust panic cannot unwind through a C ABI boundary: it aborts the process. The JNI layer
/// wraps its calls in `support::guard` (which catches panics and raises a Java exception), but
/// that guard sits on the FAR side of this `extern "C"` export, so it never sees the unwind — the
/// process is already gone. `block_on_worker` makes this reachable rather than theoretical: it
/// `.expect`s on the tokio `JoinError`, so any panic inside the proving future (Halo 2 synthesis,
/// note bookkeeping, the SDK) re-panics right here inside the export.
///
/// The `code` must be chosen per operation to preserve that operation's result contract — a
/// panic can strike after side effects (note reservation, a broadcast) have happened, so the
/// outcome is genuinely ambiguous and the code must never promise a definitive failure. See
/// `shielded_send`'s `catch_spend_panic` and the per-export call sites.
///
/// NOTE: this guard is only effective where panics unwind. The Android (`*-android`) and
/// host/test profiles build with `panic = "unwind"`, so it works there; the iOS profiles
/// (`dev-ios` / `release-ios`) build with `panic = "abort"` as part of their staticlib size
/// tuning (see the workspace `Cargo.toml` profile comments), so on iOS a panic still aborts the
/// process before this guard can see it.
pub(crate) fn catch_panic_to_code(
    operation: &str,
    code: PlatformWalletFFIResultCode,
    guidance: &str,
    body: impl FnOnce() -> PlatformWalletFFIResult,
) -> PlatformWalletFFIResult {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(result) => result,
        Err(payload) => PlatformWalletFFIResult::err(
            code,
            format!(
                "{operation} panicked: {}. {guidance}",
                panic_payload_message(payload.as_ref())
            ),
        ),
    }
}
