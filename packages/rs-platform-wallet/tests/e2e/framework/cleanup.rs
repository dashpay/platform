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

/// Minimum sweep amount: skip wallets whose total balance is below
/// this. Acts as the dust gate so sweeps don't churn the chain for
/// negligible recoveries; the fee is absorbed from the output via
/// `ReduceOutput(0)` so no fee-headroom margin is needed here.
const SWEEP_DUST_THRESHOLD: Credits = 5_000_000;

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
    if total <= SWEEP_DUST_THRESHOLD {
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
    if total > SWEEP_DUST_THRESHOLD {
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

/// Drain every owned platform address back to `bank_addr` in a single
/// transition. Inputs map = full balances, output = the sum, fee comes
/// out of the bank's incoming amount via `ReduceOutput(0)`. Sweep gate
/// is "address balance > 0".
async fn drain_to_bank<S>(
    wallet: &Arc<PlatformWallet>,
    signer: &S,
    bank_addr: &PlatformAddress,
) -> FrameworkResult<()>
where
    S: Signer<PlatformAddress> + Send + Sync,
{
    let inputs: BTreeMap<PlatformAddress, Credits> = wallet
        .platform()
        .addresses_with_balances()
        .await
        .into_iter()
        .filter(|(_, b)| *b > 0)
        .collect();
    if inputs.is_empty() {
        return Ok(());
    }

    let total: Credits = inputs.values().sum();
    let outputs: BTreeMap<PlatformAddress, Credits> =
        std::iter::once((*bank_addr, total)).collect();
    let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];

    tracing::debug!(
        target: "platform_wallet::e2e::cleanup",
        wallet_id = %hex::encode(wallet.wallet_id()),
        total,
        input_count = inputs.len(),
        "drain_to_bank: ReduceOutput(0) sweep"
    );

    wallet
        .platform()
        .transfer(
            super::wallet_factory::DEFAULT_ACCOUNT_INDEX_PUB,
            InputSelection::Explicit(inputs),
            outputs,
            fee_strategy,
            Some(PlatformVersion::latest()),
            signer,
        )
        .await
        .map_err(wallet_err)?;
    Ok(())
}
