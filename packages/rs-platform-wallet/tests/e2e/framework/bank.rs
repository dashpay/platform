//! Pre-funded bank wallet — funding source for every test wallet.
//!
//! Loaded from `PLATFORM_WALLET_E2E_BANK_MNEMONIC` at
//! `E2eContext::init` time. `fund_address` serialises in-process
//! calls on [`FUNDING_MUTEX`] so concurrent tests don't race nonces;
//! cross-process isolation is the operator's concern (distinct
//! mnemonic per environment, distinct workdir slot per process).

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
/// `bank.fund_address` calls so nonces don't race.
static FUNDING_MUTEX: AsyncMutex<()> = AsyncMutex::const_new(());

/// Bank wallet handle wrapping a synced `PlatformWallet` and its
/// signer. All funding flows through `fund_address` so the
/// `FUNDING_MUTEX` invariant lives in one place.
pub struct BankWallet {
    wallet: Arc<PlatformWallet>,
    signer: SeedBackedPlatformAddressSigner,
    /// Cached for under-funded panic messages and log breadcrumbs.
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
    /// Load the bank from its BIP-39 mnemonic, sync once, and check
    /// the balance covers [`Config::min_bank_credits`].
    ///
    /// Under-funded balances PANIC with a "top up at <address>"
    /// pointer; surfacing one clear actionable failure beats burying
    /// it under per-test "insufficient balance" errors.
    pub async fn load(
        manager: &Arc<PlatformWalletManager<NoPlatformPersistence>>,
        config: &Config,
    ) -> FrameworkResult<Self> {
        if config.bank_mnemonic.trim().is_empty() {
            return Err(FrameworkError::Bank(
                "bank mnemonic is empty — set PLATFORM_WALLET_E2E_BANK_MNEMONIC".into(),
            ));
        }
        // Validate up front and derive the 64-byte seed once so the
        // seed-backed signer can pre-build its key cache below.
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

        // Seed balances; a sync failure here makes every test fail.
        wallet
            .platform()
            .sync_balances(None)
            .await
            .map_err(wallet_err)?;

        // Capture the receive address before the funded-floor check
        // so the under-funded panic message can name a top-up target.
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
            // Under-funded bank is a hard operator error; panic with
            // the README's bank-pre-funding format so operators hit
            // the same actionable pointer in CI as in the docs.
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

    /// Borrow the underlying `PlatformWallet`.
    pub fn platform_wallet(&self) -> &Arc<PlatformWallet> {
        &self.wallet
    }

    /// Primary receive address — the sweep destination for
    /// `cleanup::teardown_one`.
    pub fn primary_receive_address(&self) -> &PlatformAddress {
        &self.primary_receive_address
    }

    /// Network the bank is operating against.
    pub fn network(&self) -> Network {
        self.wallet.sdk().network
    }

    /// Fund `target` with `credits` from the bank's primary
    /// account.
    ///
    /// Submits the transfer immediately and returns the resulting
    /// [`PlatformAddressChangeSet`]. Does NOT wait for the chain to
    /// observe the credit — callers follow up with
    /// [`super::wait::wait_for_balance`] on the recipient wallet.
    /// Concurrent in-process calls serialise on [`FUNDING_MUTEX`]
    /// to avoid nonce races.
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

    /// Resync the bank's balances.
    pub async fn sync_balances(&self) -> FrameworkResult<()> {
        self.wallet
            .platform()
            .sync_balances(None)
            .await
            .map(|_| ())
            .map_err(wallet_err)
    }

    /// Total credits the bank currently has cached. Reflects the
    /// last sync — call [`Self::sync_balances`] first for a fresh
    /// view.
    pub async fn total_credits(&self) -> Credits {
        self.wallet.platform().total_credits().await
    }
}

/// Case-insensitive network parser; rejects unknown values so
/// config typos surface loudly.
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
