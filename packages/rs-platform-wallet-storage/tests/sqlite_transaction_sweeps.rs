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
use platform_wallet_storage::sqlite::schema::core_state;
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
