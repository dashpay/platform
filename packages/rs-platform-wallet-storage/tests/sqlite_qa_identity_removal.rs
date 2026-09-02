#![allow(clippy::field_reassign_with_default)]

//! Write-path coverage for the `IdentityChangeSet.removed` branch. The
//! removal runs a wallet-scoped, NULL-safe `DELETE FROM identities WHERE
//! identity_id = ?1 AND wallet_id IS ?2`, mirroring the upsert's
//! per-entry wallet cross-check. These tests pin that a removed identity
//! leaves nothing behind for `load_state` to see, that re-adding its id
//! starts from a blank identity, and that a foreign wallet's `removed`
//! set cannot reach this wallet's identity.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{ensure_wallet_meta, fresh_persister, wid};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    IdentityChangeSet, IdentityEntry, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::identity::IdentityStatus;
use platform_wallet_storage::sqlite::schema::identities;

fn reopen(path: &std::path::Path) -> platform_wallet_storage::SqlitePersister {
    platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(path),
    )
    .expect("reopen persister")
}

/// Build an `IdentityEntry` parented to a specific wallet (so the upsert
/// cross-check passes and the typed `wallet_id` column is populated).
///
/// The derivation slot is keyed off `id` because `(wallet_id,
/// identity_index)` names exactly one identity: two identities of one
/// wallet sharing a slot is the corruption `store` refuses to write.
fn entry_for(id: u8, wallet_id: [u8; 32]) -> IdentityEntry {
    IdentityEntry {
        id: Identifier::from([id; 32]),
        balance: u64::from(id),
        revision: 1,
        identity_index: Some(u32::from(id)),
        last_updated_balance_block_time: None,
        last_synced_keys_block_time: None,
        dpns_names: Vec::new(),
        contested_dpns_names: Vec::new(),
        status: IdentityStatus::Active,
        wallet_id: Some(wallet_id),
        dashpay_profile: None,
        dashpay_payments: Default::default(),
        contact_profiles: Default::default(),
        ignored_senders: Default::default(),
    }
}

/// An identity routed through `IdentityChangeSet.removed` is deleted and
/// disappears from the per-wallet `load_state` while a sibling,
/// non-removed identity survives.
#[test]
fn qa_rm1_removed_identity_excluded_from_load() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD0);
    ensure_wallet_meta(&persister, &w);

    let keep = entry_for(0x01, w);
    let drop_me = entry_for(0x02, w);
    let mut idents: BTreeMap<Identifier, IdentityEntry> = BTreeMap::new();
    idents.insert(keep.id, keep.clone());
    idents.insert(drop_me.id, drop_me.clone());

    // First flush: insert both.
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities: idents,
                    removed: Default::default(),
                }),
                ..Default::default()
            },
        )
        .unwrap();

    // Second flush: remove drop_me.
    let mut removed: BTreeSet<Identifier> = BTreeSet::new();
    removed.insert(drop_me.id);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities: Default::default(),
                    removed,
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();

    // The removed row is physically gone; only the survivor remains.
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM identities WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(total, 1, "a removal deletes the row, it does not flag it");

    // load_state must surface only the survivor.
    let state = identities::load_state(&conn, &w).unwrap();
    drop(conn);
    let wallet_idents = state.wallet_identities.get(&w).expect("wallet bucket");
    assert_eq!(
        wallet_idents.len(),
        1,
        "load_state must surface only the surviving identity"
    );
    let surviving_ids: Vec<Identifier> = wallet_idents.values().map(|m| m.identity.id()).collect();
    assert!(
        surviving_ids.contains(&keep.id),
        "kept identity must survive load"
    );
    assert!(
        !surviving_ids.contains(&drop_me.id),
        "removed identity must NOT appear in load"
    );
}

/// Re-adding a removed identity id is legal and idempotent — the upsert
/// simply finds no conflicting row — but it starts from a blank
/// identity, never the removed one's state. The in-memory
/// `IdentityManager` drops the whole `ManagedIdentity` on removal, so
/// this is what keeps storage and memory agreeing.
#[test]
fn qa_rm2_re_add_after_removal_is_a_fresh_row() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD1);
    ensure_wallet_meta(&persister, &w);

    let e = entry_for(0x05, w);
    let mut idents: BTreeMap<Identifier, IdentityEntry> = BTreeMap::new();
    idents.insert(e.id, e.clone());
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities: idents.clone(),
                    removed: Default::default(),
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let mut removed: BTreeSet<Identifier> = BTreeSet::new();
    removed.insert(e.id);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities: Default::default(),
                    removed,
                }),
                ..Default::default()
            },
        )
        .unwrap();

    // The removal is observable between the two writes: nothing is left
    // for the re-add to inherit.
    {
        let conn = persister.lock_conn_for_test();
        let rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM identities WHERE identity_id = ?1",
                rusqlite::params![e.id.as_slice()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rows, 0, "the removal deleted the row");
    }

    // Re-add the same id with a different balance: a plain insert now,
    // not an update of a surviving row.
    let re_added = IdentityEntry {
        balance: 9_999,
        ..e.clone()
    };
    let mut re_add: BTreeMap<Identifier, IdentityEntry> = BTreeMap::new();
    re_add.insert(re_added.id, re_added);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities: re_add,
                    removed: Default::default(),
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let state = identities::load_state(&conn, &w).unwrap();
    drop(conn);
    let bucket = state.wallet_identities.get(&w).expect("wallet bucket");
    assert_eq!(bucket.len(), 1, "the re-added identity is loadable again");
    assert_eq!(
        bucket
            .values()
            .next()
            .expect("one identity")
            .identity
            .balance(),
        9_999,
        "the re-added blob wins; no trace of the removed one survives"
    );
}

/// The removal DELETE is scoped by `wallet_id`: a `removed` entry naming
/// an identity parented to a different wallet is a no-op against that
/// wallet's row (NULL-safe `wallet_id IS ?2` predicate). An identity_id
/// is globally unique to one wallet, so this is defense-in-depth
/// enforcing the isolation the data model assumes.
#[test]
fn qa_rm3_removal_is_wallet_scoped() {
    let (persister, _tmp, path) = fresh_persister();
    let wa = wid(0xE0);
    let wb = wid(0xE1);
    ensure_wallet_meta(&persister, &wa);
    ensure_wallet_meta(&persister, &wb);

    // Identity 0x07 is parented to wallet B.
    let b_ident = entry_for(0x07, wb);
    let mut b_map: BTreeMap<Identifier, IdentityEntry> = BTreeMap::new();
    b_map.insert(b_ident.id, b_ident.clone());
    persister
        .store(
            wb,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities: b_map,
                    removed: Default::default(),
                }),
                ..Default::default()
            },
        )
        .unwrap();

    // Wallet A flushes a `removed` set naming wallet B's identity id.
    let mut removed: BTreeSet<Identifier> = BTreeSet::new();
    removed.insert(b_ident.id);
    persister
        .store(
            wa,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities: Default::default(),
                    removed,
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM identities WHERE identity_id = ?1",
            rusqlite::params![b_ident.id.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    let b_state = identities::load_state(&conn, &wb).unwrap();
    drop(conn);

    // Cross-wallet isolation: wallet A's `removed` set names wallet B's
    // identity, but the wallet-scoped DELETE leaves B's row untouched,
    // so B's load still surfaces the identity.
    assert_eq!(
        rows, 1,
        "wallet-scoped removal: A's removed set must NOT delete B's identity"
    );
    assert_eq!(
        b_state
            .wallet_identities
            .get(&wb)
            .map(|m| m.len())
            .unwrap_or(0),
        1,
        "B's identity must survive A's unrelated removal"
    );
}
