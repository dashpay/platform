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
//!
//! # Parallelism contract
//!
//! The harness is designed to support `--test-threads>1`. Tests share
//! one [`E2eContext`] (`OnceCell`-backed singleton), one bank wallet,
//! one SPV runtime, and one workdir slot. Per-test isolation comes
//! from:
//!
//! 1. **Disjoint test wallets** — every [`setup`] call mints a fresh
//!    OS-random 64-byte seed via [`wallet_factory::fresh_seed`]. Two
//!    parallel tests have distinct wallet ids with cryptographic
//!    probability; their on-chain identities, addresses, and nonces
//!    don't collide.
//! 2. **Serialised bank funding** — [`bank::BankWallet::fund_address`]
//!    and [`bank::BankWallet::send_core_to`] take an in-process
//!    [`tokio::sync::Mutex`] (`FUNDING_MUTEX`) so concurrent callers
//!    can't race the bank's UTXO selection / nonce assignment. Tests
//!    waiting on `wait_for_balance` and friends do NOT hold the mutex.
//! 3. **Cross-process workdir slots** — [`workdir::pick_available_workdir`]
//!    walks `0..MAX_SLOTS` and acquires an exclusive `flock` on each.
//!    A second `cargo test` invocation against the same machine lands
//!    on a separate slot, so SPV caches and registries don't share
//!    state across processes. Slot 0 is reusable across runs of the
//!    same process when its lock is released cleanly.
//! 4. **Process-shared singletons** are limited to thread-safe
//!    primitives: [`tokio::sync::OnceCell`] for `CTX`,
//!    `std::sync::Mutex<Option<Arc<SpvRuntime>>>` for `IN_FLIGHT_SPV`,
//!    `tokio::sync::Mutex<()>` for `FUNDING_MUTEX`, `parking_lot::Mutex`
//!    for the registry's in-memory map.
//!
//! ## Tests that need special handling under parallelism
//!
//! - [`cases::pa_008c_funding_mutex_observable`] reads the
//!   process-global `FUNDING_MUTEX_HISTORY` ring buffer. The buffer is
//!   written to by EVERY `bank.fund_address` call across all tests, so
//!   the test asserts a **lower bound** on entry count (`>= 3`) and the
//!   pairwise non-overlap property that holds across ALL entries — not
//!   strict equality on its own three entries.
//!
//! All other cases mint fresh seeds and reach for shared resources only
//! via the serialised paths above.
//!
//! Background reading: `dash-evo-tool/tests/backend-e2e/framework/`
//! pioneered this pattern (`harness.rs::FUNDING_MUTEX`,
//! `BackendTestContext::create_funded_test_wallet`); the structure
//! here mirrors it.

#![allow(dead_code)]

pub mod bank;
pub mod bank_identity;
pub mod bank_plan;
pub mod bank_rebalance;
pub mod cleanup;
pub mod config;
pub mod context_provider;
pub mod funding_ledger;
pub mod gap_limit;
pub mod harness;
pub mod identities;
pub mod identity_sync;
pub mod registry;
pub mod sdk;
#[cfg(feature = "shielded")]
pub mod shielded;
pub mod signer;
pub mod spv;
pub mod tokens;
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
    pub use super::wait::{
        wait_for, wait_for_address_balance_chain_confirmed,
        wait_for_address_balance_chain_confirmed_strong, wait_for_address_known_to_platform,
        wait_for_balance, wait_for_bank_funded, wait_for_core_balance,
        wait_for_identity_visible_to_platform,
    };
    pub use super::wait_hub::WaitEventHub;
    pub use super::{setup, FrameworkError, FrameworkResult, SetupGuard};
}

pub use wallet_factory::SetupGuard;

use harness::E2eContext;

// Parallelism guard rails: enforce at compile time that the types
// shared across worker threads under `--test-threads>1` are `Send + Sync`.
// `E2eContext` is held behind a `&'static` so all tests reach for the
// same instance; `SetupGuard` is held by the running test body. Any
// future field addition that breaks `Send + Sync` (e.g. an `Rc`, a
// non-`Send` future, an inadvertent `RefCell`) trips this static assert
// at compile time rather than at runtime through a flaky parallel run.
static_assertions::assert_impl_all!(E2eContext: Send, Sync);
static_assertions::assert_impl_all!(SetupGuard: Send, Sync);

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

    /// SDK construction / wiring failure (e.g. `SdkBuilder::build`,
    /// `TrustedHttpContextProvider::new`, DAPI address parsing).
    /// Carries the upstream error stringified so CI logs and any
    /// `Result`-matching caller see the underlying cause.
    #[error("e2e sdk: {0}")]
    Sdk(String),

    /// SPV (`dash-spv`) construction / sync failure. Distinct from
    /// [`Self::Sdk`] so SPV-only deferred-runtime issues are easy to
    /// filter when the SPV path comes back online (Task #15).
    #[error("e2e spv: {0}")]
    Spv(String),
}

/// Convenience alias used across the harness.
pub type FrameworkResult<T> = Result<T, FrameworkError>;

/// Derive a per-test label from a Rust source file path recorded by
/// [`std::panic::Location`] or the compile-time [`file!`] macro.
///
/// Returns `Some(stem)` when the file lives under the `cases/` subtree
/// (e.g. `"tests/e2e/cases/pa_001_multi_output.rs"` →
/// `Some("pa_001_multi_output")`). Returns `None` for framework /
/// harness files (e.g. `"tests/e2e/framework/mod.rs"`) so internal
/// helper calls don't pollute the per-test table with spurious
/// framework entries.
fn label_from_file(file: &str) -> Option<String> {
    if !file.contains("cases/") {
        return None;
    }
    std::path::Path::new(file)
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
}

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
///
/// This is a **non-async wrapper** around [`setup_async`] so that
/// `#[track_caller]` can capture the calling source file before the
/// async state machine is created (`#[track_caller]` on `async fn` is
/// a no-op on stable Rust — see issue #110011). The per-test label
/// derived here is stored on the returned [`SetupGuard`] and used in
/// [`SetupGuard::teardown`] to attribute sweep recoveries to the test.
#[track_caller]
pub fn setup() -> impl std::future::Future<Output = FrameworkResult<SetupGuard>> {
    // Capture the call site NOW (sync context) so #[track_caller] works.
    // When called from inside an outer setup_with_* async block, the
    // file path is the framework file (no "cases/" segment), so
    // label_from_file() returns None. The async body below handles that
    // case by reading the task-local set by the outer scope.
    let site_label = label_from_file(std::panic::Location::caller().file());
    async move {
        // Prefer an existing task-local label (set by an outer setup_with_*
        // that already entered a maybe_with_test_label scope); fall back to
        // the site label captured at the synchronous call above.
        let existing = funding_ledger::CURRENT_TEST_LABEL
            .try_with(|o| o.clone())
            .ok()
            .flatten();
        let label = existing.or(site_label).unwrap_or_default();

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

        // Constructor wires up the counter increment AFTER struct
        // assembly so a pre-construction panic doesn't leak a slot —
        // see [`SetupGuard::new`] / V27-004.
        Ok(SetupGuard::new(ctx, test_wallet, label))
    }
}

/// Multi-identity counterpart of [`setup`]. Builds a fresh test
/// wallet, funds `n` distinct platform addresses from the bank, and
/// registers an identity at DIP-9 indices `0..n` on each.
///
/// Returns a [`MultiIdentitySetupGuard`] wrapping the original
/// [`SetupGuard`] plus the `Vec<RegisteredIdentity>` so test
/// authors can drive multi-identity flows (DP-002 contact requests,
/// ID-003 transfers) without re-deriving the registration boilerplate.
///
/// Funding policy: every identity is registered with `funding_per`
/// credits charged to a freshly-derived address, so each call costs
/// `n * (funding_per + register_fee)` credits from the bank. Tests
/// with tight balance windows should pass conservative values —
/// `30_000_000` per identity is the reference; the bank's
/// `min_bank_credits` floor must cover `n * funding_per` plus
/// per-tx fees.
///
/// Non-async sync wrapper so `#[track_caller]` captures the test
/// file before the async state machine is created.
#[track_caller]
pub fn setup_with_n_identities(
    n: u32,
    funding_per: dpp::fee::Credits,
) -> impl std::future::Future<Output = FrameworkResult<MultiIdentitySetupGuard>> {
    let site_label = label_from_file(std::panic::Location::caller().file());
    async move {
        let existing = funding_ledger::CURRENT_TEST_LABEL
            .try_with(|o| o.clone())
            .ok()
            .flatten();
        let label = existing.or(site_label);
        funding_ledger::maybe_with_test_label(
            label,
            setup_with_n_identities_with_step_timeout(n, funding_per, DEFAULT_SETUP_STEP_TIMEOUT),
        )
        .await
    }
}

/// Default per-step propagation budget used by [`setup_with_n_identities`]
/// and the token-suite `setup_with_token_*` helpers. Sized for the common
/// case (per-identity funding under a few-hundred-million credits clearing
/// inside ~30 s); raise it via [`setup_with_n_identities_with_step_timeout`]
/// when a single test is known to need a larger budget — typically the
/// "transfer multiple billions of credits while seven sibling guards
/// compete on the bank under `--test-threads=8`" shape that TK-005 hits.
pub const DEFAULT_SETUP_STEP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// Per-test override of [`setup_with_n_identities`]'s propagation budget.
///
/// Each waiter inside the per-identity loop (the proof-verified
/// chain-confirmed funding gate, the strong chain-confirmed gate, and the
/// identity-visibility gate) uses `step_timeout` independently. Raising it
/// lets a single test (e.g.
/// TK-005's high-credit funding under contention) survive without softening
/// the global default — keeping a tight default surfaces genuinely-stuck
/// tests in the majority of cases.
pub async fn setup_with_n_identities_with_step_timeout(
    n: u32,
    funding_per: dpp::fee::Credits,
    step_timeout: std::time::Duration,
) -> FrameworkResult<MultiIdentitySetupGuard> {
    let funding = vec![funding_per; n as usize];
    setup_with_per_identity_funding(&funding, step_timeout).await
}

/// Per-identity-funding counterpart to
/// [`setup_with_n_identities_with_step_timeout`]. Each entry in
/// `funding_per_identity` is the credits charged to its identity's
/// fresh funding address — registers identity at DIP-9 slot `i` with
/// `funding_per_identity[i]`.
///
/// Used by the token-suite `setup_with_token_*` helpers to fund the
/// contract owner separately from the peer(s) — the owner pays the
/// 20.2 B / 30.2 B contract-create floor while peers only need
/// transition-fee headroom. (#348)
///
/// Non-async sync wrapper so `#[track_caller]` captures the test
/// file before the async state machine is created. Clones the slice
/// into a `Vec` to avoid lifetime constraints on the returned future.
#[track_caller]
pub fn setup_with_per_identity_funding(
    funding_per_identity: &[dpp::fee::Credits],
    step_timeout: std::time::Duration,
) -> impl std::future::Future<Output = FrameworkResult<MultiIdentitySetupGuard>> {
    let site_label = label_from_file(std::panic::Location::caller().file());
    // Clone into owned Vec so the returned future is 'static (no lifetime
    // tied to the caller's stack). The slice is typically 1–3 elements.
    let funding_owned = funding_per_identity.to_vec();
    async move {
        // Inherit an outer scope's label (e.g. from setup_with_n_identities);
        // fall back to the site label captured synchronously above.
        let existing = funding_ledger::CURRENT_TEST_LABEL
            .try_with(|o| o.clone())
            .ok()
            .flatten();
        let label = existing.or(site_label);
        funding_ledger::maybe_with_test_label(
            label,
            setup_with_per_identity_funding_inner(funding_owned, step_timeout),
        )
        .await
    }
}

async fn setup_with_per_identity_funding_inner(
    funding_per_identity: Vec<dpp::fee::Credits>,
    step_timeout: std::time::Duration,
) -> FrameworkResult<MultiIdentitySetupGuard> {
    use super::framework::wait::{
        wait_for_address_balance_chain_confirmed_n, wait_for_address_known_to_platform,
        wait_for_identity_visible_to_platform, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
    };

    let base = setup().await?;
    let mut identities = Vec::with_capacity(funding_per_identity.len());

    // Rec 6 — bank balance breadcrumb at per-test setup entry (DEBUG; opt-in via
    // RUST_LOG=platform_wallet::e2e::bank=debug). Cached read — no DAPI round-trip.
    // Creates a depletion-detection breadcrumb across a long suite run.
    tracing::debug!(
        target: "platform_wallet::e2e::bank",
        bank_credits = base.ctx.bank().total_credits().await,
        identities = funding_per_identity.len(),
        "bank.setup: cached bank balance at per-identity funding entry"
    );

    // Each identity gets a distinct funding address so the bank's
    // FUNDING_MUTEX serialises funding without contending on the
    // same destination. We fund + observe before registration so
    // `register_from_addresses` finds the credits already
    // committed to platform.
    //
    // bank.fund_address delivers exactly the requested amount; the chain
    // then charges the `IdentityCreateFromAddresses` dynamic fee from
    // the address residual after registration consumes `funding_per`.
    // The current testnet dynamic fee is ~110.86M credits — a ~96M
    // baseline (validate_fees_of_event_v0 PaidFromAddressInputs) plus
    // ~14.85M for the slot-2 TRANSFER key's storage cost. Fund each
    // address with `funding_per + 150M` so the residual (150M) covers
    // the dynamic fee with ~39M buffer for protocol-version drift.
    const REGISTRATION_HEADROOM: u64 = 150_000_000;

    for (identity_index, &funding_per) in funding_per_identity.iter().enumerate() {
        let identity_index = identity_index as u32;
        let funding_addr = base.test_wallet.next_unused_address().await?;
        let bank_amount = funding_per + REGISTRATION_HEADROOM;
        base.ctx
            .bank()
            .fund_address(&funding_addr, bank_amount)
            .await?;
        // Found-025 (rs-sdk address-sync silently discards a fetched balance
        // update when the address is not yet in `pending_addresses`) poisons
        // the wallet's local sync map: `wait_for_balance`'s local-view
        // precondition never reaches target under 14-thread churn, so its
        // proof-verified hand-off never runs and the gate times out
        // (TK-001 / TK-014, v53). Observe the funding directly via the
        // proof-verified `AddressInfo::fetch` path — the same chain-state
        // read the validator itself walks — bypassing the poisoned map.
        wait_for_address_balance_chain_confirmed_n(
            base.ctx.sdk(),
            &funding_addr,
            bank_amount,
            CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
            step_timeout,
        )
        .await?;

        // QA-802 — the gate above already runs a 2-success chain-confirmed
        // check, but Marvin's TK-007 / ID-007 timeline shows the streak
        // clearing while a third Platform replica is still lagging — the
        // immediately-following `register_identity_from_addresses` lands on
        // that lagging node and panics with `AddressDoesNotExistError`.
        // The strong gate (4 successes × 1 s gap) samples more distinct
        // sockets before we hand the address to the registration broadcast.
        wait_for_address_known_to_platform(
            base.ctx.sdk(),
            &funding_addr,
            bank_amount,
            step_timeout,
        )
        .await?;

        let registered = base
            .test_wallet
            .register_identity_from_addresses(funding_addr, funding_per, identity_index)
            .await?;

        // QA-805 — registration returned `Ok` on whichever DAPI node served
        // the broadcast, but the next state transition referencing this
        // identity (transfer, top-up, contract update) may round-robin onto
        // a sibling that hasn't replicated the new identity yet. A
        // 2-success visibility gate on `Identity::fetch` mirrors the
        // existing `wait_for_data_contract_visible` pattern from QA-802.
        wait_for_identity_visible_to_platform(base.ctx.sdk(), registered.id, step_timeout, 2)
            .await?;

        identities.push(registered);
    }

    // `register_from_addresses` consumes the funding addresses without
    // refreshing the cached `(balance, nonce)` pair on each — by design
    // (see `register_from_addresses.rs` cache TODO). Without a sync the
    // returned wallet would still report each address at its
    // pre-registration balance, and a follow-up auto-select would pick
    // already-spent inputs. One sync at the end refreshes balances and
    // nonces together for every consumed address in a single round-trip.
    base.test_wallet.sync_balances().await?;

    Ok(MultiIdentitySetupGuard { base, identities })
}

/// Set up a fresh test wallet pre-funded with Core (Layer-1) duffs
/// drawn from the bank's BIP-44 account 0.
///
/// Companion to [`setup`] / [`setup_with_n_identities`] for cases that
/// need an asset-lock-buildable balance on the test wallet's own Core
/// side — `CR-003` is the canonical caller. The flow:
///
/// 1. Build a fresh test wallet via [`setup`].
/// 2. Derive the test wallet's first Core receive address on BIP-44
///    account 0 via [`platform_wallet::wallet::core::CoreWallet::next_receive_address_for_account`].
/// 3. Send `duffs` from the bank's Core account to that address using
///    [`super::bank::BankWallet::send_core_to`] (gated on
///    `confirmed >= duffs + CORE_TX_FEE_RESERVE`; under-funded errors
///    surface as [`FrameworkError::Bank`] with the bank's Core receive
///    address embedded).
/// 4. Wait up to [`CORE_FUNDING_TIMEOUT`] for the test wallet's
///    confirmed Core balance (sourced from the SPV-updated atomic via
///    `WalletBalance::confirmed`) to reach `duffs`.
///
/// On success the test wallet's `core_balance_confirmed()` is
/// guaranteed to be `>= duffs`, so downstream callers (e.g.
/// `IdentityWallet::register_identity_with_funding`
/// with `IdentityFunding::FromWalletBalance { amount_duffs, account_index }`) can
/// build an asset lock without a follow-up Core sync race.
///
/// Errors:
/// - [`FrameworkError::Bank`] when the bank itself is under-funded —
///   propagated verbatim from [`super::bank::BankWallet::send_core_to`]
///   so the operator-actionable "top up at &lt;addr&gt;" message reaches
///   the test log unchanged.
/// - [`FrameworkError::Wallet`] for any failure deriving the test
///   wallet's Core address.
/// - [`FrameworkError::Cleanup`] (via [`wait::wait_for_core_balance`])
///   when the SPV bloom filter doesn't observe the inbound UTXO
///   within [`CORE_FUNDING_TIMEOUT`].
/// Non-async sync wrapper so `#[track_caller]` captures the test
/// file before the async state machine is created.
#[track_caller]
pub fn setup_with_core_funded_test_wallet(
    duffs: u64,
) -> impl std::future::Future<Output = FrameworkResult<SetupGuard>> {
    let site_label = label_from_file(std::panic::Location::caller().file());
    async move {
        use super::framework::wait::wait_for_core_balance;

        let existing = funding_ledger::CURRENT_TEST_LABEL
            .try_with(|o| o.clone())
            .ok()
            .flatten();
        let label = existing.or(site_label);

        // setup() is also a sync wrapper: when called here (from inside an
        // async block in a framework file), its #[track_caller] captures
        // framework/mod.rs (not a cases/ file), so site_label will be None.
        // The maybe_with_test_label scope below sets CURRENT_TEST_LABEL so
        // setup()'s async body reads the label from try_with() instead.
        let base = funding_ledger::maybe_with_test_label(label, setup()).await?;

        let core_recv = base
            .test_wallet
            .platform_wallet()
            .core()
            .next_receive_address_for_account(0)
            .await
            .map_err(|err| {
                FrameworkError::Wallet(format!(
                    "setup_with_core_funded_test_wallet: derive test-wallet Core receive \
                     address (account=0): {err}"
                ))
            })?;

        let txid = base.ctx.bank().send_core_to(&core_recv, duffs).await?;
        tracing::info!(
            target: "platform_wallet::e2e::setup",
            %txid,
            target_addr = %core_recv,
            duffs,
            "setup_with_core_funded_test_wallet: bank.send_core_to broadcast"
        );

        // Wait for the SPV bloom filter to observe the inbound UTXO and
        // raise the test wallet's confirmed Core balance to at least
        // `duffs`. The bank's send is non-blocking — `send_core_to` does
        // NOT wait for instant-lock — so `wait_for_core_balance` is what
        // gives the caller a positive-arrival signal.
        wait_for_core_balance(&base.test_wallet, duffs, CORE_FUNDING_TIMEOUT).await?;

        Ok(base)
    }
}

/// Default deadline for the test wallet's confirmed Core balance to
/// reach the funding amount in [`setup_with_core_funded_test_wallet`].
/// 180 s gives ~1.8x margin over the worst observed cold-testnet
/// success (~100 s); anything longer only pads the failure path and
/// hides a peer-list or mn-list problem the harness should surface.
pub const CORE_FUNDING_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(180);

/// Guard returned by [`setup_with_n_identities`]. Wraps the base
/// [`SetupGuard`] plus the freshly-registered identities.
///
/// Calling [`MultiIdentitySetupGuard::teardown`] consumes the guard
/// and forwards to the inner [`SetupGuard::teardown`], which sweeps
/// both platform-address balances and identity-credit balances.
/// Identity sweep drains every identity owned by this wallet whose
/// balance exceeds the per-identity floor back to the shared bank
/// identity (see [`cleanup::sweep_identities_with_seed`]); identities
/// whose balance is below the floor are intentionally left for the
/// next-run orphan sweep to retry.
pub struct MultiIdentitySetupGuard {
    /// Inner single-wallet guard. Holds the [`E2eContext`] and the
    /// shared [`wallet_factory::TestWallet`] every identity is
    /// derived from.
    pub base: SetupGuard,
    /// Identities registered during setup, ordered by DIP-9 index
    /// `0..n`.
    pub identities: Vec<wallet_factory::RegisteredIdentity>,
}

impl MultiIdentitySetupGuard {
    /// Forward to the inner [`SetupGuard::teardown`].
    pub async fn teardown(self) -> FrameworkResult<cleanup::SweepReport> {
        self.base.teardown().await
    }
}
