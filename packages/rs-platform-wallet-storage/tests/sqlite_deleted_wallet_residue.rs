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

    persister
        .delete_wallet_skip_backup(doomed)
        .expect("delete the doomed wallet");
    // Closing the persister checkpoints the WAL into the main database, so the
    // scan below reads the pages the delete actually left behind.
    drop(persister);

    let bytes = std::fs::read(&path).expect("read the database file");

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
