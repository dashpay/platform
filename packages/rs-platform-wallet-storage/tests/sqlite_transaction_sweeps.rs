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

/// Mined height carried by every block-context sweep in these tests
/// unless a test pins its own. High enough that the pre-seeded funding
/// heights (10) and default watermarks sit below it.
const WINNER_HEIGHT: u32 = 400;

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
                winner_mined_height: Some(WINNER_HEIGHT),
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
            winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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

/// The reviewer scenario for the pruned-finalized-release defect: upstream
/// computes `released_outpoints` from its live records, and a chainlocked
/// spender F is pruned to a bare txid under the default
/// `keep-finalized-transactions = off` — so a loser L that arrived after the
/// pruning, reusing F's input alongside an attacker-owned one, reports F's
/// input as released when a final W beats L on the attacker input. The
/// pinned `TransactionsSwept` doc calls this unresolvable at its layer; this
/// store is the layer that CAN resolve it, because F's full record survives
/// in `core_transactions`. The release must be refused for F's coin — and
/// still honoured for a coin only the swept loser claimed, in the same
/// batch, or the guard would strand legitimately freed money.
#[test]
fn a_release_naming_a_coin_a_stored_finalized_record_claims_is_refused() {
    use key_wallet::transaction_checking::transaction_context::BlockInfo;

    let (persister, _tmp, path) = fresh_persister();
    let w: WalletId = wid(0xE9);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x09);
    let funding_txid = Txid::from_byte_array([0x90; 32]);
    // F's coin — consumed on chain, must never come back.
    let settled_coin = OutPoint::new(funding_txid, 0);
    // Claimed only by the loser — must come free.
    let losers_own_coin = OutPoint::new(funding_txid, 1);
    // Attacker-owned input shared by L and W — never ours, no row.
    let attacker_input = OutPoint::new(Txid::from_byte_array([0x9A; 32]), 0);

    let finalized_txid = Txid::from_byte_array([0x91; 32]);
    let loser_txid = Txid::from_byte_array([0x92; 32]);
    let winner_txid = Txid::from_byte_array([0x93; 32]);

    // F: chainlocked spender of `settled_coin`. Upstream keeps only its
    // txid from here on; this row keeps everything.
    let mut finalized = tx_record(
        finalized_txid,
        vec![settled_coin],
        vec![TxOut {
            value: 400,
            script_pubkey: addr.script_pubkey(),
        }],
    );
    finalized.context = TransactionContext::InChainLockedBlock(BlockInfo::new(
        42,
        dashcore::BlockHash::from_byte_array([0x9B; 32]),
        1_735_689_600,
    ));

    // L: arrives after F's pruning, pays this wallet, reuses F's input and
    // the attacker's.
    let loser = tx_record(
        loser_txid,
        vec![settled_coin, attacker_input, losers_own_coin],
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
                make_utxo(&addr, funding_txid, 0, 400),
                make_utxo(&addr, funding_txid, 1, 600),
            ],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![finalized],
            spent_utxos: vec![make_utxo(&addr, funding_txid, 0, 400)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![loser],
            spent_utxos: vec![make_utxo(&addr, funding_txid, 1, 600)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    // W (final) conflicts with L on the attacker input alone. Upstream's
    // release set — computed from live records that no longer include F —
    // wrongly names F's coin alongside the loser's own.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![loser_txid],
                superseded_by: winner_txid,
                winner_mined_height: Some(WINNER_HEIGHT),
                released_outpoints: vec![settled_coin, losers_own_coin],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    let visible = unspent(&conn, &w);
    assert!(
        !visible.contains(&settled_coin),
        "a released coin a stored finalized record still claims must stay spent"
    );
    assert!(
        visible.contains(&losers_own_coin),
        "a coin only the swept loser claimed must still come free"
    );

    drop(conn);
    drop(persister);

    // Restart: the guard's verdict must be what a relaunch loads — this is
    // exactly where the unguarded release manufactured the double spend.
    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let conn = persister.lock_conn_for_test();
    let visible = unspent(&conn, &w);
    assert!(
        !visible.contains(&settled_coin),
        "the refused release must hold across a restart"
    );
    assert!(
        visible.contains(&losers_own_coin),
        "the honoured release must hold across a restart"
    );
}

/// The stored-claims veto is settled-evidence only: a bare mempool row must
/// not outrank an authoritative release. A mempool record is the one context
/// that can go stale forever — an evicted or abandoned mempool transaction
/// has no removal path in this store other than a later sweep, and
/// restoration does not repopulate ordinary history — so a stale claimant
/// surviving a restart must not veto the release of a coin a later loser
/// claimed, or the coin is attributed to an unrelated winner and stranded
/// durably spent: the mirror image of the wrong-release bug the veto exists
/// to stop.
#[test]
fn a_stale_mempool_claimant_does_not_veto_an_authoritative_release() {
    let (persister, _tmp, path) = fresh_persister();
    let w: WalletId = wid(0xEA);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x0A);
    let funding_txid = Txid::from_byte_array([0xA0; 32]);
    let coin = OutPoint::new(funding_txid, 0);

    let stale_txid = Txid::from_byte_array([0xA1; 32]);
    let loser_txid = Txid::from_byte_array([0xA2; 32]);
    let winner_txid = Txid::from_byte_array([0xA3; 32]);

    // M: a mempool spend of the coin, marked spent when recorded. It is
    // then evicted from the network's mempool without the wallet ever
    // hearing — its row simply goes stale.
    let stale = tx_record(
        stale_txid,
        vec![coin],
        vec![TxOut {
            value: 400,
            script_pubkey: addr.script_pubkey(),
        }],
    );

    {
        let mut conn = persister.lock_conn_for_test();
        derive_address(&conn, &w, 0, &addr);
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            new_utxos: vec![make_utxo(&addr, funding_txid, 0, 400)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();

        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![stale],
            spent_utxos: vec![make_utxo(&addr, funding_txid, 0, 400)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    // Restart: upstream's memory of M is gone for good; only the stale row
    // remains.
    drop(persister);
    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let mut conn = persister.lock_conn_for_test();

    // A fresh loser claims the same coin, and an authoritative sweep later
    // frees it — upstream's word, computed from the wallet it actually
    // holds.
    let loser = tx_record(
        loser_txid,
        vec![coin],
        vec![TxOut {
            value: 300,
            script_pubkey: addr.script_pubkey(),
        }],
    );
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![loser],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![loser_txid],
                superseded_by: winner_txid,
                winner_mined_height: Some(WINNER_HEIGHT),
                released_outpoints: vec![coin],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    assert!(
        unspent(&conn, &w).contains(&coin),
        "a stale mempool claimant must not veto an authoritative release"
    );
}

/// The stored-claims scan is the final guard against re-crediting a
/// consumed coin, so corrupt claimant rows fail the round instead of
/// silently losing their veto: a wrong-length `txid` key and a record blob
/// whose decoded txid disagrees with its typed key are both `BlobDecode`
/// errors.
#[test]
fn a_corrupt_stored_claimant_fails_the_sweep_round_closed() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xEB);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x0B);
    let funding_txid = Txid::from_byte_array([0xB0; 32]);
    let coin = OutPoint::new(funding_txid, 0);
    let loser_txid = Txid::from_byte_array([0xB1; 32]);
    let winner_txid = Txid::from_byte_array([0xB2; 32]);

    let loser = tx_record(
        loser_txid,
        vec![coin],
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
            new_utxos: vec![make_utxo(&addr, funding_txid, 0, 400)],
            records: vec![loser],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    // A claimant row whose typed key is not 32 bytes. The blob is a real,
    // decodable record so the failure is attributable to the key alone.
    let honest = tx_record(Txid::from_byte_array([0xB3; 32]), vec![coin], Vec::new());
    let honest_blob = blob::encode(&honest).unwrap();
    conn.execute(
        "INSERT INTO core_transactions \
            (wallet_id, txid, height, block_hash, block_time, finalized, record_blob) \
         VALUES (?1, ?2, NULL, NULL, NULL, 1, ?3)",
        params![w.as_slice(), &[0xB3u8; 31][..], &honest_blob[..]],
    )
    .unwrap();

    let sweep_cs = CoreChangeSet {
        sweeps: vec![SweepBatch {
            txids: vec![loser_txid],
            superseded_by: winner_txid,
            winner_mined_height: Some(WINNER_HEIGHT),
            released_outpoints: vec![coin],
        }],
        ..Default::default()
    };
    {
        let tx = conn.transaction().unwrap();
        let err = core_state::apply(&tx, &w, &sweep_cs).unwrap_err();
        assert!(
            matches!(
                err,
                platform_wallet_storage::sqlite::error::WalletStorageError::BlobDecode { .. }
            ),
            "a wrong-length claimant key must fail the round closed, got {err:?}"
        );
    }

    // Repair the key length but leave it disagreeing with the record's own
    // txid — the typed key decides swept-loser exclusion, so the mismatch
    // must fail too.
    conn.execute(
        "DELETE FROM core_transactions WHERE wallet_id = ?1 AND length(txid) = 31",
        params![w.as_slice()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO core_transactions \
            (wallet_id, txid, height, block_hash, block_time, finalized, record_blob) \
         VALUES (?1, ?2, NULL, NULL, NULL, 1, ?3)",
        params![w.as_slice(), &[0xB4u8; 32][..], &honest_blob[..]],
    )
    .unwrap();
    {
        let tx = conn.transaction().unwrap();
        let err = core_state::apply(&tx, &w, &sweep_cs).unwrap_err();
        assert!(
            matches!(
                err,
                platform_wallet_storage::sqlite::error::WalletStorageError::BlobDecode { .. }
            ),
            "a key/record txid mismatch must fail the round closed, got {err:?}"
        );
    }
}

/// The sweep's own lookup must validate what it decodes, not only the claim
/// scan's rows. The two readers key on the same column for opposite reasons:
/// `surviving_stored_input_claims` skips a row whose typed key is a swept
/// loser (its blob never joins the veto set), and `apply_sweep` selects a row
/// BY that key and then acts on the blob's inputs. A row keyed as loser L but
/// holding settled record F therefore lands in the one gap where neither
/// reader looks at the other's evidence: F's claim is waived by key, and F's
/// inputs are processed as L's. If the release set names a coin F consumed,
/// the unvalidated path marks that coin unspent and deletes F's row — the
/// only stored proof of its spender — manufacturing exactly the double spend
/// the veto exists to stop.
///
/// The sibling test above pins the mismatch on a row that is NOT in the sweep
/// batch, which the claim scan rejects on its own; this one puts the
/// mismatched key inside the batch, where only `apply_sweep`'s check stands
/// between the corrupt row and the coin.
#[test]
fn a_swept_key_disagreeing_with_its_stored_record_fails_the_round_closed() {
    use key_wallet::transaction_checking::transaction_context::BlockInfo;

    let (persister, _tmp, path) = fresh_persister();
    let w: WalletId = wid(0xEC);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x0C);
    let funding_txid = Txid::from_byte_array([0xC0; 32]);
    // F's coin — consumed on chain by a settled record, must never come back.
    let settled_coin = OutPoint::new(funding_txid, 0);

    let finalized_txid = Txid::from_byte_array([0xC5; 32]);
    let loser_txid = Txid::from_byte_array([0xC1; 32]);
    let winner_txid = Txid::from_byte_array([0xC2; 32]);

    // F: chainlocked spender of `settled_coin`.
    let mut finalized = tx_record(
        finalized_txid,
        vec![settled_coin],
        vec![TxOut {
            value: 300,
            script_pubkey: addr.script_pubkey(),
        }],
    );
    finalized.context = TransactionContext::InChainLockedBlock(BlockInfo::new(
        42,
        dashcore::BlockHash::from_byte_array([0xCB; 32]),
        1_735_689_600,
    ));

    let mut conn = persister.lock_conn_for_test();
    derive_address(&conn, &w, 0, &addr);
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            new_utxos: vec![make_utxo(&addr, funding_txid, 0, 400)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            spent_utxos: vec![make_utxo(&addr, funding_txid, 0, 400)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    // The corruption: F's blob stored under L's typed key. Written straight
    // through SQL because no writer in this store can produce it — the point
    // is what the reader does when the invariant is already broken on disk.
    let finalized_blob = blob::encode(&finalized).unwrap();
    conn.execute(
        "INSERT INTO core_transactions \
            (wallet_id, txid, height, block_hash, block_time, finalized, record_blob) \
         VALUES (?1, ?2, NULL, NULL, NULL, 1, ?3)",
        params![
            w.as_slice(),
            AsRef::<[u8]>::as_ref(&loser_txid),
            &finalized_blob[..]
        ],
    )
    .unwrap();

    // The sweep names L and releases the coin F consumed.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![loser_txid],
                superseded_by: winner_txid,
                winner_mined_height: Some(WINNER_HEIGHT),
                released_outpoints: vec![settled_coin],
            }],
            ..Default::default()
        };
        let err = core_state::apply(&tx, &w, &cs).unwrap_err();
        assert!(
            matches!(
                err,
                platform_wallet_storage::sqlite::error::WalletStorageError::BlobDecode { .. }
            ),
            "a swept key disagreeing with its stored record must fail the round closed, got {err:?}"
        );
    }

    // Failing closed is only worth anything if the round left nothing behind:
    // the coin stays consumed and the row survives to be repaired.
    let visible = unspent(&conn, &w);
    assert!(
        !visible.contains(&settled_coin),
        "the refused round must not release the coin the stored record consumed"
    );
    let rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![w.as_slice(), AsRef::<[u8]>::as_ref(&loser_txid)],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        rows, 1,
        "the refused round must not delete the record it could not validate"
    );

    drop(conn);
    drop(persister);

    // And the verdict is what a relaunch loads — the unguarded path's damage
    // was durable, so the guard's refusal has to be too.
    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let conn = persister.lock_conn_for_test();
    assert!(
        !unspent(&conn, &w).contains(&settled_coin),
        "the refused release must hold across a restart"
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                    winner_mined_height: Some(WINNER_HEIGHT),
                    released_outpoints: vec![contested],
                },
                // The second winner consumed the coin, so this sweep frees
                // nothing — and that has to override the release above.
                SweepBatch {
                    txids: vec![second_loser],
                    superseded_by: Txid::from_byte_array([0x7b; 32]),
                    winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
                released_outpoints: vec![unfunded_input],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    assert!(
        !row_exists(&conn, &w, &unfunded_input),
        "the chained sweep released this input while its funding TXO is \
         still unobserved — the placeholder must be deleted outright, not \
         flipped to a zero-value phantom that list_unspent would report"
    );
    assert!(
        !unspent(&conn, &w).contains(&funded_input),
        "the second sweep's winner took the other input"
    );

    // The funding output finally classifies: with the dead claim's row gone,
    // the ordinary upsert creates the coin freshly unspent with real data.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            new_utxos: vec![make_utxo(&addr, funding_txid, 0, 50_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    assert!(
        unspent(&conn, &w).contains(&unfunded_input),
        "the released coin arrives as an ordinary spendable UTXO once its \
         funding output classifies"
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
            !row_exists(&conn, &w, &p),
            "the release must reach the placeholder with no loser row to walk \
             — and delete it outright, since it never materialised"
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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
                winner_mined_height: Some(WINNER_HEIGHT),
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

// ───────────────────────── tombstone collection ─────────────────────────
//
// A held-but-absent placeholder exists only for a block-context sweep and
// stores the winner's own mined height; `collect_finalized_tombstones`
// deletes it exactly when `min(chainlock_height, synced_height)` reaches
// that height — upstream's `prune_finalized_observed_spends` condition
// verbatim, no observation-age margin. These tests drive both the creation
// gate and the collector through ordinary `core_state::apply` rounds.

fn chain_lock_at(height: u32) -> dashcore::ephemerealdata::chain_lock::ChainLock {
    use dashcore::bls_sig_utils::BLSSignature;
    use dashcore::BlockHash;
    dashcore::ephemerealdata::chain_lock::ChainLock {
        block_height: height,
        block_hash: BlockHash::from_byte_array([0xCC; 32]),
        signature: BLSSignature::from([0u8; 96]),
    }
}

/// Apply a round carrying only chain progress: processed/synced watermarks
/// and a chainlock at `height`.
fn apply_heights(conn: &mut rusqlite::Connection, w: &WalletId, height: u32) {
    let tx = conn.transaction().unwrap();
    let cs = CoreChangeSet {
        last_processed_height: Some(height),
        synced_height: Some(height),
        last_applied_chain_lock: Some(chain_lock_at(height)),
        ..Default::default()
    };
    core_state::apply(&tx, w, &cs).unwrap();
    tx.commit().unwrap();
}

/// `(spent, height, winner_mined_height)` of a `core_utxos` row, or `None`
/// when absent.
fn utxo_row_state(
    conn: &rusqlite::Connection,
    w: &WalletId,
    op: &OutPoint,
) -> Option<(bool, Option<i64>, Option<i64>)> {
    let bytes = blob::encode_outpoint(op).unwrap();
    conn.query_row(
        "SELECT spent, height, winner_mined_height FROM core_utxos \
         WHERE wallet_id = ?1 AND outpoint = ?2",
        params![w.as_slice(), &bytes[..]],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .optional()
    .unwrap()
}

/// Record a loser spending `input` (no funding row exists), then sweep it
/// in the given winner context — `Some(height)` leaves the held-but-absent
/// placeholder stamped with the winner's mined height, `None` (an
/// IS-locked, unmined winner) leaves the same placeholder unstamped, which
/// the collector never touches.
fn seed_tombstone(
    conn: &mut rusqlite::Connection,
    w: &WalletId,
    input: OutPoint,
    loser: Txid,
    winner: Txid,
    winner_mined_height: Option<u32>,
) {
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![tx_record(loser, vec![input], vec![])],
            ..Default::default()
        };
        core_state::apply(&tx, w, &cs).unwrap();
        tx.commit().unwrap();
    }
    let tx = conn.transaction().unwrap();
    let cs = CoreChangeSet {
        sweeps: vec![SweepBatch {
            txids: vec![loser],
            superseded_by: winner,
            winner_mined_height,
            released_outpoints: vec![],
        }],
        ..Default::default()
    };
    core_state::apply(&tx, w, &cs).unwrap();
    tx.commit().unwrap();
}

/// A block-context placeholder stores the WINNER'S mined height and is
/// collected exactly when `min(chainlock_height, synced_height)` reaches
/// it — upstream's `prune_finalized_observed_spends` condition verbatim,
/// no observation-age margin. At that boundary the funding transaction of
/// the outpoint (necessarily mined at or below the winner's height) has
/// been filter-scanned with no false negatives, so an unmaterialised row
/// is provably not the wallet's coin.
#[test]
fn a_never_materialised_tombstone_is_collected_at_finality_and_not_before() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xF1);
    ensure_wallet_meta(&persister, &w);

    let p = OutPoint::new(Txid::from_byte_array([0x50; 32]), 0);
    let loser = Txid::from_byte_array([0x51; 32]);
    let winner = Txid::from_byte_array([0x52; 32]);

    let mut conn = persister.lock_conn_for_test();
    apply_heights(&mut conn, &w, 100);
    seed_tombstone(&mut conn, &w, p, loser, winner, Some(WINNER_HEIGHT));

    assert_eq!(
        utxo_row_state(&conn, &w, &p),
        Some((true, None, Some(i64::from(WINNER_HEIGHT)))),
        "sanity: the sweep left a held, never-materialised row stamped with \
         the winner's own mined height — not any observation watermark"
    );

    // Boundary one below the winner's height: the winner's block is not
    // yet inside the finality boundary, so the hold must survive.
    apply_heights(&mut conn, &w, WINNER_HEIGHT - 1);
    assert!(
        row_exists(&conn, &w, &p),
        "boundary {} has not reached the winner's height {} — the hold stays",
        WINNER_HEIGHT - 1,
        WINNER_HEIGHT
    );

    apply_heights(&mut conn, &w, WINNER_HEIGHT);
    assert!(
        !row_exists(&conn, &w, &p),
        "the boundary reaching the winner's height collects the row"
    );
}

/// The reviewer's unrelated-advancement scenario, block-context half: the
/// chainlock can run arbitrarily far ahead, but while `synced_height` sits
/// below the winner's mined height the boundary has not reached the spend
/// and the hold must survive — the funding output could still be delivered
/// by the unscanned range. It collects the moment the synced height
/// catches up.
#[test]
fn a_block_context_tombstone_outlives_unrelated_advancement_below_its_winners_height() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xF7);
    ensure_wallet_meta(&persister, &w);

    let p = OutPoint::new(Txid::from_byte_array([0x62; 32]), 0);
    let loser = Txid::from_byte_array([0x63; 32]);
    let winner = Txid::from_byte_array([0x64; 32]);

    let mut conn = persister.lock_conn_for_test();
    seed_tombstone(&mut conn, &w, p, loser, winner, Some(WINNER_HEIGHT));

    // Chainlocks race ahead by thousands of blocks; the filter scan has
    // only reached one block short of the winner.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            synced_height: Some(WINNER_HEIGHT - 1),
            last_applied_chain_lock: Some(chain_lock_at(WINNER_HEIGHT + 10_000)),
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    assert!(
        row_exists(&conn, &w, &p),
        "min(chainlock, synced) = {} is below the winner's height {} — any \
         amount of unrelated chainlock progress must not collect the hold",
        WINNER_HEIGHT - 1,
        WINNER_HEIGHT
    );

    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            synced_height: Some(WINNER_HEIGHT),
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    assert!(
        !row_exists(&conn, &w, &p),
        "the scan reaching the winner's height completes the boundary and collects"
    );
}

/// A mempool-context sweep — an InstantSend-locked winner that has not
/// mined — preserves an UNSTAMPED tombstone for every held-but-unfunded
/// input. Under DIP-10 the IS lock alone settles those inputs: upstream's
/// `drop_conflicted_transactions` deletes the loser and retains them in
/// the account's `spent_outpoints`, a hold that carries no height and
/// that nothing can reconstruct from records once the loser is gone (the
/// winner need not be wallet-relevant). The row is that hold's only
/// durable carrier — `CORE_SWEEP_REMOVAL` requires every non-released
/// input to keep a durable spend claim before its funding TXO
/// materialises — and it is unstamped because an IS-locked winner has no
/// mining deadline, so no boundary may ever collect it; resolution is the
/// funding upsert, a later block-context re-stamp, or a release.
#[test]
fn a_mempool_context_sweep_preserves_an_unstamped_tombstone() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xF8);
    ensure_wallet_meta(&persister, &w);

    let mut conn = persister.lock_conn_for_test();
    // Several IS-context sweeps in a row, each with a distinct
    // held-but-unfunded input.
    for i in 0u8..3 {
        let p = OutPoint::new(Txid::from_byte_array([0x70 + i; 32]), 0);
        let loser = Txid::from_byte_array([0x80 + i; 32]);
        let winner = Txid::from_byte_array([0x90 + i; 32]);
        seed_tombstone(&mut conn, &w, p, loser, winner, None);
        assert_eq!(
            utxo_row_state(&conn, &w, &p),
            Some((true, None, None)),
            "an unmined IS-locked winner must leave a held, unstamped \
             placeholder for input #{i}"
        );
    }
    // Arbitrary chainlock/height advancement never collects an unstamped
    // hold — two rounds, so a back-filling collector would be caught too.
    apply_heights(&mut conn, &w, 1_000_000);
    apply_heights(&mut conn, &w, 1_000_010);
    let rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_utxos WHERE wallet_id = ?1 \
             AND spent = 1 AND winner_mined_height IS NULL",
            params![w.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        rows, 3,
        "every unstamped hold outlasts any boundary — only funding \
         materialisation, a block-context re-stamp, or a release resolves one"
    );
}

/// The mempool-context sweep still spend-marks a coin that HAS
/// materialised: the row carries real funding data, so holding it costs
/// nothing an attacker controls, and the winner's own record (or its
/// eventual block delivery) is the durable evidence. Its stamp stays NULL
/// — a materialised row is outside the collector's reach anyway.
#[test]
fn a_mempool_context_sweep_still_spend_marks_a_materialised_coin() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xF9);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x65);
    let funding_txid = Txid::from_byte_array([0x66; 32]);
    let p = OutPoint::new(funding_txid, 0);
    let loser = Txid::from_byte_array([0x67; 32]);
    let winner = Txid::from_byte_array([0x68; 32]);

    let mut conn = persister.lock_conn_for_test();
    derive_address(&conn, &w, 0, &addr);
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            new_utxos: vec![make_utxo(&addr, funding_txid, 0, 50_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    seed_tombstone(&mut conn, &w, p, loser, winner, None);
    assert_eq!(
        utxo_row_state(&conn, &w, &p),
        Some((true, Some(10), None)),
        "a materialised coin is spend-marked by the IS-locked winner, with \
         no stamp — its funding data is real and the collector never sees it"
    );
}

/// The reviewer's named regression: an IS-locked winner sweeps on the
/// mempool path and never mines, the app restarts, chainlocks and heights
/// advance arbitrarily, and only then is the funding output delivered.
/// Under DIP-10 the IS lock already settled that input — upstream deleted
/// the loser and retained the hold in the account's `spent_outpoints`, a
/// set rebuilt from records on load that no surviving record can
/// reconstruct (the winner need not be wallet-relevant). The unstamped
/// tombstone is therefore the claim's only durable carrier, and the
/// funding upsert must land ON it and stay spent: crediting the coin
/// would hand coin selection an outpoint the network has provably
/// consumed. This is `CORE_SWEEP_REMOVAL`'s contract verbatim — every
/// non-released input retains a durable spend claim even before its
/// funding TXO materialises.
#[test]
fn a_funding_output_arriving_after_a_mempool_sweep_and_restart_lands_spent() {
    let (persister, tmp, path) = fresh_persister();
    let w: WalletId = wid(0xFA);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x69);
    let funding_txid = Txid::from_byte_array([0x6A; 32]);
    let p = OutPoint::new(funding_txid, 0);
    let loser = Txid::from_byte_array([0x6B; 32]);
    let winner = Txid::from_byte_array([0x6C; 32]);

    {
        let mut conn = persister.lock_conn_for_test();
        derive_address(&conn, &w, 0, &addr);
        seed_tombstone(&mut conn, &w, p, loser, winner, None);
    }
    // Restart.
    drop(persister);
    let cfg = SqlitePersisterConfig::new(&path);
    let persister = SqlitePersister::open(cfg).expect("reopen");

    let mut conn = persister.lock_conn_for_test();
    // Arbitrary chainlock/height advancement while the winner stays
    // unmined — none of it may collect the unstamped hold.
    apply_heights(&mut conn, &w, 25_000);
    assert_eq!(
        utxo_row_state(&conn, &w, &p),
        Some((true, None, None)),
        "the unstamped hold survives the restart and every boundary"
    );

    // The funding output is finally delivered and classified: the upsert
    // materialises the row (real height, stamp stays clear) and the
    // `spent_in_txid` valve keeps the coin spent.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            new_utxos: vec![make_utxo(&addr, funding_txid, 0, 50_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    assert!(
        !unspent(&conn, &w).contains(&p),
        "an input the IS-locked winner consumed must never come back \
         spendable — the sweep's claim outlives the restart"
    );
    assert_eq!(
        utxo_row_state(&conn, &w, &p),
        Some((true, Some(10), None)),
        "materialised on the tombstone: real funding height, still spent, \
         permanently outside the collector's reach"
    );
    drop(conn);
    drop(tmp);
}

/// Synced height alone is not finality: with no chainlock ever persisted
/// the collector must not run, mirroring upstream's "no-op until a
/// chainlock has been applied".
#[test]
fn a_tombstone_is_never_collected_without_a_persisted_chainlock() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xF2);
    ensure_wallet_meta(&persister, &w);

    let p = OutPoint::new(Txid::from_byte_array([0x53; 32]), 0);
    let loser = Txid::from_byte_array([0x54; 32]);
    let winner = Txid::from_byte_array([0x55; 32]);

    let mut conn = persister.lock_conn_for_test();
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            last_processed_height: Some(100),
            synced_height: Some(100),
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    seed_tombstone(&mut conn, &w, p, loser, winner, Some(WINNER_HEIGHT));
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            last_processed_height: Some(500),
            synced_height: Some(500),
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    assert!(
        row_exists(&conn, &w, &p),
        "without a chainlock there is no finality boundary — the hold must \
         outlast any amount of synced-height progress"
    );

    // The moment a chainlock does land, the boundary exists and the
    // winner's height sits inside it — the row collects immediately.
    apply_heights(&mut conn, &w, 500);
    assert!(
        !row_exists(&conn, &w, &p),
        "the first persisted chainlock supplies the boundary and the \
         winner-height stamp collects"
    );
}

/// The genuine claim the tombstone exists for: its funding output
/// classifies, the upsert's valve keeps it spent, and materialising
/// (gaining a real `height`) takes it out of the collector's reach forever.
#[test]
fn a_materialised_claim_is_never_collected() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xF3);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x61);
    let funding_txid = Txid::from_byte_array([0x56; 32]);
    let p = OutPoint::new(funding_txid, 0);
    let loser = Txid::from_byte_array([0x57; 32]);
    let winner = Txid::from_byte_array([0x58; 32]);

    let mut conn = persister.lock_conn_for_test();
    derive_address(&conn, &w, 0, &addr);
    apply_heights(&mut conn, &w, 100);
    seed_tombstone(&mut conn, &w, p, loser, winner, Some(WINNER_HEIGHT));
    assert_eq!(
        utxo_row_state(&conn, &w, &p),
        Some((true, None, Some(i64::from(WINNER_HEIGHT)))),
        "sanity: held, unmaterialised, stamped with the winner's height"
    );

    // The funding output classifies: the valve keeps the coin spent, the
    // row gains real funding data, and the stale stamp clears.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            new_utxos: vec![make_utxo(&addr, funding_txid, 0, 50_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    assert_eq!(
        utxo_row_state(&conn, &w, &p),
        Some((true, Some(10), None)),
        "sanity: materialised — real height, stamp cleared, still spent"
    );

    apply_heights(&mut conn, &w, 10_000);
    assert_eq!(
        utxo_row_state(&conn, &w, &p),
        Some((true, Some(10), None)),
        "a materialised claim is the wallet's own coin held spent — no \
         boundary may ever collect it"
    );
}

/// A MATERIALISED claim is releasable, and release is the only thing that
/// frees it.
///
/// The collector deliberately never takes such a row
/// (`a_materialised_claim_is_never_collected`): once the funding output has
/// classified, the row carries real funding data and is the wallet's own coin
/// held spent, so no finality boundary may reclaim it. That leaves exactly one
/// way back — a later sweep naming the outpoint in `released_outpoints`, which
/// the unmaterialised path handles by DELETE and this one by an in-place
/// `spent = 0, spent_in_txid = NULL`.
///
/// Pinned because the two paths diverge on `height IS NULL` and every other
/// release test exercises the placeholder half; without this one, a release
/// that silently skipped materialised rows would leave a live coin spent
/// forever with nothing else able to free it.
#[test]
fn a_release_frees_a_materialised_claim_in_place() {
    let (persister, _tmp, path) = fresh_persister();
    let w: WalletId = wid(0xF7);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x62);
    let funding_txid = Txid::from_byte_array([0x71; 32]);
    let p = OutPoint::new(funding_txid, 0);
    let loser = Txid::from_byte_array([0x72; 32]);
    let winner = Txid::from_byte_array([0x73; 32]);
    let final_winner = Txid::from_byte_array([0x74; 32]);

    let mut conn = persister.lock_conn_for_test();
    derive_address(&conn, &w, 0, &addr);
    apply_heights(&mut conn, &w, 100);
    seed_tombstone(&mut conn, &w, p, loser, winner, Some(WINNER_HEIGHT));

    // The funding output classifies: real data, stamp cleared, still spent.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            new_utxos: vec![make_utxo(&addr, funding_txid, 0, 50_000)],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    assert_eq!(
        utxo_row_state(&conn, &w, &p),
        Some((true, Some(10), None)),
        "sanity: materialised — real height, stamp cleared, still spent"
    );
    assert!(
        !unspent(&conn, &w).contains(&p),
        "sanity: a held coin is not spendable"
    );

    // The winner is itself swept, and this time the coin comes back free.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![winner],
                superseded_by: final_winner,
                winner_mined_height: Some(WINNER_HEIGHT),
                released_outpoints: vec![p],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    assert_eq!(
        utxo_row_state(&conn, &w, &p),
        Some((false, Some(10), None)),
        "a released materialised claim is freed in place, keeping its funding data"
    );
    assert!(
        unspent(&conn, &w).contains(&p),
        "and the coin is spendable again"
    );

    // Durable: the in-place release is not a memory-only flip.
    drop(conn);
    drop(persister);
    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let conn = persister.lock_conn_for_test();
    assert!(
        unspent(&conn, &w).contains(&p),
        "the release must hold across a restart"
    );
}

/// A held, unmaterialised row with a NULL winner height is never
/// collected. The mempool-context sweep path writes exactly this shape
/// (an IS-locked, unmined winner has no finality horizon to stamp), and
/// legacy rows read identically — either way the safe reading is to hold
/// it forever rather than guess it collectible.
#[test]
fn a_tombstone_without_a_winner_height_is_never_collected() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xF4);
    ensure_wallet_meta(&persister, &w);

    let p = OutPoint::new(Txid::from_byte_array([0x59; 32]), 0);
    let loser = Txid::from_byte_array([0x5A; 32]);
    let winner = Txid::from_byte_array([0x5B; 32]);

    let mut conn = persister.lock_conn_for_test();
    // The real writer: an IS-context sweep of a loser whose funding row
    // never arrived.
    seed_tombstone(&mut conn, &w, p, loser, winner, None);

    // Two rounds, not one: a back-filling collector (the rejected design)
    // would stamp the row on the first round and collect it on the second.
    apply_heights(&mut conn, &w, 1_000_000);
    apply_heights(&mut conn, &w, 1_000_010);
    assert_eq!(
        utxo_row_state(&conn, &w, &p),
        Some((true, None, None)),
        "no winner height, no proof of finality — the hold outlasts any boundary"
    );
}

/// A chained sweep that re-points a still-unfunded claim to a new
/// block-context winner also re-stamps it with THAT winner's mined
/// height: the claim now belongs to a spend anchored at a later block,
/// and its collection horizon moves with it.
#[test]
fn a_repointed_tombstone_is_restamped_to_the_later_winners_height() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xF5);
    ensure_wallet_meta(&persister, &w);

    let p = OutPoint::new(Txid::from_byte_array([0x5C; 32]), 0);
    let first_loser = Txid::from_byte_array([0x5D; 32]);
    let second_loser = Txid::from_byte_array([0x5E; 32]);
    let final_winner = Txid::from_byte_array([0x5F; 32]);

    let mut conn = persister.lock_conn_for_test();
    seed_tombstone(
        &mut conn,
        &w,
        p,
        first_loser,
        second_loser,
        Some(WINNER_HEIGHT),
    );
    assert_eq!(
        utxo_row_state(&conn, &w, &p).and_then(|(_, _, s)| s),
        Some(i64::from(WINNER_HEIGHT)),
        "sanity: stamped with the first winner's mined height"
    );

    // The first winner is itself swept — by a winner mined 50 blocks
    // later — still holding the unfunded input.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![tx_record(second_loser, vec![p], vec![])],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![second_loser],
                superseded_by: final_winner,
                winner_mined_height: Some(WINNER_HEIGHT + 50),
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    assert_eq!(
        utxo_row_state(&conn, &w, &p),
        Some((true, None, Some(i64::from(WINNER_HEIGHT + 50)))),
        "the re-pointed claim is re-stamped to the later winner's mined height"
    );
}

/// The IS-locked half of the chained case: an unmined winner re-points
/// the claim but must NOT disturb the earlier block-context stamp —
/// upstream's observed-spend entry is never retracted by an unconfirmed
/// conflict. Collection at the retained height stays sound (the funding
/// output is mined at or below the FIRST spender's height regardless of
/// who claims the coin now), so the row still collects at that boundary.
#[test]
fn a_mempool_repointed_tombstone_keeps_its_block_context_stamp() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xFB);
    ensure_wallet_meta(&persister, &w);

    let p = OutPoint::new(Txid::from_byte_array([0x6D; 32]), 0);
    let first_loser = Txid::from_byte_array([0x6E; 32]);
    let second_loser = Txid::from_byte_array([0x6F; 32]);
    let final_winner = Txid::from_byte_array([0x71; 32]);

    let mut conn = persister.lock_conn_for_test();
    seed_tombstone(
        &mut conn,
        &w,
        p,
        first_loser,
        second_loser,
        Some(WINNER_HEIGHT),
    );

    // The first winner is evicted by an IS-locked, unmined conflict that
    // also claims the unfunded input.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![tx_record(second_loser, vec![p], vec![])],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![second_loser],
                superseded_by: final_winner,
                winner_mined_height: None,
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    assert_eq!(
        utxo_row_state(&conn, &w, &p),
        Some((true, None, Some(i64::from(WINNER_HEIGHT)))),
        "an unmined winner re-points the claim without touching the earlier \
         block-context stamp"
    );

    apply_heights(&mut conn, &w, WINNER_HEIGHT);
    assert!(
        !row_exists(&conn, &w, &p),
        "the retained stamp still bounds the row: the funding output sits at \
         or below the first spender's height, so the boundary reaching it \
         proves delivery-or-never"
    );
}

/// The other direction of the chained case: an UNSTAMPED hold (IS-context
/// sweep) re-pointed by a later BLOCK-context sweep gains that winner's
/// stamp — the claim now belongs to a spend anchored in a real block, so
/// it enters the collectible set and the boundary reaching the new
/// winner's height collects it. This is one of the three resolution
/// channels that bound the unstamped population.
#[test]
fn an_unstamped_tombstone_restamped_by_a_block_context_sweep_becomes_collectible() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xFC);
    ensure_wallet_meta(&persister, &w);

    let p = OutPoint::new(Txid::from_byte_array([0x72; 32]), 0);
    let first_loser = Txid::from_byte_array([0x73; 32]);
    let second_loser = Txid::from_byte_array([0x74; 32]);
    let final_winner = Txid::from_byte_array([0x75; 32]);

    let mut conn = persister.lock_conn_for_test();
    // IS-context sweep: the hold lands unstamped.
    seed_tombstone(&mut conn, &w, p, first_loser, second_loser, None);
    assert_eq!(
        utxo_row_state(&conn, &w, &p),
        Some((true, None, None)),
        "sanity: held and unstamped"
    );

    // The IS-locked first winner is itself beaten by a mined conflict
    // still claiming the unfunded input.
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            records: vec![tx_record(second_loser, vec![p], vec![])],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            sweeps: vec![SweepBatch {
                txids: vec![second_loser],
                superseded_by: final_winner,
                winner_mined_height: Some(WINNER_HEIGHT),
                released_outpoints: vec![],
            }],
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    assert_eq!(
        utxo_row_state(&conn, &w, &p),
        Some((true, None, Some(i64::from(WINNER_HEIGHT)))),
        "the block-context re-point stamps the previously unstamped hold"
    );

    apply_heights(&mut conn, &w, WINNER_HEIGHT);
    assert!(
        !row_exists(&conn, &w, &p),
        "once stamped, the ordinary finality boundary collects the row"
    );
}

/// Legacy shape self-heal: a zero-value released placeholder written
/// before the release path deleted them (`height` NULL, `spent = 0`) holds
/// no claim and is swept up by the collector's first pass — chainlock or
/// not — instead of reading as a phantom spendable coin forever.
#[test]
fn a_legacy_released_placeholder_is_swept_up_by_the_collector() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xF6);
    ensure_wallet_meta(&persister, &w);

    let p = OutPoint::new(Txid::from_byte_array([0x60; 32]), 0);
    let mut conn = persister.lock_conn_for_test();
    // Plant the pre-fix shape directly — the current release path can no
    // longer produce it.
    {
        let bytes = blob::encode_outpoint(&p).unwrap();
        conn.execute(
            "INSERT INTO core_utxos \
                (wallet_id, outpoint, value, script, height, account_index, spent, spent_in_txid) \
             VALUES (?1, ?2, 0, X'', NULL, 0, 0, NULL)",
            params![w.as_slice(), &bytes[..]],
        )
        .unwrap();
    }
    assert!(
        unspent(&conn, &w).contains(&p),
        "sanity: the legacy phantom"
    );

    {
        let tx = conn.transaction().unwrap();
        let cs = CoreChangeSet {
            last_processed_height: Some(100),
            synced_height: Some(100),
            ..Default::default()
        };
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }
    assert!(
        !row_exists(&conn, &w, &p),
        "the first height-carrying round deletes the claimless leftover"
    );
}
