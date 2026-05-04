//! Cleanup paths: startup [`sweep_orphans`] and per-test
//! [`teardown_one`]. Both reconstruct the wallet from the registry
//! seed, sync, and drain every fund source back to the bank by
//! walking the per-source-type sweep helpers. Best-effort: errors
//! are logged and the registry retains the entry for the next run.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use dpp::address_funds::{AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::version::PlatformVersion;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::wallet::platform_addresses::InputSelection;
use platform_wallet::{PlatformWallet, PlatformWalletError, PlatformWalletManager};

use super::bank::BankWallet;
use super::registry::{EntryStatus, PersistentTestWalletRegistry, RegistryEntry, WalletSeedHash};
use super::wallet_factory::TestWallet;
use super::{make_platform_signer, FrameworkError, FrameworkResult};

/// Sweep gate: a wallet is only swept if its total balance can plausibly
/// satisfy the protocol's `min_input_amount`. Below that, no input can
/// pass `address_funds` validation and the broadcast would fail anyway.
/// Pulled from `PlatformVersion` rather than a hardcoded constant so we
/// stay in lock-step with whatever the active version dictates.
fn min_input_amount(version: &PlatformVersion) -> Credits {
    version.dpp.state_transitions.address_funds.min_input_amount
}

/// Default per-step timeout for cleanup polls.
pub const CLEANUP_STEP_TIMEOUT: Duration = Duration::from_secs(60);

/// Sweep wallets left over from prior (likely panicked) runs.
/// For each registry entry: reconstruct the wallet, sync, drain to
/// the bank if above [`min_input_amount`], then drop the entry.
/// Per-entry failures mark the entry [`EntryStatus::Failed`] for
/// next-run retry; the loop never aborts.
pub async fn sweep_orphans(
    manager: &Arc<PlatformWalletManager<NoPlatformPersistence>>,
    bank: &BankWallet,
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

    let mut swept = 0usize;
    for (hash, entry) in orphans {
        match sweep_one(manager, bank, &hash, &entry, network).await {
            Ok(()) => {
                if let Err(err) = registry.remove(&hash) {
                    tracing::warn!(
                        wallet_id = %hex::encode(hash),
                        error = %err,
                        "swept funds but failed to drop registry entry"
                    );
                }
                swept += 1;
            }
            Err(err) => {
                tracing::warn!(
                    wallet_id = %hex::encode(hash),
                    error = %err,
                    "sweep failed; entry retained for next-run retry"
                );
                let _ = registry.set_status(&hash, EntryStatus::Failed);
            }
        }
    }
    Ok(swept)
}

async fn sweep_one(
    manager: &Arc<PlatformWalletManager<NoPlatformPersistence>>,
    bank: &BankWallet,
    hash: &WalletSeedHash,
    entry: &RegistryEntry,
    network: Network,
) -> FrameworkResult<()> {
    let seed_bytes: [u8; 64] = parse_seed_hex(&entry.seed_hex)?;
    let wallet = manager
        .create_wallet_from_seed_bytes(network, seed_bytes, WalletAccountCreationOptions::Default)
        .await
        .map_err(wallet_err)?;
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
    let signer = make_platform_signer(&seed_bytes, network)?;

    let platform_version = PlatformVersion::latest();
    let dust_gate = min_input_amount(platform_version);
    let total = wallet.platform().total_credits().await;
    if total >= dust_gate {
        sweep_platform_addresses(&wallet, &signer, bank.primary_receive_address()).await?;
    } else {
        tracing::debug!(
            wallet_id = %hex::encode(hash),
            total,
            min_input = dust_gate,
            "orphan platform total below protocol min_input_amount; skipping"
        );
    }
    sweep_identities(&wallet).await?;
    sweep_core_addresses(&wallet).await?;
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
    Ok(())
}

/// Per-test teardown: drain back to bank, drop the registry entry,
/// and unregister from the manager. Best-effort — failures retain
/// the entry so the next startup's [`sweep_orphans`] retries.
pub async fn teardown_one(
    manager: &Arc<PlatformWalletManager<NoPlatformPersistence>>,
    bank: &BankWallet,
    registry: &PersistentTestWalletRegistry,
    test_wallet: &TestWallet,
) -> FrameworkResult<()> {
    test_wallet.sync_balances().await?;
    let platform_version = PlatformVersion::latest();
    let dust_gate = min_input_amount(platform_version);
    let total = test_wallet.total_credits().await;
    if total >= dust_gate {
        sweep_platform_addresses(
            test_wallet.platform_wallet(),
            test_wallet.address_signer(),
            bank.primary_receive_address(),
        )
        .await?;
    } else {
        tracing::debug!(
            wallet_id = %hex::encode(test_wallet.id()),
            total,
            min_input = dust_gate,
            "test wallet total below protocol min_input_amount; skipping platform sweep"
        );
    }
    sweep_identities(test_wallet.platform_wallet()).await?;
    sweep_core_addresses(test_wallet.platform_wallet()).await?;
    sweep_unused_core_asset_locks(test_wallet.platform_wallet()).await?;
    sweep_shielded(test_wallet.platform_wallet()).await?;

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
    Ok(())
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
/// single transition. Inputs map = balances ≥ `min_input_amount`,
/// output = the sum, fee comes out of the bank's incoming amount via
/// `ReduceOutput(0)`.
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
) -> FrameworkResult<()>
where
    S: Signer<PlatformAddress> + Send + Sync,
{
    let platform_version = PlatformVersion::latest();
    let candidates: Vec<(PlatformAddress, Credits)> =
        wallet.platform().addresses_with_balances().await;
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

    wallet
        .platform()
        .transfer(
            super::wallet_factory::DEFAULT_ACCOUNT_INDEX_PUB,
            InputSelection::Explicit(inputs),
            outputs,
            fee_strategy,
            Some(platform_version),
            signer,
        )
        .await
        .map_err(wallet_err)?;
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

/// Drain identity credit balances back to the bank identity. Noop until
/// the identity-transfer wiring lands.
// TODO(rs-platform-wallet/e2e #identity-sweep): implement once a
// Signer<IdentityPublicKey> is wired through `TestWallet` and the
// CreditTransfer transition is reachable from this harness.
async fn sweep_identities(_wallet: &Arc<PlatformWallet>) -> FrameworkResult<()> {
    Ok(())
}

/// Drain core (Layer 1) UTXOs to the bank's core address. Noop until
/// the SPV wallet runtime is back online in this harness.
// TODO(rs-platform-wallet/e2e #core-sweep): implement once the SPV
// runtime (Task #15) lets us sign and broadcast core transactions.
async fn sweep_core_addresses(_wallet: &Arc<PlatformWallet>) -> FrameworkResult<()> {
    Ok(())
}

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
}
