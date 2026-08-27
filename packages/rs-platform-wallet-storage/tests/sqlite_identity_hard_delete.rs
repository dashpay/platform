#![allow(clippy::field_reassign_with_default)]

//! `IdentityChangeSet.removed` is a physical `DELETE FROM identities`.
//!
//! Every dependent row goes with it — through the native FK cascade
//! where one is live, and through `cascade_children_on_identity_delete`
//! where none is. Three shapes need the trigger rather than the FKs:
//! an out-of-wallet identity's `identity_keys` (the compound FK is
//! dormant once `wallet_id` is NULL), and `contacts` / `ignored_senders`
//! (keyed by `owner_id` with no FK to `identities` at all).

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{ensure_wallet_meta, fresh_persister, wid};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    ContactChangeSet, IdentityChangeSet, IdentityEntry, IdentityKeyEntry, IdentityKeysChangeSet,
    PersistenceError, PersistenceErrorKind, PlatformWalletChangeSet, PlatformWalletPersistence,
    SentContactRequestKey, TokenBalanceChangeSet,
};
use platform_wallet::wallet::identity::{ContactRequest, EstablishedContact, IdentityStatus};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::migrations as mig;
use rusqlite::{params, Connection};

/// The all-zero scope every reader and writer maps to a NULL `wallet_id`.
const UNOWNED: WalletId = [0u8; 32];

fn reopen(path: &std::path::Path) -> platform_wallet_storage::SqlitePersister {
    platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(path),
    )
    .expect("reopen persister")
}

fn iid(byte: u8) -> Identifier {
    Identifier::from([byte; 32])
}

fn entry(id: Identifier, wallet_id: Option<WalletId>, index: Option<u32>) -> IdentityEntry {
    IdentityEntry {
        id,
        balance: 1_000,
        revision: 1,
        identity_index: index,
        last_updated_balance_block_time: None,
        last_synced_keys_block_time: None,
        dpns_names: Vec::new(),
        contested_dpns_names: Vec::new(),
        status: IdentityStatus::Active,
        wallet_id,
        dashpay_profile: None,
        dashpay_payments: Default::default(),
        contact_profiles: Default::default(),
        ignored_senders: Default::default(),
    }
}

fn key_entry(id: Identifier, key_id: u32, byte: u8) -> IdentityKeyEntry {
    IdentityKeyEntry {
        identity_id: id,
        key_id,
        public_key: IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: key_id,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![byte; 33]),
            disabled_at: None,
        }),
        public_key_hash: [byte; 20],
        wallet_id: None,
        derivation_indices: None,
    }
}

fn keys_of(id: Identifier, key_id: u32, byte: u8) -> IdentityKeysChangeSet {
    let mut cs = IdentityKeysChangeSet::default();
    cs.upserts.insert((id, key_id), key_entry(id, key_id, byte));
    cs
}

fn contact_request(sender: Identifier, recipient: Identifier) -> ContactRequest {
    ContactRequest {
        sender_id: sender,
        recipient_id: recipient,
        sender_key_index: 1,
        recipient_key_index: 2,
        account_reference: 3,
        encrypted_account_label: None,
        encrypted_public_key: vec![9, 9, 9],
        auto_accept_proof: None,
        core_height_created_at: 42,
        created_at: 7,
    }
}

/// An established pair plus an ignored sender, both owned by `owner` —
/// the two `owner_id`-keyed tables that carry no FK to `identities`.
fn contacts_of(owner: Identifier, contact: Identifier) -> ContactChangeSet {
    let mut established = BTreeMap::new();
    established.insert(
        SentContactRequestKey {
            owner_id: owner,
            recipient_id: contact,
        },
        EstablishedContact {
            contact_identity_id: contact,
            outgoing_request: contact_request(owner, contact),
            incoming_request: contact_request(contact, owner),
            alias: Some("friend".into()),
            note: None,
            is_hidden: false,
            accepted_accounts: vec![0],
            payment_channel_broken: false,
            contact_account_label: None,
            external_account_reference: None,
        },
    );
    ContactChangeSet {
        established,
        ignored: BTreeSet::from([(owner, contact)]),
        ..Default::default()
    }
}

fn balances_of(owner: Identifier, token: Identifier) -> TokenBalanceChangeSet {
    let mut cs = TokenBalanceChangeSet::default();
    cs.balances.insert((owner, token), 77);
    cs
}

fn removal_of(id: Identifier) -> IdentityChangeSet {
    let mut cs = IdentityChangeSet::default();
    cs.removed.insert(id);
    cs
}

fn upsert_of(e: IdentityEntry) -> IdentityChangeSet {
    let mut cs = IdentityChangeSet::default();
    cs.identities.insert(e.id, e);
    cs
}

/// `SELECT COUNT(*)` with one 32-byte-identifier parameter.
fn count_by_id(conn: &Connection, sql: &str, id: Identifier) -> i64 {
    conn.query_row(sql, params![id.as_slice()], |r| r.get(0))
        .expect("count query")
}

/// Every row a removed identity owns, counted in one pass.
fn dependents_of(conn: &Connection, id: Identifier) -> Vec<(&'static str, i64)> {
    [
        (
            "identities",
            "SELECT COUNT(*) FROM identities WHERE identity_id = ?1",
        ),
        (
            "identity_keys",
            "SELECT COUNT(*) FROM identity_keys WHERE identity_id = ?1",
        ),
        (
            "contacts",
            "SELECT COUNT(*) FROM contacts WHERE owner_id = ?1",
        ),
        (
            "ignored_senders",
            "SELECT COUNT(*) FROM ignored_senders WHERE owner_id = ?1",
        ),
        (
            "token_balances",
            "SELECT COUNT(*) FROM token_balances WHERE identity_id = ?1",
        ),
        (
            "meta_identity",
            "SELECT COUNT(*) FROM meta_identity WHERE identity_id = ?1",
        ),
        (
            "meta_token",
            "SELECT COUNT(*) FROM meta_token WHERE identity_id = ?1",
        ),
    ]
    .into_iter()
    .map(|(table, sql)| (table, count_by_id(conn, sql, id)))
    .collect()
}

/// The base case: `removed` deletes the row rather than flagging it, and
/// leaves its wallet-mate alone.
#[test]
fn removed_identity_row_is_physically_deleted() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD0);
    ensure_wallet_meta(&persister, &w);
    let keep = iid(0x01);
    let drop_me = iid(0x02);

    let mut both = IdentityChangeSet::default();
    both.identities.insert(keep, entry(keep, Some(w), Some(1)));
    both.identities
        .insert(drop_me, entry(drop_me, Some(w), Some(2)));
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(both),
                ..Default::default()
            },
        )
        .expect("seed both identities");
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(removal_of(drop_me)),
                ..Default::default()
            },
        )
        .expect("remove one");
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    assert_eq!(
        count_by_id(
            &conn,
            "SELECT COUNT(*) FROM identities WHERE identity_id = ?1",
            drop_me
        ),
        0,
        "a removed identity leaves no row behind"
    );
    assert_eq!(
        count_by_id(
            &conn,
            "SELECT COUNT(*) FROM identities WHERE identity_id = ?1",
            keep
        ),
        1,
        "the wallet-mate must survive its sibling's removal"
    );
}

/// A wallet-owned identity's whole dependent set goes with it: keys and
/// balances via live FKs, `contacts` / `ignored_senders` / `meta_*` via
/// the delete triggers.
#[test]
fn removing_a_wallet_identity_sweeps_every_dependent() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD1);
    ensure_wallet_meta(&persister, &w);
    let owner = iid(0x11);
    let contact = iid(0x12);
    let token = iid(0x13);

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(upsert_of(entry(owner, Some(w), Some(1)))),
                identity_keys: Some(keys_of(owner, 0, 0xAA)),
                contacts: Some(contacts_of(owner, contact)),
                token_balances: Some(balances_of(owner, token)),
                ..Default::default()
            },
        )
        .expect("seed identity and dependents");

    // Metadata carries no FK at all, so seed it directly — the AFTER
    // DELETE brooms are its only cleanup path.
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO meta_identity (identity_id, key, value) VALUES (?1, 'alias', X'00')",
            params![owner.as_slice()],
        )
        .expect("seed meta_identity");
        conn.execute(
            "INSERT INTO meta_token (identity_id, token_id, key, value) \
             VALUES (?1, ?2, 'note', X'00')",
            params![owner.as_slice(), token.as_slice()],
        )
        .expect("seed meta_token");
    }

    {
        let conn = persister.lock_conn_for_test();
        for (table, rows) in dependents_of(&conn, owner) {
            assert_eq!(rows, 1, "`{table}` must be seeded before the removal");
        }
    }

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(removal_of(owner)),
                ..Default::default()
            },
        )
        .expect("remove the identity");
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    for (table, rows) in dependents_of(&conn, owner) {
        assert_eq!(rows, 0, "`{table}` must be swept by the identity removal");
    }
}

/// The dormant-FK case. `identity_keys`' FK to `identities` is compound
/// (`wallet_id, identity_id`), and SQLite's MATCH SIMPLE skips
/// enforcement entirely once a child key column is NULL — so an
/// out-of-wallet identity's keys are reachable ONLY through the trigger.
#[test]
fn removing_an_out_of_wallet_identity_deletes_its_keys() {
    let (persister, _tmp, path) = fresh_persister();
    let id = iid(0x7A);

    persister
        .store(
            UNOWNED,
            PlatformWalletChangeSet {
                identities: Some(upsert_of(entry(id, None, None))),
                identity_keys: Some(keys_of(id, 0, 0xAB)),
                ..Default::default()
            },
        )
        .expect("seed unowned identity with a key");

    {
        let conn = persister.lock_conn_for_test();
        let null_scoped: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM identity_keys \
                 WHERE identity_id = ?1 AND wallet_id IS NULL",
                params![id.as_slice()],
                |r| r.get(0),
            )
            .expect("count null-scoped keys");
        assert_eq!(
            null_scoped, 1,
            "the key must be NULL-scoped, else this test is not exercising \
             the dormant FK at all"
        );
    }

    persister
        .store(
            UNOWNED,
            PlatformWalletChangeSet {
                identities: Some(removal_of(id)),
                ..Default::default()
            },
        )
        .expect("remove the unowned identity");
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    assert_eq!(
        count_by_id(
            &conn,
            "SELECT COUNT(*) FROM identities WHERE identity_id = ?1",
            id
        ),
        0,
        "the unowned identity row must be gone"
    );
    assert_eq!(
        count_by_id(
            &conn,
            "SELECT COUNT(*) FROM identity_keys WHERE identity_id = ?1",
            id
        ),
        0,
        "the dormant compound FK cannot cascade a NULL-scoped key; the \
         AFTER DELETE trigger must"
    );
}

/// A merged buffer can carry `removed` for an identity alongside key /
/// contact / balance upserts for that same identity — sync writes its
/// keys, the host then removes it, both land in one flush. The removal
/// must run AFTER every identity-scoped child writer: doing it first
/// pulls the FK parent out from under those inserts and fails the whole
/// flush.
#[test]
fn removal_wins_over_child_writes_in_one_changeset() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD2);
    ensure_wallet_meta(&persister, &w);
    let owner = iid(0x21);
    let contact = iid(0x22);
    let token = iid(0x23);

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(upsert_of(entry(owner, Some(w), Some(1)))),
                ..Default::default()
            },
        )
        .expect("seed the identity");

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(removal_of(owner)),
                identity_keys: Some(keys_of(owner, 0, 0xCC)),
                contacts: Some(contacts_of(owner, contact)),
                token_balances: Some(balances_of(owner, token)),
                ..Default::default()
            },
        )
        .expect("a removal alongside child writes must commit, not FK-fail");
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    for (table, rows) in dependents_of(&conn, owner) {
        assert_eq!(
            rows, 0,
            "`{table}`: the removal must win over same-changeset child writes"
        );
    }
}

/// Removal is terminal, not reversible: re-adding the same identity id
/// gets a blank identity, never its old keys / contacts / balances back.
/// This is what the in-memory `IdentityManager` already does — it drops
/// the whole `ManagedIdentity` — so storage and memory agree.
#[test]
fn re_added_identity_starts_from_zero() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD3);
    ensure_wallet_meta(&persister, &w);
    let owner = iid(0x31);
    let contact = iid(0x32);
    let token = iid(0x33);

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(upsert_of(entry(owner, Some(w), Some(1)))),
                identity_keys: Some(keys_of(owner, 0, 0xAA)),
                contacts: Some(contacts_of(owner, contact)),
                token_balances: Some(balances_of(owner, token)),
                ..Default::default()
            },
        )
        .expect("seed identity and dependents");
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(removal_of(owner)),
                ..Default::default()
            },
        )
        .expect("remove");
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(upsert_of(entry(owner, Some(w), Some(1)))),
                ..Default::default()
            },
        )
        .expect("re-add the same identity id");
    drop(persister);

    let p2 = reopen(&path);
    let state = p2.load().expect("load after re-add");
    let managed = &state.wallets[&w].identity_manager.wallet_identities[&w][&1];
    assert_eq!(managed.identity.id(), owner, "the identity is back");
    assert!(
        managed.identity.public_keys().is_empty(),
        "a re-added identity must not inherit the removed one's keys"
    );
    assert!(
        managed.dashpay().established_contacts().is_empty(),
        "a re-added identity must not inherit the removed one's contacts"
    );

    let conn = p2.lock_conn_for_test();
    assert_eq!(
        count_by_id(
            &conn,
            "SELECT COUNT(*) FROM token_balances WHERE identity_id = ?1",
            owner
        ),
        0,
        "a re-added identity must not inherit the removed one's balances"
    );
}

/// The wallet stays loadable under the STRICT policy after a removal.
/// Leftover `identity_keys` / `contacts` rows would surface as
/// `OrphanedIdentityEntry` and take the whole wallet's load down with
/// them, turning an ordinary `remove_identity` into a bricked wallet.
#[test]
fn strict_load_survives_a_removed_identity() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD4);
    ensure_wallet_meta(&persister, &w);
    let removed = iid(0x41);
    let contact = iid(0x42);
    let survivor = iid(0x43);

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(upsert_of(entry(removed, Some(w), Some(1)))),
                identity_keys: Some(keys_of(removed, 0, 0x99)),
                contacts: Some(contacts_of(removed, contact)),
                ..Default::default()
            },
        )
        .expect("seed the identity that will be removed");
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(upsert_of(entry(survivor, Some(w), Some(2)))),
                ..Default::default()
            },
        )
        .expect("seed the survivor");
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(removal_of(removed)),
                ..Default::default()
            },
        )
        .expect("remove");
    drop(persister);

    let p2 = reopen(&path);
    let state = p2.load().expect("a strict load must survive a removal");
    assert!(
        !p2.last_load_degradation().degraded,
        "an ordinary removal is not a degraded load"
    );
    let bucket = &state.wallets[&w].identity_manager.wallet_identities[&w];
    assert_eq!(bucket.len(), 1, "only the survivor remains");
    assert_eq!(bucket[&2].identity.id(), survivor);
}

/// The seam between V014 and V015: an `identity_keys` orphan can be
/// neither written nor left behind.
///
/// V014's NULL-scope trigger refuses a key naming an identity that is not
/// there, closing the only door MATCH SIMPLE's dormant compound FK left
/// open on the write side. V015's delete broom closes the read side, by
/// sweeping such keys when their identity goes. Either migration alone
/// leaves `identity_keys` able to orphan; this pins that together they
/// do not, so `MissingIdentityOwner` is reachable only through the
/// FK-less `contacts` / `ignored_senders` tables (covered by
/// `sqlite_contacts_keys_rehydration`).
#[test]
fn null_scoped_key_can_neither_name_a_missing_identity_nor_outlive_one() {
    let (persister, _tmp, _path) = fresh_persister();
    let ghost = iid(0x6B);

    // Write side (V014): the key is refused, and nothing lands.
    let err = persister
        .store(
            UNOWNED,
            PlatformWalletChangeSet {
                identity_keys: Some(keys_of(ghost, 0, 0x6C)),
                ..Default::default()
            },
        )
        .expect_err("a NULL-scoped key naming no identity must be refused");
    let PersistenceError::Backend { kind, .. } = &err else {
        panic!("expected a typed backend error, got {err:?}");
    };
    assert_eq!(
        *kind,
        PersistenceErrorKind::Constraint,
        "a guard trigger firing is an integrity violation, not an engine fault"
    );
    {
        let conn = persister.lock_conn_for_test();
        assert_eq!(
            count_by_id(
                &conn,
                "SELECT COUNT(*) FROM identity_keys WHERE identity_id = ?1",
                ghost
            ),
            0,
            "the refused key must not have reached disk"
        );
    }

    // Read side (V015): the same row, written legitimately under a live
    // identity, does not survive that identity's removal.
    persister
        .store(
            UNOWNED,
            PlatformWalletChangeSet {
                identities: Some(upsert_of(entry(ghost, None, None))),
                identity_keys: Some(keys_of(ghost, 0, 0x6C)),
                ..Default::default()
            },
        )
        .expect("the same key is legal once its identity exists");
    persister
        .store(
            UNOWNED,
            PlatformWalletChangeSet {
                identities: Some(removal_of(ghost)),
                ..Default::default()
            },
        )
        .expect("remove the unowned identity");

    let conn = persister.lock_conn_for_test();
    assert_eq!(
        count_by_id(
            &conn,
            "SELECT COUNT(*) FROM identity_keys WHERE identity_id = ?1",
            ghost
        ),
        0,
        "the removal must sweep the key the dormant FK cannot cascade"
    );
}

/// V015 retires the `tombstoned` column, and purges the rows and
/// dependents a pre-V015 database had already logically deleted.
#[test]
fn v015_purges_tombstoned_rows_and_retires_the_column() {
    let mut conn = Connection::open_in_memory().expect("open in-memory db");
    conn.pragma_update(None, "foreign_keys", true)
        .expect("enable foreign keys");

    // Stand the database up at every migration BEFORE this one, so the
    // fixture is written against the schema V015 has to upgrade.
    mig::runner()
        .set_target(refinery::Target::Version(14))
        .run(&mut conn)
        .expect("migrate to the last pre-V015 schema");

    let w = [0x5Au8; 32];
    let tombstoned = [0x51u8; 32];
    let live = [0x52u8; 32];
    let contact = [0x53u8; 32];
    let token = [0x54u8; 32];
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![w.as_slice()],
    )
    .expect("insert wallet");
    for (id, flag) in [(tombstoned, 1), (live, 0)] {
        conn.execute(
            "INSERT INTO identities (identity_id, wallet_id, identity_index, entry_blob, tombstoned) \
             VALUES (?1, ?2, NULL, X'00', ?3)",
            params![id.as_slice(), w.as_slice(), flag],
        )
        .expect("insert identity");
        conn.execute(
            "INSERT INTO identity_keys \
                (wallet_id, identity_id, key_id, public_key_blob, public_key_hash) \
             VALUES (?1, ?2, 0, X'00', X'00')",
            params![w.as_slice(), id.as_slice()],
        )
        .expect("insert key");
        conn.execute(
            "INSERT INTO contacts (wallet_id, owner_id, contact_id, state) \
             VALUES (?1, ?2, ?3, 'established')",
            params![w.as_slice(), id.as_slice(), contact.as_slice()],
        )
        .expect("insert contact");
        conn.execute(
            "INSERT INTO ignored_senders (wallet_id, owner_id, sender_id) VALUES (?1, ?2, ?3)",
            params![w.as_slice(), id.as_slice(), contact.as_slice()],
        )
        .expect("insert ignored sender");
        conn.execute(
            "INSERT INTO token_balances (identity_id, token_id, balance, updated_at) \
             VALUES (?1, ?2, 1, 0)",
            params![id.as_slice(), token.as_slice()],
        )
        .expect("insert balance");
        conn.execute(
            "INSERT INTO meta_identity (identity_id, key, value) VALUES (?1, 'alias', X'00')",
            params![id.as_slice()],
        )
        .expect("insert meta_identity");
    }

    mig::run(&mut conn).expect("migrate to the newest version");

    let columns: Vec<String> = {
        let mut stmt = conn
            .prepare("PRAGMA table_info(identities)")
            .expect("prepare table_info");
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .expect("query table_info");
        rows.map(|r| r.expect("column name")).collect()
    };
    assert!(
        !columns.contains(&"tombstoned".to_string()),
        "V015 must drop the tombstoned column, got {columns:?}"
    );

    for (table, sql) in [
        (
            "identities",
            "SELECT COUNT(*) FROM identities WHERE identity_id = ?1",
        ),
        (
            "identity_keys",
            "SELECT COUNT(*) FROM identity_keys WHERE identity_id = ?1",
        ),
        (
            "contacts",
            "SELECT COUNT(*) FROM contacts WHERE owner_id = ?1",
        ),
        (
            "ignored_senders",
            "SELECT COUNT(*) FROM ignored_senders WHERE owner_id = ?1",
        ),
        (
            "token_balances",
            "SELECT COUNT(*) FROM token_balances WHERE identity_id = ?1",
        ),
        (
            "meta_identity",
            "SELECT COUNT(*) FROM meta_identity WHERE identity_id = ?1",
        ),
    ] {
        let purged: i64 = conn
            .query_row(sql, params![tombstoned.as_slice()], |r| r.get(0))
            .expect("count purged");
        assert_eq!(
            purged, 0,
            "`{table}` must be purged for a tombstoned identity"
        );
        let kept: i64 = conn
            .query_row(sql, params![live.as_slice()], |r| r.get(0))
            .expect("count kept");
        assert_eq!(kept, 1, "`{table}` must be untouched for a live identity");
    }
}
