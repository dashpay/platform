//! Pre-funded bank wallet — funding source for every test wallet.
//!
//! Loaded from `PLATFORM_WALLET_E2E_BANK_MNEMONIC` at
//! `E2eContext::init` time. `fund_address` serialises in-process
//! calls on [`FUNDING_MUTEX`] so concurrent tests don't race nonces;
//! cross-process isolation is the operator's concern (distinct
//! mnemonic per environment, distinct workdir slot per process).

use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use bip39::Mnemonic as Bip39Mnemonic;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::util::hash::ripemd160_sha256;
use dpp::version::PlatformVersion;
use key_wallet::{AccountType, ChildNumber, Network};
use parking_lot::Mutex as SyncMutex;
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::wallet::platform_addresses::InputSelection;
use platform_wallet::{
    PlatformAddressChangeSet, PlatformWallet, PlatformWalletError, PlatformWalletManager,
};
use tokio::sync::Mutex as AsyncMutex;

use simple_signer::signer::SimpleSigner;

use super::config::Config;
use super::wallet_factory::{bank_fee_strategy, DEFAULT_ACCOUNT_INDEX_PUB, DEFAULT_KEY_CLASS_PUB};
use super::{make_platform_signer, FrameworkError, FrameworkResult};

/// In-process funding mutex — serialises concurrent
/// `bank.fund_address` calls so nonces don't race.
static FUNDING_MUTEX: AsyncMutex<()> = AsyncMutex::const_new(());

/// Monotonic sequence for [`FUNDING_MUTEX`] entries. Each successful
/// acquisition of [`FUNDING_MUTEX`] inside [`BankWallet::fund_address`]
/// increments this counter by `1`; the value at increment time is the
/// entry's serialisation rank, recorded in [`FundingMutexHistoryEntry`].
///
/// Test-only: read by [`BankWallet::funding_mutex_history`] for PA-008c
/// (observable serialisation contract). Production correctness does not
/// depend on this counter.
static FUNDING_MUTEX_SEQ: AtomicU64 = AtomicU64::new(0);

/// Capped ring buffer of the last [`FUNDING_MUTEX_HISTORY_CAP`] entries
/// recorded by [`BankWallet::fund_address`]. PA-008c drains it via
/// [`BankWallet::funding_mutex_history`] to assert pairwise non-overlap
/// of the `[entry_ns, exit_ns]` intervals.
///
/// `parking_lot::Mutex` (sync) so the recording sites in `fund_address`
/// don't have to `.await` the lock — recording a timestamp must not
/// itself yield, or the "exit" sample becomes lossy under contention.
static FUNDING_MUTEX_HISTORY: SyncMutex<VecDeque<FundingMutexHistoryEntry>> =
    SyncMutex::new(VecDeque::new());

/// Soft cap on [`FUNDING_MUTEX_HISTORY`] retained entries. Picked
/// arbitrarily large enough that PA-008c's three-task fan-in plus
/// adjacent test traffic never overflow the window in a single test
/// run, but small enough that the buffer doesn't grow unboundedly
/// under sustained contention from larger test fan-ins.
const FUNDING_MUTEX_HISTORY_CAP: usize = 256;

/// One observation of a [`FUNDING_MUTEX`] critical section.
///
/// Sampled inside [`BankWallet::fund_address`] using a single
/// [`Instant`] anchor captured at module init: `entry_ns` and
/// `exit_ns` are nanoseconds since that anchor, so cross-entry
/// comparisons are monotonic and platform-independent. `seq` is the
/// post-increment value of [`FUNDING_MUTEX_SEQ`] at acquisition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FundingMutexHistoryEntry {
    /// Monotonic sequence number from [`FUNDING_MUTEX_SEQ`].
    pub seq: u64,
    /// Nanoseconds since [`history_anchor()`] when the lock was
    /// acquired. Read after `lock().await` returns, so the value
    /// reflects "we are inside the critical section".
    pub entry_ns: u64,
    /// Nanoseconds since [`history_anchor()`] when the
    /// `fund_address` body returned and the [`FUNDING_MUTEX`] guard
    /// was about to drop. Sampled before `_guard` falls out of scope.
    pub exit_ns: u64,
}

/// Process-shared monotonic anchor for [`FundingMutexHistoryEntry`]
/// timestamps. `LazyLock` means every recorded entry shares the same
/// reference instant, so absolute ordering across entries is well-defined.
fn history_anchor() -> Instant {
    use std::sync::OnceLock;
    static ANCHOR: OnceLock<Instant> = OnceLock::new();
    *ANCHOR.get_or_init(Instant::now)
}

/// Drain the in-memory [`FUNDING_MUTEX`] history. Test-only; production
/// callers never invoke this.
///
/// Returns the entries in insertion order and clears the buffer so
/// successive PA-008c-style asserts don't observe entries from a prior
/// test's fan-in. PA-008b runs adjacent and may itself populate the
/// buffer; tests that care about specific entries must drain BEFORE
/// the spawn fan-out and assert on the post-await drain.
fn drain_funding_mutex_history() -> Vec<FundingMutexHistoryEntry> {
    let mut guard = FUNDING_MUTEX_HISTORY.lock();
    let drained: Vec<_> = guard.drain(..).collect();
    drained
}

/// Append `entry` to [`FUNDING_MUTEX_HISTORY`], honouring the
/// soft cap. Older entries fall off the front when the buffer is full.
fn record_funding_mutex_entry(entry: FundingMutexHistoryEntry) {
    let mut guard = FUNDING_MUTEX_HISTORY.lock();
    if guard.len() >= FUNDING_MUTEX_HISTORY_CAP {
        guard.pop_front();
    }
    guard.push_back(entry);
}

/// Bank wallet handle wrapping a synced `PlatformWallet` and its
/// signer. All funding flows through `fund_address` so the
/// `FUNDING_MUTEX` invariant lives in one place.
pub struct BankWallet {
    wallet: Arc<PlatformWallet>,
    signer: SimpleSigner,
    /// 64-byte BIP-39 seed retained so the bank-identity helpers can
    /// derive identity-side keys without re-parsing the mnemonic.
    seed_bytes: [u8; 64],
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

        let network = config.network;
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

        // Pin the bank's sweep target to DIP-17 index 0 deterministically
        // so the same address absorbs sweep-back funds across every test
        // run. `next_unused_receive_address` would otherwise advance past
        // index 0 once it gets marked used, accumulating empty addresses.
        let primary_receive_address = derive_platform_address_at_index(
            &wallet,
            network,
            DEFAULT_ACCOUNT_INDEX_PUB,
            DEFAULT_KEY_CLASS_PUB,
            0,
        )
        .await?;

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

        tracing::info!(
            address = %primary_receive_address.to_bech32m_string(network),
            balance = total,
            network = %network,
            "Bank wallet ready",
        );

        let signer = make_platform_signer(&seed_bytes, network)?;
        Ok(Self {
            wallet,
            signer,
            seed_bytes,
            primary_receive_address,
        })
    }

    /// 64-byte BIP-39 seed used to derive both the bank's address keys
    /// and (optionally) its identity keys. Tests/sweep helpers reach
    /// for this when building a `SeedBackedIdentitySigner` over the
    /// bank identity.
    pub fn seed_bytes(&self) -> &[u8; 64] {
        &self.seed_bytes
    }

    /// Bank's platform-address signer. The same `Signer<PlatformAddress>`
    /// used by `fund_address`; exposed so the bank-identity bootstrap
    /// can sign the funding-address inputs of the registration
    /// transition without rebuilding it.
    pub fn address_signer(&self) -> &SimpleSigner {
        &self.signer
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
    /// Recipients receive the **exact** `credits` amount; the fee
    /// is deducted from the bank's input via
    /// [`bank_fee_strategy`]. The bank therefore consumes
    /// `credits + fee` from its own platform-addresses pool —
    /// verify the bank balance is sufficiently above
    /// `min_bank_credits` before calling.
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
        // Sample entry AFTER `lock().await` resolves: we are now
        // inside the critical section. PA-008c asserts the
        // `[entry_ns, exit_ns]` intervals are pairwise non-overlapping,
        // which only holds if the entry timestamp is captured under
        // the lock — sampling before `lock().await` would record
        // queue-arrival time and the windows would overlap by
        // construction.
        let anchor = history_anchor();
        let seq = FUNDING_MUTEX_SEQ
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        let entry_ns = anchor.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;

        let outputs: BTreeMap<PlatformAddress, Credits> =
            std::iter::once((*target, credits)).collect();
        let result = self
            .wallet
            .platform()
            .transfer(
                DEFAULT_ACCOUNT_INDEX_PUB,
                InputSelection::Auto,
                outputs,
                bank_fee_strategy(),
                Some(PlatformVersion::latest()),
                &self.signer,
            )
            .await
            .map_err(wallet_err);

        // Sample exit BEFORE `_guard` drops so the recorded interval
        // is a strict subset of the time the lock was actually held.
        // Errors are still recorded — PA-008c cares about
        // serialisation, not success.
        let exit_ns = anchor.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
        record_funding_mutex_entry(FundingMutexHistoryEntry {
            seq,
            entry_ns,
            exit_ns,
        });
        result
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

    /// Drain and return the [`FUNDING_MUTEX`] critical-section
    /// observations recorded since the last drain. Test-only; pins
    /// the observable serialisation contract for PA-008c.
    ///
    /// Each entry covers ONE `fund_address` call and is the
    /// `[entry_ns, exit_ns]` window for that call's hold of
    /// [`FUNDING_MUTEX`]. PA-008c asserts:
    ///   1. There are entries for every `fund_address` it spawned
    ///      (entry count matches fan-in).
    ///   2. `seq` is strictly monotonic across the drain (mutex
    ///      acquisition order is well-defined).
    ///   3. Sorted by `seq`, every consecutive pair `(i, i+1)` has
    ///      `entries[i].exit_ns <= entries[i+1].entry_ns` — the
    ///      windows are pairwise non-overlapping, i.e. the mutex
    ///      actually serialises.
    ///
    /// This drains the buffer; back-to-back PA-008c-style tests
    /// don't observe each other's entries.
    pub fn funding_mutex_history(&self) -> Vec<FundingMutexHistoryEntry> {
        drain_funding_mutex_history()
    }

    /// Send `duffs` of Layer-1 Core duffs from the bank to a Core
    /// `dashcore::Address`. Stubbed `unimplemented!()` — the bank
    /// today holds Platform credits, not Core coins (see CR-003's
    /// "Core-funded bank wallet helper" prerequisite). Wired in when
    /// Task #15 (SPV runtime) lands and the bank gains a Core-funded
    /// account.
    ///
    /// Used by `ID-007` to attempt a Layer-1 send to a DIP-9
    /// identity-auth address; the assertion side of that test
    /// pins "the Core balance does NOT increase" against the
    /// pinned `key-wallet` revision's contract.
    pub async fn send_core_to(
        &self,
        target: &dashcore::Address,
        duffs: u64,
    ) -> FrameworkResult<dashcore::Txid> {
        let _ = (target, duffs);
        unimplemented!(
            "BankWallet::send_core_to — CR-003 prerequisite. The bank \
             today holds Platform credits via DIP-17 platform-payment \
             accounts, not Core duffs on a DIP-9 / BIP-44 receive \
             account. Wire through when Task #15 (SPV runtime) lands \
             and the bank exposes a Core-funded account; see TEST_SPEC.md \
             § ID-007 / § CR-003 for the gating discussion."
        );
    }
}

fn wallet_err(err: PlatformWalletError) -> FrameworkError {
    FrameworkError::Wallet(err.to_string())
}

/// Derive the DIP-17 platform-payment address at `index` from the
/// already-loaded `PlatformWallet`, using path
/// `m/9'/coin_type'/17'/account'/key_class'/index`.
///
/// Bank-only helper: lets us pin the bank's sweep target to index 0
/// without going through the address pool's "next unused" cursor.
/// Routes through [`key_wallet::Wallet::derive_public_key`] on the live
/// wallet rather than re-running BIP-32 from raw seed bytes — keeps a
/// single derivation surface.
async fn derive_platform_address_at_index(
    wallet: &Arc<PlatformWallet>,
    network: Network,
    account: u32,
    key_class: u32,
    index: u32,
) -> FrameworkResult<PlatformAddress> {
    let account_path = AccountType::PlatformPayment { account, key_class }
        .derivation_path(network)
        .map_err(|err| FrameworkError::Bank(format!("DIP-17 account path: {err}")))?;
    let leaf = ChildNumber::from_normal_idx(index)
        .map_err(|err| FrameworkError::Bank(format!("invalid child index {index}: {err}")))?;
    let leaf_path = account_path.extend([leaf]);

    let pubkey = wallet
        .state()
        .await
        .wallet()
        .derive_public_key(&leaf_path)
        .map_err(|err| {
            FrameworkError::Bank(format!("derive_public_key at index {index}: {err}"))
        })?;
    let pkh = ripemd160_sha256(&pubkey.serialize());
    Ok(PlatformAddress::P2pkh(pkh))
}
