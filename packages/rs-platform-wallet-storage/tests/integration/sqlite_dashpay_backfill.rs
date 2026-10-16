//! Cross-launch persistence of the DashPay coreHeight backfill record.
//!
//! dashpay/platform#4302: the guard that stops the DIP-15 §12.6 rescan from
//! re-lowering the cursor lived only in memory while the lowered cursor was
//! durable, so every launch restored a cursor inside the climb and rewound
//! again. These tests drive the public [`PlatformWalletPersistence`] surface
//! and assert on what `load()` hands back, plus the stored cursor the record
//! vouches for.

use crate::common;

use std::path::Path;

use common::{
    ensure_wallet_meta, fresh_persister, fresh_persister_with_mode, store_and_flush, wid,
};
use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    persists_backfill_coverage, ClientWalletStartState, CoreChangeSet,
    DashPayBackfillCoveredContact, DashPayBackfillRecord, IdentityChangeSet, IdentityEntry,
    PlatformWalletChangeSet, PlatformWalletPersistence, SentContactRequestKey,
};
use platform_wallet::wallet::identity::{ContactRequest, IdentityStatus};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::migrations as mig;
use platform_wallet_storage::sqlite::schema::{blob, contacts, dashpay_backfill};
use platform_wallet_storage::{
    FlushMode, LoadCtx, SqlitePersister, SqlitePersisterConfig, WalletStorageError,
};
use rusqlite::{params, Connection, ErrorCode};

fn covered(owner: u8, contact: u8, account_index: u32, from: u32) -> DashPayBackfillCoveredContact {
    DashPayBackfillCoveredContact {
        owner: Identifier::from([owner; 32]),
        contact: Identifier::from([contact; 32]),
        account_index,
        covered_from: from,
    }
}

fn record(
    floor: u32,
    rewound_from: u32,
    entries: Vec<DashPayBackfillCoveredContact>,
) -> DashPayBackfillRecord {
    DashPayBackfillRecord::from_entries(floor, rewound_from, entries)
}

fn record_cs(record: DashPayBackfillRecord) -> PlatformWalletChangeSet {
    PlatformWalletChangeSet {
        dashpay_backfill: Some(record),
        ..Default::default()
    }
}

fn cursor_cs(height: u32, rewind: bool) -> PlatformWalletChangeSet {
    PlatformWalletChangeSet {
        core: Some(CoreChangeSet {
            synced_height: Some(height),
            synced_height_is_rewind: rewind,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn stage(persister: &SqlitePersister, wallet_id: WalletId, cs: PlatformWalletChangeSet) {
    persister.store(wallet_id, cs).expect("store");
}

fn stored_cursor(persister: &SqlitePersister, wallet_id: &WalletId) -> Option<u32> {
    persister
        .lock_conn_for_test()
        .query_row(
            "SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| row.get(0),
        )
        .expect("core_sync_state row")
}

fn stored_records(persister: &SqlitePersister, wallet_id: &WalletId) -> i64 {
    persister
        .lock_conn_for_test()
        .query_row(
            "SELECT COUNT(*) FROM dashpay_backfill WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| row.get(0),
        )
        .expect("count dashpay_backfill")
}

/// Reopen the database at `path` and return what `load()` restored for
/// `wallet_id`. The seeding persister must already be dropped — the
/// process-wide open-path registry refuses a second live persister on a path.
fn reload_wallet(path: &Path, wallet_id: &WalletId) -> ClientWalletStartState {
    let persister =
        SqlitePersister::open(SqlitePersisterConfig::new(path)).expect("reopen persister");
    let mut state = persister.load().expect("load");
    state
        .wallets
        .remove(wallet_id)
        .expect("the seeded wallet must come back from load()")
}

/// A transient SQLite failure, so a failed flush restores its buffer for retry.
fn busy_error() -> WalletStorageError {
    WalletStorageError::Sqlite(rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error {
            code: ErrorCode::DatabaseBusy,
            extended_code: rusqlite::ffi::SQLITE_BUSY,
        },
        Some("database is busy".into()),
    ))
}

fn identity_entry(id: Identifier, wallet_id: WalletId, index: u32) -> IdentityEntry {
    IdentityEntry {
        id,
        balance: 1_000,
        revision: 1,
        identity_index: Some(index),
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

#[test]
fn should_hold_backfill_coverage_durably_only_when_store_commits_inline() {
    let (immediate, _tmp, _path) = fresh_persister();
    assert!(immediate.stores_dashpay_backfill());
    assert!(
        persists_backfill_coverage(&immediate),
        "an Immediate-mode store is the commit, so coverage may be durable"
    );

    let (manual, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
    assert!(
        manual.stores_dashpay_backfill(),
        "a Manual-mode store can still write (and so invalidate) a record"
    );
    assert!(
        !persists_backfill_coverage(&manual),
        "a buffered round can still be dropped at flush, so coverage stays session-only"
    );
}

/// The regression #4302 names: a backfill recorded in one process must still
/// be known in the next, so it resumes instead of rewinding again.
#[test]
fn should_restore_the_backfill_record_across_a_reopen() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x51);
    ensure_wallet_meta(&persister, &w);
    let written = record(
        1_000,
        5_000,
        vec![covered(0x01, 0x02, 0, 1_200), covered(0x01, 0x03, 1, 1_000)],
    );
    store_and_flush(&persister, w, record_cs(written.clone()));
    drop(persister);

    let restored = reload_wallet(&path, &w).dashpay_backfill;

    assert_eq!(
        restored, written,
        "the record must survive the reopen exactly"
    );
    assert_eq!(
        restored.covered_from(
            &Identifier::from([0x01; 32]),
            &Identifier::from([0x03; 32]),
            1
        ),
        Some(1_000)
    );
}

/// Absence reads as "no backfill recorded", which upstream answers by
/// re-running the backfill — the behaviour every wallet had before.
#[test]
fn should_restore_an_empty_record_for_a_wallet_that_never_backfilled() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x52);
    ensure_wallet_meta(&persister, &w);
    drop(persister);

    assert!(reload_wallet(&path, &w).dashpay_backfill.is_empty());
}

/// A cover set that is not a whole number of entries must not be read as a
/// shorter one: the reader treats it as "no record", which only costs a
/// re-run. (The CHECK constraint keeps one off disk, and the open-time
/// integrity check refuses a file that holds one anyway, so this pins the
/// reader's own fallback on a raw connection.)
#[test]
fn should_read_a_torn_cover_set_as_no_record() {
    let mut conn = Connection::open_in_memory().expect("open in-memory db");
    mig::run(&mut conn).expect("migrate");
    let w = wid(0x5D);
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![w.as_slice()],
    )
    .expect("insert wallet");
    conn.pragma_update(None, "ignore_check_constraints", true)
        .expect("bypass CHECK");
    conn.execute(
        "INSERT INTO dashpay_backfill (wallet_id, floor, rewound_from, covered) \
         VALUES (?1, 1000, 5000, ?2)",
        params![w.as_slice(), vec![0u8; 71]],
    )
    .expect("insert torn record");

    assert!(dashpay_backfill::load_for_wallet(&conn, &w)
        .expect("backfill load")
        .is_empty());
}

/// Every write carries the whole record: an account missing from the new one
/// must not survive from the old one, or it would never be backfilled again.
#[test]
fn should_replace_the_stored_record_wholesale() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x53);
    ensure_wallet_meta(&persister, &w);
    store_and_flush(
        &persister,
        w,
        record_cs(record(
            1_000,
            5_000,
            vec![covered(0x01, 0x02, 0, 1_200), covered(0x01, 0x03, 0, 1_100)],
        )),
    );
    let replacement = record(900, 6_000, vec![covered(0x01, 0x02, 0, 900)]);
    store_and_flush(&persister, w, record_cs(replacement.clone()));
    drop(persister);

    assert_eq!(reload_wallet(&path, &w).dashpay_backfill, replacement);
}

/// An empty record is how upstream retracts coverage it can no longer vouch
/// for; it has to clear the stored record.
#[test]
fn should_clear_the_stored_record_when_an_empty_one_is_written() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x54);
    ensure_wallet_meta(&persister, &w);
    store_and_flush(
        &persister,
        w,
        record_cs(record(1_000, 5_000, vec![covered(0x01, 0x02, 0, 1_000)])),
    );
    store_and_flush(&persister, w, record_cs(DashPayBackfillRecord::default()));
    assert_eq!(stored_records(&persister, &w), 0);
    drop(persister);

    assert!(reload_wallet(&path, &w).dashpay_backfill.is_empty());
}

/// The rewind the record vouches for must actually lower the stored cursor;
/// an ordinary advance below the stored cursor must still lose to the max.
#[test]
fn should_lower_the_stored_cursor_only_on_a_rewind() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0x55);
    ensure_wallet_meta(&persister, &w);

    store_and_flush(&persister, w, cursor_cs(5_000, false));
    store_and_flush(&persister, w, cursor_cs(4_000, false));
    assert_eq!(
        stored_cursor(&persister, &w),
        Some(5_000),
        "an advance keeps the monotonic max"
    );

    store_and_flush(&persister, w, cursor_cs(1_000, true));
    assert_eq!(
        stored_cursor(&persister, &w),
        Some(1_000),
        "a rewind overwrites the stored cursor"
    );

    store_and_flush(&persister, w, cursor_cs(1_500, false));
    assert_eq!(
        stored_cursor(&persister, &w),
        Some(1_500),
        "the climb after a rewind advances from the lowered cursor"
    );
}

/// The round the rescan writes — the lowered cursor plus the record that
/// vouches for it — lands as one unit, and a relaunch reads both back.
#[test]
fn should_restore_the_rewound_cursor_beside_its_record() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x56);
    ensure_wallet_meta(&persister, &w);
    store_and_flush(&persister, w, cursor_cs(5_000, false));
    let written = record(1_000, 5_000, vec![covered(0x01, 0x02, 0, 1_000)]);
    store_and_flush(
        &persister,
        w,
        PlatformWalletChangeSet {
            dashpay_backfill: Some(written.clone()),
            ..cursor_cs(1_000, true)
        },
    );
    drop(persister);

    let wallet = reload_wallet(&path, &w);
    assert_eq!(wallet.dashpay_backfill, written);
    assert_eq!(
        wallet.wallet_info.metadata.synced_height, 1_000,
        "a relaunch must resume the climb from the lowered cursor"
    );
}

/// A buffered record is last-write-wins like every other record write, so an
/// invalidation staged after a record leaves nothing on disk at flush.
#[test]
fn should_flush_only_the_last_buffered_record_in_manual_mode() {
    let (persister, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0x57);
    ensure_wallet_meta(&persister, &w);
    stage(
        &persister,
        w,
        record_cs(record(1_000, 5_000, vec![covered(0x01, 0x02, 0, 1_000)])),
    );
    stage(&persister, w, record_cs(DashPayBackfillRecord::default()));
    persister.flush(w).expect("flush");
    drop(persister);

    assert!(reload_wallet(&path, &w).dashpay_backfill.is_empty());
}

/// The record is wallet-scoped state, so deleting the wallet takes it with it.
#[test]
fn should_drop_the_backfill_record_when_its_wallet_is_deleted() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0x58);
    ensure_wallet_meta(&persister, &w);
    store_and_flush(
        &persister,
        w,
        record_cs(record(1_000, 5_000, vec![covered(0x01, 0x02, 0, 1_000)])),
    );

    persister.delete_wallet(w).expect("delete_wallet");

    assert_eq!(stored_records(&persister, &w), 0);
}

/// A buffered rewind has to survive the advances merged around it: the
/// advance before it is replaced, the climb after it starts from it.
#[test]
fn should_flush_a_buffered_rewind_merged_between_advances() {
    let (persister, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0x59);
    ensure_wallet_meta(&persister, &w);
    stage(&persister, w, cursor_cs(5_000, false));
    persister.flush(w).expect("seed flush");

    stage(&persister, w, cursor_cs(6_000, false));
    stage(&persister, w, cursor_cs(1_000, true));
    stage(&persister, w, cursor_cs(1_500, false));
    persister.flush(w).expect("flush");

    assert_eq!(stored_cursor(&persister, &w), Some(1_500));
}

/// A failed flush puts its changeset back in front of whatever arrived
/// meanwhile, so the rewind keeps its place on both sides of the restore.
#[test]
fn should_keep_a_rewind_in_order_across_a_failed_flush() {
    for (restored, arrived, expected) in [
        // The restored round is the rewind; the climb lands after it.
        ((1_000, true), (1_200, false), 1_200),
        // The restored round is an advance; the rewind replaces it.
        ((6_000, false), (1_000, true), 1_000),
    ] {
        let (persister, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
        let w = wid(0x5A);
        ensure_wallet_meta(&persister, &w);
        stage(&persister, w, cursor_cs(5_000, false));
        persister.flush(w).expect("seed flush");

        stage(&persister, w, cursor_cs(restored.0, restored.1));
        persister.force_next_flush_to_fail(busy_error());
        persister.flush(w).expect_err("injected transient failure");
        stage(&persister, w, cursor_cs(arrived.0, arrived.1));
        persister.flush(w).expect("retry");

        assert_eq!(
            stored_cursor(&persister, &w),
            Some(expected),
            "restored {restored:?}, then {arrived:?}"
        );
    }
}

/// An identity removed and re-added inside one buffer splits it into
/// segments that apply in order: a rewind staged in the earlier segment must
/// land before the advance staged in the later one.
#[test]
fn should_apply_a_rewind_and_its_climb_in_segment_order() {
    let (persister, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0x5B);
    ensure_wallet_meta(&persister, &w);
    let owner = Identifier::from([0x61; 32]);
    let mut upsert = IdentityChangeSet::default();
    upsert.identities.insert(owner, identity_entry(owner, w, 0));
    let mut removal = IdentityChangeSet::default();
    removal.removed.insert(owner);

    stage(
        &persister,
        w,
        PlatformWalletChangeSet {
            identities: Some(upsert.clone()),
            ..cursor_cs(5_000, false)
        },
    );
    persister.flush(w).expect("seed flush");

    stage(
        &persister,
        w,
        PlatformWalletChangeSet {
            identities: Some(removal),
            ..cursor_cs(1_000, true)
        },
    );
    stage(
        &persister,
        w,
        PlatformWalletChangeSet {
            identities: Some(upsert),
            ..cursor_cs(1_200, false)
        },
    );
    persister.flush(w).expect("flush");

    assert_eq!(stored_cursor(&persister, &w), Some(1_200));
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

/// An upgraded database holds facts no earlier schema could: every
/// established contact reads back with no outbound marker (one rebuild) and
/// every wallet with no backfill recorded (one re-run).
#[test]
fn should_upgrade_a_v018_database_to_no_marker_and_no_backfill() {
    let mut conn = Connection::open_in_memory().expect("open in-memory db");
    conn.pragma_update(None, "foreign_keys", true)
        .expect("enable foreign keys");
    mig::runner()
        .set_target(refinery::Target::Version(18))
        .run(&mut conn)
        .expect("migrate to the last pre-V019 schema");

    let w = wid(0x5C);
    let owner = Identifier::from([0x71; 32]);
    let contact = Identifier::from([0x72; 32]);
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![w.as_slice()],
    )
    .expect("insert wallet");
    conn.execute(
        "INSERT INTO contacts \
            (wallet_id, owner_id, contact_id, state, outgoing_request, incoming_request) \
         VALUES (?1, ?2, ?3, 'established', ?4, ?5)",
        params![
            w.as_slice(),
            owner.as_slice(),
            contact.as_slice(),
            blob::encode(&contact_request(owner, contact)).expect("encode outgoing"),
            blob::encode(&contact_request(contact, owner)).expect("encode incoming"),
        ],
    )
    .expect("insert established contact");

    mig::run(&mut conn).expect("migrate to the newest version");

    let state =
        contacts::load_state_for_test(&conn, &w, &LoadCtx::strict()).expect("contacts load");
    let restored = state
        .established
        .get(&SentContactRequestKey {
            owner_id: owner,
            recipient_id: contact,
        })
        .expect("the established contact survives the upgrade");
    assert_eq!(restored.external_account_reference, None);
    assert!(dashpay_backfill::load_for_wallet(&conn, &w)
        .expect("backfill load")
        .is_empty());
}
