//! Cleanup paths: startup-sweep + per-test teardown.
//!
//! Two flows share the same building blocks:
//!
//! - [`sweep_orphans`] runs once at framework init. It walks every
//!   entry in the persistent registry, reconstructs the wallet from
//!   `seed_hex`, syncs balances, and drains anything left on its
//!   addresses back to the bank. Failures are logged and the entry
//!   stays in the registry for the next run to retry.
//! - [`teardown_one`] is the happy-path cleanup invoked from
//!   [`super::wallet_factory::SetupGuard::teardown`] after a test
//!   finishes. It does the same drain-to-bank dance for one wallet
//!   and removes the registry entry on success.
//!
//! Both functions are best-effort: a single failure should not
//! cascade and abort an entire test session. Errors are surfaced
//! to the caller (which logs them) and the registry continues to
//! protect the funds.
//!
//! Wave 3a delivers both bodies. Wave 4 wires them into
//! `E2eContext::init` (sweep) and `SetupGuard::teardown` (per-test).

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::wallet::platform_addresses::InputSelection;
use platform_wallet::{PlatformWalletError, PlatformWalletManager};

use super::bank::BankWallet;
use super::registry::{EntryStatus, PersistentTestWalletRegistry, RegistryEntry, WalletSeedHash};
use super::signer::SeedBackedPlatformAddressSigner;
use super::wallet_factory::{default_fee_strategy, TestWallet};
use super::{FrameworkError, FrameworkResult};

/// Dust threshold below which a sweep is skipped — sweeping a few
/// credits costs more in fees than it recovers. Mirrors the
/// `dash-evo-tool` constant; conservative enough to leave a clear
/// margin above realistic transfer fees.
const SWEEP_DUST_THRESHOLD: Credits = 1_000_000;

/// Approximate fee for a 1-input / 1-output sweep transfer. The
/// real fee depends on platform-version + transition size; this
/// estimate is used only to decide whether a sweep is worth
/// attempting and which amount to send.
const SWEEP_FEE_ESTIMATE: Credits = 1_000_000;

/// Default per-step timeout for cleanup polls (sync, balance
/// observation). Matches the plan's 60s default for human-scale
/// sanity bounds.
pub const CLEANUP_STEP_TIMEOUT: Duration = Duration::from_secs(60);

/// Sweep wallets left over from previous (likely panicked) test
/// runs.
///
/// For each entry:
/// 1. Reconstruct the wallet from `seed_hex` via
///    `manager.create_wallet_from_seed_bytes`.
/// 2. Run a single BLAST sync to populate balances.
/// 3. If the total exceeds [`SWEEP_DUST_THRESHOLD`], drain to the
///    bank's primary receive address.
/// 4. Remove the entry from the registry on success; mark
///    [`EntryStatus::Failed`] otherwise so the next run retries
///    rather than re-using the same hash silently.
///
/// Returns the number of entries successfully swept; non-fatal
/// per-entry failures are logged via `tracing` but don't abort the
/// rest of the loop.
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
        // Below the worth-sweeping threshold; treat as success and
        // remove the registry entry (caller does the removal).
        tracing::debug!(
            wallet_id = %hex::encode(hash),
            total,
            "orphan total below sweep threshold; dropping registry entry"
        );
        // Best-effort manager unregister — leaks are harmless here
        // because the wallet has no balance and the manager is
        // recreated on next run anyway. Log failures so operators
        // can spot leaked manager state in CI logs (e.g. SPV still
        // tracking a wallet's addresses on subsequent passes).
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
    let amount = total.saturating_sub(SWEEP_FEE_ESTIMATE);
    let outputs: BTreeMap<PlatformAddress, Credits> =
        std::iter::once((*bank.primary_receive_address(), amount)).collect();

    wallet
        .platform()
        .transfer(
            super::wallet_factory::DEFAULT_ACCOUNT_INDEX_PUB,
            InputSelection::Auto,
            outputs,
            default_fee_strategy(),
            Some(PlatformVersion::latest()),
            &signer,
        )
        .await
        .map_err(wallet_err)?;

    // Best-effort manager unregister — keeps SPV from continuing
    // to track this wallet's addresses on subsequent passes. Log
    // failures explicitly so operators can spot leaked manager
    // state.
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

/// Per-test teardown: drain `test_wallet`'s remaining credits back
/// to the bank, remove its registry entry, and unregister it from
/// the manager so future syncs skip its addresses.
///
/// Best-effort: any failure is reported but the registry entry is
/// retained so the next process startup retries via
/// [`sweep_orphans`].
pub async fn teardown_one(
    manager: &Arc<PlatformWalletManager<NoPlatformPersistence>>,
    bank: &BankWallet,
    registry: &PersistentTestWalletRegistry,
    test_wallet: &TestWallet,
) -> FrameworkResult<()> {
    test_wallet.sync_balances().await?;
    let total = test_wallet.total_credits().await;
    if total > SWEEP_DUST_THRESHOLD.saturating_add(SWEEP_FEE_ESTIMATE) {
        let amount = total.saturating_sub(SWEEP_FEE_ESTIMATE);
        let outputs: BTreeMap<PlatformAddress, Credits> =
            std::iter::once((*bank.primary_receive_address(), amount)).collect();
        test_wallet.transfer(outputs).await?;
    }

    // Drop the entry first so a subsequent unregister failure
    // doesn't leak the registry entry — the wallet already has no
    // balance to recover. Log unregister failures so operators
    // can spot leaked manager state across long-lived test runs.
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

/// Parse the registry's hex-encoded seed (BIP-39 64-byte seed) into
/// raw bytes. A short / over-long string surfaces as
/// [`FrameworkError::Cleanup`] so the caller can mark the entry
/// failed without panicking.
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
