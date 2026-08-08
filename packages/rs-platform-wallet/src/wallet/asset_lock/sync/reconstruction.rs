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
//! Reconstructed entries carry
//! [`AssetLockStatus::RecoveredFromChain`] — Platform-side consumption
//! is unknown after a restore, so they must land in neither the pending
//! nor the consumed bucket (see the variant's doc).
//!
//! The live entry point is the wallet-event adapter
//! ([`crate::changeset::core_bridge`]): every `TransactionDetected` /
//! `BlockProcessed` record flows through
//! [`reconstruct_tracked_asset_locks`], which inserts missing entries
//! and returns an [`AssetLockChangeSet`] the adapter persists in the
//! same store round-trip as the core rows. Insertion is
//! insert-if-absent: locks tracked live by the build pipeline (which
//! tracks *before* broadcast) always win over a reconstruction.

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
/// on-chain context. Always [`AssetLockStatus::RecoveredFromChain`];
/// the proof is attached when the context proves chain finality so an
/// explicit resume can consume the lock without another proof wait.
fn recovered_status(
    record: &TransactionRecord,
    out_point: OutPoint,
) -> (AssetLockStatus, Option<dpp::prelude::AssetLockProof>) {
    use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    let proof = match &record.context {
        TransactionContext::InChainLockedBlock(_) => record.height().map(|height| {
            dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
                core_chain_locked_height: height,
                out_point,
            })
        }),
        _ => None,
    };
    (AssetLockStatus::RecoveredFromChain, proof)
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

/// Rebuild missing tracked-asset-lock entries from scan records.
///
/// For each record that passes [`is_reconstruction_candidate`], match
/// its credit outputs against the owning funding account and insert a
/// [`AssetLockStatus::RecoveredFromChain`] entry for every outpoint not
/// already tracked. Returns the changeset describing the inserted
/// entries (empty when nothing was reconstructed) for the caller to
/// persist alongside whatever else it is flushing.
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
        // `reconstruct_candidates` reads `info.tracked_asset_locks`
        // under the same write lock that guards the insert below, so
        // insert-if-absent holds without a re-check.
        for lock in reconstruct_candidates(info, record) {
            tracing::info!(
                outpoint = %lock.out_point,
                funding_type = ?lock.funding_type,
                amount = lock.amount,
                has_proof = lock.proof.is_some(),
                "reconstructed tracked asset lock from on-chain record"
            );
            cs.asset_locks.insert(lock.out_point, (&lock).into());
            info.tracked_asset_locks.insert(lock.out_point, lock);
        }
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
    /// reconstructs — but with no proof to attach.
    #[tokio::test]
    async fn unconfirmed_record_reconstructs_without_proof() {
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
        assert_eq!(entry.status, AssetLockStatus::RecoveredFromChain);
        assert!(entry.proof.is_none(), "no finality context ⇒ no proof");
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
    /// wait, and the status advances to `ChainLocked` on the way out.
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
    }
}
