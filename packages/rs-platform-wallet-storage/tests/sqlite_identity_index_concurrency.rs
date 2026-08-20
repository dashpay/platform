#![allow(clippy::field_reassign_with_default)]

//! `store()` never reports `Ok` for an identity it did not persist,
//! however two threads interleave.
//!
//! The slot check and the buffer merge share one critical section: the
//! check runs inside `Buffer::store_checked`, under the buffer lock,
//! with the connection held, and against the merged (buffered plus
//! incoming) view. Two threads racing the same wallet + same slot are
//! therefore serialized — the loser sees the winner as the occupant and
//! is refused at store time, before its changeset can join a
//! contradictory merge that a later flush would drop whole while the
//! caller walks away holding an `Ok(())`.
//!
//! `flush_inner` takes the connection BEFORE draining the buffer and
//! holds it through the write, which closes the matching window on the
//! other side: mid-flush the changeset is in neither the buffer nor the
//! database, and a probe that ran there would read a slot as free that
//! is not.
//!
//! The loop is a stress harness, not the proof — the property holds by
//! lock structure. It exists because the original defect's window was a
//! handful of instructions wide (Marvin reproduced it once in ~500-2000
//! jammer-assisted attempts) and a regression would be just as quiet.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};

use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    IdentityChangeSet, IdentityEntry, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::identity::IdentityStatus;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::SqlitePersister;
use rusqlite::{params, OptionalExtension};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;

fn iid(byte: u8) -> Identifier {
    Identifier::from([byte; 32])
}

fn identity_entry(id: u8, index: u32) -> IdentityEntry {
    IdentityEntry {
        id: iid(id),
        balance: u64::from(id),
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
    }
}

fn identity_cs(entry: IdentityEntry) -> PlatformWalletChangeSet {
    PlatformWalletChangeSet {
        identities: Some(IdentityChangeSet {
            identities: [(entry.id, entry)].into_iter().collect(),
            removed: Default::default(),
        }),
        ..Default::default()
    }
}

fn live_occupant(p: &SqlitePersister, wallet_id: &WalletId, index: u32) -> Option<[u8; 32]> {
    let conn = p.lock_conn_for_test();
    conn.query_row(
        "SELECT identity_id FROM identities \
         WHERE wallet_id IS ?1 AND wallet_index = ?2 AND tombstoned = 0",
        params![wallet_id.as_slice(), i64::from(index)],
        |row| row.get::<_, Vec<u8>>(0),
    )
    .optional()
    .expect("query live occupant")
    .map(|raw| raw.try_into().expect("32-byte identity_id"))
}

/// Exactly one of two racing claims on a slot may succeed, and the
/// identity it named must be on disk when `store()` returns `Ok` in
/// `FlushMode::Immediate`. Any single attempt that breaks either half is
/// a confirmed regression; the loop just keeps shaking the interleaving.
#[test]
fn racing_stores_never_report_ok_for_an_identity_that_was_dropped() {
    for attempt in 0..500 {
        let (p, _tmp, _path) = fresh_persister();
        let p = Arc::new(p);
        let w = wid(0xAA);
        ensure_wallet_meta(&p, &w);
        let barrier = Arc::new(Barrier::new(2));

        // Jammer threads: hammer the SAME connection mutex `store()`'s
        // probe locks internally (exposed test-only via
        // `lock_conn_for_test`) to inject scheduling noise around the
        // probe/buffer-merge boundary — widening the otherwise tiny
        // TOCTOU window enough to observe it deterministically.
        let jam_run = Arc::new(AtomicBool::new(true));
        let jammers: Vec<_> = (0..4)
            .map(|_| {
                let jp = Arc::clone(&p);
                let jr = Arc::clone(&jam_run);
                thread::spawn(move || {
                    while jr.load(Ordering::Relaxed) {
                        let g = jp.lock_conn_for_test();
                        drop(g);
                        thread::yield_now();
                    }
                })
            })
            .collect();

        let (p1, b1) = (Arc::clone(&p), Arc::clone(&barrier));
        let t1 = thread::spawn(move || {
            b1.wait();
            p1.store(w, identity_cs(identity_entry(0x01, 1)))
        });
        let (p2, b2) = (Arc::clone(&p), Arc::clone(&barrier));
        let t2 = thread::spawn(move || {
            b2.wait();
            p2.store(w, identity_cs(identity_entry(0x02, 1)))
        });

        let r1 = t1.join().expect("thread 1 panicked");
        let r2 = t2.join().expect("thread 2 panicked");
        jam_run.store(false, Ordering::Relaxed);
        for j in jammers {
            j.join().expect("jammer panicked");
        }

        if r1.is_ok() && r2.is_ok() {
            panic!(
                "attempt {attempt}: both concurrent stores reported Ok(()) — two \
                 identities cannot both legitimately hold index 1"
            );
        }

        let occupant = live_occupant(&p, &w, 1);
        let ok_wants_01 = r1.is_ok();
        let ok_wants_02 = r2.is_ok();
        let lost_ok = (ok_wants_01 && occupant != Some([0x01; 32]))
            || (ok_wants_02 && occupant != Some([0x02; 32]));

        if lost_ok {
            panic!(
                "attempt {attempt}: store() returned Ok(()) for an identity that never \
                 reached disk — r1={r1:?} r2={r2:?} occupant={occupant:?}"
            );
        }
        if r1.is_err() && r2.is_err() {
            panic!(
                "attempt {attempt}: the slot was free and uncontested by anyone else — \
                 one of the two claims had to win: r1={r1:?} r2={r2:?}"
            );
        }
    }
}
