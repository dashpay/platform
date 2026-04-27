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
//! - [`persistence`] — wraps the no-op persister test wallets use.
//!
//! Wave 3 adds `bank`, `wallet_factory`, `signer`, `registry`,
//! `cleanup`, `sdk`, `spv`, and `context_provider` modules
//! alongside these (see plan for the full split).

pub mod config;
pub mod harness;
pub mod panic_hook;
pub mod persistence;
pub mod wait;
pub mod workdir;

/// Common imports for test authors. Populated as Wave 3 / Wave 4
/// stabilise the concrete signatures — kept minimal in the
/// skeleton so the prelude itself stays meaningful.
pub mod prelude {
    pub use super::config::Config;
    pub use super::harness::E2eContext;
    pub use super::wait::{wait_for, wait_for_balance};
    pub use super::{setup, FrameworkError, FrameworkResult, SetupGuard};
}

use harness::E2eContext;

/// Errors surfaced by the e2e framework.
///
/// Wave 2 ships a single `NotImplemented` variant so every stub can
/// return a meaningful error; Wave 3 expands with concrete variants
/// (config / workdir / SDK / SPV / bank / registry / teardown).
#[derive(Debug, thiserror::Error)]
pub enum FrameworkError {
    /// Stub returned by every Wave 2 placeholder. The static string
    /// names the call site so test failures during scaffolding work
    /// point at the right module.
    #[error("e2e framework not yet implemented: {0}")]
    NotImplemented(&'static str),
}

/// Convenience alias used across the harness.
pub type FrameworkResult<T> = Result<T, FrameworkError>;

/// One-shot setup entry point for test cases.
///
/// Wave 2 stub — returns [`FrameworkError::NotImplemented`]. Wave 3
/// + Wave 4 wire:
///
/// 1. Lazily initialise [`E2eContext`] via `OnceCell`.
/// 2. Generate a fresh seed and create a [`SetupGuard::test_wallet`]
///    via `manager.create_wallet_from_seed_bytes`.
/// 3. Pre-register the wallet in the persistent registry **before**
///    returning, so a panic in the test body still leaves the
///    wallet recoverable on next startup.
pub async fn setup() -> FrameworkResult<SetupGuard> {
    Err(FrameworkError::NotImplemented(
        "framework::setup — wired in Wave 3/4",
    ))
}

/// Guard returned by [`setup`].
///
/// Wave 2 stub — concrete fields and the `Drop` impl land in
/// Wave 3 alongside the registry. Holding the guard pre-registers
/// the wallet for cleanup; explicit [`SetupGuard::teardown`] is the
/// happy path, [`Drop`] is the panic-safety fallback.
pub struct SetupGuard {
    /// Shared, lazily-initialised `E2eContext`. Wave 3 fills in.
    pub ctx: &'static E2eContext,
    /// Per-test wallet, fresh seed, registered for cleanup. Wave 3
    /// replaces the placeholder unit type with the real
    /// `TestWallet`.
    pub test_wallet: (),
    /// Tracks whether [`SetupGuard::teardown`] ran successfully so
    /// `Drop` can decide whether to leave the wallet for the next
    /// startup sweep.
    teardown_called: bool,
}

impl SetupGuard {
    /// Sweep funds back to the bank and remove this wallet from the
    /// persistent registry.
    ///
    /// Wave 2 stub.
    pub async fn teardown(self) -> FrameworkResult<()> {
        Err(FrameworkError::NotImplemented(
            "SetupGuard::teardown — wired in Wave 3",
        ))
    }
}
