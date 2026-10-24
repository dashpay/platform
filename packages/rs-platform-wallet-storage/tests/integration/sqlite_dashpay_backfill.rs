//! Cross-launch persistence of the DashPay coreHeight backfill record.
//!
//! dashpay/platform#4302: the guard that stops the DIP-15 §12.6 rescan from
//! re-lowering the cursor lived only in memory while the lowered cursor was
//! durable, so every launch restored a cursor inside the climb and rewound
//! again. These tests drive the public [`PlatformWalletPersistence`] surface
//! and assert on what `load()` hands back, plus the stored cursor the record
//! vouches for. The record has no table: its extent is two `core_sync_state`
//! columns and each covered entry is `dashpay_covered_from` on the receiving
//! account's `account_registrations` row.

use crate::common;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use common::{
    ensure_wallet_meta, fresh_persister, fresh_persister_with_mode, store_and_flush, wid,
};
use dpp::prelude::Identifier;
use key_wallet::account::AccountType;
use key_wallet::bip32::ExtendedPubKey;
use platform_wallet::changeset::{
    persists_backfill_coverage, AccountRegistrationEntry, ClientWalletStartState, CoreChangeSet,
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
use rusqlite::trace::{TraceEvent, TraceEventCodes};
use rusqlite::{params, Connection, ErrorCode, OptionalExtension};

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

/// A distinct extended public key per `seed` byte.
fn xpub(seed: u8) -> ExtendedPubKey {
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::Wallet;
    Wallet::from_seed_bytes(
        [seed; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet")
    .accounts
    .all_accounts()
    .first()
    .expect("at least one account")
    .account_xpub
}

/// The receiving account a covered entry names, registered under `xpub_seed`.
fn receiving_account(
    entry: &DashPayBackfillCoveredContact,
    xpub_seed: u8,
) -> AccountRegistrationEntry {
    AccountRegistrationEntry {
        account_type: AccountType::DashpayReceivingFunds {
            index: entry.account_index,
            user_identity_id: entry.owner.to_buffer(),
            friend_identity_id: entry.contact.to_buffer(),
        },
        account_xpub: xpub(xpub_seed),
    }
}

/// The record plus the receiving accounts it covers, registered in the same
/// round as the wallet does before it can cover one.
fn record_cs(record: DashPayBackfillRecord) -> PlatformWalletChangeSet {
    PlatformWalletChangeSet {
        account_registrations: record
            .covered()
            .iter()
            .map(|entry| receiving_account(entry, 0x01))
            .collect(),
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

/// What is on file for the record: its stored extent (`None` when unset or
/// when the wallet has no `core_sync_state` row) and how many account rows
/// carry a covered height.
fn stored_record_parts(
    persister: &SqlitePersister,
    wallet_id: &WalletId,
) -> (Option<(i64, i64)>, i64) {
    let conn = persister.lock_conn_for_test();
    let extent: Option<(Option<i64>, Option<i64>)> = conn
        .query_row(
            "SELECT dashpay_backfill_floor, dashpay_backfill_rewound_from \
             FROM core_sync_state WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .expect("read extent");
    let covered = conn
        .query_row(
            "SELECT COUNT(*) FROM account_registrations \
             WHERE wallet_id = ?1 AND dashpay_covered_from IS NOT NULL",
            params![wallet_id.as_slice()],
            |row| row.get(0),
        )
        .expect("count covered accounts");
    let extent = match extent {
        Some((Some(floor), Some(rewound_from))) => Some((floor, rewound_from)),
        _ => None,
    };
    (extent, covered)
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

/// Covered accounts with no extent beside them are not a record the writer
/// ever leaves; the reader takes them as "no record", which only costs a
/// re-run, rather than inventing an extent.
#[test]
fn should_read_coverage_without_an_extent_as_no_record() {
    let mut conn = Connection::open_in_memory().expect("open in-memory db");
    mig::run(&mut conn).expect("migrate");
    let w = wid(0x5D);
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![w.as_slice()],
    )
    .expect("insert wallet");
    conn.execute(
        "INSERT INTO account_registrations \
            (wallet_id, account_type, account_index, user_identity_id, friend_identity_id, \
             account_xpub_bytes, dashpay_covered_from) \
         VALUES (?1, 'dashpay_receiving', 0, ?2, ?3, x'00', 1000)",
        params![
            w.as_slice(),
            [0x01u8; 32].as_slice(),
            [0x02u8; 32].as_slice()
        ],
    )
    .expect("insert covered account");

    assert!(dashpay_backfill::load_for_wallet(&conn, &w)
        .expect("backfill load")
        .is_empty());
}

/// A covered entry is stored on its receiving account's row, so an entry whose
/// account has no row is dropped — re-armed, never lost the other way. When
/// nothing is left the extent goes too, and the record reads back empty.
#[test]
fn should_drop_coverage_for_a_receiving_account_with_no_row() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x60);
    ensure_wallet_meta(&persister, &w);
    let registered = covered(0x01, 0x02, 0, 1_200);
    let unregistered = covered(0x01, 0x09, 0, 1_000);
    store_and_flush(
        &persister,
        w,
        PlatformWalletChangeSet {
            account_registrations: vec![receiving_account(&registered, 0x01)],
            dashpay_backfill: Some(record(1_000, 5_000, vec![registered, unregistered])),
            ..Default::default()
        },
    );
    assert_eq!(
        stored_record_parts(&persister, &w),
        (Some((1_000, 5_000)), 1)
    );

    store_and_flush(
        &persister,
        w,
        PlatformWalletChangeSet {
            dashpay_backfill: Some(record(900, 5_000, vec![unregistered])),
            ..Default::default()
        },
    );
    assert_eq!(stored_record_parts(&persister, &w), (None, 0));
    drop(persister);

    assert!(reload_wallet(&path, &w).dashpay_backfill.is_empty());
}

/// Re-registering an account (a rotated xpub) rewrites its row; the covered
/// height on that row must survive, or the next launch would backfill again.
#[test]
fn should_keep_coverage_when_its_receiving_account_is_re_registered() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x61);
    ensure_wallet_meta(&persister, &w);
    let entry = covered(0x01, 0x02, 0, 1_200);
    let written = record(1_000, 5_000, vec![entry]);
    store_and_flush(&persister, w, record_cs(written.clone()));
    store_and_flush(
        &persister,
        w,
        PlatformWalletChangeSet {
            account_registrations: vec![receiving_account(&entry, 0x02)],
            ..Default::default()
        },
    );
    drop(persister);

    assert_eq!(reload_wallet(&path, &w).dashpay_backfill, written);
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
    assert_eq!(stored_record_parts(&persister, &w), (None, 0));
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
            core: cursor_cs(1_000, true).core,
            ..record_cs(written.clone())
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

/// The concurrent write [`commit_rewind_mid_load`] makes: where it goes, and
/// what it reported once it ran. A trace callback is a plain `fn`, so its
/// state is a static; only this file's snapshot test arms it.
static MID_LOAD_WRITE: Mutex<MidLoadWrite> = Mutex::new(MidLoadWrite::Disarmed);

enum MidLoadWrite {
    Disarmed,
    Armed(PathBuf, WalletId),
    Done(Result<(), String>),
}

/// Trace callback on the loading connection. On the first `asset_locks`
/// read — after `load()` has read the cursor, before it reads the coverage —
/// another connection commits a rewind to 1 000 and the coverage that
/// vouches for it, as a second process would. Errors are reported through
/// the static, since rusqlite swallows a panic raised inside the callback.
fn commit_rewind_mid_load(event: TraceEvent<'_>) {
    let TraceEvent::Stmt(_, sql) = event else {
        return;
    };
    if !sql.contains("FROM asset_locks") {
        return;
    }
    let mut state = MID_LOAD_WRITE.lock().unwrap_or_else(|p| p.into_inner());
    let MidLoadWrite::Armed(path, wallet_id) = &*state else {
        return;
    };
    let result = Connection::open(path)
        .and_then(|mut other| {
            let tx = other.transaction()?;
            tx.execute(
                "UPDATE core_sync_state SET synced_height = 1000, \
                    dashpay_backfill_floor = 1000, dashpay_backfill_rewound_from = 5000 \
                 WHERE wallet_id = ?1",
                params![wallet_id.as_slice()],
            )?;
            tx.execute(
                "UPDATE account_registrations SET dashpay_covered_from = 1000 \
                 WHERE wallet_id = ?1 AND account_type = 'dashpay_receiving'",
                params![wallet_id.as_slice()],
            )?;
            tx.commit()
        })
        .map_err(|e| e.to_string());
    *state = MidLoadWrite::Done(result);
}

/// `load()` reads the cursor and the coverage that suppresses the rescan
/// below it from one snapshot. Another process committing a rewind plus its
/// coverage between those two reads must not hand back the old cursor beside
/// the new coverage: the contact would read as covered, the rescan would be
/// skipped, and the range below the old cursor never checked.
#[test]
fn should_load_the_cursor_and_its_coverage_from_one_snapshot() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x5D);
    ensure_wallet_meta(&persister, &w);
    let entry = covered(0x01, 0x02, 0, 1_000);
    store_and_flush(
        &persister,
        w,
        PlatformWalletChangeSet {
            account_registrations: vec![receiving_account(&entry, 0x01)],
            core: cursor_cs(5_000, false).core,
            ..Default::default()
        },
    );

    *MID_LOAD_WRITE.lock().unwrap() = MidLoadWrite::Armed(path.clone(), w);
    persister.lock_conn_for_test().trace_v2(
        TraceEventCodes::SQLITE_TRACE_STMT,
        Some(commit_rewind_mid_load),
    );
    let mut state = persister.load().expect("load");
    persister
        .lock_conn_for_test()
        .trace_v2(TraceEventCodes::empty(), None);

    match std::mem::replace(&mut *MID_LOAD_WRITE.lock().unwrap(), MidLoadWrite::Disarmed) {
        MidLoadWrite::Done(result) => result.expect("the concurrent rewind must commit"),
        _ => panic!("load() must have read asset_locks, where the rewind is injected"),
    }
    assert_eq!(
        stored_cursor(&persister, &w),
        Some(1_000),
        "the concurrent rewind is on disk"
    );
    let wallet = state.wallets.remove(&w).expect("the seeded wallet");
    assert_eq!(
        (
            wallet.wallet_info.metadata.synced_height,
            wallet.dashpay_backfill
        ),
        (5_000, DashPayBackfillRecord::default()),
        "load() must see the state from before the rewind, cursor and coverage alike"
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

    assert_eq!(stored_record_parts(&persister, &w), (None, 0));
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
fn should_upgrade_a_pre_v022_database_to_no_marker_and_no_backfill() {
    let mut conn = Connection::open_in_memory().expect("open in-memory db");
    conn.pragma_update(None, "foreign_keys", true)
        .expect("enable foreign keys");
    mig::runner()
        .set_target(refinery::Target::Version(19))
        .run(&mut conn)
        .expect("migrate to the last pre-V022 schema");

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
    conn.execute(
        "INSERT INTO core_sync_state (wallet_id, synced_height) VALUES (?1, 5000)",
        params![w.as_slice()],
    )
    .expect("insert sync state");
    conn.execute(
        "INSERT INTO account_registrations \
            (wallet_id, account_type, account_index, user_identity_id, friend_identity_id, \
             account_xpub_bytes) \
         VALUES (?1, 'dashpay_receiving', 0, ?2, ?3, x'00')",
        params![w.as_slice(), owner.as_slice(), contact.as_slice()],
    )
    .expect("insert receiving account");

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

/// A backfill rewind can lower the cursor below the wallet's birth: the
/// rescan hands the contact's checkpoint to `update_synced_height` unclamped.
/// The relaunch must restore that cursor, not the `birth_height - 1` seed,
/// or the range between them is skipped for a contact the record calls
/// covered.
#[test]
fn should_restore_a_rewound_cursor_below_the_wallet_birth() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x5E);
    persister
        .lock_conn_for_test()
        .execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 501)",
            params![w.as_slice()],
        )
        .expect("insert wallet born at 501");
    store_and_flush(&persister, w, cursor_cs(1_000, false));
    store_and_flush(
        &persister,
        w,
        PlatformWalletChangeSet {
            core: cursor_cs(100, true).core,
            ..record_cs(record(100, 1_000, vec![covered(0x01, 0x02, 0, 100)]))
        },
    );
    drop(persister);

    let wallet = reload_wallet(&path, &w);
    assert_eq!(
        wallet.wallet_info.metadata.synced_height, 100,
        "the stored rewind must win over the birth seed"
    );
    assert_eq!(
        wallet.dashpay_backfill.covered_from(
            &Identifier::from([0x01; 32]),
            &Identifier::from([0x02; 32]),
            0
        ),
        Some(100)
    );
}
