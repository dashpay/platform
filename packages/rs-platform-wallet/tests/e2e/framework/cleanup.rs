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
use dpp::prelude::Identifier;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::version::PlatformVersion;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::wallet::platform_addresses::InputSelection;
use platform_wallet::{PlatformWallet, PlatformWalletError, PlatformWalletManager};

use super::signer::SeedBackedIdentitySigner;

use super::bank::{core_send, BankWallet, CORE_TX_FEE_RESERVE};
use super::bank_identity::BankIdentity;
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

/// Sweep wallets left over from prior (likely panicked) runs.
/// For each registry entry: reconstruct the wallet, sync, drain to
/// the bank if above [`min_input_amount`], then drop the entry.
/// Per-entry failures mark the entry [`EntryStatus::Failed`] for
/// next-run retry; the loop never aborts.
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

    let mut swept = 0usize;
    for (hash, entry) in orphans {
        match sweep_one(manager, bank, bank_identity, &hash, &entry, network).await {
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
    bank_identity: &BankIdentity,
    hash: &WalletSeedHash,
    entry: &RegistryEntry,
    network: Network,
) -> FrameworkResult<()> {
    let seed_bytes: [u8; 64] = parse_seed_hex(&entry.seed_hex)?;
    let wallet = manager
        .create_wallet_from_seed_bytes(
            network,
            seed_bytes,
            WalletAccountCreationOptions::Default,
            None,
        )
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
    sweep_identities_with_seed(&wallet, &seed_bytes, network, bank_identity).await?;
    sweep_core_addresses(&wallet, bank).await?;
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
    bank_identity: &BankIdentity,
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
    sweep_identities_with_seed(
        test_wallet.platform_wallet(),
        &test_wallet.seed_bytes(),
        bank.network(),
        bank_identity,
    )
    .await?;
    sweep_core_addresses(test_wallet.platform_wallet(), bank).await?;
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

/// Drain identity credit balances back to the bank identity by
/// broadcasting a `CreditTransfer` state transition for each
/// non-empty identity owned by `wallet`.
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
///    `transfer_credits_with_external_signer(.., to = bank_identity.id, ..)`.
///
/// The sweep skips the bank identity itself — a wallet that happens to
/// own the bank identity would otherwise self-transfer (typed error).
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
    bank_identity: &BankIdentity,
) -> FrameworkResult<()> {
    // Phase 1 — discovery walk.
    for identity_index in 0..IDENTITY_DISCOVERY_GAP {
        match wallet
            .identity()
            .load_identity_by_index(identity_index)
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

    // Phase 2 — collect (identity_id, balance, registration_index)
    // tuples under a short read lock so we don't hold the wallet
    // manager lock across SDK round-trips.
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

    for (identity_id, balance, identity_index) in candidates {
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

        match wallet
            .identity()
            .transfer_credits_with_external_signer(
                &identity_id,
                &bank_identity.id,
                amount,
                &signer,
                None,
            )
            .await
        {
            Ok(()) => {
                tracing::info!(
                    target: "platform_wallet::e2e::cleanup",
                    wallet_id = %hex::encode(wallet_id),
                    %identity_id,
                    identity_index,
                    amount,
                    bank_identity_id = %bank_identity.id,
                    "identity sweep: drained credits to bank identity"
                );
            }
            Err(err) => {
                tracing::warn!(
                    target: "platform_wallet::e2e::cleanup",
                    wallet_id = %hex::encode(wallet_id),
                    %identity_id,
                    identity_index,
                    amount,
                    error = %err,
                    "identity sweep: CreditTransfer failed; entry retained"
                );
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
/// `CreditTransfer` — protocol-level transfer fees would consume
/// most of the would-be transferred amount. Sized roughly at 2x the
/// empirical CreditTransfer fee on testnet. Identities below this
/// floor effectively burn until a future ID-005 (identity →
/// addresses) sweep variant lands.
const IDENTITY_SWEEP_FLOOR: Credits = 50_000_000;

/// Headroom reserved for the on-chain fee when computing the
/// `CreditTransfer` amount. Protocol returns a typed
/// `InsufficientIdentityBalance` if the requested amount plus fee
/// exceeds the identity's balance, so the floor must comfortably
/// exceed the chain-time fee. Empirically ~12-15M on testnet.
const IDENTITY_SWEEP_FEE_RESERVE: Credits = 30_000_000;

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
    bank: &BankWallet,
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

    match core_send(wallet, &bank_core_addr, amount).await {
        Ok(txid) => {
            tracing::info!(
                target: "platform_wallet::e2e::cleanup",
                wallet_id = %hex::encode(wallet.wallet_id()),
                %txid,
                amount,
                bank_core_addr = %bank_core_addr,
                "core sweep: drained Core duffs to bank"
            );
        }
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::cleanup",
                wallet_id = %hex::encode(wallet.wallet_id()),
                amount,
                error = %err,
                "core sweep: broadcast failed; entry retained"
            );
            return Err(err);
        }
    }
    Ok(())
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
}
