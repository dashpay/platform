//! E2E test harness for `rs-platform-wallet`.
//!
//! Test authors call [`setup`] to obtain a [`SetupGuard`] holding a
//! fresh-seeded [`wallet_factory::TestWallet`] and the
//! process-shared [`E2eContext`] (bank, SDK, registry). After the
//! test body, call [`SetupGuard::teardown`] to drain the wallet
//! back to the bank.
//!
//! ```ignore
//! let s = setup().await?;
//! let addr = s.test_wallet.next_unused_address().await?;
//! s.ctx.bank().fund_address(&addr, 50_000_000).await?;
//! wait_for_balance(&s.test_wallet, &addr, 50_000_000, ...).await?;
//! s.teardown().await?;
//! ```
//!
//! Convenience imports: [`prelude`].

#![allow(dead_code)]

pub mod bank;
pub mod cleanup;
pub mod config;
pub mod context_provider;
pub mod harness;
pub mod registry;
pub mod sdk;
pub mod spv;
pub mod wait;
pub mod wait_hub;
pub mod wallet_factory;
pub mod workdir;

use key_wallet::gap_limit::DIP17_GAP_LIMIT;
use key_wallet::Network;
use simple_signer::signer::SimpleSigner;

/// DIP-17 default account / key-class for clear-funds platform
/// payments. Matches `WalletAccountCreationOptions::Default`.
const DEFAULT_ACCOUNT_INDEX: u32 = 0;
const DEFAULT_KEY_CLASS: u32 = 0;

/// Build a [`SimpleSigner`] populated with the DIP-17 platform-payment
/// gap window for `seed_bytes` on `network`. Pins to
/// `account=0`/`key_class=0` to match
/// `WalletAccountCreationOptions::Default`. `SimpleSigner` already
/// implements `Signer<PlatformAddress>` directly, so callers can pass
/// the returned value straight to `PlatformAddressWallet::transfer`.
pub(super) fn make_platform_signer(
    seed_bytes: &[u8; 64],
    network: Network,
) -> FrameworkResult<SimpleSigner> {
    SimpleSigner::from_seed_for_platform_address_account(
        seed_bytes,
        network,
        DEFAULT_ACCOUNT_INDEX,
        DEFAULT_KEY_CLASS,
        DIP17_GAP_LIMIT,
    )
    .map_err(|err| FrameworkError::Wallet(format!("simple-signer: {err}")))
}

/// Common imports for test authors.
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
#[derive(Debug, thiserror::Error)]
pub enum FrameworkError {
    /// Placeholder returned by paths that surface an underlying
    /// error through tracing; the static string names the call site.
    #[error("e2e framework not yet implemented: {0}")]
    NotImplemented(&'static str),

    /// Filesystem error — registry IO, workdir creation, lockfile.
    /// Message is preformatted with the offending path.
    #[error("e2e framework I/O: {0}")]
    Io(String),

    /// Wallet error from `platform_wallet`. Stored as String to
    /// avoid pulling upstream-error feature flags into the test crate.
    #[error("e2e framework wallet error: {0}")]
    Wallet(String),

    /// Bank-wallet failure (under-funded, missing mnemonic).
    /// Distinct from `Wallet` so CI can treat operator-actionable
    /// bank issues separately from transient sync failures.
    #[error("e2e bank wallet: {0}")]
    Bank(String),

    /// Cleanup / teardown error. Non-fatal — the registry retains
    /// the wallet so the next startup's sweep recovers it.
    #[error("e2e cleanup: {0}")]
    Cleanup(String),

    /// Configuration / env-parsing failure surfaced by helpers in
    /// [`config`].
    #[error("e2e config: {0}")]
    Config(String),
}

/// Convenience alias used across the harness.
pub type FrameworkResult<T> = Result<T, FrameworkError>;

/// One-shot setup entry point.
///
/// Lazily initialises the process-shared [`E2eContext`] (bank, SDK,
/// registry) on first call and returns a [`SetupGuard`] wrapping a
/// fresh-seeded [`wallet_factory::TestWallet`].
///
/// The wallet is **registered in the persistent registry BEFORE
/// being returned**, so a panic between `setup` and the test's
/// `SetupGuard::teardown` leaves a recoverable trail for the next
/// process startup's sweep.
///
/// Errors: any failure during context init, wallet creation, or
/// registry insert is surfaced as [`FrameworkError`].
pub async fn setup() -> FrameworkResult<SetupGuard> {
    let ctx = E2eContext::init().await?;

    let (seed_bytes, seed_hex) = wallet_factory::fresh_seed();

    // Build the wallet first so we can derive the id for the
    // registry entry; on failure there is nothing to persist.
    let network = ctx.bank().network();
    let test_wallet = wallet_factory::TestWallet::create(
        ctx.manager(),
        seed_bytes,
        network,
        std::sync::Arc::clone(ctx.wait_hub()),
    )
    .await?;

    // Persist BEFORE handing the wallet to the test body so a panic
    // mid-test surfaces to the next process startup's sweep.
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
