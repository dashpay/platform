#![allow(clippy::field_reassign_with_default)]

//! `delete_wallet` cascade + concurrency regressions.

mod common;

use std::sync::Arc;

use common::{ensure_wallet_meta, fresh_persister, wid};
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};

/// A concurrent `store()` racing against `delete_wallet` must not leave
/// persisted rows behind. We spawn a worker hammering
/// `store(wallet_id, cs)` while the main thread calls `delete_wallet`,
/// then commit any remaining buffered writes and assert every
/// per-wallet table holds zero rows for that wallet_id.
#[test]
fn concurrent_store_does_not_resurrect_deleted_wallet() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;

    let (persister, _tmp, _path) = fresh_persister();
    let persister = Arc::new(persister);
    let w = wid(0xCC);
    ensure_wallet_meta(&persister, &w);

    // Seed at least one persisted child row.
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        synced_height: Some(1),
        last_processed_height: Some(1),
        ..Default::default()
    });
    persister.store(w, cs).expect("seed store");

    let stop = Arc::new(AtomicBool::new(false));
    let worker = {
        let persister = Arc::clone(&persister);
        let stop = Arc::clone(&stop);
        thread::spawn(move || {
            let mut i = 2u32;
            while !stop.load(Ordering::Relaxed) {
                let mut cs = PlatformWalletChangeSet::default();
                cs.core = Some(CoreChangeSet {
                    synced_height: Some(i),
                    last_processed_height: Some(i),
                    ..Default::default()
                });
                // Race against delete — either succeeds or hits a
                // wallet-gone state. Both are acceptable: we only care
                // that no row survives the delete commit.
                let _ = persister.store(w, cs);
                i = i.wrapping_add(1);
                std::thread::yield_now();
            }
        })
    };

    // Give the worker a moment to land some racing stores.
    std::thread::sleep(std::time::Duration::from_millis(20));
    persister
        .delete_wallet(w)
        .expect("delete_wallet should succeed in concurrent-store regression");
    stop.store(true, Ordering::Relaxed);
    worker.join().unwrap();

    // Drain any remaining buffered writes — these MUST also leave the
    // wallet at zero rows because delete_wallet re-drains the buffer
    // after its commit.
    let _ = persister.commit_writes();

    // The wallet's parent row and the seeded child must both be gone —
    // exhaustive per-table coverage lives in the cascade-completeness
    // test; here we only guard against a racing store resurrecting the
    // wallet after the delete commit.
    let conn = persister.lock_conn_for_test();
    for table in ["wallet_metadata", "core_sync_state"] {
        let n: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE wallet_id = ?1"),
                rusqlite::params![w.as_slice()],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("COUNT(*) query failed for table `{table}`: {e}"));
        assert_eq!(
            n, 0,
            "table `{table}` still has rows for deleted wallet — concurrent-store race regression"
        );
    }
}
