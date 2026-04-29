//! Test-wallet factory plus the [`SetupGuard`] returned by
//! [`super::setup`]. Every wallet is registered in the persistent
//! registry BEFORE returning to the test body, so a panic between
//! `setup` and `teardown` leaves a recoverable trail for the next
//! startup's sweep.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::SystemTime;

use dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use key_wallet::account::account_collection::PlatformPaymentAccountKey;
use key_wallet::wallet::initialization::{
    PlatformPaymentAccountSpec, WalletAccountCreationOptions,
};
use key_wallet::Network;
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::wallet::platform_addresses::InputSelection;
use platform_wallet::{
    PlatformAddressChangeSet, PlatformWallet, PlatformWalletError, PlatformWalletManager,
};
use rand::rngs::OsRng;
use rand::RngCore;

use super::harness::E2eContext;
use super::registry::{EntryStatus, PersistentTestWalletRegistry, RegistryEntry, WalletSeedHash};
use super::signer::SeedBackedPlatformAddressSigner;
use super::wait_hub::WaitEventHub;
use super::{FrameworkError, FrameworkResult};

/// DIP-17 default PlatformPayment account spec — pinned to
/// `PlatformPaymentAccountSpec` field defaults so a struct-shape change
/// upstream fails to compile here.
const DEFAULT_PLATFORM_PAYMENT_ACCOUNT_SPEC: PlatformPaymentAccountSpec =
    PlatformPaymentAccountSpec {
        account: 0,
        key_class: 0,
    };

pub(super) const DEFAULT_ACCOUNT_INDEX_PUB: u32 = DEFAULT_PLATFORM_PAYMENT_ACCOUNT_SPEC.account;
pub(super) const DEFAULT_KEY_CLASS_PUB: u32 = DEFAULT_PLATFORM_PAYMENT_ACCOUNT_SPEC.key_class;

/// `PlatformPaymentAccountKey` for the default DIP-17 account, derived
/// from the canonical [`PlatformPaymentAccountSpec`] in `key_wallet`.
fn default_platform_payment_account_key() -> PlatformPaymentAccountKey {
    let PlatformPaymentAccountSpec { account, key_class } = PlatformPaymentAccountSpec::default();
    PlatformPaymentAccountKey { account, key_class }
}

/// Per-test wallet handle. Exposes the high-level operations test
/// cases reach for (`next_unused_address`, `transfer`, `balances`,
/// `sync_balances`) without leaking the underlying `PlatformWallet`
/// surface.
pub struct TestWallet {
    seed_bytes: [u8; 64],
    pub(crate) wallet: Arc<PlatformWallet>,
    signer: SeedBackedPlatformAddressSigner,
    /// Cloned from the [`E2eContext`]; backs
    /// [`super::wait::wait_for_balance`].
    wait_hub: Arc<WaitEventHub>,
}

impl std::fmt::Debug for TestWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestWallet")
            .field("wallet_id", &hex::encode(self.wallet.wallet_id()))
            .finish_non_exhaustive()
    }
}

impl TestWallet {
    /// Create a fresh-seeded test wallet, register with the
    /// manager, and eagerly initialise its platform-address
    /// provider so `next_unused_address` / `transfer` work
    /// immediately on return.
    ///
    /// The caller passes `seed_bytes` (typically via `OsRng`) so the
    /// registry can persist them BEFORE the wallet is returned —
    /// a crashed test still has a recoverable record.
    pub async fn create(
        manager: &Arc<PlatformWalletManager<NoPlatformPersistence>>,
        seed_bytes: [u8; 64],
        network: Network,
        wait_hub: Arc<WaitEventHub>,
    ) -> FrameworkResult<Self> {
        let wallet = manager
            .create_wallet_from_seed_bytes(
                network,
                seed_bytes,
                WalletAccountCreationOptions::Default,
            )
            .await
            .map_err(wallet_err)?;
        // Force the lazy platform-address init now so test code
        // doesn't see a surprise first-use latency hit.
        wallet.platform().initialize().await;
        let signer = SeedBackedPlatformAddressSigner::new(&seed_bytes, network)?;
        Ok(Self {
            seed_bytes,
            wallet,
            signer,
            wait_hub,
        })
    }

    /// Stable wallet id used as the registry key.
    pub fn id(&self) -> WalletSeedHash {
        self.wallet.wallet_id()
    }

    /// 64-byte seed used to derive this wallet (persisted in the
    /// registry so a sweep can reconstruct the wallet).
    pub fn seed_bytes(&self) -> [u8; 64] {
        self.seed_bytes
    }

    /// Underlying `PlatformWallet` — for tests that reach into
    /// identity / token / core APIs.
    pub fn platform_wallet(&self) -> &Arc<PlatformWallet> {
        &self.wallet
    }

    /// Seed-backed address signer used by `transfer`; tests that
    /// broadcast transitions via the SDK directly can pass it in.
    pub fn address_signer(&self) -> &SeedBackedPlatformAddressSigner {
        &self.signer
    }

    /// Process-shared event hub — backs
    /// [`super::wait::wait_for_balance`].
    pub fn wait_hub(&self) -> &Arc<WaitEventHub> {
        &self.wait_hub
    }

    /// Next unused receive address on the wallet's default
    /// platform-payment account. Pool advances only after a sync
    /// observes an inbound credit on the prior address; a freshly
    /// returned address has balance `0` until the next sync sees it
    /// funded. Returns a new address if the gap window is exhausted.
    pub async fn next_unused_address(&self) -> FrameworkResult<PlatformAddress> {
        self.wallet
            .platform()
            .next_unused_receive_address(default_platform_payment_account_key())
            .await
            .map_err(wallet_err)
    }

    /// Run a BLAST sync pass and refresh balances for every
    /// tracked address.
    pub async fn sync_balances(&self) -> FrameworkResult<()> {
        self.wallet
            .platform()
            .sync_balances(None)
            .await
            .map(|_| ())
            .map_err(wallet_err)
    }

    /// Snapshot of cached balances per tracked address. Reflects
    /// the last `sync_balances` — call it first if you need a fresh
    /// view.
    pub async fn balances(&self) -> BTreeMap<PlatformAddress, Credits> {
        self.wallet
            .platform()
            .addresses_with_balances()
            .await
            .into_iter()
            .collect()
    }

    /// Total credits across every tracked address.
    pub async fn total_credits(&self) -> Credits {
        self.wallet.platform().total_credits().await
    }

    /// Transfer credits to one or more outputs. Auto-selects inputs
    /// from the default account and uses [`default_fee_strategy`]
    /// (reduce output #0). `outputs` maps each recipient address
    /// to its credit amount.
    pub async fn transfer(
        &self,
        outputs: BTreeMap<PlatformAddress, Credits>,
    ) -> FrameworkResult<PlatformAddressChangeSet> {
        self.wallet
            .platform()
            .transfer(
                DEFAULT_ACCOUNT_INDEX_PUB,
                InputSelection::Auto,
                outputs,
                default_fee_strategy(),
                Some(PlatformVersion::latest()),
                &self.signer,
            )
            .await
            .map_err(wallet_err)
    }
}

/// Default fee strategy: reduce output #0 by the fee amount.
pub(crate) fn default_fee_strategy() -> AddressFundsFeeStrategy {
    vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]
}

/// Generate a fresh 64-byte seed plus its hex encoding for the
/// registry. Single source so signer + registry stay in sync.
pub fn fresh_seed() -> ([u8; 64], String) {
    let mut seed = [0u8; 64];
    OsRng.fill_bytes(&mut seed);
    let hex = hex::encode(seed);
    (seed, hex)
}

/// Build a registry entry for a fresh seed. Insert it BEFORE
/// handing the wallet to the test body so a panic between insert
/// and teardown leaves a recoverable trail.
pub fn registry_entry_from_seed(seed: &[u8; 64], note: Option<String>) -> RegistryEntry {
    RegistryEntry {
        seed_hex: hex::encode(seed),
        created_at: SystemTime::now(),
        status: EntryStatus::Active,
        note,
    }
}

/// Guard returned by [`super::setup`].
///
/// Tests SHOULD call [`SetupGuard::teardown`] explicitly once
/// they're done; the [`Drop`] impl is a panic-safety fallback that
/// logs a warning and relies on the next-startup
/// `cleanup::sweep_orphans` to recover funds.
pub struct SetupGuard {
    /// Process-shared context (`&'static` — `E2eContext::init`
    /// returns a singleton).
    pub ctx: &'static E2eContext,
    /// Fresh-seed test wallet, already registered for cleanup.
    pub test_wallet: TestWallet,
    /// Set to `true` by a successful [`SetupGuard::teardown`] so
    /// [`Drop`] skips its warning.
    pub(crate) teardown_called: bool,
}

impl SetupGuard {
    /// Sweep the test wallet's funds back to the bank and remove
    /// its registry entry.
    ///
    /// Best-effort: a transient sync / transfer failure retains the
    /// registry entry, so the next process startup retries via
    /// [`super::cleanup::sweep_orphans`].
    pub async fn teardown(mut self) -> FrameworkResult<()> {
        let result = super::cleanup::teardown_one(
            self.ctx.manager(),
            self.ctx.bank(),
            self.ctx.registry(),
            &self.test_wallet,
        )
        .await;
        if result.is_ok() {
            self.teardown_called = true;
        }
        result
    }
}

impl Drop for SetupGuard {
    fn drop(&mut self) {
        if !self.teardown_called {
            tracing::warn!(
                wallet_id = %hex::encode(self.test_wallet.id()),
                "SetupGuard dropped without explicit teardown — wallet will be \
                 swept on next test process startup"
            );
        }
    }
}

/// `PlatformWalletError` → framework error envelope.
fn wallet_err(err: PlatformWalletError) -> FrameworkError {
    FrameworkError::Wallet(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drift guard: our pinned defaults must match `PlatformPaymentAccountSpec::default()`.
    /// If `key_wallet` ever changes its canonical defaults, this test fires.
    #[test]
    fn default_spec_matches_pinned_constants() {
        let canonical = PlatformPaymentAccountSpec::default();
        assert_eq!(canonical.account, DEFAULT_ACCOUNT_INDEX_PUB);
        assert_eq!(canonical.key_class, DEFAULT_KEY_CLASS_PUB);
        assert_eq!(canonical, DEFAULT_PLATFORM_PAYMENT_ACCOUNT_SPEC);
    }
}
