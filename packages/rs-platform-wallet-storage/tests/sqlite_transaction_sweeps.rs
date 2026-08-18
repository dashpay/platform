#![allow(clippy::field_reassign_with_default)]

//! Coverage for `core_state::apply`'s handling of `CoreChangeSet::swept_transactions`
//! (the subtractive sweep-removal field — see `core_state.rs::apply_sweep`).
//!
//! Exercises the writer directly through `core_state::apply` on a hand-rolled
//! `rusqlite::Transaction`, same style as `sqlite_structural_hardening.rs`, so
//! each case can pre-seed exactly the rows a sweep needs to reason about
//! without going through the full changeset-merge/buffer machinery.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid, SqlitePersister, SqlitePersisterConfig};

use dashcore::hashes::Hash;
use dashcore::{Address, Network, OutPoint, Transaction, TxIn, TxOut, Txid};
use key_wallet::account::{AccountType, StandardAccountType};
use key_wallet::managed_account::transaction_record::{TransactionDirection, TransactionRecord};
use key_wallet::transaction_checking::{TransactionContext, TransactionType};
use key_wallet::Utxo;
use platform_wallet::changeset::changeset::SweepBatch;
use platform_wallet::changeset::CoreChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::schema::{blob, core_state};
use rusqlite::params;

fn p2pkh(byte: u8) -> Address {
    use dashcore::address::Payload;
    use dashcore::hashes::Hash;
    use dashcore::PubkeyHash;
    let hash = PubkeyHash::from_byte_array([byte; 20]);
    Address::new(Network::Testnet, Payload::PubkeyHash(hash))
}

fn make_utxo(addr: &Address, txid: Txid, vout: u32, value: u64) -> Utxo {
    let outpoint = OutPoint::new(txid, vout);
    let txout = TxOut {
        value,
        script_pubkey: addr.script_pubkey(),
    };
    Utxo::new(outpoint, txout, addr.clone(), 10, false)
}

fn derive_address(conn: &rusqlite::Connection, w: &WalletId, account_index: u32, addr: &Address) {
    conn.execute(
        "INSERT INTO core_derived_addresses \
            (wallet_id, account_type, account_index, address, derivation_path, used) \
         VALUES (?1, 'standard', ?2, ?3, '0/0', 0)",
        params![w.as_slice(), account_index as i64, addr.to_string()],
    )
    .unwrap();
}

/// Build a `TransactionRecord` whose `transaction.input`/`.output` are the
/// real, decodable fields `apply_sweep` reads back for its outpoint math —
/// as opposed to `input_details`/`output_details`, which only cover the
/// wallet-relevant subset and are left empty here on purpose.
fn tx_record(txid: Txid, inputs: Vec<OutPoint>, outputs: Vec<TxOut>) -> TransactionRecord {
    let inner = Transaction {
        version: 3,
        lock_time: 0,
        input: inputs
            .into_iter()
            .map(|previous_output| TxIn {
                previous_output,
                ..Default::default()
            })
            .collect(),
        output: outputs,
        special_transaction_payload: None,
    };
    let mut record = TransactionRecord::new(
        inner,
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        },
        TransactionContext::Mempool,
        TransactionType::Standard,
        TransactionDirection::Outgoing,
        Vec::new(),
        Vec::new(),
        0,
    );
    record.txid = txid;
    record
}

fn unspent(conn: &rusqlite::Connection, w: &WalletId) -> std::collections::BTreeSet<OutPoint> {
    core_state::list_unspent_utxos(conn, w)
        .unwrap()
        .into_values()
        .flatten()
        .map(|row| row.outpoint)
        .collect()
}

fn row_exists(conn: &rusqlite::Connection, w: &WalletId, op: &OutPoint) -> bool {
    let bytes = platform_wallet_storage::sqlite::schema::blob::encode_outpoint(op).unwrap();
    conn.query_row(
        "SELECT 1 FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
        params![w.as_slice(), &bytes[..]],
        |_| Ok(()),
    )
    .optional()
    .unwrap()
    .is_some()
}

use rusqlite::OptionalExtension;

/// A changeset carrying nothing but a sweep still deletes: the loser's
/// `core_transactions` row and every `core_utxos` row it created go, even
/// though `records` / `new_utxos` / everything else on the changeset is
/// empty. This is the guard against the bug the review finding described —
/// `apply` skipping `swept_transactions` entirely because every other
/// `if !cs.<field>.is_empty()` block was false.
#[test]
fn sweep_only_changeset_deletes_loser_row_and_its_outputs() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xE0);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x01);
    let loser_txid = Txid::from_byte_array([0x10; 32]);
    let loser = tx_record(
        loser_txid,
        vec![],
        vec![TxOut {
            value: 5_000,
            script_pubkey: addr.script_pubkey(),
        }],
    );
    let loser_output = OutPoint::new(loser_txid, 0);

    {
        let mut conn = persister.lock_conn_for_test();
        derive_address(&conn, &w, 0, &addr);
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![loser],
            new_utxos: vec![make_utxo(&addr, loser_txid, 0, 5_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    {
        let conn = persister.lock_conn_for_test();
        assert!(
            row_exists(&conn, &w, &loser_output),
            "sanity: the loser's output must exist before the sweep"
        );
    }

    // The sweep-only round: nothing else populated on the changeset.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![loser_txid],
                superseded_by: Txid::from_byte_array([0x11; 32]),
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    let conn = persister.lock_conn_for_test();
    let record: Option<Vec<u8>> = conn
        .query_row(
            "SELECT record_blob FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![w.as_slice(), AsRef::<[u8]>::as_ref(&loser_txid)],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    assert!(record.is_none(), "swept transaction row must be gone");
    assert!(
        !row_exists(&conn, &w, &loser_output),
        "the swept transaction's own output must be gone"
    );
}

/// A sweep naming a txid this store never recorded is a successful
/// no-op — sweeps are idempotent and can arrive for a transaction this
/// wallet dropped, or ran again after the first sweep already applied.
#[test]
fn sweeping_an_unknown_txid_is_a_no_op() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xE1);
    ensure_wallet_meta(&persister, &w);

    let mut conn = persister.lock_conn_for_test();
    let tx = conn.transaction().unwrap();
    let cs = CoreChangeSet {
        sweeps: vec![SweepBatch {
            txids: vec![Txid::from_byte_array([0x20; 32])],
            superseded_by: Txid::from_byte_array([0x21; 32]),
            released_outpoints: vec![],
        }],
        ..Default::default()
    };
    core_state::apply(&tx, &w, &cs).expect("unknown txid must not error");
    tx.commit().unwrap();
}

/// The released set is applied verbatim: an outpoint it names becomes
/// spendable again, and every other input the loser claimed stays out of
/// the unspent set because the transaction that beat the loser took it.
#[test]
fn the_released_set_frees_exactly_the_inputs_it_names() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xE2);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x02);
    let funding_txid = Txid::from_byte_array([0x30; 32]);
    let shared_input = OutPoint::new(funding_txid, 0);
    let exclusive_input = OutPoint::new(funding_txid, 1);

    let loser_txid = Txid::from_byte_array([0x31; 32]);
    let winner_txid = Txid::from_byte_array([0x32; 32]);

    let loser = tx_record(
        loser_txid,
        vec![shared_input, exclusive_input],
        vec![TxOut {
            value: 1_000,
            script_pubkey: addr.script_pubkey(),
        }],
    );
    // The winner only claimed the shared input.
    let winner = tx_record(
        winner_txid,
        vec![shared_input],
        vec![TxOut {
            value: 900,
            script_pubkey: addr.script_pubkey(),
        }],
    );

    let mut conn = persister.lock_conn_for_test();
    derive_address(&conn, &w, 0, &addr);

    // Fund both inputs as ordinary unspent UTXOs, then record the loser
    // spending both (mirroring the ordinary flow before it was swept).
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            new_utxos: vec![
                make_utxo(&addr, funding_txid, 0, 500),
                make_utxo(&addr, funding_txid, 1, 500),
            ],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![loser],
            spent_utxos: vec![
                make_utxo(&addr, funding_txid, 0, 500),
                make_utxo(&addr, funding_txid, 1, 500),
            ],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    // Record the winner, which re-claims only the shared input.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![winner],
            spent_utxos: vec![make_utxo(&addr, funding_txid, 0, 500)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    // Sanity: before the sweep, neither input shows up as unspent.
    assert!(!unspent(&conn, &w).contains(&shared_input));
    assert!(!unspent(&conn, &w).contains(&exclusive_input));

    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![loser_txid],
                superseded_by: winner_txid,
                released_outpoints: vec![exclusive_input],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    let after = unspent(&conn, &w);
    assert!(
        after.contains(&exclusive_input),
        "an outpoint the sweep released must come back as spendable"
    );
    assert!(
        !after.contains(&shared_input),
        "shared input stays spent — the winner took it"
    );
}

/// The winner does not have to reach this store at all: it can spend our
/// coin while paying only external addresses, and then no record for it is
/// ever written here. The released set still resolves both inputs
/// correctly, which is the whole reason it is carried rather than
/// recomputed from the rows on hand.
#[test]
fn an_absent_winner_still_keeps_its_own_input_spent() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xE3);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x03);
    let funding_txid = Txid::from_byte_array([0x40; 32]);
    let taken_by_winner = OutPoint::new(funding_txid, 0);
    let loser_exclusive = OutPoint::new(funding_txid, 1);

    let loser_txid = Txid::from_byte_array([0x41; 32]);
    let unrecorded_winner_txid = Txid::from_byte_array([0x42; 32]);

    let loser = tx_record(
        loser_txid,
        vec![taken_by_winner, loser_exclusive],
        vec![TxOut {
            value: 1_000,
            script_pubkey: addr.script_pubkey(),
        }],
    );

    let mut conn = persister.lock_conn_for_test();
    derive_address(&conn, &w, 0, &addr);
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            new_utxos: vec![
                make_utxo(&addr, funding_txid, 0, 500),
                make_utxo(&addr, funding_txid, 1, 500),
            ],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![loser],
            spent_utxos: vec![
                make_utxo(&addr, funding_txid, 0, 500),
                make_utxo(&addr, funding_txid, 1, 500),
            ],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            // `superseded_by` never arrives in this store; upstream still
            // knows which of the loser's inputs it did not take.
            sweeps: vec![SweepBatch {
                txids: vec![loser_txid],
                superseded_by: unrecorded_winner_txid,
                released_outpoints: vec![loser_exclusive],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    let after = unspent(&conn, &w);
    assert!(
        !after.contains(&taken_by_winner),
        "a coin the chain has already spent must not return as spendable"
    );
    assert!(
        after.contains(&loser_exclusive),
        "the loser's own input is free, winner record or not"
    );
    // Both rows survive either way — held or freed, never deleted.
    assert!(row_exists(&conn, &w, &taken_by_winner));
    assert!(row_exists(&conn, &w, &loser_exclusive));
}

/// A round can carry both a release and a later transaction that legitimately
/// spends the freed coin: merging folds several events together, and every
/// record is applied before sweeps. `core_utxos` never records who spent a
/// row, so the release has to defer to the surviving record in the changeset
/// itself — otherwise it hands a coin the later transaction consumed back to
/// the unspent set.
#[test]
fn a_released_coin_a_surviving_record_reclaims_stays_spent() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xE4);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x04);
    let funding_txid = Txid::from_byte_array([0x50; 32]);
    let freed_coin = OutPoint::new(funding_txid, 1);

    let loser_txid = Txid::from_byte_array([0x51; 32]);
    let winner_txid = Txid::from_byte_array([0x52; 32]);
    let reclaimer_txid = Txid::from_byte_array([0x53; 32]);

    let loser = tx_record(
        loser_txid,
        vec![OutPoint::new(funding_txid, 0), freed_coin],
        vec![TxOut {
            value: 1_000,
            script_pubkey: addr.script_pubkey(),
        }],
    );
    let reclaimer = tx_record(
        reclaimer_txid,
        vec![freed_coin],
        vec![TxOut {
            value: 400,
            script_pubkey: addr.script_pubkey(),
        }],
    );

    let mut conn = persister.lock_conn_for_test();
    derive_address(&conn, &w, 0, &addr);
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            new_utxos: vec![
                make_utxo(&addr, funding_txid, 0, 500),
                make_utxo(&addr, funding_txid, 1, 500),
            ],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![loser],
            spent_utxos: vec![
                make_utxo(&addr, funding_txid, 0, 500),
                make_utxo(&addr, funding_txid, 1, 500),
            ],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    // One round: the sweep frees the coin, and a surviving record in the very
    // same round already spent it.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![reclaimer],
            spent_utxos: vec![make_utxo(&addr, funding_txid, 1, 500)],
            sweeps: vec![SweepBatch {
                txids: vec![loser_txid],
                superseded_by: winner_txid,
                released_outpoints: vec![freed_coin],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    assert!(
        !unspent(&conn, &w).contains(&freed_coin),
        "a coin a surviving record in the same round already spent must stay spent"
    );
}

/// A chainlocked winner may evict an InstantSend-locked loser, so a swept
/// transaction can own a row in `core_instant_locks`. Nothing ties that table
/// to `core_transactions`, so the lock has to be deleted explicitly or it
/// outlives the transaction it describes forever.
#[test]
fn sweeping_a_transaction_deletes_its_instant_lock() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xE5);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x05);
    let loser_txid = Txid::from_byte_array([0x60; 32]);
    let loser = tx_record(
        loser_txid,
        vec![],
        vec![TxOut {
            value: 1_000,
            script_pubkey: addr.script_pubkey(),
        }],
    );

    let mut conn = persister.lock_conn_for_test();
    derive_address(&conn, &w, 0, &addr);
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![loser],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.execute(
            "INSERT INTO core_instant_locks (wallet_id, txid, islock_blob) VALUES (?1, ?2, ?3)",
            params![
                w.as_slice(),
                AsRef::<[u8]>::as_ref(&loser_txid),
                vec![0u8; 8]
            ],
        )
        .unwrap();
        tx.commit().unwrap();
    }

    assert_eq!(
        instant_lock_count(&conn, &w, &loser_txid),
        1,
        "sanity: the lock is there"
    );

    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![loser_txid],
                superseded_by: Txid::from_byte_array([0x61; 32]),
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    assert_eq!(
        instant_lock_count(&conn, &w, &loser_txid),
        0,
        "the swept transaction's InstantLock must go with it"
    );
}

fn instant_lock_count(conn: &rusqlite::Connection, w: &WalletId, txid: &Txid) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM core_instant_locks WHERE wallet_id = ?1 AND txid = ?2",
        params![w.as_slice(), AsRef::<[u8]>::as_ref(txid)],
        |row| row.get(0),
    )
    .unwrap()
}

/// Two sweeps in one round, and the later one disagrees with the earlier.
///
/// The first frees a coin; a transaction then spends it; the second sweep
/// removes that spender but keeps the coin spent, because its own winner
/// took it. The later answer is the true one, and only replaying the batches
/// in order makes it stick — folding the release sets together leaves the
/// first "free" outliving the last "spent".
#[test]
fn a_later_sweep_keeping_a_coin_spent_overrides_an_earlier_release() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xE6);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x06);
    let funding_txid = Txid::from_byte_array([0x70; 32]);
    let contested = OutPoint::new(funding_txid, 0);

    let first_loser = Txid::from_byte_array([0x71; 32]);
    let second_loser = Txid::from_byte_array([0x72; 32]);

    let first = tx_record(
        first_loser,
        vec![contested],
        vec![TxOut {
            value: 400,
            script_pubkey: addr.script_pubkey(),
        }],
    );
    // The transaction that took the freed coin, and that the second sweep
    // removes. It is a loser too, so it is not a surviving claim.
    let second = tx_record(
        second_loser,
        vec![contested],
        vec![TxOut {
            value: 300,
            script_pubkey: addr.script_pubkey(),
        }],
    );

    let mut conn = persister.lock_conn_for_test();
    derive_address(&conn, &w, 0, &addr);
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            new_utxos: vec![make_utxo(&addr, funding_txid, 0, 500)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![first, second],
            spent_utxos: vec![make_utxo(&addr, funding_txid, 0, 500)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![
                SweepBatch {
                    txids: vec![first_loser],
                    superseded_by: Txid::from_byte_array([0x7a; 32]),
                    released_outpoints: vec![contested],
                },
                // The second winner consumed the coin, so this sweep frees
                // nothing — and that has to override the release above.
                SweepBatch {
                    txids: vec![second_loser],
                    superseded_by: Txid::from_byte_array([0x7b; 32]),
                    released_outpoints: vec![],
                },
            ],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    assert!(
        !unspent(&conn, &w).contains(&contested),
        "the later sweep kept the coin spent, so it must not be spendable"
    );
}

/// A loser can be persisted before its own funding output is: this store
/// only learns about a TXO through `new_utxos`/`spent_utxos`, so a spend can
/// name an outpoint `core_utxos` has never heard of. When such an input is
/// held (not released) by the sweep, `apply_sweep` has no row to update and
/// must leave a claim of its own — otherwise deleting the loser's
/// `core_transactions` row (the only place that input was ever recorded)
/// erases the claim entirely, and the funding output arriving later — even
/// after a full restart — would insert it back as a plain unspent UTXO.
#[test]
fn a_held_input_with_no_utxo_row_survives_restart_and_stays_spent_when_funded() {
    let (persister, _tmp, path) = fresh_persister();
    let w: WalletId = wid(0xE7);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x07);
    let funding_txid = Txid::from_byte_array([0x80; 32]);
    let unfunded_input = OutPoint::new(funding_txid, 0);

    let loser_txid = Txid::from_byte_array([0x81; 32]);
    let winner_txid = Txid::from_byte_array([0x82; 32]);

    let loser = tx_record(
        loser_txid,
        vec![unfunded_input],
        vec![TxOut {
            value: 1_000,
            script_pubkey: addr.script_pubkey(),
        }],
    );

    {
        let mut conn = persister.lock_conn_for_test();
        derive_address(&conn, &w, 0, &addr);
        // The loser's spend arrives with no prior `new_utxos`/`spent_utxos`
        // for `unfunded_input` — the funding side of that outpoint has not
        // been observed yet.
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![loser],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();

        assert!(
            !row_exists(&conn, &w, &unfunded_input),
            "sanity: no core_utxos row exists for the unfunded input yet"
        );
    }

    // The sweep holds the input (it is not in `released_outpoints`), with
    // nothing on hand to update.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![loser_txid],
                superseded_by: winner_txid,
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    drop(persister);

    // Restart: a fresh persister loading the same on-disk store, exactly as
    // a relaunch would see it.
    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();

    // The funding transaction finally arrives and hands the outpoint back
    // as a UTXO — the ordinary path a rescan or late block takes.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            new_utxos: vec![make_utxo(&addr, funding_txid, 0, 1_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    let conn = persister.lock_conn_for_test();
    assert!(
        !unspent(&conn, &w).contains(&unfunded_input),
        "the winner's claim on this input must survive the loser's deletion, \
         a restart, and the funding UTXO's own arrival"
    );
}

/// A held-but-unfunded input's placeholder (see the test above) can itself
/// need to move again: its first winner can go on to lose a later sweep
/// while the outpoint is still unfunded. Unlike the mobile backends' pending-
/// input table, this schema has no separate relationship the placeholder
/// detaches from — `apply_sweep` always looks up the loser's inputs fresh
/// from its own `core_transactions` blob and touches `core_utxos` by
/// outpoint alone, so the second sweep finds the same placeholder row the
/// first one wrote without any chain-specific bookkeeping. This is the
/// released half: L spends P; W spends P and Q and sweeps L holding P (P is
/// still unfunded); X spends Q and sweeps W, this time releasing P.
#[test]
fn a_chained_sweep_before_funding_still_frees_an_earlier_tombstone_on_release() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xE8);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x08);
    let funding_txid = Txid::from_byte_array([0x90; 32]);
    let unfunded_input = OutPoint::new(funding_txid, 0);
    let funded_input = OutPoint::new(funding_txid, 1);

    let first_loser = Txid::from_byte_array([0x91; 32]); // L
    let second_loser = Txid::from_byte_array([0x92; 32]); // W
    let final_winner = Txid::from_byte_array([0x93; 32]); // X

    let l = tx_record(
        first_loser,
        vec![unfunded_input],
        vec![TxOut {
            value: 1_000,
            script_pubkey: addr.script_pubkey(),
        }],
    );
    let w_record = tx_record(
        second_loser,
        vec![unfunded_input, funded_input],
        vec![TxOut {
            value: 900,
            script_pubkey: addr.script_pubkey(),
        }],
    );

    let mut conn = persister.lock_conn_for_test();
    derive_address(&conn, &w, 0, &addr);

    // `funded_input` is an ordinary UTXO from the start; `unfunded_input`'s
    // funding side is never observed until the very end.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            new_utxos: vec![make_utxo(&addr, funding_txid, 1, 500)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    // L's spend of the unfunded input arrives with no core_utxos row for it.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![l],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    // First sweep: W beats L, holding the still-unfunded input. This is what
    // writes the placeholder row this test is about.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![first_loser],
                superseded_by: second_loser,
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    assert!(
        row_exists(&conn, &w, &unfunded_input),
        "sanity: the first sweep must have left a placeholder row"
    );
    // W's own record has to be on hand for the second sweep to look its
    // inputs up — the same requirement any ordinary (non-chained) sweep has.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![w_record],
            spent_utxos: vec![make_utxo(&addr, funding_txid, 1, 500)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    // Second sweep: X beats W, and this time releases the input that has
    // been sitting unfunded since the first sweep.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![second_loser],
                superseded_by: final_winner,
                released_outpoints: vec![unfunded_input],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    assert!(
        unspent(&conn, &w).contains(&unfunded_input),
        "the chained sweep released this input, and its own funding TXO is \
         still unobserved — it must read as an ordinary spendable UTXO, not \
         stay stuck under the first sweep's placeholder"
    );
    assert!(
        !unspent(&conn, &w).contains(&funded_input),
        "the second sweep's winner took the other input"
    );
}

/// The held (not released) half of the chained-before-funding scenario
/// above: the second sweep keeps the still-unfunded input spent instead of
/// releasing it, and the placeholder must end up attributed to the NEW
/// winner rather than the one the second sweep just removed. Verified
/// across a full restart, then confirmed by finally funding the input — it
/// must still read as spent, and the persisted placeholder must name the
/// final winner rather than the intermediate one that no longer has a row.
#[test]
fn a_chained_sweep_before_funding_repoints_an_earlier_tombstone_to_the_new_winner() {
    let (persister, _tmp, path) = fresh_persister();
    let w: WalletId = wid(0xE9);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x09);
    let funding_txid = Txid::from_byte_array([0xA0; 32]);
    let unfunded_input = OutPoint::new(funding_txid, 0);

    let first_loser = Txid::from_byte_array([0xA1; 32]); // L
    let second_loser = Txid::from_byte_array([0xA2; 32]); // W
    let final_winner = Txid::from_byte_array([0xA3; 32]); // X

    let l = tx_record(
        first_loser,
        vec![unfunded_input],
        vec![TxOut {
            value: 1_000,
            script_pubkey: addr.script_pubkey(),
        }],
    );
    let w_record = tx_record(
        second_loser,
        vec![unfunded_input],
        vec![TxOut {
            value: 900,
            script_pubkey: addr.script_pubkey(),
        }],
    );

    {
        let mut conn = persister.lock_conn_for_test();
        derive_address(&conn, &w, 0, &addr);
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![l],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();

        // First sweep: W beats L, holding the unfunded input.
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![first_loser],
                superseded_by: second_loser,
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();

        // W's own record, needed by the second sweep below.
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![w_record],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();

        // Second sweep: X beats W, still holding the same unfunded input.
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![second_loser],
                superseded_by: final_winner,
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    drop(persister);
    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();

    // The funding transaction finally arrives.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            new_utxos: vec![make_utxo(&addr, funding_txid, 0, 1_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    let conn = persister.lock_conn_for_test();
    assert!(
        !unspent(&conn, &w).contains(&unfunded_input),
        "the final winner's claim must survive both sweeps, the restart, \
         and the funding UTXO's own arrival"
    );
    let spent_in_txid: Vec<u8> = conn
        .query_row(
            "SELECT spent_in_txid FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
            params![
                w.as_slice(),
                &blob::encode_outpoint(&unfunded_input).unwrap()[..]
            ],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        spent_in_txid,
        AsRef::<[u8]>::as_ref(&final_winner).to_vec(),
        "the placeholder must be attributed to the final winner, not the \
         intermediate one the second sweep already removed"
    );
}

/// Confirmation, not a fix, of this round's BLOCKING finding on the mobile
/// backends' missing-row early return: there, one wallet's callback can
/// delete the shared winner row while a second wallet's detached tombstones
/// still name it, and the second wallet's own sweep of that winner then has
/// to reconcile them against a row that no longer exists. No such moment
/// exists here. `core_transactions` is keyed `(wallet_id, txid)`, so each
/// wallet sweeps its own copy of the winner and no other wallet's call can
/// have removed it first; and the tombstone is not a detached side-table row
/// but the wallet's own `core_utxos` placeholder, matched by `apply_sweep`
/// through the winner's own stored inputs — `(wallet_id, outpoint)`-scoped,
/// so the chain continues per wallet with nothing shared to lose.
///
/// This is the reviewer's multi-wallet chained-sweep-before-funding shape
/// end to end: the same loser txid in two wallets, each claiming a
/// still-unfunded coin of its own; W beats L (both coins held as
/// placeholders); W's own record lands; X beats W, with wallet 1 releasing
/// its coin and wallet 2 holding — in that order, so wallet 1's whole chain
/// including its deletion of (its copy of) W commits before wallet 2's
/// callback runs. Each wallet's decision must land on its own coin only, and
/// each coin's eventual funding must respect it.
#[test]
fn a_multi_wallet_chained_sweep_before_funding_reconciles_each_wallets_own_tombstones() {
    let (persister, _tmp, _path) = fresh_persister();
    let w1: WalletId = wid(0xF1);
    let w2: WalletId = wid(0xF2);
    ensure_wallet_meta(&persister, &w1);
    ensure_wallet_meta(&persister, &w2);

    let addr1 = p2pkh(0x51);
    let addr2 = p2pkh(0x52);
    let funding_txid = Txid::from_byte_array([0x50; 32]);
    // Wallet 1's coin and wallet 2's coin. Neither funding side has been
    // observed in either wallet until the very end.
    let p1 = OutPoint::new(funding_txid, 0);
    let p2 = OutPoint::new(funding_txid, 1);
    let shared_loser = Txid::from_byte_array([0x53; 32]); // L
    let shared_winner = Txid::from_byte_array([0x54; 32]); // W
    let final_winner = Txid::from_byte_array([0x55; 32]); // X

    // The raw transactions are the same for both wallets — a record is the
    // whole on-chain transaction, inputs included — so each wallet's copy
    // claims both outpoints even though only one is its own coin.
    for w in [&w1, &w2] {
        let mut conn = persister.lock_conn_for_test();
        derive_address(&conn, w, 0, if w == &w1 { &addr1 } else { &addr2 });
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![tx_record(shared_loser, vec![p1, p2], vec![])],
            ..Default::default()
        };
        core_state::apply(&tx, w, &cs).unwrap();
        tx.commit().unwrap();
    }

    // First sweep in both wallets: W beats L, holding everything. Leaves
    // each wallet a placeholder row per claimed outpoint, attributed to W.
    for w in [&w1, &w2] {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![shared_loser],
                superseded_by: shared_winner,
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, w, &cs).unwrap();
        tx.commit().unwrap();
    }

    // W's own record lands in both wallets, as any wallet-relevant winner's
    // eventually does.
    for w in [&w1, &w2] {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![tx_record(shared_winner, vec![p1, p2], vec![])],
            ..Default::default()
        };
        core_state::apply(&tx, w, &cs).unwrap();
        tx.commit().unwrap();
    }

    // Second sweep, wallet 1 first: X beats W and wallet 1 releases its own
    // coin. Its copy of W's row is deleted in the same call.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![shared_winner],
                superseded_by: final_winner,
                released_outpoints: vec![p1],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w1, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let conn = persister.lock_conn_for_test();
        let gone: Option<Vec<u8>> = conn
            .query_row(
                "SELECT record_blob FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
                params![w1.as_slice(), AsRef::<[u8]>::as_ref(&shared_winner)],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert!(gone.is_none(), "wallet 1's own copy of W is deleted");
        let w2_placeholder: Option<Vec<u8>> = conn
            .query_row(
                "SELECT spent_in_txid FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
                params![w2.as_slice(), &blob::encode_outpoint(&p2).unwrap()[..]],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            w2_placeholder,
            Some(AsRef::<[u8]>::as_ref(&shared_winner).to_vec()),
            "wallet 1's whole chained sweep, deletion included, must leave wallet 2's \
             placeholder exactly where wallet 2's own first sweep put it"
        );
    }

    // Wallet 2's callback runs only now, holding its coin. Its own copy of
    // W is still on hand — nothing wallet 1 committed could have removed a
    // `(wallet_id, txid)`-keyed row of wallet 2's.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![shared_winner],
                superseded_by: final_winner,
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w2, &cs).unwrap();
        tx.commit().unwrap();
    }

    // The funding transaction finally arrives, each coin through its own
    // wallet's round.
    for (w, addr, vout) in [(&w1, &addr1, 0u32), (&w2, &addr2, 1u32)] {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            new_utxos: vec![make_utxo(addr, funding_txid, vout, 1_000)],
            ..Default::default()
        };
        core_state::apply(&tx, w, &cs).unwrap();
        tx.commit().unwrap();
    }

    let conn = persister.lock_conn_for_test();
    assert!(
        unspent(&conn, &w1).contains(&p1),
        "wallet 1's released coin comes back spendable once funded"
    );
    assert!(
        !unspent(&conn, &w2).contains(&p2),
        "wallet 2's held coin stays spent"
    );
    let (spent, spent_in_txid): (i64, Option<Vec<u8>>) = conn
        .query_row(
            "SELECT spent, spent_in_txid FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
            params![w2.as_slice(), &blob::encode_outpoint(&p2).unwrap()[..]],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(spent, 1);
    assert_eq!(
        spent_in_txid,
        Some(AsRef::<[u8]>::as_ref(&final_winner).to_vec()),
        "wallet 2's placeholder followed its own chain to the final winner, \
         driven entirely by wallet 2's own calls"
    );
}

/// Confirmation, not a fix: the review finding that motivated the Swift/
/// Kotlin backend changes (a shared `PersistentTransaction` row updated with
/// one wallet's `released_outpoints` before another wallet's own callback
/// gets a turn) has no analog here. `core_transactions` and `core_utxos` are
/// keyed by `(wallet_id, txid)` / `(wallet_id, outpoint)` — there is no row
/// for a "loser shared across wallets" to BE, only two wallets each holding
/// their own copy of a transaction that happens to carry the same txid.
/// `apply_sweep` re-derives every input from the loser's own stored blob and
/// matches `core_utxos` strictly within the calling wallet's rows, so one
/// wallet's sweep call cannot see, let alone touch, another wallet's copy.
///
/// This seeds the reviewer's exact shape — the same loser txid persisted
/// independently by two wallets, each holding a different coin of its own —
/// and sweeps them in opposite decisions (wallet 1 releases its coin,
/// wallet 2 holds its own) to show neither call perturbs the other wallet's
/// row at all, regardless of which runs first.
#[test]
fn sweep_of_a_shared_loser_txid_is_independent_per_wallet() {
    let (persister, _tmp, _path) = fresh_persister();
    let w1: WalletId = wid(0xE8);
    let w2: WalletId = wid(0xE9);
    ensure_wallet_meta(&persister, &w1);
    ensure_wallet_meta(&persister, &w2);

    let addr1 = p2pkh(0x31);
    let addr2 = p2pkh(0x32);
    let funding_txid = Txid::from_byte_array([0x30; 32]);
    // Same txid recorded independently in both wallets' storage — as two
    // wallets sharing one on-chain transaction each would.
    let loser_txid = Txid::from_byte_array([0x33; 32]);
    let winner_txid = Txid::from_byte_array([0x34; 32]);
    let coin = OutPoint::new(funding_txid, 0);

    for (w, addr) in [(&w1, &addr1), (&w2, &addr2)] {
        let mut conn = persister.lock_conn_for_test();
        derive_address(&conn, w, 0, addr);
        let tx = conn.transaction().unwrap();
        let funding = tx_record(
            funding_txid,
            vec![],
            vec![TxOut {
                value: 100_000,
                script_pubkey: addr.script_pubkey(),
            }],
        );
        let loser = tx_record(loser_txid, vec![coin], vec![]);
        let cs = CoreChangeSet {
            records: vec![funding, loser],
            new_utxos: vec![make_utxo(addr, funding_txid, 0, 100_000)],
            ..Default::default()
        };
        core_state::apply(&tx, w, &cs).unwrap();
        tx.commit().unwrap();
    }

    // Wallet 1 sweeps its copy of the loser and releases its own coin.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![loser_txid],
                superseded_by: winner_txid,
                released_outpoints: vec![coin],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w1, &cs).unwrap();
        tx.commit().unwrap();
    }

    {
        let conn = persister.lock_conn_for_test();
        let (spent, spent_in_txid): (i64, Option<Vec<u8>>) = conn
            .query_row(
                "SELECT spent, spent_in_txid FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
                params![w1.as_slice(), &blob::encode_outpoint(&coin).unwrap()[..]],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(spent, 0, "wallet 1's release frees its own coin");
        assert!(spent_in_txid.is_none());
        let w2_loser: Option<Vec<u8>> = conn
            .query_row(
                "SELECT record_blob FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
                params![w2.as_slice(), AsRef::<[u8]>::as_ref(&loser_txid)],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert!(
            w2_loser.is_some(),
            "wallet 2's own copy of the same-txid loser is a separate row, \
             untouched by wallet 1's sweep"
        );
        assert!(
            row_exists(&conn, &w2, &coin),
            "wallet 2's coin is unaffected — it has not swept yet"
        );
    }

    // Wallet 2 now sweeps its own copy of the same txid, releasing nothing.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![loser_txid],
                superseded_by: winner_txid,
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w2, &cs).unwrap();
        tx.commit().unwrap();
    }

    let conn = persister.lock_conn_for_test();
    let w2_loser: Option<Vec<u8>> = conn
        .query_row(
            "SELECT record_blob FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![w2.as_slice(), AsRef::<[u8]>::as_ref(&loser_txid)],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    assert!(
        w2_loser.is_none(),
        "wallet 2's own sweep removes its own row"
    );

    assert!(
        row_exists(&conn, &w2, &coin),
        "wallet 2 released nothing, so its coin stays held with a row of its own"
    );
    let (spent, spent_in_txid): (i64, Option<Vec<u8>>) = conn
        .query_row(
            "SELECT spent, spent_in_txid FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
            params![w2.as_slice(), &blob::encode_outpoint(&coin).unwrap()[..]],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(spent, 1, "wallet 2's coin is held spent");
    assert_eq!(
        spent_in_txid,
        Some(AsRef::<[u8]>::as_ref(&winner_txid).to_vec()),
        "held and attributed to wallet 2's own winner, per apply_sweep's hold contract — \
         wallet 1's earlier release of the SAME txid's other coin never touched this row"
    );
}

/// Confirmation, not a fix, of this round's BLOCKING finding (a shared row
/// acknowledged as durably swept by one wallet's commit while a second
/// wallet's own callback is still outstanding — see the Swift/Kotlin
/// `PersistentTransaction.isGloballySwept` / `TransactionEntity.
/// isGloballySwept` flag those backends needed to add). The finding does not
/// apply here for the same structural reason as the independence test
/// above: there is no shared row for a second wallet's callback to hold
/// back in the first place, so wallet 1's own deletion has no cross-wallet
/// dependency to be durable *despite*.
///
/// This confirms the corollary directly: wallet 1 sweeps and commits, wallet
/// 2's own callback for the same loser txid is never called again in this
/// test at all (a crash, a rejection, or it simply never coming), and the
/// persister is restarted from disk. Wallet 1's phantom output and row must
/// already be gone — nothing about their absence was waiting on wallet 2.
#[test]
fn sweep_deletion_is_durable_even_when_the_other_wallets_callback_never_arrives() {
    let (persister, _tmp, path) = fresh_persister();
    let w1: WalletId = wid(0xEA);
    let w2: WalletId = wid(0xEB);
    ensure_wallet_meta(&persister, &w1);
    ensure_wallet_meta(&persister, &w2);

    let addr1 = p2pkh(0x41);
    let addr2 = p2pkh(0x42);
    // Same loser txid recorded independently by both wallets, each with an
    // output of its own — the "phantom money" the blocking finding is about.
    let loser_txid = Txid::from_byte_array([0x43; 32]);
    let winner_txid = Txid::from_byte_array([0x44; 32]);

    for (w, addr) in [(&w1, &addr1), (&w2, &addr2)] {
        let mut conn = persister.lock_conn_for_test();
        derive_address(&conn, w, 0, addr);
        let tx = conn.transaction().unwrap();
        let loser = tx_record(
            loser_txid,
            vec![],
            vec![TxOut {
                value: 60_000,
                script_pubkey: addr.script_pubkey(),
            }],
        );
        let cs = CoreChangeSet {
            records: vec![loser],
            ..Default::default()
        };
        core_state::apply(&tx, w, &cs).unwrap();
        tx.commit().unwrap();
    }

    // Only wallet 1 ever sweeps. Wallet 2's own callback for this sweep
    // never arrives — this test never calls `apply` for w2 again.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![loser_txid],
                superseded_by: winner_txid,
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w1, &cs).unwrap();
        tx.commit().unwrap();
    }

    drop(persister);
    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let conn = persister.lock_conn_for_test();

    let w1_loser: Option<Vec<u8>> = conn
        .query_row(
            "SELECT record_blob FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![w1.as_slice(), AsRef::<[u8]>::as_ref(&loser_txid)],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    assert!(
        w1_loser.is_none(),
        "wallet 1's own sweep commit is durable across a restart on its own — \
         nothing about it was waiting on wallet 2's callback"
    );
    assert!(
        !row_exists(&conn, &w1, &OutPoint::new(loser_txid, 0)),
        "wallet 1's phantom output must not survive — its deletion never depended \
         on wallet 2's callback, which never arrives in this test"
    );

    // Wallet 2 never swept, so its own independent copy legitimately still
    // stands — that is correct per-wallet state, not the bug under test.
    let w2_loser: Option<Vec<u8>> = conn
        .query_row(
            "SELECT record_blob FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![w2.as_slice(), AsRef::<[u8]>::as_ref(&loser_txid)],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    assert!(
        w2_loser.is_some(),
        "wallet 2's own row is untouched — it never ran its own sweep"
    );
}

/// Confirmation, not a fix, of this round's BLOCKING finding on the mobile
/// backends (Swift `PersistentTransaction.isGloballySwept` / Kotlin
/// `TransactionEntity.isGloballySwept`): once a sweep is reversed by a
/// chainlocked return, a later-arriving record for the same txid must be
/// accepted as reinstatement rather than permanently rejected.
///
/// That guard exists on the mobile backends only because their
/// `PersistentTransaction` / `TransactionEntity` rows are shared across
/// wallets and durably flagged the moment *any* wallet's callback observes
/// the sweep, before every wallet's own claim is known to be gone — a
/// second wallet's still-outstanding claim can keep the row physically
/// present after the first wallet's commit, which is exactly what forces a
/// flag instead of relying on row-absence. `apply_sweep` here has no such
/// row to hold onto: it is keyed `(wallet_id, txid)`, so the delete is
/// unconditional and wallet-local (`sweep_of_a_shared_loser_txid_is_
/// independent_per_wallet` above), and a second wallet's own claim on the
/// same on-chain txid lives in an entirely separate row this wallet's sweep
/// never touches. There is therefore nothing left standing after `apply`
/// runs a sweep for the row's txid — no tombstone to clear, because there
/// is no row to protect from resurrection in the first place. A later round
/// carrying a plain record for the same `(wallet_id, txid)` is just an
/// ordinary `INSERT … ON CONFLICT DO UPDATE` into empty space, so this test
/// exercises that "reinstatement" is unconditionally already correct here,
/// across a separate `apply` call *and* a restart — the same cross-round
/// shape the mobile fix had to add tombstone-clearing for.
#[test]
fn a_record_reinstating_a_swept_txid_in_a_later_round_is_accepted_and_durable() {
    let (persister, _tmp, path) = fresh_persister();
    let w: WalletId = wid(0xEC);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x51);
    let txid = Txid::from_byte_array([0x53; 32]);
    let winner_txid = Txid::from_byte_array([0x54; 32]);
    let output = OutPoint::new(txid, 0);

    // Round 1: the transaction is recorded normally, with its own output.
    {
        let mut conn = persister.lock_conn_for_test();
        derive_address(&conn, &w, 0, &addr);
        let tx = conn.transaction().unwrap();
        let record = tx_record(
            txid,
            vec![],
            vec![TxOut {
                value: 45_000,
                script_pubkey: addr.script_pubkey(),
            }],
        );
        let cs = CoreChangeSet {
            records: vec![record],
            new_utxos: vec![make_utxo(&addr, txid, 0, 45_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    // Round 2, a separate `apply` call: an IS-locked conflict sweeps it —
    // the row and its output are gone, same as `sweep_only_changeset_
    // deletes_loser_row_and_its_outputs` above.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![txid],
                superseded_by: winner_txid,
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    {
        let conn = persister.lock_conn_for_test();
        let swept: Option<Vec<u8>> = conn
            .query_row(
                "SELECT record_blob FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
                params![w.as_slice(), AsRef::<[u8]>::as_ref(&txid)],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert!(swept.is_none(), "sanity: the sweep removed the row");
        assert!(
            !row_exists(&conn, &w, &output),
            "sanity: its output is gone too"
        );
    }

    // Round 3, yet another separate `apply` call: the wallet returns
    // chainlocked and sweeps the conflict in turn — upstream's newer word,
    // carried here as a plain record the same way any fresh transaction
    // would arrive. Nothing on this backend needs to know it is a
    // "reinstatement" rather than a first sighting.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let record = tx_record(
            txid,
            vec![],
            vec![TxOut {
                value: 45_000,
                script_pubkey: addr.script_pubkey(),
            }],
        );
        let cs = CoreChangeSet {
            records: vec![record],
            new_utxos: vec![make_utxo(&addr, txid, 0, 45_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    // Durable across a restart — not merely visible within the open
    // connection that just wrote it.
    drop(persister);
    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let conn = persister.lock_conn_for_test();

    let reinstated: Option<Vec<u8>> = conn
        .query_row(
            "SELECT record_blob FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![w.as_slice(), AsRef::<[u8]>::as_ref(&txid)],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    assert!(
        reinstated.is_some(),
        "the reinstating record must be live and durable — a later round is \
         upstream's newer word, and this backend has no tombstone standing \
         in its way"
    );
    assert!(
        row_exists(&conn, &w, &output),
        "the reinstated transaction's own output must be live and durable too"
    );
    assert!(
        unspent(&conn, &w).contains(&output),
        "and spendable — not left behind in some half-restored state"
    );
}

/// A release must land even when the swept txid has no `core_transactions`
/// row of its own. A chained-sweep claim is a `core_utxos` placeholder that
/// exists independently of any transaction row, and the loser now freeing
/// it need not have one — a fatal flush error wipes a buffered round (the
/// winner's record with it) while the faulted wallet keeps persisting later
/// rounds. `apply_sweep` returns before its input loop for a missing row,
/// so if that loop were the only place releases were applied the set would
/// be silently dropped and the upsert valve would hold the placeholder's
/// `spent_in_txid` forever.
#[test]
fn a_release_applies_even_when_the_swept_txid_has_no_row() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xE7);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x31);
    let funding_txid = Txid::from_byte_array([0x30; 32]);
    let p = OutPoint::new(funding_txid, 0);
    let loser_txid = Txid::from_byte_array([0x31; 32]); // L
    let winner_txid = Txid::from_byte_array([0x32; 32]); // W — never recorded
    let final_winner = Txid::from_byte_array([0x33; 32]); // X

    // Round 1: L, spending the still-unfunded P, is recorded and then swept
    // by W with nothing released — leaving the held-but-absent placeholder.
    {
        let mut conn = persister.lock_conn_for_test();
        derive_address(&conn, &w, 0, &addr);
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![tx_record(loser_txid, vec![p], vec![])],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![loser_txid],
                superseded_by: winner_txid,
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let conn = persister.lock_conn_for_test();
        assert!(
            row_exists(&conn, &w, &p),
            "sanity: the held claim left its placeholder"
        );
        assert!(unspent(&conn, &w).is_empty());
    }

    // Round 2: W is swept in turn, releasing P — but W's own record never
    // reached this store, so there is no row and no input loop to walk.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![winner_txid],
                superseded_by: final_winner,
                released_outpoints: vec![p],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let conn = persister.lock_conn_for_test();
        assert!(
            unspent(&conn, &w).contains(&p),
            "the release must reach the placeholder with no loser row to walk"
        );
    }

    // The funding output finally arrives: the shed hold must let the
    // upsert's valve accept the coin as unspent, with its real value.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            new_utxos: vec![make_utxo(&addr, funding_txid, 0, 50_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    let conn = persister.lock_conn_for_test();
    assert!(
        unspent(&conn, &w).contains(&p),
        "the funded coin stays spendable — the valve has no stale claim to defend"
    );
}

/// One batch can sweep a parent and the child that spends its output —
/// upstream's descendant closure always removes them together, and its
/// release computation filters out outpoints whose txid is itself a loser.
/// With the parent ordered first, its pass deletes the output row; the
/// child's pass must not re-create it as a held placeholder. The
/// placeholder's `spent_in_txid` is exactly what the funding upsert's
/// valve defends, so a chainlocked reinstatement of the parent — the one
/// event that can bring the coin back — would find its genuinely unspent
/// output locked out of the restore set forever.
#[test]
fn a_batch_sweeping_parent_and_child_leaves_no_placeholder_for_the_parents_output() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xE8);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x41);
    let parent_txid = Txid::from_byte_array([0x40; 32]); // L
    let child_txid = Txid::from_byte_array([0x41; 32]); // C
    let winner_txid = Txid::from_byte_array([0x42; 32]); // W
    let parent_output = OutPoint::new(parent_txid, 0);

    // L pays us and is funded; C spends L's output.
    {
        let mut conn = persister.lock_conn_for_test();
        derive_address(&conn, &w, 0, &addr);
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![
                tx_record(
                    parent_txid,
                    vec![],
                    vec![TxOut {
                        value: 5_000,
                        script_pubkey: addr.script_pubkey(),
                    }],
                ),
                tx_record(child_txid, vec![parent_output], vec![]),
            ],
            new_utxos: vec![make_utxo(&addr, parent_txid, 0, 5_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    // The batch removes both, parent first — the ordering that deletes the
    // output row before the child's pass walks its inputs.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![parent_txid, child_txid],
                superseded_by: winner_txid,
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let conn = persister.lock_conn_for_test();
        assert!(
            !row_exists(&conn, &w, &parent_output),
            "a dead parent's output is nobody's coin — no placeholder may re-create it"
        );
    }

    // The chainlocked return: L is reinstated with its output re-emitted.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![tx_record(
                parent_txid,
                vec![],
                vec![TxOut {
                    value: 5_000,
                    script_pubkey: addr.script_pubkey(),
                }],
            )],
            new_utxos: vec![make_utxo(&addr, parent_txid, 0, 5_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    let conn = persister.lock_conn_for_test();
    assert!(
        unspent(&conn, &w).contains(&parent_output),
        "the reinstated parent's genuinely unspent output must restore — no stale \
         spent_in_txid claim may stand in its way"
    );
}

/// The record-loss half of the co-swept rule. A parent whose record this
/// store lost (the same threat the by-outpoint release pass exists for)
/// deletes nothing in its own pass, so the child's pass must take the
/// surviving output row out of the restore set itself — leaving it
/// `spent = 0` would hand back a phantom spendable coin. And it must do
/// so by DELETING the row, not by holding it: a `spent_in_txid` claim is
/// exactly what the funding upsert's valve defends, which would lock out
/// the chainlocked reinstatement that is the one event able to bring the
/// coin back for real.
#[test]
fn a_co_swept_parent_with_no_row_still_has_its_output_removed() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xE9);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x51);
    let parent_txid = Txid::from_byte_array([0x50; 32]); // P — record lost
    let child_txid = Txid::from_byte_array([0x51; 32]); // C
    let winner_txid = Txid::from_byte_array([0x52; 32]); // W
    let parent_output = OutPoint::new(parent_txid, 0);

    // P's record round was wiped, but its funded output row and C's record
    // both persisted.
    {
        let mut conn = persister.lock_conn_for_test();
        derive_address(&conn, &w, 0, &addr);
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![tx_record(child_txid, vec![parent_output], vec![])],
            new_utxos: vec![make_utxo(&addr, parent_txid, 0, 5_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let conn = persister.lock_conn_for_test();
        assert!(
            unspent(&conn, &w).contains(&parent_output),
            "sanity: the parent's output starts live"
        );
    }

    // The batch sweeps both. P's pass finds no row and deletes nothing; the
    // child's claim on P:0 is the only thing that can take the dead coin
    // out of the unspent set.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![parent_txid, child_txid],
                superseded_by: winner_txid,
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let conn = persister.lock_conn_for_test();
        assert!(
            !unspent(&conn, &w).contains(&parent_output),
            "a dead parent's output must not survive as a phantom spendable coin \
             just because the parent's own record was lost"
        );
        assert!(
            !row_exists(&conn, &w, &parent_output),
            "and it must be deleted, not held — a spent_in_txid claim would lock \
             out the reinstatement below"
        );
    }

    // The chainlocked return: P is reinstated with its output re-emitted,
    // and nothing this sweep left behind may stand in its way.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![tx_record(
                parent_txid,
                vec![],
                vec![TxOut {
                    value: 5_000,
                    script_pubkey: addr.script_pubkey(),
                }],
            )],
            new_utxos: vec![make_utxo(&addr, parent_txid, 0, 5_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    let conn = persister.lock_conn_for_test();
    assert!(
        unspent(&conn, &w).contains(&parent_output),
        "the reinstated parent's genuinely unspent output must restore even when \
         its record was lost at sweep time"
    );
}

/// The second route into the co-swept-parent corner: P:0's row exists only
/// as the synthetic spent-only row `derive_spent_utxos` wrote when C's
/// record arrived IN ORDER (P's own record and funding never persisted —
/// weaker preconditions than the record-loss shape, no lost round needed).
/// The co-swept rule must treat it exactly like any other row for a dead
/// parent's output: DELETE it, never attribute it to the winner — a
/// `spent_in_txid` hold on it would survive into the upsert valve and lock
/// out P's chainlocked reinstatement forever, since no release ever names
/// a loser-funded outpoint.
#[test]
fn a_co_swept_parent_known_only_through_the_childs_spend_is_still_removed() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xEA);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x61);
    let parent_txid = Txid::from_byte_array([0x60; 32]); // P — never recorded
    let child_txid = Txid::from_byte_array([0x61; 32]); // C
    let winner_txid = Txid::from_byte_array([0x62; 32]); // W
    let parent_output = OutPoint::new(parent_txid, 0);

    // C arrives in order, spending P:0 — the spent-utxos apply writes the
    // synthetic spent-only row because no funded row exists.
    {
        let mut conn = persister.lock_conn_for_test();
        derive_address(&conn, &w, 0, &addr);
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![tx_record(child_txid, vec![parent_output], vec![])],
            spent_utxos: vec![make_utxo(&addr, parent_txid, 0, 5_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let conn = persister.lock_conn_for_test();
        assert!(
            row_exists(&conn, &w, &parent_output),
            "sanity: the synthetic spent-only row exists"
        );
        assert!(unspent(&conn, &w).is_empty());
    }

    // The batch sweeps both; P's pass has no record to walk, so only the
    // co-swept rule in C's pass can decide the synthetic row's fate.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![parent_txid, child_txid],
                superseded_by: winner_txid,
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let conn = persister.lock_conn_for_test();
        assert!(
            !row_exists(&conn, &w, &parent_output),
            "the dead parent's output must be deleted, not attributed to the winner"
        );
    }

    // The chainlocked return: P reinstated with its output re-emitted must
    // land spendable — nothing this sweep left behind may block the valve.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![tx_record(
                parent_txid,
                vec![],
                vec![TxOut {
                    value: 5_000,
                    script_pubkey: addr.script_pubkey(),
                }],
            )],
            new_utxos: vec![make_utxo(&addr, parent_txid, 0, 5_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    let conn = persister.lock_conn_for_test();
    assert!(
        unspent(&conn, &w).contains(&parent_output),
        "the reinstated parent's output must restore even when its pre-sweep row \
         was only ever the synthetic spent-only one"
    );
}
