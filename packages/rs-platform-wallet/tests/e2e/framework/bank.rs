//! Pre-funded bank wallet — funding source for every test wallet.
//!
//! Loaded from the `PLATFORM_WALLET_E2E_BANK_MNEMONIC` env var at
//! `E2eContext::init` time and held for the lifetime of the suite.
//! `fund_address` consumes a small slice of the bank's credits and
//! transfers them to a target [`PlatformAddress`]; in-process funding
//! calls serialise on a static `tokio::sync::Mutex` so concurrent
//! tests don't trip over each other's nonces.
//!
//! Cross-process isolation is the operator's concern: distinct
//! `PLATFORM_WALLET_E2E_BANK_MNEMONIC` per environment, distinct
//! workdir slots per process on the same machine.
//!
//! Wave 3a delivers the full implementation. Wave 4 wires
//! `BankWallet::load` into `E2eContext::init`.

use std::collections::BTreeMap;
use std::sync::Arc;

use bip39::Mnemonic as Bip39Mnemonic;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use key_wallet::Network;
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::wallet::platform_addresses::InputSelection;
use platform_wallet::{
    PlatformAddressChangeSet, PlatformWallet, PlatformWalletError, PlatformWalletManager,
};
use tokio::sync::Mutex as AsyncMutex;

use super::config::Config;
use super::signer::SeedBackedPlatformAddressSigner;
use super::wallet_factory::{
    default_fee_strategy, DEFAULT_ACCOUNT_INDEX_PUB, DEFAULT_KEY_CLASS_PUB,
};
use super::{FrameworkError, FrameworkResult};

/// In-process funding mutex — serialises concurrent
/// `bank.fund_address` calls so nonces don't race. Cross-process
/// concurrency is handled by giving each process a distinct workdir
/// slot (see [`super::workdir::pick_available_workdir`]); the bank
/// itself is not cross-process safe.
static FUNDING_MUTEX: AsyncMutex<()> = AsyncMutex::const_new(());

/// Bank wallet handle — wraps a fully-synced `PlatformWallet` plus
/// its dedicated signer. Funding requests go through `fund_address`
/// rather than touching the underlying wallet directly so we keep
/// the FUNDING_MUTEX invariant in one place.
pub struct BankWallet {
    wallet: Arc<PlatformWallet>,
    signer: SeedBackedPlatformAddressSigner,
    /// Cached for log breadcrumbs / under-funded panic messages.
    primary_receive_address: PlatformAddress,
}

impl std::fmt::Debug for BankWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BankWallet")
            .field("wallet_id", &hex::encode(self.wallet.wallet_id()))
            .field("primary_receive_address", &self.primary_receive_address)
            .finish_non_exhaustive()
    }
}

impl BankWallet {
    /// Load the bank from its BIP-39 mnemonic, run a single BLAST
    /// sync pass, and verify the balance covers the configured
    /// [`Config::min_bank_credits`] floor.
    ///
    /// Under-funded balances PANIC with an actionable message
    /// pointing at the bank's primary receive address, mirroring
    /// `dash-evo-tool`'s convention. The panic is intentional — a
    /// silent under-funded run would just produce confusing
    /// downstream "insufficient balance" errors inside individual
    /// tests instead of a single clear "top up the bank" pointer.
    pub async fn load(
        manager: &Arc<PlatformWalletManager<NoPlatformPersistence>>,
        config: &Config,
    ) -> FrameworkResult<Self> {
        if config.bank_mnemonic.trim().is_empty() {
            return Err(FrameworkError::Bank(
                "bank mnemonic is empty — set PLATFORM_WALLET_E2E_BANK_MNEMONIC".into(),
            ));
        }
        // bip39's `Mnemonic::parse` accepts every BIP-39 wordlist
        // automatically; key-wallet's typed loader is then handled
        // inside `create_wallet_from_mnemonic`. We also derive the
        // 64-byte seed here so the seed-backed address signer can
        // pre-derive its key cache in [`Self::build_signer`].
        let validated: Bip39Mnemonic =
            config.bank_mnemonic.parse().map_err(|err: bip39::Error| {
                FrameworkError::Bank(format!("invalid BIP-39 mnemonic: {err}"))
            })?;
        let seed_bytes = validated.to_seed("");

        let network = parse_network(&config.network)?;
        let wallet = manager
            .create_wallet_from_mnemonic(
                &config.bank_mnemonic,
                network,
                key_wallet::wallet::initialization::WalletAccountCreationOptions::Default,
            )
            .await
            .map_err(wallet_err)?;
        wallet.platform().initialize().await;

        // Single BLAST pass to seed balances. Sync errors are
        // surfaced — a bank that can't even sync at startup will
        // make every test fail anyway.
        wallet
            .platform()
            .sync_balances(None)
            .await
            .map_err(wallet_err)?;

        // Capture the bank's primary receive address before checking
        // the funded floor so the under-funded panic message can
        // tell the operator exactly where to top up.
        let primary_receive_address = wallet
            .platform()
            .next_unused_receive_address(
                key_wallet::account::account_collection::PlatformPaymentAccountKey {
                    account: DEFAULT_ACCOUNT_INDEX_PUB,
                    key_class: DEFAULT_KEY_CLASS_PUB,
                },
            )
            .await
            .map_err(wallet_err)?;

        let total = wallet.platform().total_credits().await;
        if total < config.min_bank_credits {
            // The framework treats an under-funded bank as a hard
            // operator error — there's nothing useful the test
            // suite can do without it. Panic so CI logs surface
            // the actionable message clearly rather than burying
            // it in a Result chain. Format mirrors the README's
            // "Bank pre-funding" section (multi-line, bech32m
            // address) so the operator-facing pointer is identical
            // whether they hit it from the README or from a CI
            // failure.
            let address_bech32m = primary_receive_address.to_bech32m_string(network);
            panic!(
                "Bank wallet under-funded.\n  \
                 balance : {balance} credits\n  \
                 required: {required} credits\n  \
                 top up at: {address_bech32m}\n\
                 \n\
                 Send testnet platform credits to the address above, then re-run the tests.",
                balance = total,
                required = config.min_bank_credits,
            );
        }

        let signer = SeedBackedPlatformAddressSigner::new(&seed_bytes, network)?;
        Ok(Self {
            wallet,
            signer,
            primary_receive_address,
        })
    }

    /// Borrow the underlying `PlatformWallet`. Used by cleanup
    /// helpers that need to inspect the bank's balance after a
    /// teardown sweep.
    pub fn platform_wallet(&self) -> &Arc<PlatformWallet> {
        &self.wallet
    }

    /// The bank's primary receive address — the destination
    /// `cleanup::teardown_one` sweeps test-wallet balances back to.
    pub fn primary_receive_address(&self) -> &PlatformAddress {
        &self.primary_receive_address
    }

    /// Network the bank is operating against. Mirrors
    /// `wallet.sdk().network`; centralised here so cleanup paths
    /// don't need to dig through the wallet handle.
    pub fn network(&self) -> Network {
        self.wallet.sdk().network
    }

    /// Fund a target address with `credits` credits. Acquires the
    /// in-process [`FUNDING_MUTEX`] for the duration of the SDK
    /// transfer so concurrent in-process calls serialise cleanly.
    ///
    /// The recipient is responsible for polling its own balance
    /// after this returns — the bank doesn't wait for the chain to
    /// see the credits, so a follow-up
    /// [`super::wait::wait_for_balance`] is the test's job.
    pub async fn fund_address(
        &self,
        target: &PlatformAddress,
        credits: Credits,
    ) -> FrameworkResult<PlatformAddressChangeSet> {
        let _guard = FUNDING_MUTEX.lock().await;
        let outputs: BTreeMap<PlatformAddress, Credits> =
            std::iter::once((*target, credits)).collect();
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

    /// Resync the bank's balances. Used by cleanup paths that need
    /// to wait for a test wallet's drained funds to land.
    pub async fn sync_balances(&self) -> FrameworkResult<()> {
        self.wallet
            .platform()
            .sync_balances(None)
            .await
            .map(|_| ())
            .map_err(wallet_err)
    }

    /// Total credits the bank currently has cached.
    pub async fn total_credits(&self) -> Credits {
        self.wallet.platform().total_credits().await
    }
}

/// Parse the configured network string into the `key-wallet` enum.
/// Mirrors the case-insensitive matching the rest of the platform
/// uses; rejects anything unrecognised so config typos surface
/// loudly.
fn parse_network(value: &str) -> FrameworkResult<Network> {
    let normalized = value.trim().to_ascii_lowercase();
    let net = match normalized.as_str() {
        "" | "testnet" => Network::Testnet,
        "mainnet" => Network::Mainnet,
        "devnet" => Network::Devnet,
        "regtest" | "local" => Network::Regtest,
        other => {
            return Err(FrameworkError::Bank(format!(
                "unrecognised network {other:?} — expected one of \
                 testnet/mainnet/devnet/regtest/local"
            )))
        }
    };
    Ok(net)
}

fn wallet_err(err: PlatformWalletError) -> FrameworkError {
    FrameworkError::Wallet(err.to_string())
}
