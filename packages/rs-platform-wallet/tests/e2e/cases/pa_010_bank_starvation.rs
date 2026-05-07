//! PA-010 — Bank starvation: typed `BankUnderfunded` error.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-010.
//! Priority: P1.
//!
//! ## Status
//!
//! `BLOCKED — needs harness refactor.` See spec status field.
//!
//! The harness today loads ONE bank wallet at process startup via
//! `E2eContext::init` (singleton `OnceCell`) and panics at load time
//! if `bank.total_credits() < config.min_bank_credits`
//! (`framework/bank.rs:117`). `fund_address` itself has no preflight
//! balance check — under-funded calls fail with a generic
//! `PlatformWalletError::AddressOperation` from inside the wallet's
//! transfer path, NOT a typed `BankError::Underfunded`.
//!
//! PA-010 wants both:
//!
//! 1. A `Bank::with_test_balance(target)` constructor that builds a
//!    fresh underfunded bank scoped to ONE test (so the singleton
//!    `OnceCell` is bypassed for the duration), AND
//! 2. A typed `BankError::Underfunded { available, requested }`
//!    variant emitted by `fund_address` when a preflight check fails.
//!
//! Both are harness refactors of the bank's lifecycle and error
//! surface — the bank is currently a process-shared singleton, and
//! routing per-test instances through `setup()` while keeping the
//! shared bank for adjacent tests is more than a "thin helper"
//! addition. The brief rules out production changes; here the
//! production API is fine — what's needed is a test-only per-test
//! bank instance OR an injectable balance override on the singleton,
//! plus a typed error variant on `framework/bank.rs`'s `BankError`.
//!
//! Until the harness gains those, this case stays `#[ignore]`'d.
//! Bank starvation is the single most common "weird CI failure"
//! mode for this suite, so the contract IS valuable to pin — just
//! not in this PR's scope.

#[tokio_shared_rt::test(shared)]
#[ignore = "BLOCKED — needs harness refactor: per-test bank instance \
            (Bank::with_test_balance) OR injectable balance override on the \
            singleton, plus a typed BankError::Underfunded variant. See spec status."]
async fn pa_010_bank_starvation_typed_error() {
    // INTENTIONAL(QA-V16-005): keep hard panic instead of #[ignore]-only — failing
    // test documents the missing per-test bank instance (Bank::with_test_balance)
    // and typed BankError::Underfunded harness gaps until they are implemented;
    // flipping to #[ignore] alone would silently hide the gap from CI signal.
    panic!(
        "PA-010 is BLOCKED on a harness refactor. The bank is a process-\
         shared singleton (E2eContext.bank, OnceCell-backed); building a \
         `with_test_balance(5_000_000)` underfunded instance for ONE test \
         conflicts with that lifecycle. The current under-funded fail mode \
         is also a generic AddressOperation error, not a typed \
         BankError::Underfunded. See TEST_SPEC.md → PA-010 → **Status**."
    );
}
