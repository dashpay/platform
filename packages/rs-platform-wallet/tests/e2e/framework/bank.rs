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
use std::time::{Duration, Instant};

use bip39::Mnemonic as Bip39Mnemonic;
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::AddressInfo;
use dash_sdk::Sdk;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::util::hash::ripemd160_sha256;
use dpp::version::PlatformVersion;
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::gap_limit::DIP17_GAP_LIMIT;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::signer::Signer;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::Wallet;
use key_wallet::{AccountType, ChildNumber, Network};
use parking_lot::Mutex as SyncMutex;
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::wallet::platform_addresses::InputSelection;
use platform_wallet::{
    PlatformAddressChangeSet, PlatformWallet, PlatformWalletError, PlatformWalletManager,
};
use tokio::sync::Mutex as AsyncMutex;

use simple_signer::signer::SimpleSigner;

use super::config::{Config, EXPECTED_TOKEN_SUITE_FLOOR};
use super::wallet_factory::{bank_fee_strategy, DEFAULT_ACCOUNT_INDEX_PUB, DEFAULT_KEY_CLASS_PUB};
use super::{make_platform_signer, FrameworkError, FrameworkResult};

/// In-process funding mutex — serialises concurrent
/// `bank.fund_address` calls so nonces don't race **during
/// broadcast**.
///
/// **Scope:** held only across STE build + sign + DAPI-accept
/// broadcast (`PlatformAddressWallet::transfer`), then dropped. The
/// post-broadcast chain-confirmation wait
/// (`wait_for_address_nonces_chain_confirmed`) runs **unlocked** so
/// 14-way concurrent `fund_address` callers don't queue up serially
/// on what's a parallelisable wait. Out-of-order STE arrival across
/// DAPI replicas is now handled by the in-`fund_address` bounded
/// retry on nonce-class chain rejects (see [`is_nonce_class_error`]).
static FUNDING_MUTEX: AsyncMutex<()> = AsyncMutex::const_new(());

/// Hard ceiling on the post-broadcast chain-confirmation wait inside
/// [`BankWallet::fund_address`]. Testnet block production is usually
/// 2–5 s but has been observed at ~75 s under contention (TK-013
/// QA-V19-001 timeline). 120 s is a safety net: if the chain hasn't
/// caught up in two minutes, something else is wrong and the test
/// should fail fast with a clear panic rather than hang the suite.
const FUNDING_TX_CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(120);

/// Number of times to retry the startup balance sync when the harness
/// wallet cache shows significantly fewer credits than the independent
/// DAPI `AddressInfo::fetch` (positive drift = lagging replica, #3611).
///
/// Each retry calls `sync_and_refresh_floor()`, which issues a fresh
/// request that the DAPI load-balancer may route to a non-lagging node.
/// After all retries the harness adopts the proof-verified independent
/// reading (see [`BankWallet::accept_independent_platform_balance`]).
pub const BALANCE_SYNC_RETRIES: usize = 3;

/// Sleep between replica-lag retry attempts. Long enough to let the
/// DAPI load-balancer pick a different node; short enough not to add
/// meaningful CI latency when the bank is well-funded.
pub const BALANCE_SYNC_RETRY_SLEEP: Duration = Duration::from_secs(2);

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

/// Result of an independent `AddressInfo::fetch` cross-check against
/// the harness's wallet-cached Platform balance. Stored on
/// [`super::harness::E2eContext`] for test introspection; logged at
/// `info` (agreement) or `warn` (disagreement) during framework init
/// (QA-V26-005).
#[derive(Debug, Clone)]
pub struct CrossCheckResult {
    /// Balance read from the harness wallet cache (via
    /// `wallet.platform().total_credits()`).
    pub harness_credits: Credits,
    /// Balance returned by a proof-verified `AddressInfo::fetch`
    /// against DAPI — independent of the wallet/manager layer.
    pub independent_credits: Credits,
    /// The bank's primary Platform address (DIP-17 `m/9'/1'/17'/0'/0'/0`).
    pub address: PlatformAddress,
    /// Address nonce from the independent fetch (`None` if the address
    /// had no on-chain record yet).
    pub nonce: Option<AddressNonce>,
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
    /// `true` when the bank's Platform balance meets the token-suite
    /// floor (`EXPECTED_TOKEN_SUITE_FLOOR`). Token tests check this at
    /// startup and skip cleanly when `false` (QA-V26-003).
    pub bank_floor_satisfied: bool,
    /// Proof-verified balance adopted from an independent `AddressInfo::fetch`
    /// when the wallet cache is known to have returned a stale low value from
    /// a lagging DAPI replica at startup (#3611).
    ///
    /// Used as a floor by [`Self::effective_platform_credits`]: the returned
    /// balance is `max(wallet_cache, adopted_platform_floor)`.  When the
    /// wallet cache eventually catches up this field becomes redundant but
    /// harmless (the max resolves to the wallet cache).
    ///
    /// Set to `0` (inactive) by default; overridden by
    /// [`Self::accept_independent_platform_balance`] only when the harness
    /// detects a large positive drift between the wallet cache and the
    /// independent DAPI reading.
    adopted_platform_floor: Credits,
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
    /// Load the bank from its BIP-39 mnemonic and sync once.
    ///
    /// Does NOT enforce the minimum-credit floor — call
    /// [`Self::assert_floor`] after [`sweep_orphans`] so the sweep can
    /// recover stranded funds before the floor check fires (QA-V26-007).
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
        // `Some(0)` requests a full historical compact-filter scan
        // from genesis. The bank is a long-lived testnet address
        // that may receive Layer-1 funding before any test run
        // starts; without this, SPV's default "scan from current
        // tip" window would miss those UTXOs and report
        // `core_balance=0` even when funded (QA-001 / QA-002 /
        // QA-003 in `/tmp/bank-core-balance-diagnosis.md`).
        let wallet = manager
            .create_wallet_from_mnemonic(
                &config.bank_mnemonic,
                network,
                key_wallet::wallet::initialization::WalletAccountCreationOptions::Default,
                Some(0),
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
        // run. The reservation allocator intentionally hands out another
        // available address after index 0 has been observed used.
        let primary_receive_address = derive_platform_address_at_index(
            &seed_bytes,
            network,
            DEFAULT_ACCOUNT_INDEX_PUB,
            DEFAULT_KEY_CLASS_PUB,
            0,
        )?;

        let total = wallet.platform().total_credits().await;
        let bank_floor_satisfied = total >= EXPECTED_TOKEN_SUITE_FLOOR;
        if !bank_floor_satisfied {
            let address_bech32m = primary_receive_address.to_bech32m_string(network);
            tracing::warn!(
                target: "platform_wallet::e2e::bank",
                balance = total,
                floor = EXPECTED_TOKEN_SUITE_FLOOR,
                address = %address_bech32m,
                "Bank balance is below the token-suite floor (~88.8B credits); \
                 token tests may exhaust funds mid-run. \
                 Top up the Platform address to continue token testing."
            );
        } else {
            tracing::info!(
                target: "platform_wallet::e2e::bank",
                balance = total,
                floor = EXPECTED_TOKEN_SUITE_FLOOR,
                "bank floor satisfied"
            );
        }

        tracing::info!(
            address = %primary_receive_address.to_bech32m_string(network),
            balance = total,
            network = %network,
            "Bank wallet loaded",
        );

        // B-2: derive the bank's Platform signer from the synced funded
        // pool, not a fixed `0..DIP17_GAP_LIMIT` window. The bank is a
        // long-lived shared testnet wallet whose on-chain Platform-address
        // pool has cycled well past the first gap window over the
        // campaign's lifetime; a fixed-window signer holds no key for the
        // higher-index funded addresses `auto_select_inputs` legitimately
        // selects, which deterministically fails the 2nd sequential
        // `fund_address` (see #556). `fund_address` uses
        // `InputSelection::Auto`, which has no change branch and generates
        // no new addresses mid-run, so the signing key set is fully
        // determined by what `sync_balances` discovered plus the eager
        // pool fill — bounded, no unbounded in-run drift.
        let signer = Self::derive_pool_signer(&wallet, &seed_bytes, network).await?;
        Ok(Self {
            wallet,
            signer,
            seed_bytes,
            primary_receive_address,
            bank_floor_satisfied,
            adopted_platform_floor: 0,
        })
    }

    /// Build the bank's Platform-payment [`SimpleSigner`] keyed for every
    /// synced/generated address index in account `DEFAULT_ACCOUNT_INDEX_PUB`
    /// plus one full forward gap window of margin.
    ///
    /// Indices come from the post-`sync_balances` managed account's
    /// `addresses` map (every synced address) and `highest_generated` (the
    /// eager `AddressPool::new` fill). The margin is `DIP17_GAP_LIMIT` — one
    /// pool-advance unit, matching how the DIP-17 pool cycles in
    /// gap-window-wide steps — covering pool addresses generated but not yet
    /// balance-synced. Bounded because the bank-funding path
    /// (`InputSelection::Auto`) creates no new change addresses at run time.
    async fn derive_pool_signer(
        wallet: &Arc<PlatformWallet>,
        seed_bytes: &[u8; 64],
        network: Network,
    ) -> FrameworkResult<SimpleSigner> {
        let highest_index = wallet
            .platform()
            .platform_payment_account_max_derived_index(DEFAULT_ACCOUNT_INDEX_PUB)
            .await
            .map_err(|err| {
                FrameworkError::Bank(format!("bank pool signer: max derived index: {err}"))
            })?;

        // `+ DIP17_GAP_LIMIT` forward margin; `0..=ceiling` also re-covers
        // the eager `0..=gap_limit-1` fill. No funded pool → fall back to
        // the plain gap window.
        let ceiling = match highest_index {
            Some(hi) => hi.saturating_add(DIP17_GAP_LIMIT),
            None => return make_platform_signer(seed_bytes, network),
        };
        SimpleSigner::from_seed_for_platform_addresses(
            seed_bytes,
            network,
            DEFAULT_ACCOUNT_INDEX_PUB,
            DEFAULT_KEY_CLASS_PUB,
            0..=ceiling,
        )
        .map_err(|err| FrameworkError::Wallet(format!("bank pool signer: {err}")))
    }

    /// Assert the bank has enough credits to run the test suite.
    ///
    /// Panics with an operator-actionable message if the current
    /// cached balance is below `min_bank_credits`. Call this AFTER
    /// [`sweep_orphans`] and a fresh [`Self::sync_balances`] so
    /// recovered orphan funds are counted (QA-V26-007).
    ///
    /// `sweep_recovered` is the number of orphan wallets successfully
    /// swept; `registry_total` and `registry_failed` are used to enrich
    /// the panic message when the balance is still below floor after
    /// sweep so operators know whether the sweep had anything to drain.
    pub async fn assert_floor(
        &self,
        config: &Config,
        sweep_recovered: usize,
        registry_total: usize,
        registry_failed: usize,
    ) {
        let network = self.wallet.sdk().network;
        // Use effective_platform_credits (max(cache, adopted)) so a
        // proof-verified independent balance adopted on replica lag (#3611)
        // also satisfies the floor here — same semantics as
        // sync_and_refresh_floor() and the fund planner's snapshot.
        let total = self.effective_platform_credits().await;
        if total >= config.min_bank_credits {
            return;
        }
        let address_bech32m = self.primary_receive_address.to_bech32m_string(network);
        if sweep_recovered > 0 || registry_total > 0 {
            panic!(
                "Bank under-funded after sweep recovery: have {balance}M credits, need at least {required}M.\n  \
                 Sweep recovered {sweep_recovered} orphan wallets; registry had {registry_total} entries \
                 ({registry_failed} Failed, {removed} removed).\n  \
                 Top up Platform address: {address_bech32m}",
                balance = total / 1_000_000,
                required = config.min_bank_credits / 1_000_000,
                removed = registry_total.saturating_sub(registry_failed),
            );
        } else {
            panic!(
                "Bank under-funded: have {balance}M credits, need at least {required}M.\n  \
                 Top up Platform address: {address_bech32m}",
                balance = total / 1_000_000,
                required = config.min_bank_credits / 1_000_000,
            );
        }
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

    /// `true` when the bank's Platform balance met the token-suite
    /// floor at init time. Token tests skip cleanly when `false`.
    pub fn bank_floor_satisfied(&self) -> bool {
        self.bank_floor_satisfied
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
        /// Max retries on nonce-class chain rejects from the broadcast leg.
        /// Total attempt count is `MAX_RETRIES + 1`.
        const MAX_RETRIES: u32 = 3;
        /// Exponential backoff between retries. Lengths must equal `MAX_RETRIES`.
        const BACKOFF: [Duration; MAX_RETRIES as usize] = [
            Duration::from_millis(500),
            Duration::from_secs(2),
            Duration::from_secs(5),
        ];

        for attempt in 0..=MAX_RETRIES {
            // Rec 2 — bank pre-funding credit snapshot (DEBUG; opt-in via
            // RUST_LOG=platform_wallet::e2e::bank=debug).
            tracing::debug!(
                target: "platform_wallet::e2e::bank",
                bank_credits_before = self.total_credits().await,
                ?target,
                credits,
                attempt,
                "bank.fund_address: bank balance at attempt entry"
            );

            // === Critical section: build STE + sign + broadcast ===
            // Lock held only across the DAPI-accept boundary. The
            // post-broadcast chain-confirmation wait runs unlocked.
            // PA-008c history is sampled inside this block so every
            // recorded `[entry_ns, exit_ns]` interval is a strict
            // subset of the time the guard was held.
            let broadcast_outcome: Result<PlatformAddressChangeSet, PlatformWalletError> = {
                let _guard = FUNDING_MUTEX.lock().await;
                let anchor = history_anchor();
                let seq = FUNDING_MUTEX_SEQ
                    .fetch_add(1, Ordering::SeqCst)
                    .saturating_add(1);
                let entry_ns = anchor.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;

                let outputs: BTreeMap<PlatformAddress, Credits> =
                    std::iter::once((*target, credits)).collect();
                let broadcast_started = Instant::now();
                let result = self
                    .wallet
                    .platform()
                    .transfer(
                        DEFAULT_ACCOUNT_INDEX_PUB,
                        InputSelection::Auto,
                        outputs.into_iter().collect(),
                        bank_fee_strategy(),
                        Some(PlatformVersion::latest()),
                        &self.signer,
                    )
                    .await;

                let exit_ns = anchor.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
                record_funding_mutex_entry(FundingMutexHistoryEntry {
                    seq,
                    entry_ns,
                    exit_ns,
                });

                match result.as_ref() {
                    Ok(cs) => {
                        tracing::info!(
                            target: "platform_wallet::e2e::bank",
                            seq,
                            attempt,
                            ?target,
                            credits,
                            elapsed_ms = broadcast_started.elapsed().as_millis() as u64,
                            "bank.fund_address: transfer broadcast accepted (lock released)"
                        );
                        // tx_hash is not in PlatformAddressChangeSet (Rec 5 deferred —
                        // requires struct field addition). The SDK logs transaction_id at
                        // TRACE in dash_sdk::platform::transition::broadcast immediately
                        // before this INFO line; correlate by timestamp or by seq above.
                        // changeset Debug is the best available fallback without struct changes.
                        tracing::trace!(
                            target: "platform_wallet::e2e::bank",
                            seq,
                            ?target,
                            changeset = ?cs,
                            "bank.fund_address: broadcast changeset (no tx_hash — grep sdk TRACE by timestamp)"
                        );
                    }
                    Err(err) => tracing::warn!(
                        target: "platform_wallet::e2e::bank",
                        seq,
                        attempt,
                        ?target,
                        credits,
                        elapsed_ms = broadcast_started.elapsed().as_millis() as u64,
                        error = %err,
                        "bank.fund_address: transfer broadcast failed"
                    ),
                }
                result
            }; // FUNDING_MUTEX dropped here

            // === Broadcast classification + retry ===
            let cs = match broadcast_outcome {
                Ok(cs) => cs,
                Err(err) if is_nonce_class_error(&err) && attempt < MAX_RETRIES => {
                    let backoff = BACKOFF[attempt as usize];
                    tracing::warn!(
                        target: "platform_wallet::e2e::bank",
                        error = %err,
                        attempt,
                        backoff_ms = backoff.as_millis() as u64,
                        "bank.fund_address: nonce-class chain reject, retrying after backoff"
                    );
                    tokio::time::sleep(backoff).await;
                    continue;
                }
                Err(err) => return Err(wallet_err(err)),
            };

            // === Unlocked chain-confirmation wait ===
            // `wait_for_address_nonces_chain_confirmed` polls all DAPI
            // replicas until they agree on the post-tx nonce. Running
            // it unlocked lets sibling `fund_address` callers progress
            // their own broadcasts in parallel — testnet block
            // production is the long pole, not the lock.
            let expected_nonces: Vec<(PlatformAddress, AddressNonce)> = cs
                .addresses
                .iter()
                .map(|entry| {
                    (
                        PlatformAddress::P2pkh(entry.address.to_bytes()),
                        entry.funds.nonce,
                    )
                })
                .collect();
            let confirm_started = Instant::now();
            match super::wait::wait_for_address_nonces_chain_confirmed(
                self.wallet.sdk(),
                &expected_nonces,
                FUNDING_TX_CONFIRMATION_TIMEOUT,
            )
            .await
            {
                Ok(()) => {
                    tracing::info!(
                        target: "platform_wallet::e2e::bank",
                        addresses = expected_nonces.len(),
                        attempt,
                        elapsed_ms = confirm_started.elapsed().as_millis() as u64,
                        "bank.fund_address: chain confirmation observed"
                    );
                    super::funding_ledger::record_platform_requested(credits);
                    return Ok(cs);
                }
                Err(err) => {
                    // 120 s without chain catch-up is a platform-level
                    // failure: silently retrying would mask it.
                    // Preserve the panic from the pre-split contract.
                    tracing::error!(
                        target: "platform_wallet::e2e::bank",
                        error = %err,
                        attempt,
                        elapsed_ms = confirm_started.elapsed().as_millis() as u64,
                        timeout_secs = FUNDING_TX_CONFIRMATION_TIMEOUT.as_secs(),
                        "bank.fund_address: chain confirmation timeout"
                    );
                    panic!(
                        "bank.fund_address: chain-confirmed nonce did not catch up within \
                         {timeout:?} (attempt={attempt}); platform-level failure, see error log: {err}",
                        timeout = FUNDING_TX_CONFIRMATION_TIMEOUT,
                    );
                }
            }
        }
        unreachable!("retry loop must return or panic before falling through")
    }

    /// Resync balances and refresh the cached `bank_floor_satisfied` flag.
    ///
    /// Called after [`sweep_orphans`] so the token-suite floor reflects
    /// the post-sweep balance rather than the stale load-time snapshot
    /// (QA-V26-007).
    ///
    /// Uses [`Self::effective_platform_credits`] (i.e. `max(cache, adopted)`)
    /// so that a previously adopted proof-verified balance continues to
    /// protect the floor gate even when a subsequent sync still hits a lagging
    /// replica (#3611).
    pub async fn sync_and_refresh_floor(&mut self) -> FrameworkResult<()> {
        self.wallet
            .platform()
            .sync_balances(None)
            .await
            .map_err(wallet_err)?;
        let total = self.effective_platform_credits().await;
        self.bank_floor_satisfied = total >= EXPECTED_TOKEN_SUITE_FLOOR;
        Ok(())
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
    ///
    /// For funding-gate decisions use [`Self::effective_platform_credits`]
    /// instead; it accounts for a proof-verified independent balance adopted
    /// on DAPI replica lag at startup (#3611).
    pub async fn total_credits(&self) -> Credits {
        self.wallet.platform().total_credits().await
    }

    /// Effective Platform credit balance for funding-gate decisions.
    ///
    /// Returns `max(wallet_cache, adopted_platform_floor)`.  Under normal
    /// operation `adopted_platform_floor` is 0 so this is identical to
    /// [`Self::total_credits`].  When the harness detects that the startup
    /// sync landed on a lagging DAPI replica (#3611) and calls
    /// [`Self::accept_independent_platform_balance`], the adopted value acts
    /// as a proof-verified floor so the floor gate and fund planner never
    /// see a false 0 even if the wallet cache is still stale.
    pub async fn effective_platform_credits(&self) -> Credits {
        let cached = self.wallet.platform().total_credits().await;
        cached.max(self.adopted_platform_floor)
    }

    /// The adopted proof-verified Platform floor, or `0` when not set.
    /// See [`Self::accept_independent_platform_balance`].
    pub fn adopted_platform_floor(&self) -> Credits {
        self.adopted_platform_floor
    }

    /// Override the balance used by funding gates with a proof-verified
    /// independent reading from `AddressInfo::fetch`.
    ///
    /// Called by the harness init when it detects a large positive drift
    /// between the wallet cache (stale 0 from a lagging DAPI replica) and
    /// the independent DAPI fetch (real ~225B), and retrying `sync_balances`
    /// did not converge (#3611).
    ///
    /// After this call:
    /// - `effective_platform_credits()` returns `max(cache, credits)`.
    /// - `bank_floor_satisfied` is recomputed from the effective value.
    /// - `assert_floor()` and `sync_and_refresh_floor()` both use
    ///   `effective_platform_credits()`, so they also see the correct value.
    pub fn accept_independent_platform_balance(&mut self, credits: Credits) {
        let prev_floor = self.bank_floor_satisfied;
        self.adopted_platform_floor = credits;
        self.bank_floor_satisfied = credits >= EXPECTED_TOKEN_SUITE_FLOOR;
        tracing::info!(
            target: "platform_wallet::e2e::bank",
            independent_credits = credits,
            floor = EXPECTED_TOKEN_SUITE_FLOOR,
            floor_satisfied = self.bank_floor_satisfied,
            prev_floor_satisfied = prev_floor,
            "bank floor gate re-evaluated from proof-verified independent \
             DAPI balance (harness wallet cache lagged replica at startup — \
             #3611). effective_platform_credits() will use max(cache, adopted)."
        );
    }

    /// Inject the proof-verified on-chain balance into the wallet's spendable
    /// cache for the bank's primary receive address.
    ///
    /// Paired with [`Self::accept_independent_platform_balance`]: that method
    /// fixes Layer 1 (floor gate / fund planner via `effective_platform_credits`);
    /// this method fixes Layer 2 (spend path via `auto_select_inputs` which reads
    /// `account.address_credit_balance` directly, not `adopted_floor`).
    ///
    /// Only call after [`Self::accept_independent_platform_balance`] has been
    /// invoked with a dual-verified balance.
    pub(crate) async fn inject_verified_balance_into_spend_cache(&self, credits: Credits) {
        let PlatformAddress::P2pkh(hash) = self.primary_receive_address else {
            tracing::warn!(
                target: "platform_wallet::e2e::bank",
                "inject_verified_balance_into_spend_cache: primary_receive_address \
                 is not P2PKH — skipping"
            );
            return;
        };
        let p2pkh = key_wallet::PlatformP2PKHAddress::new(hash);
        if let Err(e) = self
            .wallet
            .platform()
            .inject_address_balance(DEFAULT_ACCOUNT_INDEX_PUB, p2pkh, credits)
            .await
        {
            tracing::error!(
                target: "platform_wallet::e2e::bank",
                error = %e,
                "inject_verified_balance_into_spend_cache failed — \
                 fund_address will see 0 available credits and fail"
            );
        }
    }

    /// Independent balance cross-check via `AddressInfo::fetch` (QA-V26-005).
    ///
    /// Reads the bank's Platform-side balance through a single proof-verified
    /// DAPI round-trip, bypassing the wallet/manager layer entirely. Call this
    /// AFTER [`Self::sync_balances`] so `harness_credits` reflects a fresh
    /// wallet-cache snapshot at the same point in time.
    ///
    /// Returns a [`CrossCheckResult`] containing both readings. The caller
    /// is responsible for logging the comparison — see `harness.rs` for the
    /// `info` / `warn` log sites.
    pub async fn cross_check_balance(&self, sdk: &Sdk) -> CrossCheckResult {
        let harness_credits = self.wallet.platform().total_credits().await;
        let addr = self.primary_receive_address;
        let fetch_result = AddressInfo::fetch(sdk, addr).await;
        let (independent_credits, nonce) = match fetch_result {
            Ok(Some(info)) => (info.balance, Some(info.nonce)),
            Ok(None) => (0, None),
            Err(err) => {
                tracing::warn!(
                    target: "platform_wallet::e2e::bank",
                    error = %err,
                    "bank balance cross-check: AddressInfo::fetch failed; \
                     independent reading unavailable"
                );
                (0, None)
            }
        };
        CrossCheckResult {
            harness_credits,
            independent_credits,
            address: addr,
            nonce,
        }
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

    /// Bank's confirmed Core (Layer-1) balance in duffs, sourced from
    /// the lock-free atomic updated by SPV. Used for pre-flight under-
    /// funded checks in [`Self::send_core_to`] and the harness init
    /// log; not transactionally consistent with the wallet's UTXO set.
    pub fn core_balance_confirmed(&self) -> u64 {
        self.wallet.balance().confirmed()
    }

    /// Bank wallet's SPV birth height — the earliest block SPV's
    /// compact-filter scan will inspect for this wallet. Surfaced in
    /// the harness init log so operators can correlate `core_balance=0`
    /// with the scan window: if the funding tx confirmed below
    /// `birth_height`, SPV won't see it.
    pub async fn birth_height(&self) -> u32 {
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
        self.wallet.state().await.birth_height()
    }

    /// Fixed BIP-44 (Core) receive address at account 0, external
    /// chain, address index 0 (`m/44'/coin'/0'/0/0`). Deterministic
    /// and stable across every process run regardless of UTXO history
    /// — the operator funds this ONE address and every test run
    /// reuses it. Surfaced in the harness init log so the operator
    /// knows where to send Layer-1 duffs to fund the bank.
    pub async fn primary_core_receive_address(&self) -> FrameworkResult<dashcore::Address> {
        derive_core_receive_address_at_index(&self.seed_bytes, self.wallet.sdk().network, 0, 0)
    }

    /// Send `duffs` of Layer-1 Core duffs from the bank's BIP-44
    /// account 0 to a Core `dashcore::Address`.
    ///
    /// Thin wrapper over [`core_send`]: serialises on
    /// [`FUNDING_MUTEX`] so concurrent Core / Platform funding flows
    /// don't race UTXO selection, runs an under-funded pre-check
    /// against the bank's confirmed Core balance, and adds the bank's
    /// primary receive address to the error message so operators
    /// know where to top up testnet duffs. Returns the broadcast
    /// `Txid` on success; does NOT wait for instant-lock or chain
    /// confirmation — callers follow up with
    /// [`super::wait::wait_for_core_balance`] when they need
    /// positive-balance arrival.
    ///
    /// Errors:
    /// - [`FrameworkError::Bank`] when the bank's confirmed Core
    ///   balance is below `duffs + CORE_TX_FEE_RESERVE`. The error
    ///   message names the bank's primary receive address so the
    ///   operator knows where to top up testnet duffs.
    /// - [`FrameworkError::Wallet`] for build/sign/broadcast failures
    ///   surfaced from the underlying `CoreWallet`.
    ///
    /// Used by `ID-007` (negative contract: identity-auth addresses
    /// are NOT in `monitored_addresses()`, so the wallet's Core
    /// balance must NOT observe this send within the test's window).
    pub async fn send_core_to(
        &self,
        target: &dashcore::Address,
        duffs: u64,
    ) -> FrameworkResult<dashcore::Txid> {
        let _guard = FUNDING_MUTEX.lock().await;

        let confirmed = self.wallet.balance().confirmed();
        let required = duffs.saturating_add(CORE_TX_FEE_RESERVE);
        if confirmed < required {
            // Surface the operator-actionable pointer same shape as
            // the `BankWallet::load` under-funded panic so operators
            // hit the same documented format whether the bank is
            // Platform-credit or Core-duff under-funded.
            let receive_addr = self.primary_core_receive_address().await?;
            return Err(FrameworkError::Bank(format!(
                "Bank Core under-funded.\n  \
                 confirmed: {confirmed} duffs\n  \
                 required : {required} duffs (send {duffs} + ~{CORE_TX_FEE_RESERVE} fee reserve)\n  \
                 short by : {short} duffs\n  \
                 top up at: {receive_addr}\n\
                 \n\
                 Send testnet Core duffs to the address above, then re-run the test.",
                short = required - confirmed,
            )));
        }

        let txid = core_send(&self.wallet, &self.seed_bytes, target, duffs).await?;
        tracing::info!(
            target: "platform_wallet::e2e::bank",
            %txid,
            target = %target,
            duffs,
            "bank.send_core_to broadcast"
        );
        super::funding_ledger::record_core_requested(duffs);
        Ok(txid)
    }
}

fn wallet_err(err: PlatformWalletError) -> FrameworkError {
    FrameworkError::Wallet(err.to_string())
}

/// Classify `err` as a nonce-class chain reject — broadcast errors
/// caused by out-of-order STE arrival across DAPI replicas under
/// concurrent funding load.
///
/// **Typed match preferred** (mirrors [`platform_wallet::error::is_instant_lock_proof_invalid`]):
/// drills into [`PlatformWalletError::Sdk`] and matches the consensus
/// error variants that indicate an address- or identity-nonce
/// conflict. String matching is reserved for the rare case where the
/// error has already been flattened (none of those reach this layer
/// today — `transfer()` propagates `dash_sdk::Error` via `Sdk`).
///
/// Variants matched (DPP `StateError`):
/// - `AddressInvalidNonceError` — the address-funds path (what
///   `bank.fund_address` actually hits when DAPI replica lag causes
///   provided/expected nonce mismatch).
/// - `InvalidIdentityNonceError` — defence-in-depth: covers the
///   identity-nonce sibling that follows the same out-of-order
///   semantics if the call path ever broadens beyond address funds.
fn is_nonce_class_error(err: &PlatformWalletError) -> bool {
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;

    let PlatformWalletError::Sdk(sdk_err) = err else {
        return false;
    };

    let consensus_error = match sdk_err {
        dash_sdk::Error::StateTransitionBroadcastError(b) => b.cause.as_ref(),
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(ce)) => Some(ce.as_ref()),
        _ => None,
    };

    matches!(
        consensus_error,
        Some(ConsensusError::StateError(
            StateError::AddressInvalidNonceError(_) | StateError::InvalidIdentityNonceError(_),
        )),
    )
}

/// Generous standard-tx fee reserve (~0.0001 DASH at 1 sat/B for a
/// typical 1-input-2-output tx). The wallet's coin selector picks the
/// actual fee from its config; this floor only gates the "is there
/// enough to even try" pre-check on `BankWallet::send_core_to` and
/// the dust-floor on the test-wallet Core sweep.
pub const CORE_TX_FEE_RESERVE: u64 = 10_000;

/// Build, sign, and broadcast a Core (Layer-1) transaction sending
/// `duffs` from `wallet`'s BIP-44 account 0 to `target`.
///
/// Free function so both [`BankWallet::send_core_to`] and the
/// `cleanup::sweep_core_addresses` test-wallet sweep can share the
/// actual broadcast path. Callers are responsible for their own
/// pre-flight checks (under-funded balance, lock serialisation) and
/// for selecting the appropriate `duffs` amount — this helper does
/// nothing more than translate the inputs into the split
/// build/sign/broadcast path and surface the resulting `Txid`.
pub(super) async fn core_send(
    wallet: &Arc<PlatformWallet>,
    seed: &[u8; 64],
    target: &dashcore::Address,
    duffs: u64,
) -> FrameworkResult<dashcore::Txid> {
    let outputs = vec![(target.clone(), duffs)];
    let signer = super::signer::SeedBackedCoreSigner::new(*seed, wallet.core().network());
    let tx = core_send_from_account(
        wallet,
        StandardAccountType::BIP44Account,
        0,
        outputs,
        &signer,
    )
    .await
    .map_err(wallet_err)?;
    Ok(tx.txid())
}

/// Build, sign, and broadcast a Core transaction from a standard account.
pub(crate) async fn core_send_from_account<S: Signer>(
    wallet: &Arc<PlatformWallet>,
    account_type: StandardAccountType,
    account_index: u32,
    outputs: Vec<(dashcore::Address, u64)>,
    signer: &S,
) -> Result<dashcore::Transaction, PlatformWalletError> {
    if outputs.is_empty() {
        return Err(PlatformWalletError::TransactionBuild(
            "No outputs specified".to_string(),
        ));
    }

    let tx = {
        let mut wm = wallet.wallet_manager().write().await;
        let wallet_id = wallet.wallet_id();
        let (key_wallet, info) = wm.get_wallet_and_info_mut(&wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "wallet {} missing from manager during Core transaction build",
                hex::encode(wallet_id)
            ))
        })?;
        let current_height = info.core_wallet.synced_height();
        let (managed_account, account) = match account_type {
            StandardAccountType::BIP44Account => (
                info.core_wallet
                    .accounts
                    .standard_bip44_accounts
                    .get_mut(&account_index),
                key_wallet
                    .accounts
                    .standard_bip44_accounts
                    .get(&account_index),
            ),
            StandardAccountType::BIP32Account => (
                info.core_wallet
                    .accounts
                    .standard_bip32_accounts
                    .get_mut(&account_index),
                key_wallet
                    .accounts
                    .standard_bip32_accounts
                    .get(&account_index),
            ),
        };
        let managed_account = managed_account.ok_or_else(|| {
            PlatformWalletError::TransactionBuild(format!(
                "{account_type:?} managed account {account_index} not found"
            ))
        })?;
        let account = account.ok_or_else(|| {
            PlatformWalletError::TransactionBuild(format!(
                "{account_type:?} account {account_index} not found in wallet"
            ))
        })?;

        let mut builder = TransactionBuilder::new()
            .set_current_height(current_height)
            .set_selection_strategy(SelectionStrategy::LargestFirst)
            .add_funding(managed_account, account);
        for (address, amount) in &outputs {
            builder = builder.add_output(address, *amount);
        }
        let (tx, _fee) = builder
            .build_signed(signer, |address| {
                managed_account.address_derivation_path(&address)
            })
            .await
            .map_err(|error| {
                let context = error.to_string();
                if context.contains("Insufficient funds") || context.contains("No UTXOs available")
                {
                    PlatformWalletError::NoSpendableInputs {
                        account_type,
                        account_index,
                        context,
                    }
                } else {
                    PlatformWalletError::TransactionBuild(context)
                }
            })?;
        tx
    };

    wallet
        .core()
        .broadcast_transaction_releasing_reservation(account_type, account_index, &tx)
        .await?;
    Ok(tx)
}

/// Rebuild a fresh, **signable** [`key_wallet::Wallet`] from the bank's
/// retained BIP-39 seed.
///
/// v3.1-dev's `PlatformWalletManager::register_wallet` downgrades every
/// managed wallet to external-signable (no private key), so the live
/// managed wallet can no longer derive hardened paths. A wallet rebuilt
/// from the same seed reproduces the identical root key — and therefore
/// the identical addresses — while retaining the private key the hardened
/// DIP-17 / BIP-44 derivations below need. This is the same derivation
/// surface `print_bank_address_offline` uses to print the operator's
/// funding addresses, so the results match the funded ones.
fn signable_bank_wallet(seed_bytes: &[u8; 64], network: Network) -> FrameworkResult<Wallet> {
    Wallet::from_seed_bytes(*seed_bytes, network, WalletAccountCreationOptions::Default).map_err(
        |err| FrameworkError::Bank(format!("rebuild signable bank wallet from seed: {err}")),
    )
}

/// Derive the DIP-17 platform-payment address at `index` from the bank's
/// retained seed, using path `m/9'/coin_type'/17'/account'/key_class'/index`.
///
/// Bank-only helper: lets us pin the bank's sweep target to index 0
/// without going through the address pool's "next unused" cursor.
/// Routes through [`key_wallet::Wallet::derive_public_key`] on a fresh
/// signable wallet rebuilt from the seed (see [`signable_bank_wallet`]) —
/// the managed wallet is external-signable post-merge and can't derive
/// hardened paths. Same root key, so the address is identical.
fn derive_platform_address_at_index(
    seed_bytes: &[u8; 64],
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

    let pubkey = signable_bank_wallet(seed_bytes, network)?
        .derive_public_key(&leaf_path)
        .map_err(|err| {
            FrameworkError::Bank(format!("derive_public_key at index {index}: {err}"))
        })?;
    let pkh = ripemd160_sha256(&pubkey.serialize());
    Ok(PlatformAddress::P2pkh(pkh))
}

/// External chain selector for the BIP-44 receive branch
/// (`m/44'/coin'/account'/0/index`). Internal/change is `1`.
const BIP44_EXTERNAL_CHAIN: u32 = 0;

/// Derive the BIP-44 Core (Layer-1) receive address at `index` from
/// the already-loaded `PlatformWallet`, using path
/// `m/44'/coin_type'/account'/0/index` (external chain `0`).
///
/// Bank-only helper, mirroring [`derive_platform_address_at_index`]
/// for Layer-1: pins the bank's Core top-up target to a fixed index
/// so the operator funds ONE address and every run reuses it.
/// `CoreWallet::next_receive_address_for_account` advances the pool's
/// "next unused" cursor off index 0 as soon as a UTXO lands there, so
/// the top-up address would otherwise drift run-to-run. Routes through
/// [`key_wallet::Wallet::derive_public_key`] on a fresh signable wallet
/// rebuilt from the seed (see [`signable_bank_wallet`]) and reconstructs
/// the P2PKH address exactly as key-wallet's own address pool does
/// (compressed ECDSA pubkey → `Address::p2pkh`).
fn derive_core_receive_address_at_index(
    seed_bytes: &[u8; 64],
    network: Network,
    account: u32,
    index: u32,
) -> FrameworkResult<dashcore::Address> {
    let account_path = AccountType::Standard {
        index: account,
        standard_account_type: StandardAccountType::BIP44Account,
    }
    .derivation_path(network)
    .map_err(|err| FrameworkError::Bank(format!("BIP-44 account path: {err}")))?;
    let chain = ChildNumber::from_normal_idx(BIP44_EXTERNAL_CHAIN)
        .map_err(|err| FrameworkError::Bank(format!("invalid external chain: {err}")))?;
    let leaf = ChildNumber::from_normal_idx(index)
        .map_err(|err| FrameworkError::Bank(format!("invalid child index {index}: {err}")))?;
    let leaf_path = account_path.extend([chain, leaf]);

    let pubkey = signable_bank_wallet(seed_bytes, network)?
        .derive_public_key(&leaf_path)
        .map_err(|err| {
            FrameworkError::Bank(format!("derive_public_key at index {index}: {err}"))
        })?;
    let dash_pubkey = dashcore::PublicKey::new(pubkey);
    Ok(dashcore::Address::p2pkh(&dash_pubkey, network))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::consensus::state::address_funds::AddressInvalidNonceError;
    use dpp::consensus::state::identity::invalid_identity_contract_nonce_error::InvalidIdentityNonceError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::identifier::Identifier;
    use dpp::identity::identity_nonce::MergeIdentityNonceResult;

    /// Build a `PlatformWalletError::Sdk` wrapping the given consensus
    /// error via the `Protocol(ConsensusError)` shape — the path that
    /// `transfer()` actually surfaces broadcast-time consensus rejects on.
    fn sdk_err_from_consensus(ce: ConsensusError) -> PlatformWalletError {
        let protocol_err = dpp::ProtocolError::ConsensusError(Box::new(ce));
        PlatformWalletError::Sdk(dash_sdk::Error::Protocol(protocol_err))
    }

    #[test]
    fn is_nonce_class_error_matches_address_invalid_nonce() {
        let inner = AddressInvalidNonceError::new(PlatformAddress::P2pkh([0u8; 20]), 42, 41);
        let err = sdk_err_from_consensus(StateError::AddressInvalidNonceError(inner).into());
        assert!(
            is_nonce_class_error(&err),
            "AddressInvalidNonceError must be classified as nonce-class"
        );
    }

    #[test]
    fn is_nonce_class_error_matches_invalid_identity_nonce() {
        let inner = InvalidIdentityNonceError::new(
            Identifier::new([0u8; 32]),
            None,
            7,
            MergeIdentityNonceResult::NonceAlreadyPresentInPast(3),
        );
        let err = sdk_err_from_consensus(StateError::InvalidIdentityNonceError(inner).into());
        assert!(
            is_nonce_class_error(&err),
            "InvalidIdentityNonceError must be classified as nonce-class"
        );
    }

    #[test]
    fn is_nonce_class_error_rejects_no_selectable_inputs() {
        // OnlyOutputAddressesFunded is the closest "insufficient funds"-shape
        // error in this codebase and must NOT be classified as
        // nonce-class — retrying it would just churn against the
        // same empty input pool.
        let err = PlatformWalletError::OnlyOutputAddressesFunded {
            funded_outputs: vec![],
            sub_min_count: 0,
            sub_min_aggregate: 0,
            min_input_amount: 0,
        };
        assert!(
            !is_nonce_class_error(&err),
            "OnlyOutputAddressesFunded must NOT be classified as nonce-class"
        );
    }

    #[test]
    fn is_nonce_class_error_rejects_unrelated_errors() {
        let err = PlatformWalletError::WalletNotFound("xyz".to_string());
        assert!(
            !is_nonce_class_error(&err),
            "WalletNotFound must NOT be classified as nonce-class"
        );
        let err2 = PlatformWalletError::AddressOperation("nope".to_string());
        assert!(
            !is_nonce_class_error(&err2),
            "AddressOperation must NOT be classified as nonce-class"
        );
    }
}
