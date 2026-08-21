//! An `Immediate`-mode `store()` reports the fate of its OWN write.
//!
//! `store()` merges into the buffer and then flushes. `flush_inner` is
//! not special to the call that triggered it — an explicit `flush()`, a
//! `commit_writes()`, or another thread's `store()` runs the same code.
//! Whichever one drains the buffer first owns the changesets it took,
//! and a fatal write failure drops them and returns `Err` to THAT
//! caller. If a bystander could drain between this call's merge and its
//! flush, this call would then find an empty buffer and report `Ok(())`
//! for a write that was silently destroyed — contradicting the
//! Immediate-mode durability contract in `store()`'s own rustdoc.
//!
//! The window is closed by lock structure, not by timing: every path
//! that drains the buffer (`flush_inner`, `delete_wallet`) holds the
//! write connection across take + write + restore, and an Immediate
//! `store()` holds that same connection continuously from before its
//! merge until after its own flush returns. `release_at_store_seam`
//! parks the bystander exactly in the window, so a regression fails
//! every run rather than one run in hundreds.

mod common;

use common::{ensure_wallet_meta, fresh_persister, release_at_store_seam, ro_conn, wid};

use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::WalletStorageError;

use std::sync::Arc;
use std::time::Duration;

/// How long a parked `store()` waits for the bystander's flush. Under
/// the lock discipline it always expires; a regression that lets the
/// drain through finishes in microseconds, far inside the budget.
const BYSTANDER_BUDGET: Duration = Duration::from_secs(1);

fn changeset(synced_height: u32) -> PlatformWalletChangeSet {
    PlatformWalletChangeSet {
        core: Some(CoreChangeSet {
            synced_height: Some(synced_height),
            last_processed_height: Some(synced_height),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn read_synced_height(path: &std::path::Path, w: &WalletId) -> Option<i64> {
    use rusqlite::OptionalExtension;
    ro_conn(path)
        .query_row(
            "SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .optional()
        .expect("query synced_height")
}

/// A bystander's flush must not be able to drain — and fatally drop —
/// a changeset the in-flight `store()` merged but has not flushed yet.
/// The caller that merged it is the caller that learns it was dropped.
#[test]
fn a_bystander_flush_cannot_swallow_the_failure_of_an_in_flight_store() {
    let (p, _tmp, path) = fresh_persister();
    let p = Arc::new(p);
    let w = wid(0x77);
    ensure_wallet_meta(&p, &w);

    // Whoever drains the buffer eats this and drops the changeset whole.
    // The question this test settles is which caller finds out.
    p.force_next_flush_to_fail(WalletStorageError::IntegrityCheckFailed {
        report: "simulated fatal".into(),
    });

    let flusher = Arc::clone(&p);
    let bystander = release_at_store_seam(&p, BYSTANDER_BUDGET, move || flusher.flush(w).is_ok());
    let stored = p.store(w, changeset(42));
    let bystander_flushed_ok = bystander.join().expect("bystander panicked");

    assert_eq!(
        read_synced_height(&path, &w),
        None,
        "the injected fatal error means nothing was written"
    );
    assert!(
        stored.is_err(),
        "store() merged the changeset that was fatally dropped, so store() is the \
         call that must report it — got Ok(()) for a write that never reached disk"
    );
    assert!(
        bystander_flushed_ok,
        "the bystander drained nothing, so it has no failure to report"
    );
}

/// The same choreography with no injected failure — a live guard that
/// holding the connection across merge and flush cannot wedge two
/// threads, and that `Ok(())` still means the row is on disk.
#[test]
fn a_successful_store_still_owns_its_flush_against_a_bystander() {
    let (p, _tmp, path) = fresh_persister();
    let p = Arc::new(p);
    let w = wid(0x78);
    ensure_wallet_meta(&p, &w);

    let flusher = Arc::clone(&p);
    let bystander = release_at_store_seam(&p, BYSTANDER_BUDGET, move || flusher.flush(w).is_ok());
    p.store(w, changeset(7)).expect("store");
    assert!(
        bystander.join().expect("bystander panicked"),
        "a flush that drains nothing succeeds"
    );

    assert_eq!(
        read_synced_height(&path, &w),
        Some(7),
        "store() returned Ok, so the row it merged is durable"
    );
}
