//! Trait-level taxonomy of `PersistenceError` (CODE-004).
//!
//! TC-CODE-004-a — `Backend { kind, source }` shape exists and the kind
//! enum exhaustively partitions retry policy.
//! TC-CODE-004-c — `source` is `Display + Send + Sync` and surfaces the
//! underlying error message.
//!
//! Storage-side mapping (TC-CODE-004-b) and the wildcard-free invariant
//! (TC-CODE-004-e) live in `platform-wallet-storage`'s test suite, where
//! the concrete `WalletStorageError` variants are in scope.

use std::error::Error;
use std::fmt;
use std::io;

use platform_wallet::changeset::{PersistenceError, PersistenceErrorKind};

/// Concrete typed source used to verify the boxed-source path on the
/// trait surface. The test asserts the Display chain reaches this
/// error's message after a round-trip through `PersistenceError`.
#[derive(Debug)]
struct DummyBackend(&'static str);

impl fmt::Display for DummyBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl Error for DummyBackend {}

/// TC-CODE-004-a — every kind variant participates in the retry
/// classification without a `_ =>` wildcard. If a new kind is added
/// later, this match (and `is_transient`) must be updated explicitly.
#[test]
fn tc_code_004_a_kind_partitions_retry_policy_exhaustively() {
    fn classify(kind: PersistenceErrorKind) -> bool {
        // Wildcard-free: a future variant breaks the compile here on
        // purpose. Do NOT collapse this into `matches!(kind, …)` with
        // a wildcard — that would defeat the exhaustiveness check.
        match kind {
            PersistenceErrorKind::Transient => true,
            PersistenceErrorKind::Fatal => false,
            PersistenceErrorKind::Constraint => false,
        }
    }

    for (kind, expected_transient) in [
        (PersistenceErrorKind::Transient, true),
        (PersistenceErrorKind::Fatal, false),
        (PersistenceErrorKind::Constraint, false),
    ] {
        assert_eq!(classify(kind), expected_transient, "classify({kind:?})");
        let err = PersistenceError::backend_with_kind(kind, DummyBackend("x"));
        assert_eq!(
            err.is_transient(),
            expected_transient,
            "is_transient mismatch for {kind:?}"
        );
    }

    // LockPoisoned is its own variant — never transient.
    assert!(!PersistenceError::LockPoisoned.is_transient());
}

/// TC-CODE-004-a (cont.) — pattern-matching `Backend` exposes both
/// `kind` and `source` and the kind round-trips losslessly.
#[test]
fn tc_code_004_a_backend_exposes_kind_and_source() {
    let err =
        PersistenceError::backend_with_kind(PersistenceErrorKind::Constraint, DummyBackend("fk"));
    match err {
        PersistenceError::Backend { kind, source } => {
            assert_eq!(kind, PersistenceErrorKind::Constraint);
            assert_eq!(source.to_string(), "fk");
        }
        other => panic!("expected Backend {{ .. }}, got {other:?}"),
    }
}

/// TC-CODE-004-c — the boxed source is `Send + Sync`, implements
/// `Display`, and the rendered message contains the original text.
#[test]
fn tc_code_004_c_source_is_send_sync_and_renders_underlying_message() {
    // Compile-time bound: a generic `assert_send_sync` only compiles if
    // the supplied type is `Send + Sync`. The source field is
    // `Box<dyn Error + Send + Sync>` so this is structural.
    fn assert_send_sync<T: Send + Sync>(_: &T) {}

    let io_err = io::Error::other("disk gone");
    let err = PersistenceError::backend(io_err);
    match &err {
        PersistenceError::Backend { source, .. } => {
            assert_send_sync(source);
            assert!(
                source.to_string().contains("disk gone"),
                "expected source message to contain 'disk gone', got: {source}"
            );
        }
        other => panic!("expected Backend {{ .. }}, got {other:?}"),
    }

    // The outer Display chain also surfaces the source.
    let rendered = err.to_string();
    assert!(
        rendered.contains("disk gone"),
        "expected outer Display to include source, got: {rendered}"
    );
}

/// TC-CODE-004-e (trait-side half) — backward-compat: `From<String>`
/// and `From<&str>` still produce a valid `Backend` and default to
/// `Fatal` kind so legacy FFI callers don't silently get classified
/// as retryable.
#[test]
fn tc_code_004_e_string_from_impls_default_to_fatal() {
    let from_owned: PersistenceError = String::from("legacy ffi message").into();
    let from_borrowed: PersistenceError = "legacy ffi message".into();

    for err in [from_owned, from_borrowed] {
        match err {
            PersistenceError::Backend { kind, source } => {
                assert_eq!(kind, PersistenceErrorKind::Fatal);
                assert_eq!(source.to_string(), "legacy ffi message");
            }
            other => panic!("expected Backend {{ .. }}, got {other:?}"),
        }
    }
}

/// The `backend(..)` helper exists for callers that don't know the
/// kind — it must default to `Fatal` so a misclassification reads as
/// "do not retry" rather than spuriously retrying.
#[test]
fn backend_helper_defaults_to_fatal() {
    let err = PersistenceError::backend(DummyBackend("boom"));
    assert!(!err.is_transient(), "default helper must not be transient");
    match err {
        PersistenceError::Backend { kind, .. } => assert_eq!(kind, PersistenceErrorKind::Fatal),
        other => panic!("expected Backend {{ .. }}, got {other:?}"),
    }
}
