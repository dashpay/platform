//! A deleted wallet's rows must not stay legible in the database file.
//!
//! SQLite moves freed pages to the freelist without overwriting them unless
//! `secure_delete` says otherwise, so the addresses, scripts, keys and contact
//! data of a deleted wallet survive in the `.db` — and `Backup` is a page-level
//! copier, so every snapshot taken afterwards carries them forward.
//!
//! The seeded rows are deliberately large enough to span whole pages that the
//! cascade releases outright. That is the case `secure_delete = FAST` does NOT
//! cover: FAST zeroes freed content only within a page it is already
//! rewriting. A test sized to fit inside one shared page would pass under FAST
//! and prove nothing about the guarantee `delete_wallet` claims.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use std::path::{Path, PathBuf};

use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::SqlitePersister;

/// Payload bytes per seeded row. At 512 B and 200 rows the wallet's metadata
/// occupies ~100 KiB — tens of 4 KiB pages, freed as whole pages by the
/// cascade rather than emptied in place.
const ROW_PAYLOAD_LEN: usize = 512;
const ROW_COUNT: usize = 200;

const DOOMED_MARKER: &str = "RESIDUE-PROBE-DOOMED-WALLET-PAYLOAD";
const SURVIVOR_MARKER: &str = "RESIDUE-PROBE-SURVIVING-WALLET-PAYLOAD";

/// Fill `wallet`'s `meta_wallet` rows with a recognisable, page-spanning
/// payload. Written straight through the connection: this test cares about the
/// bytes on disk, not about the changeset path that produced them.
fn seed_recognisable_rows(persister: &SqlitePersister, wallet: &WalletId, marker: &str) {
    let payload: Vec<u8> = marker
        .bytes()
        .cycle()
        .take(ROW_PAYLOAD_LEN)
        .collect::<Vec<u8>>();
    let conn = persister.lock_conn_for_test();
    for i in 0..ROW_COUNT {
        conn.execute(
            "INSERT INTO meta_wallet (wallet_id, key, value) VALUES (?1, ?2, ?3)",
            rusqlite::params![wallet.as_slice(), format!("residue-probe-{i}"), payload],
        )
        .expect("seed meta_wallet row");
    }
}

/// Pages sitting on the database's freelist — whole pages the cascade released
/// rather than merely emptied in place.
fn pages_on_freelist(persister: &SqlitePersister) -> i64 {
    persister
        .lock_conn_for_test()
        .query_row("PRAGMA freelist_count", [], |row| row.get(0))
        .expect("read freelist_count")
}

/// Every byte the database occupies: the main file AND its write-ahead log.
///
/// Recently written pages live in the `-wal` until a checkpoint moves them, so
/// a scan of the `.db` alone would miss data that is plainly still there — and
/// after the delete it would miss residue parked in a WAL that outlived the
/// handle. Both directions matter, so both files are read.
fn database_bytes(path: &Path) -> Vec<u8> {
    let mut bytes = std::fs::read(path).expect("read the database file");
    let mut wal_name = path.as_os_str().to_os_string();
    wal_name.push("-wal");
    if let Ok(wal) = std::fs::read(PathBuf::from(wal_name)) {
        bytes.extend_from_slice(&wal);
    }
    bytes
}

fn occurrences(haystack: &[u8], needle: &str) -> usize {
    let needle = needle.as_bytes();
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

#[test]
fn deleted_wallet_rows_are_not_legible_in_the_database_file() {
    let (persister, _tmp, path) = fresh_persister();
    let doomed = wid(0xD1);
    let survivor = wid(0x5A);

    ensure_wallet_meta(&persister, &doomed);
    ensure_wallet_meta(&persister, &survivor);
    seed_recognisable_rows(&persister, &doomed, DOOMED_MARKER);
    seed_recognisable_rows(&persister, &survivor, SURVIVOR_MARKER);

    // Pre-delete sanity: the scanner must be able to see the doomed wallet's
    // payload while it is still there. A scratch run of this experiment once
    // reported zero hits in every mode because `grep` refused to read a binary
    // file — a clean bill of health manufactured entirely by a broken scanner.
    // Without this line, "absent after the delete" would also be what a scan
    // that never matches anything reports.
    let before = database_bytes(&path);
    assert!(
        occurrences(&before, DOOMED_MARKER) > 0,
        "the scan cannot see the doomed wallet's rows even before the delete — \
         it is measuring nothing"
    );
    assert_eq!(
        pages_on_freelist(&persister),
        0,
        "fixture expects the seeded pages to be in use before the delete"
    );

    persister
        .delete_wallet_skip_backup(doomed)
        .expect("delete the doomed wallet");

    // The freed-page case is the whole point: `secure_delete = FAST` clears
    // only the part of a page it is already rewriting, so a fixture small
    // enough to sit inside one shared page would pass under FAST and prove
    // nothing about what `delete_wallet` claims.
    let freed = pages_on_freelist(&persister);
    assert!(
        freed > 0,
        "the cascade released no whole page, so this run does not exercise the \
         case FAST cannot cover"
    );
    // Closing the persister checkpoints the WAL into the main database, so the
    // scan below reads the pages the delete actually left behind.
    drop(persister);

    let bytes = database_bytes(&path);

    // Positive control. Without it, "the marker is absent" is also what a
    // broken scan, an empty file, or a mis-seeded fixture would report.
    assert!(
        occurrences(&bytes, SURVIVOR_MARKER) > 0,
        "the scan must be able to find rows that are still present — it found \
         none, so its verdict on the deleted wallet means nothing"
    );

    assert_eq!(
        occurrences(&bytes, DOOMED_MARKER),
        0,
        "the deleted wallet's row content is still readable in the database \
         file; every backup taken from here carries it forward"
    );
}

/// A `secure_delete` mode set outside a transaction is still in force inside
/// one on the same connection.
///
/// `delete_wallet` raises the mode and then runs the cascade in an EXCLUSIVE
/// transaction. If the setting did not survive into that context the delete
/// would report success and leave the pages legible, and nothing would error —
/// the failure mode is silence. `delete_wallet_inner` reads the mode back from
/// inside its own transaction for that reason; this pins the SQLite behaviour
/// that read-back depends on, so a change in it surfaces here rather than as
/// unexplained residue.
#[test]
fn a_secure_delete_mode_survives_into_a_transaction_on_the_same_connection() {
    let (persister, _tmp, _path) = fresh_persister();
    let mut conn = persister.lock_conn_for_test();

    let outside: i64 = conn
        .query_row("PRAGMA secure_delete", [], |row| row.get(0))
        .expect("read the steady-state mode");
    assert_eq!(outside, 2, "the persister opens at secure_delete = FAST");

    conn.pragma_update(None, "secure_delete", "ON")
        .expect("raise to the erasing mode");
    let tx = conn.transaction().expect("begin");
    let inside: i64 = tx
        .query_row("PRAGMA secure_delete", [], |row| row.get(0))
        .expect("read the mode from inside the transaction");
    assert_eq!(
        inside, 1,
        "the erasing mode set before the transaction must be in force inside it"
    );
}
