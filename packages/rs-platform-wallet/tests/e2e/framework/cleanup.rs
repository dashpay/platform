//! Cleanup paths: startup [`sweep_orphans`] and per-test
//! [`teardown_one`]. Both reconstruct the wallet from the registry
//! seed, sync, and drain every fund source back to the bank by
//! walking the per-source-type sweep helpers. Best-effort: errors
//! are logged and the registry retains the entry for the next run.
//!
//! Sink architecture: Platform-side sweeps (addresses AND identities)
//! land on the bank's Platform address —
//! [`super::bank::BankWallet::primary_receive_address`] — the single
//! Platform-side funding pool. See [`super::bank_rebalance`] for the
//! design contract.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityBalance;
use dpp::address_funds::{AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::prelude::Identifier;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::version::PlatformVersion;
use key_wallet::bip32::ExtendedPrivKey;
use key_wallet::gap_limit::DIP17_GAP_LIMIT;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::wallet::platform_addresses::InputSelection;
use platform_wallet::{PlatformWallet, PlatformWalletError, PlatformWalletManager};
use simple_signer::signer::SimpleSigner;

use super::signer::SeedBackedIdentitySigner;

use super::bank::{core_send, BankWallet, CORE_TX_FEE_RESERVE};
use super::bank_identity::BankIdentity;
use super::registry::{EntryStatus, PersistentTestWalletRegistry, RegistryEntry, WalletSeedHash};
use super::wallet_factory::{TestWallet, DEFAULT_ACCOUNT_INDEX_PUB, DEFAULT_KEY_CLASS_PUB};
use super::{make_platform_signer, FrameworkError, FrameworkResult};

/// Sweep gate: a wallet is only swept if its total balance can plausibly
/// satisfy the protocol's `min_input_amount`. Below that, no input can
/// pass `address_funds` validation and the broadcast would fail anyway.
/// Pulled from `PlatformVersion` rather than a hardcoded constant so we
/// stay in lock-step with whatever the active version dictates.
fn min_input_amount(version: &PlatformVersion) -> Credits {
    version.dpp.state_transitions.address_funds.min_input_amount
}

/// Public mirror of [`min_input_amount`] for tests that want to pin
/// the cleanup gate against the active platform version (PA-004b /
/// PA-009 boundary cases). Reads the same field, so a protocol bump
/// shifts both the harness gate and the test's expected value in
/// lockstep.
pub fn cleanup_dust_gate(version: &PlatformVersion) -> Credits {
    min_input_amount(version)
}

/// Default per-step timeout for cleanup polls.
pub const CLEANUP_STEP_TIMEOUT: Duration = Duration::from_secs(60);

/// Best-effort sweep of a wallet's residual platform credits back to
/// the bank.
///
/// Used by [`sweep_orphans`] / [`teardown_one`] to decide whether to
/// drop the registry entry or retain it as `Failed` for next-run
/// retry. The contract is:
///
/// - If residual is below the protocol's `min_input_amount` (the
///   sweep-fee minimum), the dust is abandoned and the registry entry
///   is removed — no recovery is possible without a bank top-up. The
///   abandoned credit total is tracked in [`Self::dust_abandoned`] and
///   surfaced in the post-sweep summary log. (V27-004 — accept-dust
///   policy.)
/// - If broadcast succeeds, the registry entry is removed.
/// - If broadcast fails (transient), the registry entry is retained
///   and marked [`EntryStatus::Failed`] so bootstrap [`sweep_orphans`]
///   can retry on a future run.
///
/// QA-V26-006 — prior to this struct every helper returned `Ok(())`
/// after logging a warn, so a broadcast failure looked identical to
/// "nothing to sweep" and the registry was purged unconditionally on
/// the happy-path branch — silently leaking the funds.
#[derive(Debug, Default)]
pub struct SweepReport {
    /// Sub-sweeps that attempted a broadcast and succeeded
    /// (transition built, signed, broadcast Ok'd by the SDK).
    pub broadcasts_succeeded: u32,
    /// Sub-sweeps that attempted a broadcast and the SDK / chain
    /// rejected it. Each entry is a one-line description with the
    /// seed-hash + step name embedded for grep-ability.
    pub broadcast_failures: Vec<String>,
    /// `true` once at least one broadcast attempt succeeded — used
    /// by [`sweep_orphans`] to keep the "swept_with_broadcast"
    /// metric distinct from the "skipped, no funds" cohort.
    pub had_funds_to_recover: bool,
    /// Total credits left behind on platform addresses whose balance
    /// fell below `min_input_amount` (the protocol-level sweep-fee
    /// minimum). The accept-dust policy (V27-004) drops the registry
    /// entry rather than retaining it — bootstrap retry can't recover
    /// dust without a bank top-up — so this counter is the only
    /// surface for tracking how much was abandoned.
    pub dust_abandoned: Credits,
    /// Σ of `amount` across every successful
    /// `transfer_credits_to_addresses` broadcast in
    /// [`sweep_identities_with_seed`]. Direct evidence that this
    /// sweep moved identity credits to the bank's Platform address —
    /// preferred over post-hoc bank-address balance deltas, which
    /// are contaminated by sibling tests' funding spends on the
    /// process-shared bank wallet under parallel execution.
    /// (QA-V39-001.)
    pub swept_identity_credits: Credits,
}

impl SweepReport {
    /// Did any sub-sweep attempt a broadcast that the SDK / chain
    /// rejected? Used to decide whether the registry entry should
    /// be removed (clean) or transitioned to `Failed` (retry next
    /// run).
    pub fn has_failures(&self) -> bool {
        !self.broadcast_failures.is_empty()
    }
}

/// Outcome buckets for the post-sweep summary log on
/// [`sweep_orphans`]. Distinguishes "successfully drained" from
/// "skipped, nothing to do" from "tried and failed" — operators
/// reading the log no longer have to assume `count = N` means N
/// wallets actually landed funds back at the bank.
#[derive(Debug, Default)]
struct OrphanSweepSummary {
    swept_with_broadcast: u32,
    skipped_no_funds: u32,
    failed_retained: u32,
    /// Σ of [`SweepReport::dust_abandoned`] across all swept entries.
    /// Reported in the summary so operators see how much was left as
    /// sub-fee residual — the only path through which credits are
    /// silently dropped from the registry under the accept-dust
    /// policy. (V27-004)
    dust_abandoned_total: Credits,
}

/// Sweep wallets left over from prior (likely panicked) runs.
/// For each registry entry: reconstruct the wallet, sync, drain to
/// the bank if above [`min_input_amount`], then drop the entry IFF
/// every sub-sweep that attempted a broadcast succeeded. Any
/// broadcast failure flips the entry to [`EntryStatus::Failed`] and
/// retains it for next-run retry — the loop never aborts. (QA-V26-006)
pub async fn sweep_orphans(
    manager: &Arc<PlatformWalletManager<NoPlatformPersistence>>,
    bank: &BankWallet,
    bank_identity: &BankIdentity,
    registry: &PersistentTestWalletRegistry,
    network: Network,
) -> FrameworkResult<usize> {
    let orphans = registry.list_orphans();
    if orphans.is_empty() {
        return Ok(0);
    }
    tracing::info!(
        count = orphans.len(),
        "sweeping orphan test wallets from prior runs"
    );

    let mut summary = OrphanSweepSummary::default();
    for (hash, entry) in orphans {
        match sweep_one(manager, bank, bank_identity, &hash, &entry, network).await {
            Ok(report) if !report.has_failures() => {
                if report.had_funds_to_recover {
                    summary.swept_with_broadcast += 1;
                } else {
                    summary.skipped_no_funds += 1;
                }
                summary.dust_abandoned_total = summary
                    .dust_abandoned_total
                    .saturating_add(report.dust_abandoned);
                if let Err(err) = registry.remove(&hash) {
                    tracing::warn!(
                        wallet_id = %hex::encode(hash),
                        error = %err,
                        "swept funds but failed to drop registry entry"
                    );
                }
            }
            Ok(report) => {
                tracing::error!(
                    wallet_id = %hex::encode(hash),
                    failure_count = report.broadcast_failures.len(),
                    failures = ?report.broadcast_failures,
                    "orphan sweep had broadcast failures; flipping registry entry to \
                     Failed for next-run retry — funds remain stranded on this seed"
                );
                if let Err(err) = registry.set_status(&hash, EntryStatus::Failed) {
                    tracing::warn!(
                        wallet_id = %hex::encode(hash),
                        error = %err,
                        "failed to set registry status to Failed"
                    );
                }
                summary.failed_retained += 1;
            }
            Err(err) => {
                tracing::error!(
                    wallet_id = %hex::encode(hash),
                    error = %err,
                    "orphan sweep aborted with hard error; entry retained as Failed \
                     for next-run retry"
                );
                let _ = registry.set_status(&hash, EntryStatus::Failed);
                summary.failed_retained += 1;
            }
        }
    }
    tracing::info!(
        target: "platform_wallet::e2e::cleanup",
        swept_with_broadcast = summary.swept_with_broadcast,
        skipped_no_funds = summary.skipped_no_funds,
        failed_retained = summary.failed_retained,
        dust_abandoned_total = summary.dust_abandoned_total,
        "orphan sweep summary"
    );
    Ok(summary.swept_with_broadcast as usize)
}

/// Build a Platform-payment [`SimpleSigner`] keyed for every
/// synced/generated address index in account `DEFAULT_ACCOUNT_INDEX_PUB`
/// plus one `DIP17_GAP_LIMIT` forward window — the sweep-path mirror of
/// `BankWallet::derive_pool_signer` (#557).
///
/// `wallet` must have completed `sync_balances()` so the managed
/// account's `addresses` map reflects the on-chain funded pool. The
/// sweep transfer uses `InputSelection::Explicit` + `ReduceOutput(0)`
/// (no `auto_select_inputs`, no change branch), so the signing key set
/// is exactly the synced funded inputs; the margin covers pool
/// addresses generated but not yet balance-synced. Bounded — no
/// run-time index advancement. Replaces the static `0..DIP17_GAP_LIMIT`
/// `make_platform_signer` window that left drifted-index sweep inputs
/// unsignable (#556/#559). No funded pool → plain gap-window fallback.
async fn derive_sweep_pool_signer(
    wallet: &Arc<PlatformWallet>,
    seed_bytes: &[u8; 64],
    network: Network,
) -> FrameworkResult<SimpleSigner> {
    let highest_index = wallet
        .platform()
        .platform_payment_account_max_derived_index(DEFAULT_ACCOUNT_INDEX_PUB)
        .await
        .map_err(|err| {
            FrameworkError::Cleanup(format!("sweep pool signer: max derived index: {err}"))
        })?;

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
    .map_err(|err| FrameworkError::Wallet(format!("sweep pool signer: {err}")))
}

async fn sweep_one(
    manager: &Arc<PlatformWalletManager<NoPlatformPersistence>>,
    bank: &BankWallet,
    bank_identity: &BankIdentity,
    hash: &WalletSeedHash,
    entry: &RegistryEntry,
    network: Network,
) -> FrameworkResult<SweepReport> {
    let seed_bytes: [u8; 64] = parse_seed_hex(&entry.seed_hex)?;
    // Create the wallet in the manager. When the SPV runtime restored
    // its persistent state across a process restart it may have already
    // re-registered this wallet, producing `WalletAlreadyExists`. That
    // is fine — retrieve the existing handle and continue sweeping.
    // Returning an error here would leave the orphan unswept and its
    // registry entry as `Failed`, causing the next startup to see the
    // same error in an infinite retry loop. (QA-T11 idempotent-sweep fix)
    let wallet = match manager
        .create_wallet_from_seed_bytes(
            network,
            &seed_bytes,
            WalletAccountCreationOptions::Default,
            None,
        )
        .await
    {
        Ok(w) => w,
        Err(PlatformWalletError::WalletAlreadyExists(_)) => {
            tracing::debug!(
                target: "platform_wallet::e2e::cleanup",
                wallet_id = %hex::encode(hash),
                "orphan sweep: wallet already registered in manager \
                 (SPV persistence across restart); retrieving existing handle"
            );
            manager.get_wallet(hash).await.ok_or_else(|| {
                FrameworkError::Cleanup(format!(
                    "wallet {} reported WalletAlreadyExists but get_wallet \
                     returned None — manager state inconsistent",
                    hex::encode(hash)
                ))
            })?
        }
        Err(err) => return Err(wallet_err(err)),
    };
    if wallet.wallet_id() != *hash {
        return Err(FrameworkError::Cleanup(format!(
            "registry hash mismatch for sweep: expected {} got {}",
            hex::encode(hash),
            hex::encode(wallet.wallet_id())
        )));
    }
    wallet.platform().initialize().await;
    wallet
        .platform()
        .sync_balances(None)
        .await
        .map_err(wallet_err)?;
    let signer = derive_sweep_pool_signer(&wallet, &seed_bytes, network).await?;

    let platform_version = PlatformVersion::latest();
    let dust_gate = min_input_amount(platform_version);
    let total = wallet.platform().total_credits().await;
    let mut report = SweepReport::default();
    if total >= dust_gate {
        sweep_platform_addresses(
            &wallet,
            &signer,
            bank.primary_receive_address(),
            &mut report,
        )
        .await?;
    } else if total > 0 {
        // Accept-dust policy (V27-004): residual is below
        // `min_input_amount`, so no transition we could build would
        // satisfy the protocol's per-input floor. Tracking the
        // abandoned amount on the report lets the summary log
        // surface the leak; the registry entry is dropped by the
        // caller (`sweep_orphans` / `teardown_one`) on the clean
        // branch.
        tracing::info!(
            target: "platform_wallet::e2e::cleanup",
            wallet_id = %hex::encode(hash),
            dust = total,
            min_input = dust_gate,
            "orphan platform residual below sweep-fee minimum; abandoning dust"
        );
        report.dust_abandoned = report.dust_abandoned.saturating_add(total);
        super::funding_ledger::record_dust_abandoned(total);
    } else {
        tracing::debug!(
            wallet_id = %hex::encode(hash),
            total,
            min_input = dust_gate,
            "orphan platform total is zero; skipping"
        );
    }
    sweep_identities_with_seed(
        &wallet,
        &seed_bytes,
        network,
        bank,
        bank_identity,
        &mut report,
    )
    .await?;
    sweep_core_addresses(&wallet, &seed_bytes, bank, &mut report).await?;
    sweep_unused_core_asset_locks(&wallet).await?;
    sweep_shielded(&wallet).await?;

    // Best-effort manager unregister so SPV stops tracking the
    // wallet's addresses on subsequent passes.
    if let Err(err) = manager.remove_wallet(hash).await {
        tracing::warn!(
            target: "platform_wallet::e2e::cleanup",
            wallet_id = %hex::encode(hash),
            error = %err,
            "manager unregister failed after sweep; wallet remains tracked"
        );
    }
    Ok(report)
}

/// Per-test teardown: drain back to bank, drop the registry entry
/// IFF every sub-sweep that attempted a broadcast succeeded, then
/// unregister from the manager. Any broadcast failure flips the
/// registry entry to [`EntryStatus::Failed`] and retains it so the
/// next startup's [`sweep_orphans`] retries. (QA-V26-006 — prior to
/// this the registry was removed unconditionally on the happy-path
/// branch even when an inner best-effort sweep silently logged-and-
/// continued, leaking the funds permanently.)
pub async fn teardown_one(
    manager: &Arc<PlatformWalletManager<NoPlatformPersistence>>,
    bank: &BankWallet,
    bank_identity: &BankIdentity,
    registry: &PersistentTestWalletRegistry,
    test_wallet: &TestWallet,
) -> FrameworkResult<SweepReport> {
    test_wallet.sync_balances().await?;
    let sweep_signer = derive_sweep_pool_signer(
        test_wallet.platform_wallet(),
        &test_wallet.seed_bytes(),
        bank.network(),
    )
    .await?;
    let platform_version = PlatformVersion::latest();
    let dust_gate = min_input_amount(platform_version);
    // QA-004: hoist the address snapshot BEFORE the gate decision so both
    // the sum used for the gate check and the candidates passed into
    // `sweep_platform_addresses` come from the same `addresses_with_balances`
    // call. A concurrent test's `sync_balances` can inject foreign addresses
    // into the tracked pool between a gate-only `total_credits()` read and
    // the live `addresses_with_balances()` query inside the sweep, causing
    // the gate to pass on a wallet-owned sum while the sweep attempts (and
    // fails) to sign for a foreign address. Using one snapshot closes the
    // TOCTOU window entirely.
    let candidates: Vec<(PlatformAddress, Credits)> = test_wallet
        .platform_wallet()
        .platform()
        .addresses_with_balances()
        .await;
    let total: Credits = candidates.iter().map(|(_, v)| *v).sum();
    let mut report = SweepReport::default();
    if total >= dust_gate {
        sweep_platform_addresses_with_candidates(
            candidates,
            test_wallet.platform_wallet(),
            &sweep_signer,
            bank.primary_receive_address(),
            &mut report,
        )
        .await?;
    } else if total > 0 {
        // Accept-dust policy (V27-004): see the matching arm in
        // [`sweep_one`]. Residual under `min_input_amount` is
        // unrecoverable without a bank top-up, so we abandon it
        // and drop the registry entry on the clean branch below.
        tracing::info!(
            target: "platform_wallet::e2e::cleanup",
            wallet_id = %hex::encode(test_wallet.id()),
            dust = total,
            min_input = dust_gate,
            "test wallet residual below sweep-fee minimum; abandoning dust"
        );
        report.dust_abandoned = report.dust_abandoned.saturating_add(total);
        super::funding_ledger::record_dust_abandoned(total);
    } else {
        tracing::debug!(
            wallet_id = %hex::encode(test_wallet.id()),
            total,
            min_input = dust_gate,
            "test wallet total is zero; skipping platform sweep"
        );
    }
    sweep_identities_with_seed(
        test_wallet.platform_wallet(),
        &test_wallet.seed_bytes(),
        bank.network(),
        bank,
        bank_identity,
        &mut report,
    )
    .await?;
    sweep_core_addresses(
        test_wallet.platform_wallet(),
        &test_wallet.seed_bytes(),
        bank,
        &mut report,
    )
    .await?;
    sweep_unused_core_asset_locks(test_wallet.platform_wallet()).await?;
    sweep_shielded(test_wallet.platform_wallet()).await?;

    if report.has_failures() {
        tracing::error!(
            target: "platform_wallet::e2e::cleanup",
            wallet_id = %hex::encode(test_wallet.id()),
            failure_count = report.broadcast_failures.len(),
            failures = ?report.broadcast_failures,
            "teardown had broadcast failures; flipping registry entry to Failed for \
             next-run sweep_orphans retry — funds remain stranded on this seed"
        );
        if let Err(err) = registry.set_status(&test_wallet.id(), EntryStatus::Failed) {
            tracing::warn!(
                target: "platform_wallet::e2e::cleanup",
                wallet_id = %hex::encode(test_wallet.id()),
                error = %err,
                "failed to set registry status to Failed after broadcast failure"
            );
        }
        // Best-effort manager unregister still happens — the wallet
        // is no longer useful in-process even if its on-chain state
        // is dirty. Return Ok so tests that already passed don't
        // retroactively fail because of a sweep race; the loud
        // `error!` above + the persisted `Failed` registry entry
        // surface the leak to the operator and to next-run sweep.
        if let Err(err) = manager.remove_wallet(&test_wallet.id()).await {
            tracing::warn!(
                target: "platform_wallet::e2e::cleanup",
                wallet_id = %hex::encode(test_wallet.id()),
                error = %err,
                "manager unregister failed after teardown-with-failures"
            );
        }
        return Ok(report);
    }

    // Drop the registry entry first so an unregister failure
    // doesn't leak it; the wallet has no balance left to recover.
    registry.remove(&test_wallet.id())?;
    if let Err(err) = manager.remove_wallet(&test_wallet.id()).await {
        tracing::warn!(
            target: "platform_wallet::e2e::cleanup",
            wallet_id = %hex::encode(test_wallet.id()),
            error = %err,
            "manager unregister failed after teardown; wallet remains tracked"
        );
    }
    Ok(report)
}

/// Parse the registry's hex-encoded 64-byte seed. Bad length /
/// non-hex surfaces as [`FrameworkError::Cleanup`] so the entry
/// is marked failed rather than panicking the sweep.
fn parse_seed_hex(hex_str: &str) -> FrameworkResult<[u8; 64]> {
    let bytes = hex::decode(hex_str)
        .map_err(|err| FrameworkError::Cleanup(format!("invalid seed hex: {err}")))?;
    let arr: [u8; 64] = bytes.try_into().map_err(|v: Vec<u8>| {
        FrameworkError::Cleanup(format!("seed hex length {} != 64", v.len()))
    })?;
    Ok(arr)
}

fn wallet_err(err: PlatformWalletError) -> FrameworkError {
    FrameworkError::Wallet(err.to_string())
}

/// Drain every recoverable platform address back to `bank_addr` in a
/// single transition. Fetches the address snapshot internally; prefer
/// [`sweep_platform_addresses_with_candidates`] when the caller has
/// already snapshotted `addresses_with_balances` (e.g. `teardown_one`
/// for the QA-004 TOCTOU fix).
///
/// Tests that distribute funds across multiple addresses (PA-004b
/// dust-boundary, PA-009 min-input) leave change on every spent
/// address; the sweep must walk the full balance map. Addresses
/// below `min_input_amount` are intentionally skipped — the protocol
/// rejects any transition that includes a sub-floor input, and
/// sweeping a dust address is impossible by definition.
async fn sweep_platform_addresses<S>(
    wallet: &Arc<PlatformWallet>,
    signer: &S,
    bank_addr: &PlatformAddress,
    report: &mut SweepReport,
) -> FrameworkResult<()>
where
    S: Signer<PlatformAddress> + Send + Sync,
{
    let candidates = wallet.platform().addresses_with_balances().await;
    sweep_platform_addresses_with_candidates(candidates, wallet, signer, bank_addr, report).await
}

/// Inner sweep implementation that operates on a pre-built candidates
/// snapshot. Called by [`sweep_platform_addresses`] (which builds the
/// snapshot itself) and by [`teardown_one`] (which hoists the snapshot
/// before the gate check to avoid the QA-004 TOCTOU window).
async fn sweep_platform_addresses_with_candidates<S>(
    candidates: Vec<(PlatformAddress, Credits)>,
    wallet: &Arc<PlatformWallet>,
    signer: &S,
    bank_addr: &PlatformAddress,
    report: &mut SweepReport,
) -> FrameworkResult<()>
where
    S: Signer<PlatformAddress> + Send + Sync,
{
    let platform_version = PlatformVersion::latest();
    let SweepPlan {
        inputs,
        skipped_dust,
        ..
    } = build_sweep_plan(&candidates, platform_version);

    if !skipped_dust.is_empty() {
        let stranded: Credits = skipped_dust.iter().map(|(_, v)| *v).sum();
        tracing::warn!(
            target: "platform_wallet::e2e::cleanup",
            wallet_id = %hex::encode(wallet.wallet_id()),
            stranded_count = skipped_dust.len(),
            stranded_total = stranded,
            min_input = min_input_amount(platform_version),
            "sweep skipping addresses below min_input_amount"
        );
    }

    if inputs.is_empty() {
        tracing::debug!(
            target: "platform_wallet::e2e::cleanup",
            wallet_id = %hex::encode(wallet.wallet_id()),
            "sweep_platform_addresses: no recoverable inputs; nothing to sweep"
        );
        return Ok(());
    }

    let total: Credits = inputs.values().sum();
    let estimated_fee =
        AddressFundsTransferTransition::estimate_min_fee(inputs.len(), 1, platform_version);
    if total <= estimated_fee {
        tracing::warn!(
            target: "platform_wallet::e2e::cleanup",
            wallet_id = %hex::encode(wallet.wallet_id()),
            total,
            estimated_fee,
            "sweep_platform_addresses: Σ recoverable ≤ estimated fee; skipping"
        );
        return Ok(());
    }

    let outputs: BTreeMap<PlatformAddress, Credits> =
        std::iter::once((*bank_addr, total)).collect();
    let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];

    tracing::debug!(
        target: "platform_wallet::e2e::cleanup",
        wallet_id = %hex::encode(wallet.wallet_id()),
        total,
        input_count = inputs.len(),
        "sweep_platform_addresses: ReduceOutput(0) sweep"
    );

    report.had_funds_to_recover = true;
    match wallet
        .platform()
        .transfer(
            super::wallet_factory::DEFAULT_ACCOUNT_INDEX_PUB,
            InputSelection::Explicit(inputs),
            outputs.into_iter().collect(),
            fee_strategy,
            Some(platform_version),
            signer,
        )
        .await
    {
        Ok(_) => {
            super::funding_ledger::record_platform_recovered(total);
            report.broadcasts_succeeded = report.broadcasts_succeeded.saturating_add(1);
        }
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::cleanup",
                wallet_id = %hex::encode(wallet.wallet_id()),
                error = %err,
                "sweep_platform_addresses: broadcast failed (residual may be below sweep fee); \
                 retaining registry entry for sweep_orphans retry"
            );
            report.broadcast_failures.push(format!(
                "platform[{}]: {}",
                hex::encode(wallet.wallet_id()),
                err
            ));
        }
    }
    Ok(())
}

/// Result of partitioning the wallet's per-address balances into a
/// recoverable input set and the dust set that falls below the
/// per-input protocol floor. Output by [`build_sweep_plan`].
#[derive(Debug, Default, PartialEq, Eq)]
struct SweepPlan {
    inputs: BTreeMap<PlatformAddress, Credits>,
    skipped_dust: Vec<(PlatformAddress, Credits)>,
}

/// Pure helper: split per-address balances into sweep inputs (balance
/// ≥ `min_input_amount`) and the dust set that would be rejected as
/// a sub-floor input. Empty / zero balances are dropped silently.
fn build_sweep_plan(
    candidates: &[(PlatformAddress, Credits)],
    platform_version: &PlatformVersion,
) -> SweepPlan {
    let floor = min_input_amount(platform_version);
    let mut inputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
    let mut skipped_dust: Vec<(PlatformAddress, Credits)> = Vec::new();
    for (addr, balance) in candidates {
        if *balance == 0 {
            continue;
        }
        if *balance >= floor {
            inputs.insert(*addr, *balance);
        } else {
            skipped_dust.push((*addr, *balance));
        }
    }
    SweepPlan {
        inputs,
        skipped_dust,
    }
}

/// Drain identity credit balances back to the bank's Platform address
/// by broadcasting a `transfer_credits_to_addresses` state transition
/// for each non-empty identity owned by `wallet`.
///
/// Operates in two phases:
///
/// 1. Walk DIP-9 identity indices `0..IDENTITY_DISCOVERY_GAP` calling
///    `load_identity_by_index` so the wallet's `IdentityManager` is
///    populated with every identity reachable from `seed_bytes`. This
///    matters for the orphan-recovery path where the
///    just-reconstructed wallet has an empty manager — without
///    discovery the sweep would observe nothing.
/// 2. Iterate every identity in the manager whose `wallet_id` matches
///    `wallet.wallet_id()` and whose balance is at least
///    [`IDENTITY_SWEEP_FLOOR`]. For each, build a
///    [`SeedBackedIdentitySigner`] at that DIP-9 slot and issue a
///    `transfer_credits_to_addresses_with_external_signer(..,
///    outputs = {bank_addr: amount}, ..)`. The bank's Platform address
///    is the single Platform-side funding pool — see
///    [`super::bank_rebalance`] for the design contract.
///
/// The sweep skips the bank identity itself — a wallet that happens to
/// own the bank identity would otherwise self-transfer back into the
/// same pool we just drained. `bank_identity` is retained as a parameter
/// for that skip + log context; the destination is the bank's
/// Platform address ([`BankWallet::primary_receive_address`]), not the
/// bank identity.
/// Skips identities whose balance is below
/// [`IDENTITY_SWEEP_FLOOR`] — the network-level transfer fee is
/// non-negligible, so attempting to drain dust just burns more
/// credits than it recovers.
///
/// Best-effort: per-identity failures are logged and the loop
/// continues. The caller treats `Ok(())` as "we tried"; the next-run
/// orphan sweep will retry whatever stayed behind.
async fn sweep_identities_with_seed(
    wallet: &Arc<PlatformWallet>,
    seed_bytes: &[u8; 64],
    network: Network,
    bank: &BankWallet,
    bank_identity: &BankIdentity,
    report: &mut SweepReport,
) -> FrameworkResult<()> {
    // Registration downgrades the wallet to external-signable (no in-process
    // key), so the resident derive fails with "External signable wallet has no
    // private key". Derive the master xprv from the seed and probe via master.
    let master = ExtendedPrivKey::new_master(network, seed_bytes)
        .map_err(|err| FrameworkError::Cleanup(format!("identity sweep: master derive: {err}")))?;

    // Phase 1 — discovery walk.
    for identity_index in 0..IDENTITY_DISCOVERY_GAP {
        match wallet
            .identity()
            .load_identity_by_index_from_master(identity_index, &master)
            .await
        {
            Ok(Some(_)) => {
                tracing::debug!(
                    target: "platform_wallet::e2e::cleanup",
                    wallet_id = %hex::encode(wallet.wallet_id()),
                    identity_index,
                    "identity sweep: discovered identity at DIP-9 index"
                );
            }
            Ok(None) => {}
            Err(err) => tracing::debug!(
                target: "platform_wallet::e2e::cleanup",
                wallet_id = %hex::encode(wallet.wallet_id()),
                identity_index,
                error = %err,
                "identity sweep: discovery probe failed; continuing"
            ),
        }
    }

    // Phase 2 — collect (identity_id, cached_balance, registration_index)
    // tuples under a short read lock so we don't hold the wallet
    // manager lock across SDK round-trips. The cached balance is kept
    // only for diagnostic logging — the authoritative value used for
    // the floor check and amount computation is refetched from chain
    // below (the cache reflects the last seen balance, typically
    // post-funding / post-registration, and goes stale once the test
    // body runs state transitions like `data_contract_create` or token
    // ops; using it leads to over-amount sweep transfers that the
    // chain rejects with `IdentityInsufficientBalance`).
    let wallet_id = wallet.wallet_id();
    let candidates: Vec<(Identifier, Credits, u32)> = {
        let state = wallet.state().await;
        let mut out = Vec::new();
        if let Some(by_index) = state.identity_manager.wallet_identities.get(&wallet_id) {
            for (idx, managed) in by_index.iter() {
                use dpp::identity::accessors::IdentityGettersV0;
                let id = managed.identity.id();
                let balance = managed.identity.balance();
                if id == bank_identity.id {
                    continue;
                }
                out.push((id, balance, *idx));
            }
        }
        out
    };

    let sdk = wallet.sdk();
    for (identity_id, cached_balance, identity_index) in candidates {
        // Refresh the balance from chain. Lightweight balance-only
        // query — full `Identity::fetch` would also work but is
        // heavier and we only need the credits value.
        let balance: Credits = match IdentityBalance::fetch(sdk, identity_id).await {
            Ok(Some(b)) => b,
            Ok(None) => {
                tracing::warn!(
                    target: "platform_wallet::e2e::cleanup",
                    wallet_id = %hex::encode(wallet_id),
                    %identity_id,
                    identity_index,
                    cached_balance,
                    "identity sweep: chain reports identity absent; skipping"
                );
                continue;
            }
            Err(err) => {
                tracing::warn!(
                    target: "platform_wallet::e2e::cleanup",
                    wallet_id = %hex::encode(wallet_id),
                    %identity_id,
                    identity_index,
                    cached_balance,
                    error = %err,
                    "identity sweep: balance refresh failed; skipping identity"
                );
                continue;
            }
        };

        // Surface material divergence between the local cache and the
        // chain so future investigations of "where did the credits
        // go?" have a breadcrumb.
        let delta = cached_balance.abs_diff(balance);
        if delta > IDENTITY_BALANCE_REFRESH_LOG_THRESHOLD {
            tracing::info!(
                target: "platform_wallet::e2e::cleanup",
                wallet_id = %hex::encode(wallet_id),
                %identity_id,
                identity_index,
                cached_balance,
                chain_balance = balance,
                delta,
                "identity sweep: cached balance diverged from chain; using chain value"
            );
        }

        if balance < IDENTITY_SWEEP_FLOOR {
            tracing::debug!(
                target: "platform_wallet::e2e::cleanup",
                wallet_id = %hex::encode(wallet_id),
                %identity_id,
                identity_index,
                balance,
                floor = IDENTITY_SWEEP_FLOOR,
                "identity sweep: balance below floor; skipping"
            );
            continue;
        }

        let signer = match SeedBackedIdentitySigner::new(seed_bytes, network, identity_index) {
            Ok(s) => s,
            Err(err) => {
                tracing::warn!(
                    target: "platform_wallet::e2e::cleanup",
                    wallet_id = %hex::encode(wallet_id),
                    %identity_id,
                    identity_index,
                    error = %err,
                    "identity sweep: signer build failed; skipping identity"
                );
                continue;
            }
        };

        // Reserve a credit headroom for the CreditTransfer fee. The
        // exact fee is protocol-version-dependent; subtract the floor
        // (~30M, sized well above empirical fee on testnet) so the
        // transition has room to land without
        // "InsufficientIdentityBalance".
        let amount = balance.saturating_sub(IDENTITY_SWEEP_FEE_RESERVE);
        if amount == 0 {
            continue;
        }

        let outputs: BTreeMap<PlatformAddress, Credits> =
            std::iter::once((*bank.primary_receive_address(), amount)).collect();

        report.had_funds_to_recover = true;
        match wallet
            .identity()
            .transfer_credits_to_addresses_with_external_signer(
                &identity_id,
                outputs,
                &signer,
                None,
            )
            .await
        {
            Ok(_new_balance) => {
                tracing::info!(
                    target: "platform_wallet::e2e::cleanup",
                    wallet_id = %hex::encode(wallet_id),
                    %identity_id,
                    identity_index,
                    amount,
                    bank_identity_id = %bank_identity.id,
                    "identity sweep: drained credits to bank Platform address"
                );
                report.broadcasts_succeeded = report.broadcasts_succeeded.saturating_add(1);
                report.swept_identity_credits =
                    report.swept_identity_credits.saturating_add(amount);
                super::funding_ledger::record_identity_recovered(amount);
            }
            Err(err) => {
                tracing::warn!(
                    target: "platform_wallet::e2e::cleanup",
                    wallet_id = %hex::encode(wallet_id),
                    %identity_id,
                    identity_index,
                    amount,
                    error = %err,
                    "identity sweep: transfer_to_addresses failed; entry retained"
                );
                report.broadcast_failures.push(format!(
                    "identity[{} idx={}]: {}",
                    identity_id, identity_index, err
                ));
            }
        }
    }
    Ok(())
}

/// Upper bound (exclusive) on DIP-9 identity indices probed during
/// orphan recovery. Conservative — DIP-17's gap-limit is 20 for
/// addresses; identities are far rarer per wallet, so 8 covers
/// every realistic test pattern with room to spare while keeping
/// the discovery cost bounded.
const IDENTITY_DISCOVERY_GAP: u32 = 8;

/// Below this balance the sweep refuses to broadcast a
/// `transfer_credits_to_addresses` transition — protocol-level
/// transfer fees would consume most of the would-be transferred
/// amount. Calibrated against observed testnet realized fees (~100M
/// for a single-output transfer) with headroom; the DPP
/// `state_transition_min_fees` schedule covers only base fees and
/// excludes dynamic per-output storage costs (proof tree updates,
/// signature verification) that dominate on testnet, so a
/// chain-schedule-derived floor would let broadcasts through at fee
/// levels the chain rejects with `IdentityInsufficientBalance`.
/// Identities below this floor are abandoned for the duration of the
/// run; future sweeps may pick them up once natural chain activity
/// nudges them above the floor.
pub const IDENTITY_SWEEP_FLOOR: Credits = 50_000_000;

/// Headroom reserved for the on-chain fee when computing the
/// `CreditTransfer` amount. Protocol returns a typed
/// `InsufficientIdentityBalance` if the requested amount plus fee
/// exceeds the identity's balance, so the reserve must comfortably
/// exceed the chain-time fee. Calibrated against observed testnet
/// fees (~12-15M base + dynamic per-output costs).
pub const IDENTITY_SWEEP_FEE_RESERVE: Credits = 30_000_000;

/// `|cached - chain| > THRESHOLD` triggers an INFO-level breadcrumb
/// during the sweep so we can spot caches that have gone materially
/// stale (e.g. the TK-cohort silent leak — owner cache holds the
/// ~35B post-funding value while the chain holds ~14.5B after
/// `data_contract_create` + token ops). 100M is well above ordinary
/// fee-tick noise yet small enough to flag suspicious gaps.
const IDENTITY_BALANCE_REFRESH_LOG_THRESHOLD: Credits = 100_000_000;

/// Drain Core (Layer-1) UTXOs to the bank's primary BIP-44 receive
/// address. No-op when the wallet's confirmed Core balance is at or
/// below [`CORE_SWEEP_DUST_FLOOR`] — sweeping below the floor would
/// either burn the entire balance to the chain fee or fail the
/// builder's coin-selection step.
///
/// Best-effort: failures (no funded address, builder error, broadcast
/// rejection) are logged at WARN and surfaced as
/// [`FrameworkError::Wallet`]. The orphan-recovery loop in
/// [`sweep_orphans`] catches that and keeps the registry entry for a
/// later retry.
async fn sweep_core_addresses(
    wallet: &Arc<PlatformWallet>,
    seed: &[u8; 64],
    bank: &BankWallet,
    report: &mut SweepReport,
) -> FrameworkResult<()> {
    let confirmed = wallet.balance().confirmed();
    if confirmed <= CORE_SWEEP_DUST_FLOOR {
        tracing::debug!(
            target: "platform_wallet::e2e::cleanup",
            wallet_id = %hex::encode(wallet.wallet_id()),
            confirmed,
            floor = CORE_SWEEP_DUST_FLOOR,
            "core sweep: balance at or below dust floor; nothing to sweep"
        );
        return Ok(());
    }

    let amount = confirmed.saturating_sub(CORE_TX_FEE_RESERVE);
    if amount == 0 {
        tracing::debug!(
            target: "platform_wallet::e2e::cleanup",
            wallet_id = %hex::encode(wallet.wallet_id()),
            confirmed,
            "core sweep: balance covers fee reserve only; skipping"
        );
        return Ok(());
    }

    // Resolve the bank's primary Core receive address — same address
    // surfaced in the harness pre-flight log so swept funds land at
    // the operator-known location.
    let bank_core_addr = bank.primary_core_receive_address().await?;

    report.had_funds_to_recover = true;
    match core_send(wallet, seed, &bank_core_addr, amount).await {
        Ok(txid) => {
            tracing::info!(
                target: "platform_wallet::e2e::cleanup",
                wallet_id = %hex::encode(wallet.wallet_id()),
                %txid,
                amount,
                bank_core_addr = %bank_core_addr,
                "core sweep: drained Core duffs to bank"
            );
            report.broadcasts_succeeded = report.broadcasts_succeeded.saturating_add(1);
            super::funding_ledger::record_core_recovered(amount);
            Ok(())
        }
        // Drain-class errors fire when a prior sweep step (or a sibling
        // run already drained the address) leaves no UTXOs. That's a
        // benign "nothing to sweep" rather than a real failure — log
        // and return Ok WITHOUT recording a broadcast failure on the
        // report, otherwise we'd flip the registry to Failed for a
        // wallet that's actually clean.
        Err(err) if is_core_drain_class(&err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::cleanup",
                wallet_id = %hex::encode(wallet.wallet_id()),
                confirmed,
                amount,
                error = %err,
                "core sweep: address already drained or below coin-selection floor; \
                 best-effort skip — registry retains entry for next-run sweep_orphans \
                 retry if anything resurfaces"
            );
            Ok(())
        }
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::cleanup",
                wallet_id = %hex::encode(wallet.wallet_id()),
                amount,
                error = %err,
                "core sweep: broadcast failed with non-drain error; entry retained"
            );
            report.broadcast_failures.push(format!(
                "core[{}]: {}",
                hex::encode(wallet.wallet_id()),
                err
            ));
            Ok(())
        }
    }
}

/// Classify whether a Core-sweep failure is a benign "address already
/// drained" / "below coin-selection floor" condition that the
/// best-effort teardown should swallow rather than panic on.
///
/// Matches the substrings produced by the wallet's coin-selection /
/// fee-builder error paths when the Core UTXO set has been emptied by
/// a sibling cleanup step (the identity-credit sweep can move funds
/// off-chain into Platform credits, which an immediately-following
/// Core sweep then sees as "no UTXOs"). Substring matching is
/// deliberate: the underlying error type chain wraps these in
/// `Wallet("Transaction building failed: ...")` so we can't pattern
/// match a structured variant from outside the wallet crate.
fn is_core_drain_class(err: &FrameworkError) -> bool {
    let s = err.to_string();
    s.contains("No UTXOs available")
        || s.contains("Insufficient balance")
        || s.contains("Insufficient funds")
        || s.contains("Coin selection error")
}

/// Below this confirmed balance the Core sweep refuses to broadcast.
/// Sized to comfortably exceed the [`CORE_TX_FEE_RESERVE`] floor so
/// the post-fee residual is always non-trivial — sweeping a balance
/// of e.g. 1.5x the fee reserve burns most of the value as fee and
/// the recovered amount is meaningless.
const CORE_SWEEP_DUST_FLOOR: u64 = 100_000;

/// Consume unspent asset-lock outputs and refund their credits to the
/// bank. Noop until the asset-lock harness is wired up.
// TODO(rs-platform-wallet/e2e #asset-lock-sweep): walk the wallet's
// unused asset-lock proofs and either redeem-to-identity or burn back
// to bank-controlled core funds.
async fn sweep_unused_core_asset_locks(_wallet: &Arc<PlatformWallet>) -> FrameworkResult<()> {
    Ok(())
}

/// Drain the wallet's shielded note set to the bank's shielded address.
/// Noop until the shielded-prover harness is wired up.
// TODO(rs-platform-wallet/e2e #shielded-sweep): build a shield/unshield
// transition that empties the note set into a bank-controlled note.
async fn sweep_shielded(_wallet: &Arc<PlatformWallet>) -> FrameworkResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(byte: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([byte; 20])
    }

    /// Mixed: one above the floor, one dust. The above-floor address
    /// becomes the only input; the dust is reported as stranded.
    #[test]
    fn build_sweep_plan_drops_dust_keeps_recoverable() {
        let pv = PlatformVersion::latest();
        let floor = min_input_amount(pv);
        let big = addr(0x01);
        let dust = addr(0x02);
        let candidates = vec![(big, floor + 100), (dust, floor.saturating_sub(1))];
        let plan = build_sweep_plan(&candidates, pv);
        assert_eq!(plan.inputs.len(), 1);
        assert_eq!(plan.inputs.get(&big).copied(), Some(floor + 100));
        assert_eq!(plan.skipped_dust, vec![(dust, floor.saturating_sub(1))]);
    }

    /// Both addresses above the floor: each becomes an input. This
    /// pins the multi-input sweep path that the original addr_1-only
    /// behaviour would have skipped.
    #[test]
    fn build_sweep_plan_keeps_two_above_floor() {
        let pv = PlatformVersion::latest();
        let floor = min_input_amount(pv);
        let a = addr(0x01);
        let b = addr(0x02);
        let candidates = vec![(a, floor + 1_000), (b, floor + 2_000)];
        let plan = build_sweep_plan(&candidates, pv);
        assert_eq!(plan.inputs.len(), 2);
        assert_eq!(plan.skipped_dust.len(), 0);
        let total: Credits = plan.inputs.values().sum();
        assert_eq!(total, 2 * floor + 3_000);
    }

    /// All addresses below the floor: no inputs, all marked dust.
    /// `sweep_platform_addresses` will short-circuit with no broadcast.
    #[test]
    fn build_sweep_plan_all_dust_yields_no_inputs() {
        let pv = PlatformVersion::latest();
        let floor = min_input_amount(pv);
        // Floor is small enough that this can fail on PlatformVersions
        // where it's at zero — guard against that pathology.
        if floor == 0 {
            return;
        }
        let a = addr(0x01);
        let b = addr(0x02);
        let candidates = vec![(a, floor - 1), (b, floor / 2)];
        let plan = build_sweep_plan(&candidates, pv);
        assert!(plan.inputs.is_empty());
        assert_eq!(plan.skipped_dust.len(), 2);
    }

    /// Zero balances are silently dropped from both buckets; they
    /// represent addresses already swept on a previous pass.
    #[test]
    fn build_sweep_plan_drops_zero_balances() {
        let pv = PlatformVersion::latest();
        let candidates = vec![(addr(0x01), 0), (addr(0x02), 0)];
        let plan = build_sweep_plan(&candidates, pv);
        assert!(plan.inputs.is_empty());
        assert!(plan.skipped_dust.is_empty());
    }

    /// Pin the [`SweepReport`] contract — `has_failures` must reflect
    /// the `broadcast_failures` vec. Pre-QA-V26-006 the helpers
    /// returned `Ok(())` after logging a warn, so a broadcast failure
    /// looked identical to a clean sweep and the registry was purged
    /// regardless. The new contract is: any non-empty
    /// `broadcast_failures` ⇒ `has_failures()` ⇒ `sweep_orphans` /
    /// `teardown_one` retain the entry as Failed.
    #[test]
    fn sweep_report_has_failures_tracks_broadcast_failures() {
        let mut report = SweepReport::default();
        assert!(!report.has_failures(), "default report is clean");
        report
            .broadcast_failures
            .push("identity[X idx=0]: foo".into());
        assert!(
            report.has_failures(),
            "any broadcast failure flips the flag"
        );
    }

    /// Pin the "had_funds_to_recover vs broadcasts_succeeded"
    /// distinction. A wallet with funds whose every sweep step
    /// succeeded must report both flags; a wallet with funds whose
    /// every step failed must report `had_funds_to_recover=true`
    /// AND `has_failures()=true` AND `broadcasts_succeeded=0`. This
    /// is what `sweep_orphans` keys on to bucket
    /// `swept_with_broadcast` vs `failed_retained`.
    #[test]
    fn sweep_report_buckets_broadcasts_correctly() {
        let clean = SweepReport {
            had_funds_to_recover: true,
            broadcasts_succeeded: 2,
            ..Default::default()
        };
        assert!(!clean.has_failures());
        assert!(clean.had_funds_to_recover);

        let leaky = SweepReport {
            had_funds_to_recover: true,
            broadcast_failures: vec!["platform[X]: bar".into()],
            ..Default::default()
        };
        assert!(leaky.has_failures());
        assert_eq!(leaky.broadcasts_succeeded, 0);
        assert!(leaky.had_funds_to_recover);
    }

    /// Regression guard for #556/#559: a sweep/funding signer MUST NOT
    /// be the static `0..DIP17_GAP_LIMIT` window — a long-lived wallet
    /// whose pool drifted past index 20 has funded addresses the static
    /// signer holds no key for, so the sweep can't sign and funds bleed
    /// one-way (the bank drain). Fails if anyone reintroduces
    /// `make_platform_signer` (or any fixed-window signer) on a
    /// sweep/funding path. Non-funded, deterministic — no bank/network.
    #[test]
    fn static_gap_window_signer_cannot_sign_drifted_index() {
        let seed = [0x42u8; 64];
        let net = Network::Testnet;
        let drifted = DIP17_GAP_LIMIT + 5; // 25 — outside the static 0..20 window

        // The pkh the production DIP-17 derivation assigns to the
        // drifted index, taken from the same constructor the sweep now
        // uses (single-index signer → its only key is that pkh).
        let single = SimpleSigner::from_seed_for_platform_addresses(
            &seed,
            net,
            DEFAULT_ACCOUNT_INDEX_PUB,
            DEFAULT_KEY_CLASS_PUB,
            [drifted],
        )
        .expect("derive single drifted-index signer");
        let drifted_pkh: [u8; 20] = *single
            .address_private_keys
            .keys()
            .next()
            .expect("single-index signer has exactly one key");

        // The OLD static-window signer (the #556/#559 bug) has NO key
        // for the drifted index — this is precisely why the sweep
        // failed and funds bled.
        let static_signer = make_platform_signer(&seed, net).expect("build static-window signer");
        assert!(
            !static_signer
                .address_private_keys
                .contains_key(&drifted_pkh),
            "static 0..DIP17_GAP_LIMIT signer must NOT key a drifted (idx={drifted}) \
             address — if this passes, a fixed-window signer is back on a \
             sweep/funding path and the bank will bleed (#556/#559)"
        );

        // The pool signer the sweep now builds (idx 0..=drifted) DOES
        // key it — the S-1 fix recovers drifted-index funds.
        let pool_signer = SimpleSigner::from_seed_for_platform_addresses(
            &seed,
            net,
            DEFAULT_ACCOUNT_INDEX_PUB,
            DEFAULT_KEY_CLASS_PUB,
            0..=drifted,
        )
        .expect("build synced-pool signer");
        assert!(
            pool_signer.address_private_keys.contains_key(&drifted_pkh),
            "synced-pool signer (0..={drifted}) MUST key the drifted address — \
             the S-1 sweep fix depends on this"
        );
    }
}
