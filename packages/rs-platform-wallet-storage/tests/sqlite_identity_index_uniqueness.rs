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

use common::{
    ensure_identity, ensure_wallet_meta, fresh_persister, fresh_persister_with_mode, wid,
};

use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    IdentityChangeSet, IdentityEntry, PersistenceError, PersistenceErrorKind,
    PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::identity::IdentityStatus;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::{
    FlushMode, SqlitePersister, SqlitePersisterConfig, WalletStorageError,
};
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
        other => panic!("unexpected persistence error: {other}"),
    }
}

fn backend_kind(err: &PersistenceError) -> PersistenceErrorKind {
    match err {
        PersistenceError::Backend { kind, .. } => *kind,
        other => panic!("unexpected persistence error: {other}"),
    }
}

/// Occupant of `(wallet_id, index)`, if any.
fn live_occupant(p: &SqlitePersister, wallet_id: &WalletId, index: u32) -> Option<[u8; 32]> {
    let conn = p.lock_conn_for_test();
    let wid_param: Option<&[u8]> = if *wallet_id == SENTINEL {
        None
    } else {
        Some(wallet_id.as_slice())
    };
    conn.query_row(
        "SELECT identity_id FROM identities \
         WHERE wallet_id IS ?1 AND identity_index = ?2",
        params![wid_param, i64::from(index)],
        |row| row.get::<_, Vec<u8>>(0),
    )
    .optional()
    .expect("query live occupant")
    .map(|raw| raw.try_into().expect("32-byte identity_id"))
}

/// Whether the identity has a row on disk. A removal deletes it, so
/// "removed" and "never written" are the same observable state.
fn row_exists(p: &SqlitePersister, id: &Identifier) -> bool {
    let conn = p.lock_conn_for_test();
    conn.query_row(
        "SELECT 1 FROM identities WHERE identity_id = ?1",
        params![id.as_slice()],
        |_| Ok(()),
    )
    .optional()
    .expect("query identity row")
    .is_some()
}

/// Claim a slot by writing an `identities` row straight to the DB —
/// stands in for a cross-process peer (a sibling `SqlitePersister` on
/// the same file) taking the slot after a changeset was already
/// buffered against a free one. That is the only way disk state moves
/// under a buffered changeset now that in-process stores check the
/// buffer too.
fn peer_claims_slot(p: &SqlitePersister, wallet_id: &WalletId, id: &Identifier, index: u32) {
    let conn = p.lock_conn_for_test();
    conn.execute(
        "INSERT INTO identities (identity_id, wallet_id, identity_index, entry_blob) \
         VALUES (?1, ?2, ?3, X'00')",
        params![id.as_slice(), wallet_id.as_slice(), i64::from(index)],
    )
    .expect("peer claims slot");
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
    assert!(
        !row_exists(&p, &iid(0x02)),
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
    assert!(!row_exists(&p, &iid(0x02)));
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

/// A removed row holds no slot: remove-then-re-register at the same
/// index is legitimate reuse, not a collision.
#[test]
fn removed_row_frees_its_slot_for_reuse() {
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0xF1);
    ensure_wallet_meta(&p, &w);
    p.store(w, identity_cs([identity_entry(0x01, Some(1))], []))
        .expect("first identity at index 1");
    p.store(w, identity_cs([], [iid(0x01)]))
        .expect("remove the first identity");

    p.store(w, identity_cs([identity_entry(0x02, Some(1))], []))
        .expect("the freed slot must be reusable");

    assert!(!row_exists(&p, &iid(0x01)));
    assert_eq!(live_occupant(&p, &w, 1), Some([0x02; 32]));
}

/// Removal and re-registration merged into ONE changeset has a legal
/// final state (the delete lands in the same transaction), so the probe
/// must treat a removed id as holding no slot.
#[test]
fn removal_and_reinsert_in_one_changeset_is_accepted() {
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0xF2);
    ensure_wallet_meta(&p, &w);
    p.store(w, identity_cs([identity_entry(0x01, Some(1))], []))
        .expect("first identity at index 1");

    p.store(w, identity_cs([identity_entry(0x02, Some(1))], [iid(0x01)]))
        .expect("removal + reinsert at the same index is legal");

    assert!(!row_exists(&p, &iid(0x01)));
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
    assert!(!row_exists(&p, &iid(0x01)));
    assert!(!row_exists(&p, &iid(0x02)));
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

/// An occupant that the SAME changeset moves to another index has
/// vacated its old slot by the time the changeset lands, exactly like
/// one it removes. Judging the final state, not the starting one, is
/// what makes the guard a uniqueness rule rather than a freeze.
#[test]
fn an_occupant_reindexed_in_the_same_changeset_frees_its_old_slot() {
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0xF7);
    ensure_wallet_meta(&p, &w);
    p.store(w, identity_cs([identity_entry(0x01, Some(1))], []))
        .expect("initial write");

    p.store(
        w,
        identity_cs(
            [identity_entry(0x01, Some(2)), identity_entry(0x02, Some(1))],
            [],
        ),
    )
    .expect("A moves to 2 and B takes 1 — the final state is unique");

    assert_eq!(live_occupant(&p, &w, 1), Some([0x02; 32]));
    assert_eq!(live_occupant(&p, &w, 2), Some([0x01; 32]));
}

/// A two-way swap: both identities block each other's target slot on
/// disk, and both vacate it in the same changeset.
#[test]
fn two_identities_swapping_indices_in_one_changeset_are_accepted() {
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0xF8);
    ensure_wallet_meta(&p, &w);
    p.store(
        w,
        identity_cs(
            [identity_entry(0x01, Some(1)), identity_entry(0x02, Some(2))],
            [],
        ),
    )
    .expect("initial write");

    p.store(
        w,
        identity_cs(
            [identity_entry(0x01, Some(2)), identity_entry(0x02, Some(1))],
            [],
        ),
    )
    .expect("a swap ends with one identity per slot");

    assert_eq!(live_occupant(&p, &w, 1), Some([0x02; 32]));
    assert_eq!(live_occupant(&p, &w, 2), Some([0x01; 32]));
}

/// Dropping an occupant's index entirely (it becomes an out-of-wallet
/// identity) frees the slot on the same terms — the row survives, the
/// claim does not.
#[test]
fn an_occupant_losing_its_index_in_the_same_changeset_frees_its_slot() {
    let (p, _tmp, _path) = fresh_persister();
    let w = wid(0xF9);
    ensure_wallet_meta(&p, &w);
    p.store(w, identity_cs([identity_entry(0x01, Some(1))], []))
        .expect("initial write");

    p.store(
        w,
        identity_cs(
            [identity_entry(0x01, None), identity_entry(0x02, Some(1))],
            [],
        ),
    )
    .expect("A gives up its index in the same changeset that B claims it");

    assert_eq!(live_occupant(&p, &w, 1), Some([0x02; 32]));
    assert!(
        row_exists(&p, &iid(0x01)),
        "A is still a live row, just no longer in a wallet slot"
    );
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
    assert!(!row_exists(&p, &iid(0x01)));
}

/// The companion positive case: a wallet-less identity WITHOUT an index
/// is first-class and still stores.
#[test]
fn walletless_identity_without_an_index_is_accepted() {
    let (p, _tmp, _path) = fresh_persister();

    p.store(SENTINEL, identity_cs([identity_entry(0x01, None)], []))
        .expect("wallet-less identities are first-class");

    assert!(row_exists(&p, &iid(0x01)));
}

/// A store checks disk and buffer as they are at that instant. A
/// cross-process peer can claim the slot afterwards, so the flush
/// re-runs the check against what is actually about to land —
/// otherwise manual mode writes the duplicate the store-time check
/// exists to prevent.
#[test]
fn flush_rejects_a_duplicate_that_a_peer_created_under_the_buffer() {
    let (p, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0xB7);
    ensure_wallet_meta(&p, &w);
    p.store(w, identity_cs([identity_entry(0x02, Some(1))], []))
        .expect("valid against a clean disk");
    peer_claims_slot(&p, &w, &iid(0x01), 1);

    let err = p.flush(w).expect_err("the slot is no longer free");

    assert_index_conflict(&err);
    assert_eq!(
        live_occupant(&p, &w, 1),
        Some([0x01; 32]),
        "the whole transaction rolls back — the peer's row stands"
    );
    assert!(!row_exists(&p, &iid(0x02)));

    // A fatal flush failure drops that wallet's buffer rather than
    // restoring an unflushable changeset, so the retry is a no-op
    // instead of the same failure forever.
    p.flush(w).expect("the poisoned changeset is not retried");
    assert_eq!(live_occupant(&p, &w, 1), Some([0x01; 32]));
}

/// The slot check runs against the buffer as well as the disk, and both
/// run inside the buffer's own critical section. A second claimant is
/// therefore refused at store time — while the error can still be
/// attributed to the caller that caused it — instead of merging into a
/// contradictory changeset the flush later drops whole.
#[test]
fn a_slot_held_by_a_buffered_write_refuses_a_second_claimant() {
    let (p, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0xB8);
    ensure_wallet_meta(&p, &w);
    p.store(w, identity_cs([identity_entry(0x01, Some(1))], []))
        .expect("first claim on a free slot");

    let err = p
        .store(w, identity_cs([identity_entry(0x02, Some(1))], []))
        .expect_err("the slot is held by a buffered write");

    assert_index_conflict(&err);
    p.flush(w)
        .expect("the buffered changeset is still flushable");
    assert_eq!(
        live_occupant(&p, &w, 1),
        Some([0x01; 32]),
        "the first caller's write is untouched by the rejection"
    );
    assert!(
        !row_exists(&p, &iid(0x02)),
        "the rejected changeset never reached the buffer"
    );
}

/// A buffered removal frees the slot it names: the check reads the
/// merged view exactly as the flush will apply it, and the flush inserts
/// before it deletes. Reclaiming the slot in a later store is legitimate
/// reuse, not a collision.
#[test]
fn a_buffered_removal_frees_its_slot_for_a_later_store() {
    let (p, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0xB9);
    ensure_wallet_meta(&p, &w);
    p.store(w, identity_cs([identity_entry(0x01, Some(1))], []))
        .expect("first claim on a free slot");

    p.store(w, identity_cs([identity_entry(0x02, Some(1))], [iid(0x01)]))
        .expect("the occupant is removed by the same buffered changeset");

    p.flush(w).expect("the merged changeset is consistent");
    assert_eq!(live_occupant(&p, &w, 1), Some([0x02; 32]));
    assert!(!row_exists(&p, &iid(0x01)));
}

/// One wallet's rejected flush is one wallet's problem: `commit_writes`
/// gives every dirty wallet its own transaction, so a neighbour's
/// legitimate writes land in the same pass.
#[test]
fn flush_rejection_leaves_other_wallets_untouched() {
    let (p, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
    let poisoned = wid(0xC7);
    let healthy = wid(0xD7);
    ensure_wallet_meta(&p, &poisoned);
    ensure_wallet_meta(&p, &healthy);
    p.store(poisoned, identity_cs([identity_entry(0x02, Some(1))], []))
        .expect("store");
    peer_claims_slot(&p, &poisoned, &iid(0x01), 1);
    p.store(healthy, identity_cs([identity_entry(0x11, Some(1))], []))
        .expect("store");

    let report = p.commit_writes().expect("commit_writes enumerates fine");

    assert_eq!(
        report.succeeded,
        vec![healthy],
        "the healthy wallet commits"
    );
    assert_eq!(report.failed.len(), 1, "exactly one wallet fails");
    assert_eq!(report.failed[0].0, poisoned);
    assert_index_conflict(&report.failed[0].1);
    assert!(report.still_pending.is_empty());
    assert_eq!(
        live_occupant(&p, &healthy, 1),
        Some([0x11; 32]),
        "the neighbour's write is not collateral"
    );
    assert_eq!(live_occupant(&p, &poisoned, 1), Some([0x01; 32]));
}

/// Pending writes that can never be persisted must not make a wallet
/// undeletable — deleting it is the remedy for exactly that state. The
/// pre-delete flush drops them and the cascade proceeds.
#[test]
fn delete_wallet_proceeds_despite_unpersistable_pending_writes() {
    let (p, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0x5A);
    ensure_wallet_meta(&p, &w);
    p.store(w, identity_cs([identity_entry(0x01, Some(1))], []))
        .expect("store");
    peer_claims_slot(&p, &w, &iid(0x02), 1);

    let report = p
        .delete_wallet_skip_backup(w)
        .expect("an unflushable buffer must not block the delete");

    assert_eq!(report.wallet_id, w);
    assert!(!row_exists(&p, &iid(0x01)));
    assert!(
        !row_exists(&p, &iid(0x02)),
        "the peer's row went with the wallet's cascade"
    );
    let conn = p.lock_conn_for_test();
    let wallets: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM wallets WHERE wallet_id = ?1",
            params![w.as_slice()],
            |row| row.get(0),
        )
        .expect("count wallets");
    assert_eq!(wallets, 0, "the wallet is gone");
}

/// Dropping those pending writes is only justified once the wallet is
/// actually gone. The carve-out fires long before the cascade commits —
/// the auto-backup, the `BEGIN EXCLUSIVE`, the cascade and its commit
/// can all still fail — and if one does, the wallet is still here, so
/// its staged writes must be too. They may bundle sub-changesets that
/// have nothing to do with the offending identity entry.
#[test]
fn a_delete_that_fails_after_the_carve_out_keeps_the_pending_writes() {
    let tmp = common::secure_tempdir().expect("tempdir");
    let path = tmp.path().join("wallet.db");
    // No auto-backup directory: `delete_wallet` then fails in
    // `run_auto_backup`, the step right after the carve-out.
    let p = SqlitePersister::open(
        SqlitePersisterConfig::new(&path)
            .with_flush_mode(FlushMode::Manual)
            .with_auto_backup_dir(None),
    )
    .expect("open persister");
    let w = wid(0x5C);
    ensure_wallet_meta(&p, &w);
    p.store(w, identity_cs([identity_entry(0x01, Some(1))], []))
        .expect("store");
    peer_claims_slot(&p, &w, &iid(0x02), 1);

    let err = p
        .delete_wallet(w)
        .expect_err("no auto-backup directory is configured");
    assert!(
        matches!(err, WalletStorageError::AutoBackupDisabled { .. }),
        "the delete must fail at the backup, after the carve-out: `{err:?}`"
    );

    let wallets: i64 = {
        let conn = p.lock_conn_for_test();
        conn.query_row(
            "SELECT COUNT(*) FROM wallets WHERE wallet_id = ?1",
            params![w.as_slice()],
            |row| row.get(0),
        )
        .expect("count wallets")
    };
    assert_eq!(wallets, 1, "the delete aborted — the wallet is still here");

    // The staged write is not merely present, it is intact: clear what
    // made it unflushable and it lands exactly as it was staged.
    {
        let conn = p.lock_conn_for_test();
        conn.execute(
            "DELETE FROM identities WHERE identity_id = ?1",
            params![iid(0x02).as_slice()],
        )
        .expect("drop the peer's row");
    }
    p.flush(w).expect("the staged changeset is still flushable");
    assert_eq!(
        live_occupant(&p, &w, 1),
        Some([0x01; 32]),
        "the carve-out dropped a changeset for a wallet that still exists"
    );
}
