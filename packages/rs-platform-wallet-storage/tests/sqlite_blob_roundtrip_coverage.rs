#![allow(clippy::field_reassign_with_default)]

//! Recurrence guard for #4133: exercise **fully-populated** values of BOTH
//! enum-bearing proof variants — an `AssetLockEntry` carrying
//! `proof: Some(AssetLockProof::Chain(..))` and one carrying
//! `proof: Some(AssetLockProof::Instant(..))` — end-to-end through the public
//! `SqlitePersister::store` → reopen → `load_unconsumed` path, plus a
//! `proof: None` control.
//!
//! The original defect escaped every test because all asset-lock fixtures used
//! `proof: None`, so the internally-tagged proof enum was never round-tripped.
//! Enumerating both variants with non-default field values (not a single
//! variant, not a partial/empty one) is what makes this a guard that would
//! actually have caught the bug.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use dashcore::hashes::Hash;
use dashcore::{OutPoint, Transaction, Txid};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::identity::state_transition::asset_lock_proof::instant::InstantAssetLockProof;
use dpp::prelude::AssetLockProof;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet::changeset::{AssetLockChangeSet, AssetLockEntry, PlatformWalletChangeSet};
use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
use platform_wallet_storage::sqlite::schema::asset_locks;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

fn reopen(path: &std::path::Path) -> SqlitePersister {
    SqlitePersister::open(SqlitePersisterConfig::new(path)).expect("reopen")
}

fn op(b: u8) -> OutPoint {
    OutPoint {
        txid: Txid::from_byte_array([b; 32]),
        vout: 0,
    }
}

fn entry(op: OutPoint, status: AssetLockStatus, proof: Option<AssetLockProof>) -> AssetLockEntry {
    AssetLockEntry {
        out_point: op,
        transaction: Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        },
        account_index: 0,
        funding_type: AssetLockFundingType::IdentityRegistration,
        identity_index: 0,
        amount_duffs: 1000,
        status,
        proof,
    }
}

/// Store the three proof shapes (`Chain`, `Instant`, `None`), reopen, and
/// assert every one rehydrates with its proof intact.
#[test]
fn proof_bearing_asset_locks_round_trip_through_persister() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xB4);
    ensure_wallet_meta(&persister, &w);

    let op_chain = op(0x10);
    let op_instant = op(0x11);
    let op_none = op(0x12);

    let chain = AssetLockProof::Chain(ChainAssetLockProof::new(99, [0x05u8; 36]));
    // Distinct, non-default field values so the round-trip proves field
    // fidelity, not merely that `default() == default()`.
    let instant = AssetLockProof::Instant({
        let mut p = InstantAssetLockProof::default();
        p.transaction.version = 3;
        p.transaction.lock_time = 111;
        p.output_index = 2;
        // `default()` leaves the nested `InstantLock.inputs` empty; a real
        // IS-lock always carries at least one input, so populate it to exercise
        // the length-prefixed-vec encoding path a genuine proof hits (QA-004).
        p.instant_lock.inputs = vec![op(0xA1), op(0xA2)];
        p
    });

    let mut cs = AssetLockChangeSet::default();
    cs.asset_locks.insert(
        op_chain,
        entry(op_chain, AssetLockStatus::ChainLocked, Some(chain.clone())),
    );
    cs.asset_locks.insert(
        op_instant,
        entry(
            op_instant,
            AssetLockStatus::InstantSendLocked,
            Some(instant.clone()),
        ),
    );
    cs.asset_locks
        .insert(op_none, entry(op_none, AssetLockStatus::Built, None));

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                asset_locks: Some(cs),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let feed = asset_locks::load_unconsumed(&conn, &w).expect("load must succeed");
    drop(conn);

    let flat: std::collections::BTreeMap<OutPoint, Option<AssetLockProof>> = feed
        .into_values()
        .flat_map(|by_op| by_op.into_iter().map(|(op, t)| (op, t.proof)))
        .collect();

    assert_eq!(
        flat.get(&op_chain),
        Some(&Some(chain)),
        "Chain proof must survive the persister round-trip"
    );
    assert_eq!(
        flat.get(&op_instant),
        Some(&Some(instant)),
        "Instant proof must survive the persister round-trip"
    );
    assert_eq!(
        flat.get(&op_none),
        Some(&None),
        "None proof must round-trip as None"
    );
}
