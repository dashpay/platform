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

/// Drain every owned platform address back to `bank_addr` in a single
/// transition. Inputs map = full balances, output = the sum, fee comes
/// out of the bank's incoming amount via `ReduceOutput(0)`. Sweep gate
/// is "address balance > 0".
async fn sweep_platform_addresses<S>(
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
        "sweep_platform_addresses: ReduceOutput(0) sweep"
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
