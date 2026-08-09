//! Rebuild tracked asset locks from on-chain history.
//!
//! `tracked_asset_locks` (and the host's persisted mirror — e.g. the
//! swift-sdk `PersistentAssetLock` store) is recorded live at
//! build/broadcast time, so it does not survive a wipe & recover: a
//! restored wallet's historical asset-lock funding transactions had no
//! tracked entry and hosts could not classify them (dashwallet-ios
//! rendered them "Internal Transfer — 0 DASH" until it grew a
//! client-side fallback).
//!
//! The classification signal *does* survive a restore: every asset-lock
//! credit output pays a one-time address derived from a purpose-specific
//! funding account (identity registration / top-up / invitation, the
//! platform-address top-up at `m/9'/coin'/5'/4'`, the shielded top-up at
//! `m/9'/coin'/5'/5'`), and the restore scan re-derives those pools and
//! files a [`TransactionRecord`] under the matching funding account
//! (key-wallet's transaction router checks all funding families for
//! `AssetLock`-type transactions). This module turns those records back
//! into [`TrackedAssetLock`] entries.
//!
//! Entries reconstructed from **chain-locked** records carry
//! [`AssetLockStatus::RecoveredFromChain`] with the chain proof
//! attached — Platform-side consumption is unknown after a restore, so
//! they must land in neither the pending nor the consumed bucket (see
//! the variant's doc). Non-final detections (mempool / unconfirmed
//! block) enter with the live pipeline's own pre-finality statuses and
//! are upgraded in place when a later record proves finality (see
//! [`recovered_status`] / [`enrich_from_record`]).
//!
//! The live entry point is the wallet-event adapter
//! ([`crate::changeset::core_bridge`]): every `TransactionDetected` /
//! `BlockProcessed` record flows through
//! [`reconstruct_tracked_asset_locks`], which inserts missing entries
//! and returns an [`AssetLockChangeSet`] the adapter persists in the
//! same store round-trip as the core rows. Insertion is
//! insert-if-absent: locks tracked live by the build pipeline (which
//! tracks *before* broadcast) always win over a reconstruction.

use std::collections::BTreeMap;
use std::sync::Arc;

use dashcore::blockdata::transaction::special_transaction::TransactionPayload;
use dashcore::OutPoint;
use key_wallet::account::AccountType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet::transaction_checking::TransactionContext;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use key_wallet_manager::{WalletId, WalletManager};
use tokio::sync::RwLock;

use crate::changeset::changeset::AssetLockChangeSet;
use crate::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// Map a record's owning account to the asset-lock funding family it
/// implies. `None` for every non-funding account (standard, CoinJoin,
/// provider keys, DashPay, …) — those records never yield a
/// reconstruction.
///
/// For [`AccountType::IdentityTopUp`] the account key *is* the
/// registration index, so the destination identity index is recovered
/// exactly. The singleton families (registration, invitation, unbound
/// top-up, address top-ups) don't encode their destination index in the
/// account — the credit-output address is just "next unused" there — so
/// the index is not recoverable from chain and reconstruction reports 0
/// (see [`reconstruct_candidates`]).
fn funding_family(account_type: &AccountType) -> Option<(AssetLockFundingType, u32)> {
    match account_type {
        AccountType::IdentityRegistration => Some((AssetLockFundingType::IdentityRegistration, 0)),
        AccountType::IdentityTopUp { registration_index } => {
            Some((AssetLockFundingType::IdentityTopUp, *registration_index))
        }
        AccountType::IdentityTopUpNotBoundToIdentity => {
            Some((AssetLockFundingType::IdentityTopUpNotBound, 0))
        }
        AccountType::IdentityInvitation => Some((AssetLockFundingType::IdentityInvitation, 0)),
        AccountType::AssetLockAddressTopUp => {
            Some((AssetLockFundingType::AssetLockAddressTopUp, 0))
        }
        AccountType::AssetLockShieldedAddressTopUp => {
            Some((AssetLockFundingType::AssetLockShieldedAddressTopUp, 0))
        }
        _ => None,
    }
}

/// Lock-free pre-filter: does this record even *look* like an
/// asset-lock funding record for one of this wallet's funding
/// accounts? Pure over the record so the adapter can skip the
/// wallet-manager write lock for the overwhelming majority of scan
/// traffic (plain payments, coinbase, provider txs, …).
pub(crate) fn is_reconstruction_candidate(record: &TransactionRecord) -> bool {
    funding_family(&record.account_type).is_some()
        && matches!(
            record.transaction.special_transaction_payload,
            Some(TransactionPayload::AssetLockPayloadType(_))
        )
}

/// Status + proof for a reconstructed lock, derived from the record's
/// on-chain context.
///
/// [`AssetLockStatus::RecoveredFromChain`] is reserved for records with
/// **proven Core finality** (`InChainLockedBlock`) — the variant's
/// invariant is "finality known, consumption unknown", and only those
/// records satisfy it. The chain proof is attached so an explicit
/// resume can consume the lock without another proof wait.
///
/// Non-final detections (a mempool sighting from a same-seed wallet on
/// another device, a not-yet-chain-locked block) get the live
/// pipeline's own statuses for exactly that state —
/// [`AssetLockStatus::Broadcast`] / [`AssetLockStatus::InstantSendLocked`],
/// mirroring `resolve_status_with_in_memory` in `sync::recovery` — so
/// `resume_asset_lock` keeps its defensive re-broadcast for a tx that
/// may still be evicted from mempools, and [`enrich_from_record`]
/// upgrades the entry when a later record proves finality.
fn recovered_status(
    record: &TransactionRecord,
    out_point: OutPoint,
) -> (AssetLockStatus, Option<dpp::prelude::AssetLockProof>) {
    match &record.context {
        TransactionContext::InChainLockedBlock(_) => match record.height() {
            Some(height) => (
                AssetLockStatus::RecoveredFromChain,
                Some(dpp::prelude::AssetLockProof::Chain(chain_proof(
                    height, out_point,
                ))),
            ),
            // Unreachable in practice (`InChainLockedBlock` always
            // carries a `BlockInfo` height), but the status must stay
            // bound to proof availability: a proof-less
            // `RecoveredFromChain` entry would have no repair path
            // (`enrich_from_record` only upgrades pre-finality
            // statuses). Enter as pre-finality instead so a later
            // heighted record can still enrich it.
            None => (AssetLockStatus::Broadcast, None),
        },
        TransactionContext::InstantSend(_) => (AssetLockStatus::InstantSendLocked, None),
        TransactionContext::Mempool | TransactionContext::InBlock(_) => {
            (AssetLockStatus::Broadcast, None)
        }
    }
}

fn chain_proof(
    core_chain_locked_height: u32,
    out_point: OutPoint,
) -> dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof {
    dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof {
        core_chain_locked_height,
        out_point,
    }
}

/// Find the fund-bearing account (BIP44 first, then CoinJoin — same
/// family order as `funding_tx_record` in `sync::proof`) that also
/// recorded `txid`, i.e. the account whose UTXOs funded the asset
/// lock. Falls back to 0 when no sibling record exists (the lookup
/// paths that consume `account_index` degrade to the persister
/// fallback on a miss, so a wrong-but-plausible index only costs a
/// round-trip).
fn funding_account_index(info: &PlatformWalletInfo, txid: &dashcore::Txid) -> u32 {
    let accounts = &info.core_wallet.accounts;
    accounts
        .standard_bip44_accounts
        .iter()
        .chain(accounts.coinjoin_accounts.iter())
        .find(|(_, account)| account.transactions().contains_key(txid))
        .map(|(index, _)| *index)
        .unwrap_or(0)
}

/// Classify one funding-account record into the tracked-asset-lock
/// entries it implies: one candidate per credit output that pays an
/// address of the record's funding account. Outpoints already present
/// in `info.tracked_asset_locks` are omitted (live-tracked locks win).
///
/// Pure over `info` — the caller owns locking and insertion.
fn reconstruct_candidates(
    info: &PlatformWalletInfo,
    record: &TransactionRecord,
) -> Vec<TrackedAssetLock> {
    let Some((funding_type, identity_index)) = funding_family(&record.account_type) else {
        return Vec::new();
    };
    let Some(TransactionPayload::AssetLockPayloadType(payload)) =
        &record.transaction.special_transaction_payload
    else {
        return Vec::new();
    };

    let accounts = &info.core_wallet.accounts;
    let funding_account = match funding_type {
        AssetLockFundingType::IdentityRegistration => accounts.identity_registration.as_ref(),
        AssetLockFundingType::IdentityTopUp => accounts.identity_topup.get(&identity_index),
        AssetLockFundingType::IdentityTopUpNotBound => accounts.identity_topup_not_bound.as_ref(),
        AssetLockFundingType::IdentityInvitation => accounts.identity_invitation.as_ref(),
        AssetLockFundingType::AssetLockAddressTopUp => accounts.asset_lock_address_topup.as_ref(),
        AssetLockFundingType::AssetLockShieldedAddressTopUp => {
            accounts.asset_lock_shielded_address_topup.as_ref()
        }
    };
    let Some(funding_account) = funding_account else {
        // The record names an account the managed collection doesn't
        // hold (raced a removal, or a load-path gap). Nothing to match
        // against; the next scan round re-emits the record.
        return Vec::new();
    };

    let account_index = funding_account_index(info, &record.txid);

    payload
        .credit_outputs
        .iter()
        .enumerate()
        .filter(|(_, credit_output)| {
            funding_account.contains_script_pub_key(&credit_output.script_pubkey)
        })
        .filter_map(|(vout, credit_output)| {
            // Asset-lock outpoints index into `credit_outputs`
            // (DIP-0027), not the transaction's regular outputs.
            let out_point = OutPoint::new(record.txid, vout as u32);
            if info.tracked_asset_locks.contains_key(&out_point) {
                return None;
            }
            let (status, proof) = recovered_status(record, out_point);
            Some(TrackedAssetLock {
                out_point,
                transaction: record.transaction.clone(),
                account_index,
                funding_type,
                identity_index,
                amount: credit_output.value,
                status,
                proof,
            })
        })
        .collect()
}

/// Upgrade an already-tracked, still-unproven entry to
/// [`AssetLockStatus::RecoveredFromChain`] when a record proves its
/// funding tx chain-locked.
///
/// Insert-if-absent protects live entries from being *replaced* by a
/// reconstruction — but it also meant a proof-less entry (a
/// reconstruction from a pre-finality detection, or a live `Broadcast`
/// row stranded by an app kill) could never receive the chain proof a
/// later finalized record carries. This closes that gap without
/// clobbering anything a live flow owns:
///
/// - only entries whose `proof` is `None` AND whose status is
///   [`Broadcast`](AssetLockStatus::Broadcast) /
///   [`InstantSendLocked`](AssetLockStatus::InstantSendLocked) are
///   touched. A lock a live flow is actively completing leaves that
///   window within seconds: `wait_for_proof` attaches the IS/CL proof
///   through `advance_asset_lock_status` (making `proof` non-`None`),
///   and consumption tombstones it. What *remains* proof-less at
///   Broadcast/IS-locked when on-chain finality arrives is, by
///   elimination, a lock nobody is completing — a restore-scan
///   reconstruction or a stranded row — so the truthful terminal is
///   `RecoveredFromChain` ("final on Core, Platform-side consumption
///   unknown"), NOT `ChainLocked`, which every consumer reads as "in
///   flight". (An earlier revision upgraded to `ChainLocked`; after a
///   restore that rendered every historical funding tx as a pending
///   transfer.) In the benign race where a live `wait_for_proof` is
///   still running, its own `advance_asset_lock_status` overwrites the
///   status unconditionally moments later, so the live pipeline still
///   wins;
/// - [`Built`](AssetLockStatus::Built) (owned by an in-flight build),
///   [`Consumed`](AssetLockStatus::Consumed) (terminal), and
///   proof-carrying entries are left untouched.
///
/// Every richer field (funding type, identity index, amount) is
/// preserved — only `status` and `proof` advance.
fn enrich_from_record(
    info: &mut PlatformWalletInfo,
    record: &TransactionRecord,
    cs: &mut AssetLockChangeSet,
) {
    let TransactionContext::InChainLockedBlock(_) = &record.context else {
        return;
    };
    let Some(height) = record.height() else {
        return;
    };
    let Some(TransactionPayload::AssetLockPayloadType(payload)) =
        &record.transaction.special_transaction_payload
    else {
        return;
    };
    for vout in 0..payload.credit_outputs.len() {
        let out_point = OutPoint::new(record.txid, vout as u32);
        let Some(entry) = info.tracked_asset_locks.get_mut(&out_point) else {
            continue;
        };
        let upgradable = entry.proof.is_none()
            && matches!(
                entry.status,
                AssetLockStatus::Broadcast | AssetLockStatus::InstantSendLocked
            );
        if !upgradable {
            continue;
        }
        tracing::info!(
            outpoint = %out_point,
            height,
            prior_status = ?entry.status,
            "attaching chain proof to tracked asset lock from finalized scan record"
        );
        entry.status = AssetLockStatus::RecoveredFromChain;
        entry.proof = Some(dpp::prelude::AssetLockProof::Chain(chain_proof(
            height, out_point,
        )));
        cs.asset_locks.insert(out_point, (&*entry).into());
    }
}

/// Rebuild missing tracked-asset-lock entries from scan records.
///
/// For each record that passes [`is_reconstruction_candidate`], match
/// its credit outputs against the owning funding account and insert an
/// entry for every outpoint not already tracked — status per
/// [`recovered_status`] (`RecoveredFromChain` + chain proof for
/// finalized records, the live pipeline's pre-finality statuses for
/// mempool/unconfirmed detections). Already-tracked outpoints are
/// never replaced, but a finalized record upgrades a still-unproven
/// entry in place (see [`enrich_from_record`]). Returns the changeset
/// describing the inserted/upgraded entries (empty when nothing
/// changed) for the caller to persist alongside whatever else it is
/// flushing.
///
/// Callers should pre-filter with [`is_reconstruction_candidate`] and
/// skip the call entirely when no record qualifies — this function
/// takes the wallet-manager **write** lock.
pub(crate) async fn reconstruct_tracked_asset_locks(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    records: &[&TransactionRecord],
) -> AssetLockChangeSet {
    let mut cs = AssetLockChangeSet::default();
    if records.is_empty() {
        return cs;
    }
    let mut wm = wallet_manager.write().await;
    let Some(info) = wm.get_wallet_info_mut(wallet_id) else {
        return cs;
    };
    for record in records {
        apply_record(info, record, &mut cs);
    }
    cs
}

/// One record's full reconstruction step: insert-if-absent, then let a
/// finalized record upgrade what's already tracked but still unproven
/// (the inserts carry their own proof already, so enrichment only ever
/// touches pre-existing entries). Callers own the wallet-manager write
/// lock — `reconstruct_candidates` reads `info.tracked_asset_locks`
/// under that same lock, so insert-if-absent holds without a re-check.
fn apply_record(
    info: &mut PlatformWalletInfo,
    record: &TransactionRecord,
    cs: &mut AssetLockChangeSet,
) {
    for lock in reconstruct_candidates(info, record) {
        tracing::info!(
            outpoint = %lock.out_point,
            funding_type = ?lock.funding_type,
            amount = lock.amount,
            status = ?lock.status,
            has_proof = lock.proof.is_some(),
            "reconstructed tracked asset lock from on-chain record"
        );
        cs.asset_locks.insert(lock.out_point, (&lock).into());
        info.tracked_asset_locks.insert(lock.out_point, lock);
    }
    enrich_from_record(info, record, cs);
}

/// [`ChainLockProcessed`](key_wallet_manager::events::WalletEvent::ChainLockProcessed)
/// sibling of [`enrich_from_record`]: upgrade the tracked entries whose
/// funding txs a chainlock just promoted to `InChainLockedBlock`.
///
/// Why this exists: during a restore scan the filter walk detects the
/// historical asset-lock funding txs *before* any chainlock is applied,
/// so their entries insert at the pre-finality
/// [`Broadcast`](AssetLockStatus::Broadcast) status. The promotion to
/// chain-locked happens later, in bulk, when the tip chainlock arrives
/// (`apply_chain_lock` promotes every `InBlock` record at height `<=`
/// the chainlock) — and that promotion surfaces ONLY as a
/// `ChainLockProcessed` event, whose records never re-flow through
/// `TransactionDetected`/`BlockProcessed`. Without this hook the
/// entries stayed pre-finality for the whole session (observed on a
/// restored testnet wallet 2026-08-09: all 9 reconstructed locks stuck
/// at `Broadcast` until an app restart re-emitted their records).
///
/// Deliberately record-free: under the default
/// `keep-finalized-transactions=OFF` feature the promotion **evicts**
/// the full records and the event retains only their txids (see
/// `ManagedCoreFundsAccount::apply_chain_lock`), so reading the record
/// back here would find nothing — an earlier record-based revision of
/// this hook silently no-opped for exactly that reason. Everything
/// needed lives elsewhere: the entries to upgrade are keyed by txid in
/// `tracked_asset_locks`, and the chainlock's own height is a valid
/// `ChainAssetLockProof` height for anything the chainlock buries (the
/// same fact the resume path's CL-from-metadata fallback relies on).
///
/// The same upgrade guard as [`enrich_from_record`] applies: only
/// proof-less entries at `Broadcast` / `InstantSendLocked` are touched
/// — nothing live is completing an entry that is still proof-less when
/// on-chain finality arrives, so `RecoveredFromChain` is the truthful
/// terminal.
///
/// The event keys promoted txids by owning account type;
/// funding-family filtering happens on those keys, so the common case
/// (a chainlock promoting plain payments, or promoting nothing) never
/// takes the wallet-manager write lock.
pub(crate) async fn enrich_tracked_asset_locks_from_chain_lock(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    chain_lock_height: u32,
    locked_transactions: &BTreeMap<AccountType, Vec<dashcore::Txid>>,
) -> AssetLockChangeSet {
    let mut cs = AssetLockChangeSet::default();
    let funding_txids: std::collections::BTreeSet<dashcore::Txid> = locked_transactions
        .iter()
        .filter(|(account_type, _)| funding_family(account_type).is_some())
        .flat_map(|(_, txids)| txids.iter().copied())
        .collect();
    if funding_txids.is_empty() {
        return cs;
    }
    let mut wm = wallet_manager.write().await;
    let Some(info) = wm.get_wallet_info_mut(wallet_id) else {
        return cs;
    };
    for (out_point, entry) in info.tracked_asset_locks.iter_mut() {
        if !funding_txids.contains(&out_point.txid) {
            continue;
        }
        let upgradable = entry.proof.is_none()
            && matches!(
                entry.status,
                AssetLockStatus::Broadcast | AssetLockStatus::InstantSendLocked
            );
        if !upgradable {
            continue;
        }
        tracing::info!(
            outpoint = %out_point,
            chain_lock_height,
            prior_status = ?entry.status,
            "attaching chain proof to tracked asset lock from chainlock promotion"
        );
        entry.status = AssetLockStatus::RecoveredFromChain;
        entry.proof = Some(dpp::prelude::AssetLockProof::Chain(chain_proof(
            chain_lock_height,
            *out_point,
        )));
        cs.asset_locks.insert(*out_point, (&*entry).into());
    }
    cs
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use dashcore::hashes::Hash;
    use dashcore::{BlockHash, Network, OutPoint, Transaction};
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::transaction_record::{
        TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::transaction_router::TransactionType;
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext};
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    use key_wallet_manager::WalletManager;
    use tokio::sync::{Notify, RwLock};

    use super::*;
    use crate::changeset::{Merge, PlatformWalletPersistence};
    use crate::test_support::{
        funded_wallet_manager, AlwaysRejectedBroadcaster, NoopTestPersister,
    };
    use crate::wallet::asset_lock::manager::AssetLockManager;
    use crate::wallet::persister::WalletPersister;

    /// Build a real asset-lock transaction through the production
    /// builder so its credit output pays a genuine address from the
    /// requested funding account's pool — exactly the shape a restore
    /// scan re-derives. Returns the manager Arc, wallet id, and tx.
    async fn wallet_with_built_asset_lock(
        funding_type: AssetLockFundingType,
        identity_index: u32,
    ) -> (
        Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        WalletId,
        Transaction,
    ) {
        let (wallet_manager, wallet_id, _generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .build()
                .expect("mock sdk"),
        );
        let manager = AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::new(NoopTestPersister) as Arc<dyn PlatformWalletPersistence>,
            ),
        );
        let (tx, _path) = manager
            .build_asset_lock_transaction(1_000_000, 0, funding_type, identity_index, &signer)
            .await
            .expect("build asset lock");
        (wallet_manager, wallet_id, tx)
    }

    fn chainlocked_context(height: u32) -> TransactionContext {
        TransactionContext::InChainLockedBlock(BlockInfo::new(
            height,
            BlockHash::from_slice(&[7u8; 32]).expect("block hash"),
            1_650_000_000,
        ))
    }

    fn record_for(
        tx: &Transaction,
        account_type: AccountType,
        context: TransactionContext,
    ) -> TransactionRecord {
        TransactionRecord::new(
            tx.clone(),
            account_type,
            context,
            TransactionType::AssetLock,
            TransactionDirection::Internal,
            vec![],
            vec![],
            0,
        )
    }

    /// The restore-scan shape end to end: a chain-locked funding-account
    /// record for a registration asset lock reconstructs a
    /// `RecoveredFromChain` entry with the credit-output amount, a
    /// `ChainAssetLockProof` at the record's height, and a changeset row
    /// for the persister.
    #[tokio::test]
    async fn registration_lock_reconstructs_from_chainlocked_record() {
        let (wallet_manager, wallet_id, tx) =
            wallet_with_built_asset_lock(AssetLockFundingType::IdentityRegistration, 4).await;
        let record = record_for(
            &tx,
            AccountType::IdentityRegistration,
            chainlocked_context(1234),
        );
        assert!(is_reconstruction_candidate(&record));

        let cs = reconstruct_tracked_asset_locks(&wallet_manager, &wallet_id, &[&record]).await;

        let out_point = OutPoint::new(tx.txid(), 0);
        let entry = cs.asset_locks.get(&out_point).expect("changeset entry");
        assert_eq!(entry.status, AssetLockStatus::RecoveredFromChain);
        assert_eq!(entry.amount_duffs, 1_000_000);
        assert_eq!(
            entry.funding_type,
            AssetLockFundingType::IdentityRegistration
        );
        // The destination identity index is NOT recoverable from chain
        // for singleton funding accounts (the build passed 4; the
        // credit-output address doesn't encode it) — reconstruction
        // reports 0. Only `IdentityTopUp` recovers the real index (from
        // the account key; covered below).
        assert_eq!(entry.identity_index, 0);
        match &entry.proof {
            Some(dpp::prelude::AssetLockProof::Chain(chain)) => {
                assert_eq!(chain.core_chain_locked_height, 1234);
                assert_eq!(chain.out_point, out_point);
            }
            other => panic!("expected a chain proof at the record height, got {other:?}"),
        }

        let wm = wallet_manager.read().await;
        let lock = wm
            .get_wallet_info(&wallet_id)
            .expect("wallet")
            .tracked_asset_locks
            .get(&out_point)
            .expect("in-memory tracked entry");
        assert_eq!(lock.status, AssetLockStatus::RecoveredFromChain);
    }

    /// A lock the build pipeline is already tracking must never be
    /// clobbered by a scan-record reconstruction (the live entry may
    /// carry a fresher status and an IS proof).
    #[tokio::test]
    async fn live_tracked_lock_is_not_overwritten() {
        let (wallet_manager, wallet_id, tx) =
            wallet_with_built_asset_lock(AssetLockFundingType::IdentityRegistration, 0).await;
        let out_point = OutPoint::new(tx.txid(), 0);
        {
            let mut wm = wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet");
            info.tracked_asset_locks.insert(
                out_point,
                TrackedAssetLock {
                    out_point,
                    transaction: tx.clone(),
                    account_index: 0,
                    funding_type: AssetLockFundingType::IdentityRegistration,
                    identity_index: 0,
                    amount: 1_000_000,
                    status: AssetLockStatus::Built,
                    proof: None,
                },
            );
        }

        let record = record_for(
            &tx,
            AccountType::IdentityRegistration,
            chainlocked_context(99),
        );
        let cs = reconstruct_tracked_asset_locks(&wallet_manager, &wallet_id, &[&record]).await;

        assert!(
            Merge::is_empty(&cs),
            "an already-tracked outpoint must produce no changeset"
        );
        let wm = wallet_manager.read().await;
        assert_eq!(
            wm.get_wallet_info(&wallet_id)
                .expect("wallet")
                .tracked_asset_locks
                .get(&out_point)
                .expect("live entry")
                .status,
            AssetLockStatus::Built,
            "the live-tracked status must survive the scan record"
        );
    }

    /// `IdentityTopUp` is the one family whose destination index IS
    /// recoverable — the account key is the registration index.
    #[tokio::test]
    async fn topup_reconstruction_recovers_registration_index() {
        const TOPUP_INDEX: u32 = 7;
        let (wallet_manager, wallet_id, tx) =
            wallet_with_built_asset_lock(AssetLockFundingType::IdentityTopUp, TOPUP_INDEX).await;
        let record = record_for(
            &tx,
            AccountType::IdentityTopUp {
                registration_index: TOPUP_INDEX,
            },
            chainlocked_context(50),
        );

        let cs = reconstruct_tracked_asset_locks(&wallet_manager, &wallet_id, &[&record]).await;

        let entry = cs
            .asset_locks
            .get(&OutPoint::new(tx.txid(), 0))
            .expect("changeset entry");
        assert_eq!(entry.funding_type, AssetLockFundingType::IdentityTopUp);
        assert_eq!(entry.identity_index, TOPUP_INDEX);
    }

    /// A record that hasn't reached chain finality (mempool detection,
    /// e.g. a same-seed wallet on another device broadcasting) still
    /// reconstructs — but with the live pipeline's own pre-finality
    /// status, NOT `RecoveredFromChain` (whose invariant is "finality
    /// known"). `Broadcast` keeps the resume path's defensive
    /// re-broadcast for a tx that may still be evicted from mempools;
    /// an IS-observed record likewise maps to `InstantSendLocked`.
    #[tokio::test]
    async fn unconfirmed_record_reconstructs_as_broadcast() {
        let (wallet_manager, wallet_id, tx) =
            wallet_with_built_asset_lock(AssetLockFundingType::AssetLockShieldedAddressTopUp, 0)
                .await;
        let record = record_for(
            &tx,
            AccountType::AssetLockShieldedAddressTopUp,
            TransactionContext::Mempool,
        );

        let cs = reconstruct_tracked_asset_locks(&wallet_manager, &wallet_id, &[&record]).await;

        let entry = cs
            .asset_locks
            .get(&OutPoint::new(tx.txid(), 0))
            .expect("changeset entry");
        assert_eq!(entry.status, AssetLockStatus::Broadcast);
        assert!(entry.proof.is_none(), "no finality context ⇒ no proof");
    }

    /// A finalized record must upgrade an already-tracked, still
    /// unproven entry in place (attach the chain proof, advance to
    /// `RecoveredFromChain` — nothing live is completing an entry that
    /// is still proof-less when finality arrives) — the
    /// insert-if-absent rule protects live entries from replacement but
    /// must not strand them proof-less forever.
    #[tokio::test]
    async fn finalized_record_enriches_unproven_tracked_entry() {
        let (wallet_manager, wallet_id, tx) =
            wallet_with_built_asset_lock(AssetLockFundingType::IdentityRegistration, 0).await;
        let out_point = OutPoint::new(tx.txid(), 0);

        // First sighting: mempool → tracked at Broadcast, no proof.
        let mempool_record = record_for(
            &tx,
            AccountType::IdentityRegistration,
            TransactionContext::Mempool,
        );
        let cs =
            reconstruct_tracked_asset_locks(&wallet_manager, &wallet_id, &[&mempool_record]).await;
        assert_eq!(
            cs.asset_locks.get(&out_point).expect("tracked").status,
            AssetLockStatus::Broadcast
        );

        // Later sighting: the same tx in a chain-locked block.
        let final_record = record_for(
            &tx,
            AccountType::IdentityRegistration,
            chainlocked_context(910),
        );
        let cs =
            reconstruct_tracked_asset_locks(&wallet_manager, &wallet_id, &[&final_record]).await;

        let entry = cs
            .asset_locks
            .get(&out_point)
            .expect("upgraded changeset entry");
        assert_eq!(entry.status, AssetLockStatus::RecoveredFromChain);
        match &entry.proof {
            Some(dpp::prelude::AssetLockProof::Chain(chain)) => {
                assert_eq!(chain.core_chain_locked_height, 910);
                assert_eq!(chain.out_point, out_point);
            }
            other => panic!("expected the attached chain proof, got {other:?}"),
        }
        let wm = wallet_manager.read().await;
        let lock = wm
            .get_wallet_info(&wallet_id)
            .expect("wallet")
            .tracked_asset_locks
            .get(&out_point)
            .expect("in-memory entry");
        assert_eq!(lock.status, AssetLockStatus::RecoveredFromChain);
        assert!(lock.proof.is_some());
    }

    /// The chainlock-promotion hook must finish what a pre-finality
    /// scan started: a lock reconstructed at `Broadcast` (the scan saw
    /// the tx before any chainlock was applied) upgrades to
    /// `RecoveredFromChain` + a chain proof at the chainlock's height
    /// when the `ChainLockProcessed` promotion names its txid — without
    /// a restart, without the record re-flowing through
    /// `TransactionDetected`/`BlockProcessed`, and without reading the
    /// record back at all (the promotion evicts it under the default
    /// `keep-finalized-transactions=OFF` feature).
    #[tokio::test]
    async fn chain_lock_promotion_upgrades_pre_finality_reconstruction() {
        let (wallet_manager, wallet_id, tx) =
            wallet_with_built_asset_lock(AssetLockFundingType::IdentityRegistration, 0).await;
        let out_point = OutPoint::new(tx.txid(), 0);

        // Restore-scan sighting before the chainlock: tracked at
        // Broadcast, no proof.
        let mempool_record = record_for(
            &tx,
            AccountType::IdentityRegistration,
            TransactionContext::Mempool,
        );
        let cs =
            reconstruct_tracked_asset_locks(&wallet_manager, &wallet_id, &[&mempool_record]).await;
        assert_eq!(
            cs.asset_locks.get(&out_point).expect("tracked").status,
            AssetLockStatus::Broadcast
        );

        // The chainlock promotion names the txid under its funding
        // account. No record is available — eviction already happened.
        let locked_transactions: BTreeMap<AccountType, Vec<dashcore::Txid>> =
            BTreeMap::from([(AccountType::IdentityRegistration, vec![tx.txid()])]);

        let cs = enrich_tracked_asset_locks_from_chain_lock(
            &wallet_manager,
            &wallet_id,
            910,
            &locked_transactions,
        )
        .await;

        let entry = cs
            .asset_locks
            .get(&out_point)
            .expect("upgraded changeset entry");
        assert_eq!(entry.status, AssetLockStatus::RecoveredFromChain);
        match &entry.proof {
            Some(dpp::prelude::AssetLockProof::Chain(chain)) => {
                assert_eq!(chain.core_chain_locked_height, 910);
                assert_eq!(chain.out_point, out_point);
            }
            other => panic!("expected a chain proof at the chainlock height, got {other:?}"),
        }

        // A promotion that names no funding-family account is a no-op
        // (and must not take the wallet-manager write lock).
        let unrelated: BTreeMap<AccountType, Vec<dashcore::Txid>> = BTreeMap::from([(
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            vec![tx.txid()],
        )]);
        let cs =
            enrich_tracked_asset_locks_from_chain_lock(&wallet_manager, &wallet_id, 911, &unrelated)
                .await;
        assert!(Merge::is_empty(&cs));
    }

    /// Enrichment must not touch entries a live flow owns: a `Built`
    /// entry (in-flight build) and a `Consumed` tombstone stay exactly
    /// as they are even when a finalized record for their tx arrives.
    #[tokio::test]
    async fn enrichment_leaves_built_and_consumed_entries_alone() {
        for protected_status in [AssetLockStatus::Built, AssetLockStatus::Consumed] {
            let (wallet_manager, wallet_id, tx) =
                wallet_with_built_asset_lock(AssetLockFundingType::IdentityRegistration, 0).await;
            let out_point = OutPoint::new(tx.txid(), 0);
            {
                let mut wm = wallet_manager.write().await;
                let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet");
                info.tracked_asset_locks.insert(
                    out_point,
                    TrackedAssetLock {
                        out_point,
                        transaction: tx.clone(),
                        account_index: 0,
                        funding_type: AssetLockFundingType::IdentityRegistration,
                        identity_index: 0,
                        amount: 1_000_000,
                        status: protected_status.clone(),
                        proof: None,
                    },
                );
            }
            let record = record_for(
                &tx,
                AccountType::IdentityRegistration,
                chainlocked_context(910),
            );
            let cs = reconstruct_tracked_asset_locks(&wallet_manager, &wallet_id, &[&record]).await;
            assert!(
                Merge::is_empty(&cs),
                "{protected_status:?} must not be enriched"
            );
            let wm = wallet_manager.read().await;
            assert_eq!(
                wm.get_wallet_info(&wallet_id)
                    .expect("wallet")
                    .tracked_asset_locks
                    .get(&out_point)
                    .expect("entry")
                    .status,
                protected_status
            );
        }
    }

    /// The lock-free pre-filter must reject everything that can't
    /// reconstruct: funding-family records without an asset-lock
    /// payload, and asset-lock payloads filed under non-funding
    /// accounts.
    #[tokio::test]
    async fn candidate_prefilter_rejects_non_funding_and_non_asset_lock() {
        let (_wm, _wallet_id, tx) =
            wallet_with_built_asset_lock(AssetLockFundingType::IdentityRegistration, 0).await;

        // Asset-lock payload, but filed under a fund-bearing account —
        // that's the UTXO-debit side of the tx, not the credit side.
        let standard = record_for(
            &tx,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            chainlocked_context(10),
        );
        assert!(!is_reconstruction_candidate(&standard));

        // Funding account, but a plain payment transaction.
        let plain_tx = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        };
        let plain = record_for(
            &plain_tx,
            AccountType::IdentityRegistration,
            chainlocked_context(10),
        );
        assert!(!is_reconstruction_candidate(&plain));
    }

    /// Credit outputs paying scripts outside the funding account's pool
    /// (someone else's asset lock that happened to be filed here, or a
    /// multi-credit-output tx with foreign outputs) reconstruct
    /// nothing.
    #[tokio::test]
    async fn foreign_credit_output_yields_nothing() {
        use dashcore::blockdata::transaction::special_transaction::asset_lock::AssetLockPayload;
        use dashcore::{ScriptBuf, TxOut};

        let (wallet_manager, wallet_id, _tx) =
            wallet_with_built_asset_lock(AssetLockFundingType::IdentityRegistration, 0).await;
        let foreign_tx = Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(
                AssetLockPayload {
                    version: 1,
                    credit_outputs: vec![TxOut {
                        value: 5_000,
                        script_pubkey: ScriptBuf::from(vec![0x76, 0xa9, 0x14, 0xab, 0x88, 0xac]),
                    }],
                },
            )),
        };
        let record = record_for(
            &foreign_tx,
            AccountType::IdentityRegistration,
            chainlocked_context(10),
        );
        assert!(
            is_reconstruction_candidate(&record),
            "the cheap pre-filter can't check pool membership — only the locked pass can"
        );

        let cs = reconstruct_tracked_asset_locks(&wallet_manager, &wallet_id, &[&record]).await;
        assert!(
            Merge::is_empty(&cs),
            "foreign credit outputs must not reconstruct locks"
        );
    }

    /// A reconstructed chain-locked lock is explicitly resumable: the
    /// attached proof feeds `resume_asset_lock` without another proof
    /// wait — and the status stays `RecoveredFromChain` on the way out
    /// (a resume proves nothing new about Platform-side consumption,
    /// so it must not re-enter the pending window).
    #[tokio::test]
    async fn recovered_lock_resumes_from_attached_proof() {
        let (wallet_manager, wallet_id, tx) =
            wallet_with_built_asset_lock(AssetLockFundingType::IdentityRegistration, 0).await;
        let record = record_for(
            &tx,
            AccountType::IdentityRegistration,
            chainlocked_context(77),
        );
        let _cs = reconstruct_tracked_asset_locks(&wallet_manager, &wallet_id, &[&record]).await;

        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .build()
                .expect("mock sdk"),
        );
        let manager = AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::new(NoopTestPersister) as Arc<dyn PlatformWalletPersistence>,
            ),
        );
        let out_point = OutPoint::new(tx.txid(), 0);
        let (proof, _path) = manager
            .resume_asset_lock(&out_point, Some(Duration::from_secs(1)))
            .await
            .expect("resume must consume the attached chain proof, not wait");
        match proof {
            dpp::prelude::AssetLockProof::Chain(chain) => {
                assert_eq!(chain.core_chain_locked_height, 77);
            }
            other => panic!("expected the reconstructed chain proof, got {other:?}"),
        }
        let wm = wallet_manager.read().await;
        assert_eq!(
            wm.get_wallet_info(&wallet_id)
                .expect("wallet")
                .tracked_asset_locks
                .get(&out_point)
                .expect("entry")
                .status,
            AssetLockStatus::RecoveredFromChain,
            "a resume must not downgrade a recovered lock into the pending window"
        );
    }

    /// The benign race with a LIVE flow, end to end: a chainlock
    /// promotion may reach a proof-less `Broadcast` entry while the
    /// live pipeline's `wait_for_proof` is still running, transiently
    /// classifying it `RecoveredFromChain`. The live pipeline's own
    /// writers don't consult the entry — `wait_for_proof` resolves the
    /// proof from records/events and the build pipeline then calls
    /// `advance_asset_lock_status`, which overwrites status and proof
    /// unconditionally — so the live flow always wins the race and the
    /// transient recovery classification never sticks; consumption
    /// still lands the `Consumed` terminal afterwards.
    #[tokio::test]
    async fn live_flow_advance_overwrites_chain_lock_recovery_classification() {
        let (wallet_manager, wallet_id, tx) =
            wallet_with_built_asset_lock(AssetLockFundingType::IdentityRegistration, 0).await;
        let out_point = OutPoint::new(tx.txid(), 0);

        // The live lock, stranded at proof-less Broadcast long enough
        // for its block to chain-lock (IS lock never arrived).
        let mempool_record = record_for(
            &tx,
            AccountType::IdentityRegistration,
            TransactionContext::Mempool,
        );
        let _ = reconstruct_tracked_asset_locks(&wallet_manager, &wallet_id, &[&mempool_record])
            .await;

        // The chainlock promotion races in first.
        let locked_transactions: BTreeMap<AccountType, Vec<dashcore::Txid>> =
            BTreeMap::from([(AccountType::IdentityRegistration, vec![tx.txid()])]);
        let cs = enrich_tracked_asset_locks_from_chain_lock(
            &wallet_manager,
            &wallet_id,
            77,
            &locked_transactions,
        )
        .await;
        assert_eq!(
            cs.asset_locks.get(&out_point).expect("entry").status,
            AssetLockStatus::RecoveredFromChain
        );

        // The live flow's proof then arrives and its pipeline advances
        // the entry exactly as `build.rs` does — the recovery
        // classification is overwritten, not merged around.
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .build()
                .expect("mock sdk"),
        );
        let manager = AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::new(NoopTestPersister) as Arc<dyn PlatformWalletPersistence>,
            ),
        );
        let live_proof = dpp::prelude::AssetLockProof::Chain(chain_proof(78, out_point));
        manager
            .advance_asset_lock_status(&out_point, AssetLockStatus::ChainLocked, Some(live_proof))
            .await
            .expect("live advance");
        {
            let wm = wallet_manager.read().await;
            assert_eq!(
                wm.get_wallet_info(&wallet_id)
                    .expect("wallet")
                    .tracked_asset_locks
                    .get(&out_point)
                    .expect("entry")
                    .status,
                AssetLockStatus::ChainLocked,
                "the live pipeline's advance must overwrite the transient recovery classification"
            );
        }

        // And consumption still reaches its terminal.
        let _ = manager.consume_asset_lock(&out_point).await.expect("consume");
        let wm = wallet_manager.read().await;
        assert_eq!(
            wm.get_wallet_info(&wallet_id)
                .expect("wallet")
                .tracked_asset_locks
                .get(&out_point)
                .expect("entry")
                .status,
            AssetLockStatus::Consumed
        );
    }
}
