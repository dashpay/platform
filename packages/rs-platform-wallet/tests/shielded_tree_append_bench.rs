//! Microbenchmark for client-side commitment tree append throughput.
//!
//! The wallet's `ClientPersistentCommitmentTree` (SQLite-backed shardtree
//! over Orchard's `MerkleHashOrchard` / Sinsemilla) is what every cmx the
//! SDK fetches gets appended into during sync. I claimed earlier that this
//! is cheap because shardtree defers internal hash computation, but never
//! actually measured it. This bench settles whether tree append is a real
//! component of cold-sync wall-clock.
//!
//! Generates N random cmx values, opens a fresh tree at a temp SQLite path,
//! and times the `append(cmx, Retention::Marked)` loop end-to-end. `Marked`
//! is what the wallet uses (mark every position so any later-bound wallet
//! can retrieve a witness — see `packages/rs-platform-wallet/src/wallet/
//! shielded/sync.rs:286-319`).
//!
//! Run:
//!   cargo test -p platform-wallet --release --features shielded \
//!     --test shielded_tree_append_bench -- --ignored --nocapture
//!
//! Env overrides:
//! - `BENCH_COUNT`  number of cmx to insert (default 100000)

#![cfg(feature = "shielded")]

use std::time::Instant;

use grovedb_commitment_tree::{ClientPersistentCommitmentTree, Retention};
use rand::{rngs::StdRng, RngCore, SeedableRng};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

#[test]
#[ignore = "Microbenchmark; opt in via --ignored --nocapture"]
fn shielded_tree_append_bench() {
    let count: usize = std::env::var("BENCH_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000);

    println!("== Client commitment tree append benchmark ==");
    println!("Insertions:        {}", count);

    // Fresh SQLite at a unique temp path each run so we measure cold tree
    // build, not incremental append on top of existing rows.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("tree_append_bench_{}.sqlite", nanos));
    println!("Tree path:         {}", path.display());

    // Generate cmx bytes up front so the inner loop doesn't include RNG cost.
    print!("Generating {} cmx values...", count);
    let gen_start = Instant::now();
    let mut rng = StdRng::seed_from_u64(0xCAFEF00D);
    let mut leaves: Vec<[u8; 32]> = Vec::with_capacity(count);
    for _ in 0..count {
        // Most random 32-byte values are valid Pallas field elements;
        // `merkle_hash_from_bytes` (called inside append) parses bytes as
        // Pallas. To get a clean measurement, we skip any that fail to parse
        // — in practice ~3/4 of random bytes are valid so this loop is fast.
        loop {
            let mut bytes = [0u8; 32];
            rng.fill_bytes(&mut bytes);
            if grovedb_commitment_tree::merkle_hash_from_bytes(&bytes).is_some() {
                leaves.push(bytes);
                break;
            }
        }
    }
    let gen_ms = gen_start.elapsed().as_millis();
    println!(" {} ms", gen_ms);

    // Run THREE variants on the same workload to attribute cost:
    //   1) Default settings (what the wallet currently uses) — DELETE journal,
    //      synchronous=FULL: fsync per commit.
    //   2) WAL + synchronous=NORMAL — typical app-grade tuning, batches fsyncs.
    //   3) In-memory tree — eliminates I/O entirely, isolating pure shardtree
    //      + Sinsemilla CPU cost.
    fn bench_one(label: &str, leaves: &[[u8; 32]], setup: impl FnOnce() -> Connection) {
        let conn = setup();
        let shared = Arc::new(Mutex::new(conn));
        let mut tree = ClientPersistentCommitmentTree::open_on_shared_connection(shared, 100)
            .expect("open commitment tree");
        let start = Instant::now();
        for cmx in leaves {
            tree.append(*cmx, Retention::Marked)
                .expect("append commitment");
        }
        let elapsed = start.elapsed();
        let per_us = elapsed.as_micros() as f64 / leaves.len() as f64;
        let rate = leaves.len() as f64 / elapsed.as_secs_f64();
        println!(
            "{:32} {:>9.2} s   {:>9.1} µs/append   {:>9.0} appends/sec   (proj 1M: {:.1} s)",
            label,
            elapsed.as_secs_f64(),
            per_us,
            rate,
            1_000_000.0 / rate
        );
    }

    println!();
    println!(
        "{:<32} {:>11}   {:>17}   {:>21}   {:>17}",
        "config", "total", "per-append", "throughput", "projected 1M"
    );
    println!("{}", "-".repeat(110));

    // Variant 1: default (what the wallet does today)
    let p1 = path.clone();
    bench_one("default (DELETE + sync=FULL)", &leaves, || {
        Connection::open(&p1).expect("open sqlite default")
    });
    let _ = std::fs::remove_file(&path);

    // Variant 2: WAL + sync=NORMAL
    let p2 = path.clone();
    bench_one("WAL + sync=NORMAL", &leaves, || {
        let c = Connection::open(&p2).expect("open sqlite wal");
        c.pragma_update(None, "journal_mode", "WAL").expect("WAL");
        c.pragma_update(None, "synchronous", "NORMAL")
            .expect("sync");
        c.pragma_update(None, "temp_store", "MEMORY")
            .expect("temp_store");
        c
    });
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));

    // Variant 3: in-memory (CPU only, no I/O)
    bench_one(":memory: (CPU only)", &leaves, || {
        Connection::open_in_memory().expect("open in-memory")
    });

    // Variant 4: the wallet's actual path via `FileBackedShieldedStore::open_path`.
    // This is the path the wallet hits during real cold sync — verifies the
    // WAL + NORMAL PRAGMA fix in `file_store.rs` lands in production code.
    {
        use platform_wallet::wallet::shielded::file_store::FileBackedShieldedStore;
        use platform_wallet::wallet::shielded::store::ShieldedStore;
        let p4 = path.clone();
        let mut store =
            FileBackedShieldedStore::open_path(&p4, 100).expect("open FileBackedShieldedStore");
        let start = Instant::now();
        for cmx in &leaves {
            store
                .append_commitment(cmx, true)
                .expect("append_commitment");
        }
        let elapsed = start.elapsed();
        let per_us = elapsed.as_micros() as f64 / count as f64;
        let rate = count as f64 / elapsed.as_secs_f64();
        println!(
            "{:32} {:>9.2} s   {:>9.1} µs/append   {:>9.0} appends/sec   (proj 1M: {:.1} s)",
            "FileBackedShieldedStore (wallet)",
            elapsed.as_secs_f64(),
            per_us,
            rate,
            1_000_000.0 / rate
        );
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}-wal", path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    }

    // `max_leaf_position` reporting omitted — the per-variant table is the
    // useful output; final tree state is identical across variants.
}
