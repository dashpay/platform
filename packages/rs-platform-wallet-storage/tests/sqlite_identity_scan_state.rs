#![allow(clippy::field_reassign_with_default)]

//! Cross-launch persistence of the gap-limit identity-scan verdict.
//!
//! The open half of dashpay/platform#4365: a scan that left an index
//! unanswered published `Ok` with the identities it did find, and the fact
//! that it was partial existed nowhere once the process exited. The next
//! launch saw an identity on file, took the warm-launch shortcut, and an
//! identity at the unanswered index stayed invisible for the life of the
//! installation.
//!
//! These tests drive the public [`PlatformWalletPersistence`] surface both
//! ways and assert on
//! [`IdentityManager::identity_scan_is_incomplete`](platform_wallet::wallet::identity::IdentityManager::identity_scan_is_incomplete)
//! — the exact call the startup sequence makes to decide whether it may skip
//! discovery. Asserting on a reader helper instead would prove the row was
//! written and read, never that the decision it exists to change can see it.

mod common;

use std::path::Path;

use common::{ensure_wallet_meta, fresh_persister, store_and_flush, wid};
use platform_wallet::changeset::{
    IdentityScanStateEntry, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::identity::IdentityManager;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::{LoadPolicy, SqlitePersister, SqlitePersisterConfig};

/// Flush a scan verdict for `wallet_id` through the public trait.
fn store_verdict(persister: &SqlitePersister, wallet_id: WalletId, entry: IdentityScanStateEntry) {
    let mut cs = PlatformWalletChangeSet::default();
    cs.identity_scan_state = Some(entry);
    store_and_flush(persister, wallet_id, cs);
}

/// Reopen the database at `path` and return what `load()` restored for
/// `wallet_id`, alongside the rebuilt manager the startup sequence queries.
///
/// The seeding persister must already be dropped — the process-wide open-path
/// registry refuses a second live persister on one path.
fn reload(path: &Path, wallet_id: &WalletId) -> (Option<IdentityScanStateEntry>, IdentityManager) {
    reload_with_policy(path, wallet_id, LoadPolicy::Strict)
}

fn reload_with_policy(
    path: &Path,
    wallet_id: &WalletId,
    policy: LoadPolicy,
) -> (Option<IdentityScanStateEntry>, IdentityManager) {
    let persister =
        SqlitePersister::open(SqlitePersisterConfig::new(path).with_load_policy(policy))
            .expect("reopen persister");
    let mut state = persister.load().expect("load");
    let wallet_state = state
        .wallets
        .remove(wallet_id)
        .expect("the seeded wallet must come back from load()");
    let restored = wallet_state
        .identity_manager
        .scan_states
        .get(wallet_id)
        .cloned();
    (
        restored,
        IdentityManager::from(wallet_state.identity_manager),
    )
}

/// The regression #4365 names: a scan that could not answer every index must
/// still be known to have been partial after a restart, so the next launch
/// rescans instead of trusting the identities already on file.
#[test]
fn should_restore_an_incomplete_scan_verdict_across_a_reopen() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x41);
    ensure_wallet_meta(&persister, &w);
    let verdict = IdentityScanStateEntry::incomplete(0, 5, vec![1]);
    store_verdict(&persister, w, verdict.clone());
    drop(persister);

    let (restored, manager) = reload(&path, &w);

    assert_eq!(
        restored.as_ref(),
        Some(&verdict),
        "the verdict must survive the reopen byte for byte"
    );
    assert!(
        manager.identity_scan_is_incomplete(&w),
        "a restored partial scan must re-open the identity question — this is the \
         warm-launch shortcut #4365 was hiding behind"
    );
}

/// The other half of the contract: a scan that answered everything must not
/// cost every later launch a rescan.
#[test]
fn should_restore_a_complete_scan_verdict_without_forcing_a_rescan() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x42);
    ensure_wallet_meta(&persister, &w);
    let verdict = IdentityScanStateEntry::completed(0, 9);
    store_verdict(&persister, w, verdict.clone());
    drop(persister);

    let (restored, manager) = reload(&path, &w);

    assert_eq!(restored.as_ref(), Some(&verdict));
    assert!(
        !manager.identity_scan_is_incomplete(&w),
        "a clean scan must leave the warm-launch shortcut intact"
    );
}

/// Absence is not completeness, and it is not incompleteness either: a wallet
/// that never scanned restores no entry at all, which upstream reads as "keep
/// the existing behaviour" rather than "rescan every launch".
#[test]
fn should_leave_the_scan_verdict_absent_for_a_wallet_that_never_scanned() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x43);
    ensure_wallet_meta(&persister, &w);
    drop(persister);

    let (restored, manager) = reload(&path, &w);

    assert!(
        restored.is_none(),
        "a wallet with no recorded scan must restore no verdict"
    );
    assert!(
        !manager.identity_scan_is_incomplete(&w),
        "an unknown verdict must not be read as an incomplete one"
    );
}

/// A suffix scan may not clear a gap it never probed, even when the verdict
/// reaching the persister was not folded first.
///
/// Discovery resumes one past the highest registered identity, so a wallet
/// with identities at 0 and 2 and no answer at 1 resumes at 3, comes back
/// clean, and publishes `complete`. In-process the manager folds that over the
/// gap; a peer process holding a staler view does not. The writer folds
/// against what is on disk so the durable record can only ever gain a gap,
/// never silently lose one.
#[test]
fn should_carry_forward_a_gap_a_later_suffix_scan_never_probed() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x44);
    ensure_wallet_meta(&persister, &w);
    store_verdict(
        &persister,
        w,
        IdentityScanStateEntry::incomplete(0, 5, vec![1]),
    );
    // Unfolded, exactly as a peer process that never saw the gap would send it.
    store_verdict(&persister, w, IdentityScanStateEntry::completed(3, 9));
    drop(persister);

    let (restored, manager) = reload(&path, &w);
    let restored = restored.expect("verdict must be present");

    assert_eq!(
        restored.failed_indices,
        vec![1],
        "index 1 lies below the suffix scan's coverage, so nothing has answered it"
    );
    assert!(
        !restored.complete,
        "a verdict carrying an unanswered index is not complete"
    );
    assert!(
        manager.identity_scan_is_incomplete(&w),
        "the carried gap must still force a rescan"
    );
}

/// An unlocated gap has no index to name it, so only a clean scan from index 0
/// may clear it — a suffix scan that comes back clean must not.
#[test]
fn should_carry_forward_an_unlocated_gap_until_a_scan_from_zero_covers_it() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x45);
    ensure_wallet_meta(&persister, &w);
    // A scan abandoned mid-await: it answered no index and failed none.
    store_verdict(
        &persister,
        w,
        IdentityScanStateEntry::incomplete(0, 0, Vec::new()),
    );
    store_verdict(&persister, w, IdentityScanStateEntry::completed(3, 9));
    drop(persister);

    let (restored, manager) = reload(&path, &w);
    let restored = restored.expect("verdict must be present");

    assert!(
        restored.unlocated_gap,
        "a suffix scan covered no more than the window it walked"
    );
    assert!(!restored.complete);
    assert!(manager.identity_scan_is_incomplete(&w));

    // A clean scan from the bottom of the index space is the one thing that
    // can speak for the region nobody could point at.
    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("reopen");
    store_verdict(&persister, w, IdentityScanStateEntry::completed(0, 12));
    drop(persister);

    let (restored, manager) = reload(&path, &w);
    let restored = restored.expect("verdict must be present");
    assert!(!restored.unlocated_gap, "a from-zero clean scan clears it");
    assert!(restored.complete);
    assert!(!manager.identity_scan_is_incomplete(&w));
}

/// Every unanswered index survives, ascending, including one that exercises
/// the full `u32` range across the `i64` storage column.
#[test]
fn should_round_trip_failed_indices_in_ascending_order() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x46);
    ensure_wallet_meta(&persister, &w);
    let failed = vec![0u32, 3, 17, u32::MAX];
    store_verdict(
        &persister,
        w,
        IdentityScanStateEntry::incomplete(0, u32::MAX, failed.clone()),
    );
    drop(persister);

    let (restored, _) = reload(&path, &w);
    let restored = restored.expect("verdict must be present");

    assert_eq!(restored.failed_indices, failed);
    assert_eq!(restored.probed_from, 0);
    assert_eq!(restored.probed_through, u32::MAX);
}

/// The verdict is wallet-scoped state, so deleting the wallet must take it —
/// and its child index rows — with it.
#[test]
fn should_drop_the_scan_verdict_when_its_wallet_is_deleted() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0x47);
    ensure_wallet_meta(&persister, &w);
    store_verdict(
        &persister,
        w,
        IdentityScanStateEntry::incomplete(0, 5, vec![1, 2]),
    );

    persister.delete_wallet(w).expect("delete_wallet");

    let conn = persister.lock_conn_for_test();
    for table in ["identity_scan_states", "identity_scan_failed_indices"] {
        let n: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE wallet_id = ?1"),
                rusqlite::params![w.as_slice()],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("COUNT(*) failed for `{table}`: {e}"));
        assert_eq!(n, 0, "`{table}` must not outlive its wallet");
    }
}

/// A row claiming a complete scan while unanswered indices sit beside it
/// contradicts itself — `superseding` can never produce one. Under the strict
/// default that is corruption and aborts the load.
#[test]
fn should_reject_a_scan_row_claiming_completeness_over_an_open_gap() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x48);
    ensure_wallet_meta(&persister, &w);
    store_verdict(&persister, w, IdentityScanStateEntry::completed(0, 9));
    forge_orphan_failed_index(&persister, &w, 4);
    drop(persister);

    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("reopen");
    let err = persister
        .load()
        .expect_err("a self-contradicting verdict row must abort a strict load");
    assert!(
        format!("{err}").contains("scan"),
        "the error must name the scan verdict, got: {err}"
    );
}

/// Under recovery the same row is tolerated rather than fatal, and clamped
/// toward incomplete. The asymmetry is deliberate: the cost of being wrong
/// this way is one extra scan, and the cost of being wrong the other way is an
/// identity that never reappears.
#[test]
fn should_downgrade_a_contradictory_scan_row_under_recovery() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x49);
    ensure_wallet_meta(&persister, &w);
    store_verdict(&persister, w, IdentityScanStateEntry::completed(0, 9));
    forge_orphan_failed_index(&persister, &w, 4);
    drop(persister);

    let (restored, manager) = reload_with_policy(&path, &w, LoadPolicy::Recovery);
    let restored = restored.expect("recovery keeps the row");

    assert!(
        !restored.complete,
        "a contradictory row must be clamped toward rescanning"
    );
    assert_eq!(restored.failed_indices, vec![4]);
    assert!(manager.identity_scan_is_incomplete(&w));
}

/// Write a failed-index row the writer itself would never produce: one sitting
/// beside a verdict that claims completeness.
fn forge_orphan_failed_index(persister: &SqlitePersister, wallet_id: &WalletId, index: u32) {
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO identity_scan_failed_indices (wallet_id, failed_index) VALUES (?1, ?2)",
        rusqlite::params![wallet_id.as_slice(), i64::from(index)],
    )
    .expect("forge contradictory failed-index row");
}

/// The upgrade path: a database standing at the previous release schema gains
/// both tables, empty, and its existing wallet rows are left alone.
#[test]
fn should_create_the_scan_verdict_tables_when_upgrading_from_v014() {
    use platform_wallet_storage::sqlite::migrations as mig;
    use rusqlite::params;

    let mut conn = rusqlite::Connection::open_in_memory().expect("open in-memory db");
    conn.pragma_update(None, "foreign_keys", true)
        .expect("enable foreign keys");

    let to_v014 = mig::runner().set_target(refinery::Target::Version(14));
    to_v014.run(&mut conn).expect("migrate to V014");

    let w = [0x4Au8; 32];
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 7)",
        params![w.as_slice()],
    )
    .expect("insert wallet");

    for table in ["identity_scan_states", "identity_scan_failed_indices"] {
        assert!(
            !table_exists(&conn, table),
            "`{table}` must not exist before V015"
        );
    }

    mig::run(&mut conn).expect("apply V015");

    for table in ["identity_scan_states", "identity_scan_failed_indices"] {
        assert!(table_exists(&conn, table), "V015 must create `{table}`");
        let n: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("count rows");
        assert_eq!(n, 0, "`{table}` starts empty — V015 backfills nothing");
    }

    let birth_height: i64 = conn
        .query_row(
            "SELECT birth_height FROM wallets WHERE wallet_id = ?1",
            params![w.as_slice()],
            |row| row.get(0),
        )
        .expect("pre-existing wallet row survives the upgrade");
    assert_eq!(birth_height, 7);
}

fn table_exists(conn: &rusqlite::Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |_| Ok(()),
    )
    .is_ok()
}
