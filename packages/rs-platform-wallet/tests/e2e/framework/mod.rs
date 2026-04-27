//! E2E test harness for `rs-platform-wallet`.
//!
//! Public surface for test authors:
//!
//! - [`setup`] — one-shot entry point; lazily builds the
//!   process-shared [`E2eContext`] and returns a [`SetupGuard`]
//!   wrapping a fresh test wallet pre-registered for cleanup.
//! - [`prelude`] — re-exports the types tests reach for most often.
//!
//! Submodule layout mirrors the plan
//! (`/home/ubuntu/.claude/plans/ok-now-we-ll-get-prancy-biscuit.md`,
//! Module Layout):
//!
//! - [`config`] — env-var loader + programmatic constructor.
//! - [`harness`] — `E2eContext`, lazily-initialised, holds workdir
//!   lock + SDK + SPV + bank + registry.
//! - [`workdir`] — `pick_available_workdir` (`flock`-based slot
//!   selection, DET pattern).
//! - [`panic_hook`] — installs a hook that trips the cancellation
//!   token so SPV / background tasks shut down cleanly.
//! - [`wait`] — generic poller + `wait_for_balance` specialisation.
//! - [`bank`] — pre-funded bank wallet (Wave 3a).
//! - [`wallet_factory`] — `TestWallet` factory + `SetupGuard` (Wave 3a).
//! - [`signer`] — seed-backed `Signer<PlatformAddress>` (Wave 3a).
//! - [`registry`] — JSON-backed test-wallet registry (Wave 3a).
//! - [`cleanup`] — startup `sweep_orphans` + per-test `teardown_one`
//!   (Wave 3a).
//!
//! Wave 3b adds `sdk`, `spv`, and `context_provider` modules
//! alongside these (see plan for the full split).

// Wave 2 / 3a stubs intentionally don't cross-reference yet — Wave 4
// turns those into hard wiring and the allow can be tightened then.
#![allow(dead_code)]

pub mod bank;
pub mod cleanup;
pub mod config;
pub mod context_provider;
pub mod harness;
pub mod panic_hook;
pub mod registry;
pub mod sdk;
pub mod signer;
pub mod spv;
pub mod wait;
pub mod wait_hub;
pub mod wallet_factory;
pub mod workdir;

/// Common imports for test authors. Populated as Wave 3 / Wave 4
/// stabilise the concrete signatures — kept minimal in the
/// skeleton so the prelude itself stays meaningful.
pub mod prelude {
    pub use super::config::Config;
    pub use super::harness::E2eContext;
    pub use super::wait::{wait_for, wait_for_balance};
    pub use super::wait_hub::WaitEventHub;
    pub use super::{setup, FrameworkError, FrameworkResult, SetupGuard};
}

pub use wallet_factory::SetupGuard;

use harness::E2eContext;

/// Errors surfaced by the e2e framework.
///
/// Wave 2 shipped a single `NotImplemented` variant. Wave 3a expands
/// the surface with `Io` / `Wallet` / `Bank` variants used by the
/// registry, factory, and bank-load paths; Wave 3b will append SDK
/// / SPV / context-provider variants alongside.
#[derive(Debug, thiserror::Error)]
pub enum FrameworkError {
    /// Stub returned by placeholders that haven't been wired yet
    /// (most still belong to Wave 4 integration glue). The static
    /// string names the call site so test failures during
    /// scaffolding work point at the right module.
    #[error("e2e framework not yet implemented: {0}")]
    NotImplemented(&'static str),

    /// Filesystem error — registry IO, workdir creation, lockfile
    /// open. The message is preformatted with the offending path so
    /// downstream `?` unwraps stay readable.
    #[error("e2e framework I/O: {0}")]
    Io(String),

    /// Wallet-creation / sync / transfer error surfaced by
    /// `platform_wallet`'s typed errors. Stored as a String so the
    /// e2e error type stays free of upstream-error feature flags
    /// (the originating error type is `large_enum_variant` already).
    #[error("e2e framework wallet error: {0}")]
    Wallet(String),

    /// Bank-wallet-specific failures — under-funded balance,
    /// missing mnemonic, etc. Distinct from `Wallet` so callers
    /// (and CI logs) can treat operator-actionable bank issues
    /// separately from ordinary transient sync failures.
    #[error("e2e bank wallet: {0}")]
    Bank(String),

    /// Test wallet teardown / cleanup error. Reported but
    /// non-fatal — the registry retains the wallet so the next
    /// startup runs `sweep_orphans` to recover.
    #[error("e2e cleanup: {0}")]
    Cleanup(String),
}

/// Convenience alias used across the harness.
pub type FrameworkResult<T> = Result<T, FrameworkError>;

/// One-shot setup entry point for test cases.
///
/// Lazily initialises the process-shared [`E2eContext`] (bank,
/// SDK, SPV, registry, panic hook) and produces a fresh-seeded
/// [`SetupGuard::test_wallet`].
///
/// The wallet is **registered in the persistent registry before
/// being returned** — that way a panic between `setup` and
/// `teardown` leaves a recoverable trail for the next process
/// startup's sweep.
pub async fn setup() -> FrameworkResult<SetupGuard> {
    let ctx = E2eContext::init().await?;

    let (seed_bytes, seed_hex) = wallet_factory::fresh_seed();

    // Build the test wallet first so we can derive the wallet id
    // for the registry entry. If creation fails we never persist —
    // there's nothing to sweep.
    let network = ctx.bank().network();
    let test_wallet = wallet_factory::TestWallet::create(
        ctx.manager(),
        seed_bytes,
        network,
        std::sync::Arc::clone(ctx.wait_hub()),
    )
    .await?;

    // Persist the registry entry BEFORE handing the wallet to the
    // test body. Once this returns the entry is durable — a panic
    // mid-test will surface to the next process startup's sweep.
    let entry = registry::RegistryEntry {
        seed_hex,
        created_at: std::time::SystemTime::now(),
        status: registry::EntryStatus::Active,
        note: None,
    };
    ctx.registry().insert(test_wallet.id(), entry)?;

    Ok(SetupGuard {
        ctx,
        test_wallet,
        teardown_called: false,
    })
}
