//! `store_and_commit` answers for exactly what it staged.
//!
//! In `Manual` mode `store()` only buffers, and durability comes from a
//! later drain: the caller's own `flush()`, or a `commit_writes()` from
//! anywhere else. Whichever drain takes the buffer owns what it took, and a
//! fatal write failure drops it and reports to THAT caller. So a caller that
//! stores and then flushes can be told `Ok(())` by a flush that found the
//! buffer empty, for a changeset another drain took and lost.
//!
//! `store_and_commit` closes the window by lock structure: it holds the
//! write connection from before its merge until its commit has succeeded or
//! failed and been dealt with, and every drain needs that connection.
//! `release_at_store_seam` parks the second actor exactly between the merge
//! and the commit, so a regression fails every run.

use crate::common;

use common::{
    ensure_identity, ensure_wallet_meta, fresh_persister_with_mode, release_at_store_seam, ro_conn,
    wid,
};

use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    CoreChangeSet, IdentityChangeSet, IdentityEntry, PersistenceErrorKind, PlatformWalletChangeSet,
    PlatformWalletPersistence, StoreAndCommitError, TokenBalanceChangeSet,
};
use platform_wallet::wallet::identity::IdentityStatus;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::{FlushMode, WalletStorageError};
use rusqlite::{ErrorCode, OptionalExtension};

use std::sync::Arc;
use std::time::Duration;

/// How long a parked step waits for the second actor. Under the lock
/// discipline it always expires; a regression that lets the drain through
/// finishes in microseconds, far inside the budget.
const BYSTANDER_BUDGET: Duration = Duration::from_secs(1);

fn height(synced_height: u32) -> PlatformWalletChangeSet {
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
    ro_conn(path)
        .query_row(
            "SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .optional()
        .expect("query synced_height")
}

/// A token balance is last-write-wins in the buffer and on disk, so the
/// order two states of it were written in shows.
fn balance(owner: [u8; 32], token: [u8; 32], amount: u64) -> PlatformWalletChangeSet {
    let mut balances = TokenBalanceChangeSet::default();
    balances
        .balances
        .insert((Identifier::from(owner), Identifier::from(token)), amount);
    PlatformWalletChangeSet {
        token_balances: Some(balances),
        ..Default::default()
    }
}

fn read_balance(path: &std::path::Path, owner: &[u8; 32], token: &[u8; 32]) -> Option<i64> {
    ro_conn(path)
        .query_row(
            "SELECT balance FROM token_balances WHERE identity_id = ?1 AND token_id = ?2",
            rusqlite::params![&owner[..], &token[..]],
            |row| row.get(0),
        )
        .optional()
        .expect("query balance")
}

fn busy() -> WalletStorageError {
    WalletStorageError::Sqlite(rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error {
            code: ErrorCode::DatabaseBusy,
            extended_code: rusqlite::ffi::SQLITE_BUSY,
        },
        Some("database is busy".into()),
    ))
}

/// A `commit_writes()` the host makes must not be able to take, and fatally
/// drop, a changeset a step merged but has not committed yet. The step that
/// staged it is the caller that learns it was dropped, and the host's commit
/// has no failure to report: it drained nothing.
#[test]
fn should_tell_the_step_that_staged_a_changeset_when_it_is_dropped_and_not_a_commit_by_the_host() {
    let (p, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let p = Arc::new(p);
    let w = wid(0x81);
    ensure_wallet_meta(&p, &w);

    // Whoever drains the buffer eats this and drops the changeset whole.
    p.force_next_flush_to_fail(WalletStorageError::IntegrityCheckFailed {
        report: "simulated fatal".into(),
    });

    let host = Arc::clone(&p);
    let host_commit = release_at_store_seam(&p, BYSTANDER_BUDGET, move || {
        host.commit_writes().expect("the dirty set is readable")
    });
    let stored = p.store_and_commit(w, height(42));
    let report = host_commit.join().expect("the host's commit panicked");

    assert_eq!(
        read_synced_height(&path, &w),
        None,
        "the injected fatal error means nothing was written"
    );
    assert!(
        matches!(&stored, Err(StoreAndCommitError::NotCommitted(e)) if !e.is_transient()),
        "the step merged the changeset that was fatally dropped, so the step is the call \
         that must report it, got {stored:?}"
    );
    assert!(
        report.failed.is_empty(),
        "the host's commit drained nothing, so it has no failure to report: {report:?}"
    );
    assert!(
        !p.buffer_has_changeset_for_test(&w),
        "a commit that failed for good dropped what it took"
    );
}

/// The same for a `flush()` of the wallet racing the step, as the wallet's
/// own sync makes from another thread: it cannot take what the step staged,
/// so it has nothing to drop and succeeds over an empty buffer, and the step
/// reports the failure of its own commit.
#[test]
fn should_keep_a_flush_racing_the_step_from_taking_what_it_staged() {
    let (p, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let p = Arc::new(p);
    let w = wid(0x86);
    ensure_wallet_meta(&p, &w);

    p.force_next_flush_to_fail(WalletStorageError::IntegrityCheckFailed {
        report: "simulated fatal".into(),
    });

    let flusher = Arc::clone(&p);
    let racing_flush = release_at_store_seam(&p, BYSTANDER_BUDGET, move || flusher.flush(w));
    let stored = p.store_and_commit(w, height(42));
    let flushed = racing_flush.join().expect("the racing flush panicked");

    assert_eq!(read_synced_height(&path, &w), None, "nothing was written");
    assert!(
        matches!(&stored, Err(StoreAndCommitError::NotCommitted(e)) if !e.is_transient()),
        "the step is the call that must report the changeset it staged, got {stored:?}"
    );
    assert!(
        flushed.is_ok(),
        "the racing flush drained nothing, so it has no failure to report: {flushed:?}"
    );
}

/// A commit that fails transiently is reported to the step that staged the
/// changeset too, as not committed and still buffered. The host's commit,
/// parked until the step is over, then finds the changeset put back and
/// writes it: the retry of a transient failure is a later drain, never the
/// changeset issued again.
#[test]
fn should_keep_a_changeset_buffered_for_a_later_commit_when_its_own_fails_transiently() {
    let (p, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let p = Arc::new(p);
    let w = wid(0x82);
    ensure_wallet_meta(&p, &w);

    p.force_next_flush_to_fail(busy());

    let host = Arc::clone(&p);
    let host_commit = release_at_store_seam(&p, BYSTANDER_BUDGET, move || {
        host.commit_writes().expect("the dirty set is readable")
    });
    let stored = p.store_and_commit(w, height(42));
    let report = host_commit.join().expect("the host's commit panicked");

    assert!(
        matches!(&stored, Err(StoreAndCommitError::NotCommitted(e)) if e.is_transient()),
        "the step's own commit met the transient failure, got {stored:?}"
    );
    assert!(
        report.failed.is_empty() && report.succeeded == vec![w],
        "the host's commit ran after the step and wrote what it had put back: {report:?}"
    );
    assert_eq!(
        read_synced_height(&path, &w),
        Some(42),
        "written by the later commit, once"
    );
}

/// The step commits what was buffered for the wallet before its changeset
/// together with it, the changeset last. An older state of a record that sat
/// in the buffer does not stay behind to be written over the newer one by a
/// later drain.
#[test]
fn should_commit_what_was_buffered_before_the_changeset_with_it_and_in_that_order() {
    let (p, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0x83);
    let (owner, token) = ([0x0A; 32], [0x0B; 32]);
    ensure_wallet_meta(&p, &w);
    ensure_identity(&p, &owner, Some(&w));

    p.store(w, balance(owner, token, 1))
        .expect("the older state is buffered");
    assert_eq!(read_balance(&path, &owner, &token), None, "buffered only");

    p.store_and_commit(w, balance(owner, token, 2))
        .expect("the newer state is committed");
    assert_eq!(
        read_balance(&path, &owner, &token),
        Some(2),
        "the newer state is on disk"
    );
    assert!(
        !p.buffer_has_changeset_for_test(&w),
        "the older state went to disk with the step, under the newer"
    );

    let report = p.commit_writes().expect("commit");
    assert!(report.is_ok(), "{report:?}");
    assert_eq!(
        read_balance(&path, &owner, &token),
        Some(2),
        "no later drain writes the older state over it"
    );
}

fn identity_at(id: [u8; 32], index: u32) -> PlatformWalletChangeSet {
    let id = Identifier::from(id);
    let entry = IdentityEntry {
        id,
        balance: 1,
        revision: 1,
        identity_index: Some(index),
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
    };
    PlatformWalletChangeSet {
        identities: Some(IdentityChangeSet {
            identities: [(id, entry)].into_iter().collect(),
            removed: Default::default(),
        }),
        ..Default::default()
    }
}

fn identity_on_disk(path: &std::path::Path, id: &[u8; 32]) -> bool {
    ro_conn(path)
        .query_row(
            "SELECT 1 FROM identities WHERE identity_id = ?1",
            rusqlite::params![&id[..]],
            |_| Ok(()),
        )
        .optional()
        .expect("query identity row")
        .is_some()
}

/// A changeset refused before it is staged is reported as such, and leaves
/// everything as it was: nothing of it is buffered or written, and what the
/// wallet had buffered before the call is neither committed nor dropped by
/// the call that was refused.
#[test]
fn should_leave_the_buffer_as_it_was_when_a_changeset_is_refused_before_it_is_staged() {
    let (p, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0x84);
    ensure_wallet_meta(&p, &w);
    let (occupant, claimant) = ([0x1A; 32], [0x1B; 32]);
    p.store_and_commit(w, identity_at(occupant, 0))
        .expect("the occupant takes slot 0");
    p.store(w, height(5)).expect("buffered before the call");

    let refused = p.store_and_commit(w, identity_at(claimant, 0));

    let Err(StoreAndCommitError::NotStaged(error)) = &refused else {
        panic!("a slot that is taken refuses the changeset before it is staged: {refused:?}");
    };
    assert_eq!(error.kind(), Some(PersistenceErrorKind::Constraint));
    assert!(!identity_on_disk(&path, &claimant), "nothing of it written");
    assert_eq!(
        read_synced_height(&path, &w),
        None,
        "the refused call committed nothing of what was buffered before it"
    );
    assert!(
        p.buffer_has_changeset_for_test(&w),
        "nor dropped it: it is buffered still"
    );

    p.flush(w).expect("a later flush");
    assert_eq!(read_synced_height(&path, &w), Some(5));
    assert!(
        !identity_on_disk(&path, &claimant),
        "the refused changeset was never in the buffer"
    );
}

/// The step is offered in both flush modes, through the trait object the
/// wallet holds, and in `Immediate` mode it is `store` with an answer that
/// tells a refusal from a failed commit.
#[test]
fn should_offer_the_step_through_the_trait_in_both_flush_modes() {
    for mode in [FlushMode::Manual, FlushMode::Immediate] {
        let (p, _tmp, path) = fresh_persister_with_mode(mode);
        let w = wid(0x85);
        ensure_wallet_meta(&p, &w);
        let p: Arc<dyn PlatformWalletPersistence> = Arc::new(p);

        assert!(p.offers_store_and_commit(), "{mode:?}");
        p.store_and_commit(w, height(7)).expect("committed");
        assert_eq!(read_synced_height(&path, &w), Some(7), "{mode:?}");
    }
}
