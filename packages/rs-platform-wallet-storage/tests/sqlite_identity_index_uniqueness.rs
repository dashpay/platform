#![allow(clippy::field_reassign_with_default)]

//! Write-path enforcement of `(wallet_id, identity_index)` uniqueness.
//!
//! `identity_index` is an HD derivation-path component, so one wallet
//! slot names exactly one identity. A duplicate that reaches disk leaves
//! the displaced identity's keys and contacts without an owner, and the
//! next `load()` rejects the WHOLE wallet's state as fatal — so the
//! offending write is refused instead, attributed to the caller that
//! made it.

mod common;

use common::{ensure_identity, ensure_wallet_meta, fresh_persister, wid};

use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    IdentityChangeSet, IdentityEntry, PersistenceError, PersistenceErrorKind,
    PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::identity::IdentityStatus;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::{SqlitePersister, WalletStorageError};
use rusqlite::{params, OptionalExtension};

/// Sentinel wallet scope — "no parent wallet known", stored as a NULL
/// `identities.wallet_id`.
const SENTINEL: WalletId = [0u8; 32];

fn iid(byte: u8) -> Identifier {
    Identifier::from([byte; 32])
}

fn identity_entry(id: u8, index: Option<u32>) -> IdentityEntry {
    IdentityEntry {
        id: iid(id),
        balance: u64::from(id),
        revision: 1,
        identity_index: index,
        last_updated_balance_block_time: None,
        last_synced_keys_block_time: None,
        dpns_names: Vec::new(),
        contested_dpns_names: Vec::new(),
        status: IdentityStatus::Active,
        wallet_id: None,
        dashpay_profile: None,
        dashpay_payments: Default::default(),
        contact_profiles: Default::default(),
        ignored_senders: Default::default(),
    }
}

fn identity_cs<E, R>(entries: E, removed: R) -> PlatformWalletChangeSet
where
    E: IntoIterator<Item = IdentityEntry>,
    R: IntoIterator<Item = Identifier>,
{
    PlatformWalletChangeSet {
        identities: Some(IdentityChangeSet {
            identities: entries.into_iter().map(|e| (e.id, e)).collect(),
            removed: removed.into_iter().collect(),
        }),
        ..Default::default()
    }
}

/// Borrow the typed backend error out of a `PersistenceError`.
fn typed(err: &PersistenceError) -> &WalletStorageError {
    match err {
        PersistenceError::Backend { source, .. } => source
            .downcast_ref::<WalletStorageError>()
            .expect("backend source is a WalletStorageError"),
        PersistenceError::LockPoisoned => panic!("unexpected LockPoisoned"),
    }
}

fn backend_kind(err: &PersistenceError) -> PersistenceErrorKind {
    match err {
        PersistenceError::Backend { kind, .. } => *kind,
        PersistenceError::LockPoisoned => panic!("unexpected LockPoisoned"),
    }
}

/// Live (non-tombstoned) occupant of `(wallet_id, index)`, if any.
fn live_occupant(p: &SqlitePersister, wallet_id: &WalletId, index: u32) -> Option<[u8; 32]> {
    let conn = p.lock_conn_for_test();
    let wid_param: Option<&[u8]> = if *wallet_id == SENTINEL {
        None
    } else {
        Some(wallet_id.as_slice())
    };
    conn.query_row(
        "SELECT identity_id FROM identities \
         WHERE wallet_id IS ?1 AND wallet_index = ?2 AND tombstoned = 0",
        params![wid_param, i64::from(index)],
        |row| row.get::<_, Vec<u8>>(0),
    )
    .optional()
    .expect("query live occupant")
    .map(|raw| raw.try_into().expect("32-byte identity_id"))
}

/// `Some(tombstoned)` when the row exists at all.
fn row_tombstoned(p: &SqlitePersister, id: &Identifier) -> Option<bool> {
    let conn = p.lock_conn_for_test();
    conn.query_row(
        "SELECT tombstoned FROM identities WHERE identity_id = ?1",
        params![id.as_slice()],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .expect("query identity row")
    .map(|t| t != 0)
}

fn assert_index_conflict(err: &PersistenceError) {
    let typed = typed(err);
    assert!(
        matches!(typed, WalletStorageError::IdentityIndexConflict { .. }),
        "expected IdentityIndexConflict, got `{typed:?}`"
    );
    assert!(
        !typed.is_transient(),
        "a duplicate index is a caller bug, never retryable"
    );
    assert_eq!(
        typed.persistence_kind(),
        PersistenceErrorKind::Constraint,
        "duplicate index is an integrity violation, not a Fatal engine failure"
    );
    assert_eq!(backend_kind(err), PersistenceErrorKind::Constraint);
}

/// A second identity claiming a live slot is refused at write time, and
/// neither the occupant nor the intruder's absence is negotiable.
#[test]
fn duplicate_index_is_rejected_at_write_time() {
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0xC1);
    ensure_wallet_meta(&p, &w);
    p.store(w, identity_cs([identity_entry(0x01, Some(1))], []))
        .expect("first identity at index 1");

    let err = p
        .store(w, identity_cs([identity_entry(0x02, Some(1))], []))
        .expect_err("a second identity at index 1 must be refused");

    assert_index_conflict(&err);
    assert_eq!(
        live_occupant(&p, &w, 1),
        Some([0x01; 32]),
        "the resident identity must keep its slot"
    );
    assert_eq!(
        row_tombstoned(&p, &iid(0x02)),
        None,
        "the rejected identity must not reach disk at all"
    );
}

/// Rejecting one wallet's write leaves another wallet's data alone, and
/// the same index in two wallets is legal — slots are scoped per wallet.
#[test]
fn rejection_is_scoped_to_the_offending_wallet() {
    let (p, _tmp, _path) = fresh_persister();
    let a = wid(0xA1);
    let b = wid(0xB1);
    ensure_wallet_meta(&p, &a);
    ensure_wallet_meta(&p, &b);
    p.store(a, identity_cs([identity_entry(0x01, Some(1))], []))
        .expect("wallet a, index 1");
    p.store(b, identity_cs([identity_entry(0x11, Some(1))], []))
        .expect("wallet b may reuse index 1 — slots are per-wallet");

    let err = p
        .store(a, identity_cs([identity_entry(0x02, Some(1))], []))
        .expect_err("wallet a's slot 1 is taken");

    assert_index_conflict(&err);
    assert_eq!(
        live_occupant(&p, &b, 1),
        Some([0x11; 32]),
        "the untouched wallet keeps its identity"
    );
}

/// The rejection happens BEFORE the shared buffer is touched, so a
/// changeset already staged for the same wallet still flushes. Rejecting
/// inside the flush would drop it as collateral.
#[test]
fn rejected_duplicate_does_not_swallow_a_staged_changeset() {
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0xD1);
    ensure_wallet_meta(&p, &w);
    p.store(w, identity_cs([identity_entry(0x01, Some(1))], []))
        .expect("resident identity at index 1");

    // A transient flush failure restores the changeset to the shared
    // buffer — the same place an interleaved `store()` would sit.
    p.force_next_flush_to_fail(WalletStorageError::Sqlite(rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error {
            code: rusqlite::ErrorCode::DatabaseBusy,
            extended_code: rusqlite::ffi::SQLITE_BUSY,
        },
        Some("database is busy".into()),
    )));
    p.store(w, identity_cs([identity_entry(0x03, Some(2))], []))
        .expect_err("primed transient failure");

    let err = p
        .store(w, identity_cs([identity_entry(0x02, Some(1))], []))
        .expect_err("duplicate index must be refused");
    assert_index_conflict(&err);

    p.flush(w).expect("staged changeset still flushes");
    assert_eq!(
        live_occupant(&p, &w, 2),
        Some([0x03; 32]),
        "the staged write survived the rejection"
    );
    assert_eq!(row_tombstoned(&p, &iid(0x02)), None);
}

/// The probe keys on the FLUSH SCOPE, not on the incoming identity's own
/// stored `wallet_id`. Identity `0x02` sits unparented (NULL wallet_id);
/// writing it under scope `w` would promote it into `w`'s slot 1, which
/// `0x01` already holds. Looking in the row's own (NULL) bucket would
/// miss the occupant entirely.
#[test]
fn probe_keys_on_the_flush_scope_not_the_stored_wallet_id() {
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0xE1);
    ensure_wallet_meta(&p, &w);
    p.store(w, identity_cs([identity_entry(0x01, Some(1))], []))
        .expect("resident identity at index 1");
    ensure_identity(&p, &[0x02; 32], None);

    let err = p
        .store(w, identity_cs([identity_entry(0x02, Some(1))], []))
        .expect_err("promotion into a taken slot must be refused");

    assert_index_conflict(&err);
    assert_eq!(live_occupant(&p, &w, 1), Some([0x01; 32]));
}

/// A tombstoned row holds no slot: remove-then-re-register at the same
/// index is legitimate reuse, not a collision.
#[test]
fn tombstoned_row_frees_its_slot_for_reuse() {
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0xF1);
    ensure_wallet_meta(&p, &w);
    p.store(w, identity_cs([identity_entry(0x01, Some(1))], []))
        .expect("first identity at index 1");
    p.store(w, identity_cs([], [iid(0x01)]))
        .expect("remove the first identity");

    p.store(w, identity_cs([identity_entry(0x02, Some(1))], []))
        .expect("the freed slot must be reusable");

    assert_eq!(row_tombstoned(&p, &iid(0x01)), Some(true));
    assert_eq!(live_occupant(&p, &w, 1), Some([0x02; 32]));
}

/// Removal and re-registration merged into ONE changeset has a legal
/// final state (the tombstone lands in the same transaction), so the
/// probe must treat a removed id as holding no slot.
#[test]
fn tombstone_and_reinsert_in_one_changeset_is_accepted() {
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0xF2);
    ensure_wallet_meta(&p, &w);
    p.store(w, identity_cs([identity_entry(0x01, Some(1))], []))
        .expect("first identity at index 1");

    p.store(w, identity_cs([identity_entry(0x02, Some(1))], [iid(0x01)]))
        .expect("tombstone + reinsert at the same index is legal");

    assert_eq!(row_tombstoned(&p, &iid(0x01)), Some(true));
    assert_eq!(live_occupant(&p, &w, 1), Some([0x02; 32]));
}

/// Two identities claiming one slot inside a SINGLE changeset are both
/// refused — no winner is picked without evidence.
#[test]
fn colliding_entries_in_one_changeset_are_rejected_without_picking_a_winner() {
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0xF3);
    ensure_wallet_meta(&p, &w);

    let err = p
        .store(
            w,
            identity_cs(
                [identity_entry(0x01, Some(1)), identity_entry(0x02, Some(1))],
                [],
            ),
        )
        .expect_err("two identities cannot share one slot");

    assert_index_conflict(&err);
    assert_eq!(row_tombstoned(&p, &iid(0x01)), None);
    assert_eq!(row_tombstoned(&p, &iid(0x02)), None);
    assert_eq!(live_occupant(&p, &w, 1), None);
}

/// Distinct indices in one changeset are ordinary traffic.
#[test]
fn distinct_indices_in_one_changeset_are_accepted() {
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0xF4);
    ensure_wallet_meta(&p, &w);

    p.store(
        w,
        identity_cs(
            [identity_entry(0x01, Some(0)), identity_entry(0x02, Some(1))],
            [],
        ),
    )
    .expect("two identities at two indices");

    assert_eq!(live_occupant(&p, &w, 0), Some([0x01; 32]));
    assert_eq!(live_occupant(&p, &w, 1), Some([0x02; 32]));
}

/// Re-writing the SAME identity at its own index is an update, not a
/// collision.
#[test]
fn reupserting_the_same_identity_at_its_own_index_is_accepted() {
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0xF5);
    ensure_wallet_meta(&p, &w);
    p.store(w, identity_cs([identity_entry(0x01, Some(1))], []))
        .expect("initial write");

    let mut updated = identity_entry(0x01, Some(1));
    updated.balance = 42;
    p.store(w, identity_cs([updated], []))
        .expect("updating an identity in place is not a duplicate");

    assert_eq!(live_occupant(&p, &w, 1), Some([0x01; 32]));
}

/// Wallet-less identities carry NO index (`out_of_wallet_identities` is
/// keyed by identity id alone), so an indexed write under the sentinel
/// scope is refused.
#[test]
fn walletless_identity_carrying_an_index_is_rejected() {
    let (p, _tmp, _path) = fresh_persister();

    let err = p
        .store(SENTINEL, identity_cs([identity_entry(0x01, Some(3))], []))
        .expect_err("a wallet-less identity has no derivation index");

    let typed = typed(&err);
    assert!(
        matches!(typed, WalletStorageError::WalletlessIdentityIndex { .. }),
        "expected WalletlessIdentityIndex, got `{typed:?}`"
    );
    assert!(!typed.is_transient());
    assert_eq!(typed.persistence_kind(), PersistenceErrorKind::Constraint);
    assert_eq!(row_tombstoned(&p, &iid(0x01)), None);
}

/// The companion positive case: a wallet-less identity WITHOUT an index
/// is first-class and still stores.
#[test]
fn walletless_identity_without_an_index_is_accepted() {
    let (p, _tmp, _path) = fresh_persister();

    p.store(SENTINEL, identity_cs([identity_entry(0x01, None)], []))
        .expect("wallet-less identities are first-class");

    assert_eq!(row_tombstoned(&p, &iid(0x01)), Some(false));
}
