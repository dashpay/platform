//! Wallet rows that embed a Dash Core transaction keep decoding across
//! rust-dashcore upgrades.
//!
//! `tests/fixtures/core_transaction_rows.txt` holds one row per line: a name and a
//! blob in hex. `asset_lock_lifecycle` is an `asset_locks.lifecycle_blob`;
//! every other row is a `core_transactions.record_blob` (a key-wallet
//! `TransactionRecord`). Both embed a `dashcore::Transaction` through
//! `bincode::serde`, so a change to its serde shape fails a wallet's whole
//! load. `TransactionPayload` derives index-based serde: a variant inserted
//! before an existing one shifts its stored tag, which the
//! `classical_type_bytes_42` (index 9) and `unknown_type_10` (index 10) rows
//! catch.
//!
//! The rows were encoded by the program below at rust-dashcore 40268cc0, the
//! revision before Dash Core v24 decoding, except `unknown_type_10`,
//! `cbtx_v4` and the `_v3` rows, which that revision cannot decode; those
//! were encoded at 6c5775cf. Each row's transaction, real or built for the
//! row, is in its blob; the rest was fixed:
//!
//! ```text
//! let record = TransactionRecord::new(
//!     tx.clone(),
//!     AccountType::Standard { index: 0, standard_account_type: StandardAccountType::BIP44Account },
//!     TransactionContext::InChainLockedBlock(BlockInfo::new(
//!         200, BlockHash::from_byte_array([0x03; 32]), 1_735_689_600,
//!     )),
//!     TransactionRouter::classify_transaction(&tx),
//!     TransactionDirection::Incoming,
//!     Vec::new(),
//!     Vec::new(),
//!     150_000,
//! );
//! println!("{name} {}", hex::encode(blob::encode(&record)?));
//!
//! let entry = AssetLockEntry {
//!     out_point: OutPoint::new(tx.txid(), 0),
//!     transaction: tx,
//!     account_index: 0,
//!     funding_type: AssetLockFundingType::IdentityRegistration,
//!     identity_index: 0,
//!     amount_duffs: 66_952,
//!     status: AssetLockStatus::Built,
//!     proof: None,
//! };
//! println!("asset_lock_lifecycle {}", hex::encode(asset_locks::encode_entry_for_test(&entry)?));
//! ```

use crate::common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use dashcore::transaction::special_transaction::TransactionPayload;
use dashcore::Transaction;
use key_wallet::managed_account::transaction_record::TransactionRecord;
use platform_wallet::changeset::AssetLockEntry;
use platform_wallet_storage::sqlite::schema::{asset_locks, blob};
use platform_wallet_storage::LoadCtx;
use rusqlite::params;

const ROWS: &str = include_str!("../fixtures/core_transaction_rows.txt");

/// Each row and the transaction it must decode to.
const EXPECTED: [(&str, &str); 19] = [
    ("classical_type_bytes_42", "classical type bytes 42"),
    ("cbtx_v1", "CbTx v1"),
    ("cbtx_v2", "CbTx v2"),
    ("cbtx_v3", "CbTx v3"),
    ("pro_reg_tx_v1", "ProRegTx v1"),
    ("pro_reg_tx_v2_evo_ipv6", "ProRegTx v2"),
    ("pro_reg_tx_v1_no_address", "ProRegTx v1"),
    ("pro_up_serv_tx_v1_ipv4_mapped", "ProUpServTx v1"),
    ("pro_up_serv_tx_v2_evo_ipv6", "ProUpServTx v2"),
    ("pro_up_serv_tx_v1_ipv4_compatible", "ProUpServTx v1"),
    ("pro_up_reg_tx_v1", "ProUpRegTx v1"),
    ("pro_up_reg_tx_v2", "ProUpRegTx v2"),
    ("asset_lock", "asset lock"),
    ("asset_lock_lifecycle", "asset lock"),
    ("unknown_type_10", "unknown type 10"),
    ("cbtx_v4", "CbTx v4"),
    ("pro_reg_tx_v3", "ProRegTx v3"),
    ("pro_up_serv_tx_v3_evo", "ProUpServTx v3"),
    ("pro_up_reg_tx_v3", "ProUpRegTx v3"),
];

fn rows() -> Vec<(&'static str, Vec<u8>)> {
    ROWS.lines()
        .map(|line| {
            let (name, blob) = line.split_once(' ').expect("`name blob` row");
            (name, hex::decode(blob).expect("hex blob"))
        })
        .collect()
}

fn describe(tx: &Transaction) -> String {
    match &tx.special_transaction_payload {
        None => "classic".to_string(),
        Some(TransactionPayload::ClassicalWithNonStandardVersionTypeBytesPayloadType(raw)) => {
            format!("classical type bytes {raw}")
        }
        Some(TransactionPayload::RawPayloadType(tx_type, _)) => {
            format!("unknown type {tx_type}")
        }
        Some(TransactionPayload::CoinbasePayloadType(p)) => format!("CbTx v{}", p.version),
        Some(TransactionPayload::ProviderRegistrationPayloadType(p)) => {
            format!("ProRegTx v{}", p.version)
        }
        Some(TransactionPayload::ProviderUpdateServicePayloadType(p)) => {
            format!("ProUpServTx v{}", p.version)
        }
        Some(TransactionPayload::ProviderUpdateRegistrarPayloadType(p)) => {
            format!("ProUpRegTx v{}", p.version)
        }
        Some(TransactionPayload::AssetLockPayloadType(_)) => "asset lock".to_string(),
        Some(other) => format!("{:?}", other.get_type()),
    }
}

/// Decode a lifecycle blob the way a wallet load does: from its row.
fn load_lifecycle(blob: &[u8], out_point: dashcore::OutPoint) -> AssetLockEntry {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0x5A);
    ensure_wallet_meta(&persister, &w);
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO asset_locks (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) \
         VALUES (?1, ?2, 'built', 0, 0, 66952, ?3)",
        params![
            w.as_slice(),
            blob::encode_outpoint(&out_point).unwrap(),
            blob
        ],
    )
    .unwrap();
    let mut locks = asset_locks::load_state(&conn, &w, &LoadCtx::strict()).expect("load");
    let lock = locks
        .remove(&0)
        .and_then(|mut by_outpoint| by_outpoint.remove(&out_point))
        .expect("the stored lock");
    AssetLockEntry {
        out_point: lock.out_point,
        transaction: lock.transaction,
        account_index: lock.account_index,
        funding_type: lock.funding_type,
        identity_index: lock.identity_index,
        amount_duffs: lock.amount,
        status: lock.status,
        proof: lock.proof,
    }
}

#[test]
fn should_decode_and_re_encode_every_stored_transaction_row_unchanged() {
    let rows = rows();
    let names: Vec<&str> = rows.iter().map(|(name, _)| *name).collect();
    let expected_names: Vec<&str> = EXPECTED.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, expected_names);

    for ((name, bytes), (_, expected)) in rows.into_iter().zip(EXPECTED) {
        let transaction = if name == "asset_lock_lifecycle" {
            // The outpoint the row was stored under: output 0 of its own
            // transaction, read from the blob's leading outpoint.
            let (out_point, _): (dashcore::OutPoint, usize) =
                bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
                    .expect("leading outpoint");
            let entry = load_lifecycle(&bytes, out_point);
            assert_eq!(
                asset_locks::encode_entry_for_test(&entry).unwrap(),
                bytes,
                "{name}: re-encodes differently"
            );
            assert_eq!(entry.out_point.txid, entry.transaction.txid(), "{name}");
            entry.transaction
        } else {
            let record: TransactionRecord =
                blob::decode(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(
                blob::encode(&record).unwrap(),
                bytes,
                "{name}: re-encodes differently"
            );
            assert_eq!(
                record.transaction.txid(),
                record.txid,
                "{name}: the txid differs from the one stored with it"
            );
            record.transaction
        };
        assert_eq!(describe(&transaction), expected, "{name}");
    }
}
