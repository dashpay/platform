//! Cleanup paths: startup [`sweep_orphans`] and per-test
//! [`teardown_one`]. Both reconstruct the wallet from the registry
//! seed, sync, and drain back to the bank. Best-effort: errors are
//! logged and the registry retains the entry for the next run.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use dpp::address_funds::{AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::version::PlatformVersion;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::wallet::platform_addresses::InputSelection;
use platform_wallet::{PlatformWallet, PlatformWalletError, PlatformWalletManager};

use super::bank::BankWallet;
use super::registry::{EntryStatus, PersistentTestWalletRegistry, RegistryEntry, WalletSeedHash};
use super::signer::SeedBackedPlatformAddressSigner;
use super::wallet_factory::TestWallet;
use super::{FrameworkError, FrameworkResult};

/// Skip sweeps where the recoverable amount is dwarfed by the fee.
/// At 5M dust + 30M fee, a successful sweep recovers ≥5M.
const SWEEP_DUST_THRESHOLD: Credits = 5_000_000;

/// Approximate fee for a 1- to 3-input → 1-output sweep transfer.
///
/// Used to (a) decide whether a sweep is worth attempting and
/// (b) reserve the fee margin at the [`AddressFundsFeeStrategyStep::DeductFromInput`]
/// target. Observed Dash testnet fees scale with input count
/// (~9.5M / ~21M / ~30M for 1 / 2 / 3 inputs); 30M covers up to
/// 3 inputs, comfortably above the typical 1-2 owned addresses
/// per test wallet.
///
/// TODO: compute dynamically against
/// `AddressFundsTransferTransition::estimate_min_fee` so this
/// constant doesn't drift if the protocol fee schedule changes.
const SWEEP_FEE_ESTIMATE: Credits = 30_000_000;

/// Default per-step timeout for cleanup polls.
pub const CLEANUP_STEP_TIMEOUT: Duration = Duration::from_secs(60);

/// Sweep wallets left over from prior (likely panicked) runs.
/// For each registry entry: reconstruct the wallet, sync, drain to
/// the bank if above [`SWEEP_DUST_THRESHOLD`], then drop the entry.
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
    let signer = SeedBackedPlatformAddressSigner::new(&seed_bytes, network)?;

    let total = wallet.platform().total_credits().await;
    if total <= SWEEP_DUST_THRESHOLD.saturating_add(SWEEP_FEE_ESTIMATE) {
        // Below worth-sweeping; let the caller drop the entry.
        tracing::debug!(
            wallet_id = %hex::encode(hash),
            total,
            "orphan total below sweep threshold; dropping registry entry"
        );
        // Best-effort manager unregister so SPV stops tracking the
        // wallet's addresses. Log failures rather than fail the sweep.
        if let Err(err) = manager.remove_wallet(hash).await {
            tracing::warn!(
                target: "platform_wallet::e2e::cleanup",
                wallet_id = %hex::encode(hash),
                error = %err,
                "manager unregister failed for dust-threshold sweep; wallet remains tracked"
            );
        }
        return Ok(());
    }
    drain_to_bank(&wallet, &signer, bank.primary_receive_address()).await?;

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
    let total = test_wallet.total_credits().await;
    if total > SWEEP_DUST_THRESHOLD.saturating_add(SWEEP_FEE_ESTIMATE) {
        drain_to_bank(
            test_wallet.platform_wallet(),
            test_wallet.address_signer(),
            bank.primary_receive_address(),
        )
        .await?;
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

/// Drain a test wallet's credits back to `bank_addr`.
///
/// Uses [`InputSelection::Explicit`] because the wallet's auto path
/// estimates fees against the protocol schedule (~5M for 1→1) while
/// the harness reserves [`SWEEP_FEE_ESTIMATE`] (30M) — passing the
/// exact `inputs`/`outputs` maps avoids the `Σ inputs == Σ outputs`
/// mismatch. The fee is paid by the fee-bearer's remaining balance
/// via [`AddressFundsFeeStrategyStep::DeductFromInput`].
async fn drain_to_bank<S>(
    wallet: &Arc<PlatformWallet>,
    signer: &S,
    bank_addr: &PlatformAddress,
) -> FrameworkResult<()>
where
    S: Signer<PlatformAddress> + Send + Sync,
{
    // BTreeMap iteration order matches the SDK's input indexing
    // for `DeductFromInput(i)`.
    let balances: BTreeMap<PlatformAddress, Credits> = wallet
        .platform()
        .addresses_with_balances()
        .await
        .into_iter()
        .filter(|(_, b)| *b > 0)
        .collect();
    if balances.is_empty() {
        return Ok(());
    }
    let total: Credits = balances.values().sum();
    if total <= SWEEP_DUST_THRESHOLD.saturating_add(SWEEP_FEE_ESTIMATE) {
        return Ok(());
    }

    // Largest-balance address is the safest fee-bearer — its
    // remaining balance must clear `SWEEP_FEE_ESTIMATE`.
    let (fee_bearer_addr, fee_bearer_balance) = balances
        .iter()
        .max_by_key(|(_, b)| **b)
        .map(|(a, b)| (*a, *b))
        .ok_or_else(|| FrameworkError::Cleanup("drain_to_bank: no candidates".into()))?;
    if fee_bearer_balance < SWEEP_FEE_ESTIMATE {
        return Err(FrameworkError::Cleanup(format!(
            "drain_to_bank: fee-bearer balance {} < SWEEP_FEE_ESTIMATE {} — \
             wallet has too many small balances to sweep in a single transition",
            fee_bearer_balance, SWEEP_FEE_ESTIMATE
        )));
    }

    // Every address contributes its full balance EXCEPT fee-bearer,
    // which contributes `balance - SWEEP_FEE_ESTIMATE` so the fee
    // margin stays on-chain for the protocol fee deduction.
    let mut inputs_map: BTreeMap<PlatformAddress, Credits> = balances.clone();
    inputs_map.insert(fee_bearer_addr, fee_bearer_balance - SWEEP_FEE_ESTIMATE);

    // Index in BTreeMap iteration order — what `DeductFromInput(N)`
    // resolves against.
    let fee_bearer_index = inputs_map
        .keys()
        .position(|k| *k == fee_bearer_addr)
        .map(|i| i as u16)
        .ok_or_else(|| {
            FrameworkError::Cleanup("drain_to_bank: fee-bearer not in inputs map".into())
        })?;

    let total_consumed: Credits = inputs_map.values().sum();
    let outputs: BTreeMap<PlatformAddress, Credits> =
        std::iter::once((*bank_addr, total_consumed)).collect();

    let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(
        fee_bearer_index,
    )];

    tracing::debug!(
        target: "platform_wallet::e2e::cleanup",
        wallet_id = %hex::encode(wallet.wallet_id()),
        total,
        total_consumed,
        fee_margin = SWEEP_FEE_ESTIMATE,
        fee_bearer_index,
        "drain_to_bank: explicit transfer"
    );

    wallet
        .platform()
        .transfer(
            super::wallet_factory::DEFAULT_ACCOUNT_INDEX_PUB,
            InputSelection::Explicit(inputs_map),
            outputs,
            fee_strategy,
            Some(PlatformVersion::latest()),
            signer,
        )
        .await
        .map_err(wallet_err)?;
    Ok(())
}
