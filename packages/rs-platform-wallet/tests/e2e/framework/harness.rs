//! Process-shared `E2eContext` lazily initialised once per test run.
//!
//! The harness sets up the bank wallet, SDK, SPV runtime, persistent
//! registry, and panic hook in one place so every test case under
//! `cases/` can reuse them. SDK / SPV initialisation is genuinely
//! expensive (~30–60s on cold start); a per-process singleton via
//! `OnceCell` amortises the cost.
//!
//! Wave 2 stub: the struct is declared with placeholder unit-typed
//! fields and a stub `init`. Wave 3 (`bank.rs`, `registry.rs`,
//! `cleanup.rs`) and Wave 3-network (`sdk.rs`, `spv.rs`,
//! `context_provider.rs`) replace each `()` slot with the real
//! type. Holding the field declarations now means subsequent waves
//! land as field-by-field swaps without re-shuffling the struct.

use std::path::PathBuf;

use super::FrameworkResult;

/// Process-shared context for the e2e suite.
///
/// Tests acquire a `&'static E2eContext` via [`super::setup`], which
/// internally calls [`E2eContext::init`]. Direct construction is
/// not part of the public surface — the lazy init enforces the
/// "one bank + one SPV runtime per process" invariant.
pub struct E2eContext {
    /// Resolved configuration loaded from env vars + `.env`.
    /// Wave 3 replaces with `super::config::Config`.
    pub config: (),
    /// Slot-locked workdir base path.
    pub workdir: PathBuf,
    /// `flock`-held lock file kept open for the context's lifetime
    /// so concurrent test processes pick a different slot.
    /// Wave 3 replaces with `std::fs::File`.
    pub workdir_lock: (),
    /// Constructed `dash_sdk::Sdk`. Wave 3-network slots in.
    pub sdk: (),
    /// `PlatformWalletManager` shared across bank + test wallets.
    /// Wave 3-network slots in.
    pub manager: (),
    /// `SpvRuntime` started during init. Wave 3-network slots in.
    pub spv_runtime: (),
    /// Pre-funded bank wallet. Wave 3 (`bank.rs`) slots in.
    pub bank: (),
    /// Persistent test-wallet registry. Wave 3 (`registry.rs`)
    /// slots in.
    pub registry: (),
    /// Cancellation token tripped by the panic hook so SPV /
    /// background tasks shut down cleanly. Wave 3 slots in
    /// `tokio_util::sync::CancellationToken`.
    pub cancel_token: (),
}

impl E2eContext {
    /// Lazily build (or reuse) the process-shared context.
    ///
    /// Wave 2 stub. Wave 3 wires the full init sequence:
    ///
    /// 1. `Config::from_env()`.
    /// 2. `pick_available_workdir(&base)` → `(PathBuf, File)`.
    /// 3. Install panic hook (cancels SPV on init panic).
    /// 4. Build SDK.
    /// 5. Construct `PlatformWalletManager`.
    /// 6. Start SPV; wait for masternode-list sync.
    /// 7. Construct `BankWallet` + verify minimum balance.
    /// 8. Open persistent registry; run startup sweep.
    pub async fn init() -> FrameworkResult<&'static Self> {
        Err(super::FrameworkError::NotImplemented(
            "E2eContext::init — wired in Wave 3",
        ))
    }
}
