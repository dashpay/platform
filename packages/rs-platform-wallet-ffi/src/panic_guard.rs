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
//! ## The panic rides OUTSIDE the operation's own error type
//!
//! Keeping code 6 and the position-zero marker is only worth something if the
//! message survives to the host *unchanged*. An earlier shape folded the
//! panic into whatever error type the guarded future already returned, and
//! that lost the contract at three different call sites: `From<Box<dyn
//! Error>>` re-codes to `ErrorUnknown` (99) and prefixes `unclassified error:
//! `; `core_wallet::transaction_builder` prefixes `add_inputs_from_outpoints
//! failed: `; the catch-all `PlatformWalletError` handlers in `shielded_send`
//! prepend operation context (dashpay/platform#4424 review).
//!
//! So the carrier is an **FFI-local outer result** — [`FfiOutcome`], whose
//! error half is [`FfiBoundaryError`] — that sits *around* the domain result
//! rather than inside it. `From<FfiBoundaryError> for PlatformWalletFFIResult`
//! is the only conversion it has, it is defined in this crate, and it copies
//! the already-composed message verbatim. Every entry point therefore
//! intercepts the boundary failure **before** any legacy `From<DomainError>`
//! mapping can re-code or re-prefix it.
//!
//! That is also why nothing here reaches into `platform-wallet`'s public
//! `PlatformWalletError`: adding a variant to a non-`#[non_exhaustive]` public
//! enum for a boundary-only failure is a source-breaking change to a
//! lower-layer domain API, and this containment work is not supposed to break
//! anything (same review).
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
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe, Location};

use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};

/// Machine-recognizable marker at position 0 of every FFI message this
/// module synthesizes from a caught panic.
///
/// Hosts (and log greps) can use it to tell an internal panic apart from an
/// ordinary generic wallet-operation failure, both of which arrive as code 6.
pub(crate) const FFI_PANIC_PREFIX: &str = "Internal panic caught at the FFI boundary: ";

/// Machine-recognizable marker at position 0 of the *other* boundary failure:
/// the guarded work never ran because this crate could not obtain somewhere to
/// run it — a tokio runtime whose driver init returned an `io::Error`, or an
/// OS that refused the 8 MB worker thread.
///
/// Distinct from [`FFI_PANIC_PREFIX`] because the two mean different things to
/// a host: a panic is an internal invariant break with an unknown outcome, a
/// missing execution context means the work provably never started. Both are
/// code 6 — neither is a domain outcome.
pub(crate) const FFI_RUNTIME_UNAVAILABLE_PREFIX: &str = "FFI execution context unavailable: ";

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

/// Make `detail` safe to carry through a `CString`.
///
/// A Rust panic payload may contain interior NUL bytes (`panic!("{}", s)` over
/// attacker- or network-supplied text, a `Debug` rendering of raw bytes).
/// `CString::new` rejects those, and [`PlatformWalletFFIResult::err`]'s
/// fallback then replaces the **whole** message with `<invalid UTF-8>` —
/// discarding the marker, the payload and the call site, i.e. exactly the
/// contract this module exists to publish (dashpay/platform#4424 review).
/// Escaping keeps the message printable and keeps the marker at position 0.
fn escape_nuls(detail: &str) -> String {
    detail.replace('\0', "\\0")
}

/// Compose the FFI-visible message for a caught panic: the marker prefix, the
/// guarded call site, and the panic payload.
pub(crate) fn ffi_panic_message(location: &Location<'_>, detail: &str) -> String {
    let detail = escape_nuls(detail);
    format!(
        "{FFI_PANIC_PREFIX}{detail} (guarded call site: {}:{})",
        location.file(),
        location.line()
    )
}

/// The error half of [`FfiOutcome`]: the guarded work produced no value
/// because **the boundary itself** failed, not the operation.
///
/// Holds the fully composed, FFI-visible message with its marker already at
/// position 0 ([`FFI_PANIC_PREFIX`] or [`FFI_RUNTIME_UNAVAILABLE_PREFIX`]).
/// Deliberately crate-private and deliberately convertible to exactly one
/// thing — [`PlatformWalletFFIResult`], verbatim, at code 6. It has no
/// conversion into any domain error type, which is what structurally prevents
/// the message from being re-coded or re-prefixed on its way out.
#[derive(Debug, Clone)]
pub(crate) struct FfiBoundaryError(String);

impl FfiBoundaryError {
    /// Wrap an already-composed panic message (from [`ffi_panic_message`] via
    /// [`report_panic`]), whose marker is already at position 0.
    pub(crate) fn caught_panic(message: String) -> Self {
        Self(message)
    }

    /// The guarded work never started: no runtime, or no thread to run it on.
    pub(crate) fn runtime_unavailable(detail: &str) -> Self {
        Self(format!(
            "{FFI_RUNTIME_UNAVAILABLE_PREFIX}{}",
            escape_nuls(detail)
        ))
    }
}

impl fmt::Display for FfiBoundaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FfiBoundaryError {}

impl From<FfiBoundaryError> for PlatformWalletFFIResult {
    /// The one conversion, and it is direct: code 6 with the message copied
    /// verbatim, so [`FFI_PANIC_PREFIX`] / [`FFI_RUNTIME_UNAVAILABLE_PREFIX`]
    /// stays at position 0 for the host.
    fn from(error: FfiBoundaryError) -> Self {
        PlatformWalletFFIResult::err(PlatformWalletFFIResultCode::ErrorWalletOperation, error.0)
    }
}

/// The FFI-local **outer** result: "the guarded work produced `T`" versus
/// "the boundary failed and there is no `T`".
///
/// Wrapping rather than folding is the whole point — see the module docs. A
/// `T` that is itself a `Result<_, DomainError>` keeps its own error channel
/// untouched, and the panic travels beside it where no domain-error mapping
/// can reach it.
#[derive(Debug)]
#[must_use]
pub(crate) enum FfiOutcome<T> {
    /// The guarded work ran to completion and produced `T`.
    Ok(T),
    /// The guarded work panicked. The `String` is the composed FFI-visible
    /// message, marker at position 0.
    Panicked(String),
}

impl<T> FfiOutcome<T> {
    /// Project into the `Result` the guarded entry helpers hand out.
    pub(crate) fn into_result(self) -> Result<T, FfiBoundaryError> {
        match self {
            Self::Ok(value) => Ok(value),
            Self::Panicked(message) => Err(FfiBoundaryError::caught_panic(message)),
        }
    }
}

/// The error channel of a guarded call whose work already had one of its own.
///
/// This is [`FfiOutcome`] flattened into the operation's own `Result`, and it
/// is the shape that makes the contract hard to lose: the boundary failure and
/// the domain failure are **separate variants**, so no call site can format,
/// re-code or re-prefix one while believing it is handling the other. The
/// compiler forces every hand-written error arm to say which it means.
///
/// The [`From`] impl below is the interception point the review asked for: it
/// answers [`Self::Boundary`] itself, verbatim, and only delegates to the
/// legacy `From<DomainError>` mapping for [`Self::Domain`].
///
/// **Deliberately not [`Display`](std::fmt::Display).** Interpolating an error
/// into a context string (`format!("{operation} failed: {e}")`) is precisely
/// how the position-zero marker got lost, so the type refuses to be
/// interpolated at all: a site that wants to add context must first say which
/// failure it is talking about, via [`peel_boundary`] or an explicit match.
/// That turns the whole class of regression the review found into a compile
/// error rather than something a future reader has to notice.
#[derive(Debug)]
pub(crate) enum GuardedError<E> {
    /// The boundary failed: a caught panic, or no execution context. Says
    /// **nothing** about whether the operation reached the network.
    Boundary(FfiBoundaryError),
    /// The operation's own error, untouched and still typed.
    Domain(E),
}

impl<E> From<GuardedError<E>> for PlatformWalletFFIResult
where
    PlatformWalletFFIResult: From<E>,
{
    fn from(error: GuardedError<E>) -> Self {
        match error {
            // Intercepted FIRST, and answered here: code 6 with the message
            // verbatim, so the marker stays at position 0. The domain mapping
            // below never sees it and therefore cannot re-code it (as
            // `From<Box<dyn Error>>` did, to `ErrorUnknown` behind an
            // `unclassified error: ` prefix) — dashpay/platform#4424 review.
            GuardedError::Boundary(error) => error.into(),
            GuardedError::Domain(error) => error.into(),
        }
    }
}

/// Peel the boundary failure off a guarded result, so a hand-written match can
/// classify domain errors without ever seeing one.
///
/// `Ok` carries the operation's own `Result` untouched, for the caller's domain
/// arms. `Err` is the **finished** FFI result for a boundary failure — code 6,
/// message verbatim, marker at position 0 — which the caller returns
/// immediately, conventionally via `unwrap_result_or_return!`:
///
/// ```ignore
/// let result = unwrap_result_or_return!(peel_boundary(result));
/// match result {
///     Err(PlatformWalletError::ShieldedSpendUnconfirmed { .. }) => /* typed arm */,
///     Err(e) => /* free to add operation context: `e` cannot be a panic */,
/// }
/// ```
///
/// This is what lets arms like `format!("{operation} failed: {e}")` keep
/// existing: by the time they run, the panic has already left through its own
/// door with its marker intact (dashpay/platform#4424 review).
pub(crate) fn peel_boundary<T, E>(
    result: Result<T, GuardedError<E>>,
) -> Result<Result<T, E>, PlatformWalletFFIResult> {
    match result {
        Ok(value) => Ok(Ok(value)),
        Err(GuardedError::Domain(error)) => Ok(Err(error)),
        Err(GuardedError::Boundary(error)) => Err(error.into()),
    }
}

/// How a guarded helper reshapes its future's output so a boundary failure has
/// somewhere honest to go.
///
/// Three shapes, three answers:
///
/// * `Result<T, E>` gains the [`GuardedError`] wrapper described above — the
///   domain error keeps its own channel, the boundary failure gets its own.
/// * [`PlatformWalletFFIResult`] absorbs it directly: it is already the FFI
///   error channel, and `err(ErrorWalletOperation, message)` is lossless.
/// * `()` swallows it, with the guard's `ERROR` log as the whole record.
///
/// Deliberately NOT implemented for bare value types (`u64`, `Vec<_>`, a
/// balance struct, …): fabricating a plausible-looking zero balance or empty
/// peer list out of a panic would convert a crash into silent data corruption,
/// which is worse than the abort this module exists to prevent. Such a call
/// site fails to compile against [`crate::runtime::RuntimeHandle::block_on`] /
/// [`crate::runtime::block_on_worker`], which is the intended signal to move it
/// to the `try_` sibling and handle the outer `Err`.
pub(crate) trait GuardedOutput {
    /// What the guarded helper returns in place of `Self`.
    type Guarded;

    /// The work completed; hand its value back in the guarded shape.
    fn into_guarded(self) -> Self::Guarded;

    /// The work did not complete because the boundary failed. `error` is
    /// already composed and already logged.
    fn from_boundary_error(error: FfiBoundaryError) -> Self::Guarded;
}

impl GuardedOutput for () {
    type Guarded = ();

    fn into_guarded(self) -> Self::Guarded {}

    /// The last-resort fallback, for fire-and-forget work whose future
    /// genuinely yields nothing (`abandon_transaction`, `release`, a stop
    /// signal). There is no value to carry a failure in, so the guard's
    /// `ERROR` log — payload plus guarded call site — is the whole record,
    /// and the entry point goes on to report whatever it was going to report.
    ///
    /// That makes a swallowed panic look like success to the host, so it is
    /// **only** acceptable where the call is already best-effort cleanup on a
    /// path that returns an error for its own reasons. Anything whose success
    /// the host acts on must use [`crate::runtime::RuntimeHandle::try_block_on`]
    /// (or [`crate::runtime::try_block_on_worker`]) and report the `Err`.
    fn from_boundary_error(_error: FfiBoundaryError) -> Self::Guarded {}
}

impl GuardedOutput for PlatformWalletFFIResult {
    type Guarded = Self;

    fn into_guarded(self) -> Self::Guarded {
        self
    }

    fn from_boundary_error(error: FfiBoundaryError) -> Self::Guarded {
        error.into()
    }
}

impl<T, E> GuardedOutput for Result<T, E> {
    type Guarded = Result<T, GuardedError<E>>;

    fn into_guarded(self) -> Self::Guarded {
        self.map_err(GuardedError::Domain)
    }

    fn from_boundary_error(error: FfiBoundaryError) -> Self::Guarded {
        Err(GuardedError::Boundary(error))
    }
}

/// Run `f`, capturing a panic as a value instead of letting it unwind into the
/// `extern "C"` abort shim.
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
pub(crate) fn guard_ffi<T>(f: impl FnOnce() -> T) -> FfiOutcome<T> {
    let location = Location::caller();
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => FfiOutcome::Ok(value),
        Err(payload) => FfiOutcome::Panicked(report_panic(
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

    /// Read an FFI result's message back as a Rust `String`.
    fn message_of(result: &PlatformWalletFFIResult) -> String {
        assert!(
            !result.message.is_null(),
            "error result must carry a message"
        );
        unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_str()
            .expect("message is UTF-8")
            .to_string()
    }

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
        let outcome: FfiOutcome<()> = guard_ffi(|| panic!("guarded boom"));
        let result: PlatformWalletFFIResult = outcome
            .into_result()
            .expect_err("the guarded work panicked")
            .into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        let message = message_of(&result);
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
    fn guard_passes_values_through_untouched() {
        let ok: Result<u64, FfiBoundaryError> = guard_ffi(|| 42).into_result();
        assert_eq!(ok.expect("no panic"), 42);
    }

    /// A NUL byte in the payload must not cost the host the whole message.
    ///
    /// `CString::new` rejects interior NULs, and `PlatformWalletFFIResult::err`
    /// answers that by substituting `<invalid UTF-8>` for the *entire* string —
    /// so without escaping, a panic whose text happened to contain a NUL would
    /// arrive with no marker, no payload and no call site.
    #[test]
    fn nul_bytes_in_a_payload_keep_the_marker_and_the_payload() {
        let outcome: FfiOutcome<()> = guard_ffi(|| panic!("boom\0with\0nuls"));
        let result: PlatformWalletFFIResult = outcome
            .into_result()
            .expect_err("the guarded work panicked")
            .into();

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        let message = message_of(&result);
        assert!(
            message.starts_with(FFI_PANIC_PREFIX),
            "the marker must survive a NUL-bearing payload: {message}"
        );
        assert!(
            message.contains("boom\\0with\\0nuls"),
            "the payload must survive, with its NULs escaped: {message}"
        );
        assert!(
            message.contains("panic_guard.rs"),
            "the call site must survive: {message}"
        );
        assert!(
            !message.contains("<invalid UTF-8>"),
            "the message must not have collapsed to the CString fallback: {message}"
        );
    }

    /// The same escaping on the other marker's constructor.
    #[test]
    fn runtime_unavailable_escapes_nuls_and_keeps_its_marker() {
        let result: PlatformWalletFFIResult =
            FfiBoundaryError::runtime_unavailable("driver\0failed").into();

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        let message = message_of(&result);
        assert!(
            message.starts_with(FFI_RUNTIME_UNAVAILABLE_PREFIX),
            "{message}"
        );
        assert!(message.contains("driver\\0failed"), "{message}");
    }

    /// The boundary error has exactly one conversion, and it is verbatim: the
    /// marker stays at position 0 and the code stays generic.
    #[test]
    fn boundary_error_converts_verbatim_at_the_generic_code() {
        let error = FfiBoundaryError::caught_panic(format!("{FFI_PANIC_PREFIX}payload"));
        let result: PlatformWalletFFIResult = error.into();

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        assert_eq!(message_of(&result), format!("{FFI_PANIC_PREFIX}payload"));
    }
}
