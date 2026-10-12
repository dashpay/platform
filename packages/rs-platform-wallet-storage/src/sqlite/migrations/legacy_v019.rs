//! Frozen V019 history repair. It backfills the raw-input index and corrects
//! stored accounting for databases migrating from V018 or earlier.
//!
//! Frozen on purpose: the live per-round repair in `schema::core_history` may
//! evolve, but V019 must keep doing exactly what it did when it shipped. It
//! shares only these live helpers, which must stay behaviour-stable: the blob
//! codec and its size/width gates (`blob`), `id32`, `wallets::parse_network`,
//! `i64_to_u64`, `i64_to_u32` and the `WalletStorageError` variants they return.
//! `TransactionRecord`'s encoding is owned upstream (key-wallet) and cannot be
//! frozen here. Do not edit.

use std::collections::BTreeMap;

use dashcore::hashes::Hash;
use dashcore::{Address, OutPoint, ScriptBuf, Txid};
use key_wallet::managed_account::transaction_record::{
    InputDetail, OutputDetail, OutputRole, TransactionDirection, TransactionRecord,
};
use key_wallet::transaction_checking::{TransactionContext, TransactionType};
use platform_wallet::wallet::platform_wallet::WalletId;
use rusqlite::{params, OptionalExtension, Transaction};

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::{blob, id32, wallets};
use crate::sqlite::util::safe_cast::{i64_to_u32, i64_to_u64};

/// Read a stored record; undecodable bytes are an error the caller classifies.
fn read_record(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    txid: &Txid,
) -> Result<Option<TransactionRecord>, WalletStorageError> {
    let key = params![wallet_id.as_slice(), txid.as_byte_array().as_slice()];
    // Gate the stored length in its own statement: SQLite evaluates every
    // result column before a row is returned, so selecting the payload
    // alongside its length would load an oversize blob before the check.
    let len: Option<Option<i64>> = tx
        .prepare_cached(
            "SELECT length(record_blob) FROM core_transactions \
             WHERE wallet_id = ?1 AND txid = ?2",
        )?
        .query_row(key, |row| row.get(0))
        .optional()?;
    let Some(Some(len)) = len else {
        return Ok(None);
    };
    blob::check_size(len)?;
    let payload: Vec<u8> = tx
        .prepare_cached(
            "SELECT record_blob FROM core_transactions \
             WHERE wallet_id = ?1 AND txid = ?2",
        )?
        .query_row(key, |row| row.get(0))?;
    let record: TransactionRecord = blob::decode(&payload)?;
    if record.txid != *txid {
        return Err(WalletStorageError::blob_decode(
            "transaction record names another transaction",
        ));
    }
    Ok(Some(record))
}

/// Whether `error` reports stored bytes that cannot be decoded, not a database failure.
fn is_unreadable(error: &WalletStorageError) -> bool {
    matches!(
        error,
        WalletStorageError::BincodeDecode { .. }
            | WalletStorageError::BlobDecode { .. }
            | WalletStorageError::BlobTooLarge { .. }
            | WalletStorageError::HashDecode { .. }
            | WalletStorageError::IntegerOverflow { .. }
    )
}

/// Drop an undecodable record that a Core resync re-delivers, and force that resync.
///
/// Only a block-confirmed record (typed `height` set) is re-delivered by a
/// filter rescan; an unconfirmed one may never be seen again, so it keeps the
/// migration failing rather than losing it. Lowering `synced_height` to just
/// below the wallet's birth height makes the next SPV start rescan the wallet
/// from its birth, which re-records the transaction and re-applies its spends.
/// The spent marks and outputs it already produced stay as they are:
/// conservative until the rescan confirms them.
fn drop_for_resync(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    txid: &Txid,
    error: WalletStorageError,
) -> Result<(), WalletStorageError> {
    let height: Option<i64> = tx.query_row(
        "SELECT height FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
        params![wallet_id.as_slice(), txid.as_byte_array().as_slice()],
        |row| row.get(0),
    )?;
    if height.is_none() {
        return Err(error);
    }
    let birth_height: i64 = tx.query_row(
        "SELECT birth_height FROM wallets WHERE wallet_id = ?1",
        params![wallet_id.as_slice()],
        |row| row.get(0),
    )?;
    // A corrupt birth height fails the migration instead of choosing a rescan height.
    let rescan_from = i64_to_u32("wallets.birth_height", birth_height)?.saturating_sub(1);
    tx.execute(
        "DELETE FROM core_transaction_inputs WHERE wallet_id = ?1 AND txid = ?2",
        params![wallet_id.as_slice(), txid.as_byte_array().as_slice()],
    )?;
    tx.execute(
        "DELETE FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
        params![wallet_id.as_slice(), txid.as_byte_array().as_slice()],
    )?;
    tx.execute(
        "UPDATE core_sync_state SET synced_height = MIN(COALESCE(synced_height, ?2), ?2) \
         WHERE wallet_id = ?1",
        params![wallet_id.as_slice(), rescan_from],
    )?;
    tracing::warn!(
        wallet_id = %hex::encode(wallet_id),
        %txid,
        %error,
        rescan_from,
        "dropped an undecodable confirmed transaction record; Core history rescans from birth"
    );
    Ok(())
}

/// Index raw inputs independently of when their ownership becomes known.
fn index_record(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    record: &TransactionRecord,
) -> Result<(), WalletStorageError> {
    let mut stmt = tx.prepare_cached(
        "INSERT OR IGNORE INTO core_transaction_inputs (wallet_id, txid, outpoint) VALUES (?1, ?2, ?3)",
    )?;
    for input in &record.transaction.input {
        stmt.execute(params![
            wallet_id.as_slice(),
            record.txid.as_byte_array().as_slice(),
            blob::encode_outpoint(&input.previous_output)?,
        ])?;
    }
    Ok(())
}

fn network(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
) -> Result<dashcore::Network, WalletStorageError> {
    let label: String = tx.query_row(
        "SELECT network FROM wallets WHERE wallet_id = ?1",
        params![wallet_id.as_slice()],
        |r| r.get(0),
    )?;
    wallets::parse_network(&label).ok_or_else(|| WalletStorageError::UnknownWalletNetwork {
        wallet_id: *wallet_id,
        label,
    })
}

fn owned_output(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    outpoint: &OutPoint,
    network: dashcore::Network,
) -> Result<Option<(u64, Address)>, WalletStorageError> {
    let encoded = blob::encode_outpoint(outpoint)?;
    let key = params![wallet_id.as_slice(), encoded];
    // Gate the script length in its own statement: SQLite evaluates every
    // result column before a row is returned.
    let Some((value, len)) = tx
        .prepare_cached(
            "SELECT value, length(script) FROM core_utxos \
             WHERE wallet_id = ?1 AND outpoint = ?2 AND is_sweep_placeholder = 0",
        )?
        .query_row(key, |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .optional()?
    else {
        return Ok(None);
    };
    let value = i64_to_u64("core_utxos.value", value)?;
    blob::check_size(len)?;
    let script: Vec<u8> = tx
        .prepare_cached(
            "SELECT script FROM core_utxos \
             WHERE wallet_id = ?1 AND outpoint = ?2 AND is_sweep_placeholder = 0",
        )?
        .query_row(key, |row| row.get(0))?;
    if contact_only_script(tx, wallet_id, &script)? {
        return Ok(None);
    }
    let address = Address::from_script(&ScriptBuf::from_bytes(script), network)?;
    Ok(Some((value, address)))
}

/// Whether `script` is tracked only by a contact's watch-only (DashPay external) chain.
fn contact_only_script(
    conn: &Transaction<'_>,
    wallet_id: &WalletId,
    script: &[u8],
) -> Result<bool, WalletStorageError> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM core_address_pool WHERE wallet_id = ?1 AND script = ?2) \
         AND NOT EXISTS(SELECT 1 FROM core_address_pool \
             WHERE wallet_id = ?1 AND script = ?2 AND account_type != 'dashpay_external')",
        params![wallet_id.as_slice(), script],
        |r| r.get(0),
    )?)
}

fn repair_record(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    mut record: TransactionRecord,
    network: dashcore::Network,
) -> Result<(), WalletStorageError> {
    let record_txid = record.txid;
    let txid = &record_txid;
    let original = blob::encode(&record)?;
    let mut inputs = BTreeMap::new();
    for detail in record.input_details.drain(..) {
        if !contact_only_script(tx, wallet_id, detail.address.script_pubkey().as_bytes())? {
            inputs.insert(detail.index, detail);
        }
    }
    for (index, input) in record.transaction.input.iter().enumerate() {
        if let Some((value, address)) =
            owned_output(tx, wallet_id, &input.previous_output, network)?
        {
            inputs.insert(
                index as u32,
                InputDetail {
                    index: index as u32,
                    value,
                    address,
                },
            );
            // Stale mempool rows cannot overrule a later sweep's release.
            if !matches!(record.context, TransactionContext::Mempool) {
                // Record the spender so the mark stays attributable and
                // reversible; an existing claim by another spender stands.
                tx.execute(
                    "UPDATE core_utxos SET spent = 1, \
                         spent_in_txid = CASE WHEN spent = 1 AND spent_in_txid IS NOT NULL \
                             THEN spent_in_txid ELSE ?3 END \
                     WHERE wallet_id = ?1 AND outpoint = ?2",
                    params![
                        wallet_id.as_slice(),
                        blob::encode_outpoint(&input.previous_output)?,
                        txid.as_byte_array().as_slice()
                    ],
                )?;
            }
        }
    }
    let mut outputs = BTreeMap::new();
    for mut detail in record.output_details.drain(..) {
        if let Some(address) = &detail.address {
            if contact_only_script(tx, wallet_id, address.script_pubkey().as_bytes())? {
                detail.role = OutputRole::Sent;
            }
        }
        outputs.insert(detail.index, detail);
    }
    for (index, output) in record.transaction.output.iter().enumerate() {
        let index = index as u32;
        if let Some((_, address)) = owned_output(
            tx,
            wallet_id,
            &OutPoint {
                txid: *txid,
                vout: index,
            },
            network,
        )? {
            let role = outputs.get(&index).map_or(OutputRole::Received, |d| {
                if d.role == OutputRole::Change {
                    OutputRole::Change
                } else {
                    OutputRole::Received
                }
            });
            outputs.insert(
                index,
                OutputDetail {
                    index,
                    role,
                    address: Some(address),
                    value: output.value,
                },
            );
        }
    }
    // Empty metadata is not accounting evidence (e.g. confirmation-only placeholders).
    if inputs.is_empty() && outputs.is_empty() {
        return Ok(());
    }
    let received: i128 = outputs
        .values()
        .filter(|d| matches!(d.role, OutputRole::Received | OutputRole::Change))
        .map(|d| i128::from(d.value))
        .sum();
    let spent: i128 = inputs.values().map(|d| i128::from(d.value)).sum();
    let net = received - spent;
    record.net_amount = i64::try_from(net).map_err(|_| WalletStorageError::NetAmountOverflow {
        wallet_id: *wallet_id,
        txid: *txid,
        value: net,
    })?;
    let has_ours = outputs
        .values()
        .any(|d| matches!(d.role, OutputRole::Received | OutputRole::Change));
    let has_external = record
        .transaction
        .output
        .iter()
        .enumerate()
        .any(|(i, output)| {
            !output.script_pubkey.is_op_return()
                && !outputs.get(&(i as u32)).is_some_and(|d| {
                    matches!(
                        d.role,
                        OutputRole::Received | OutputRole::Change | OutputRole::Unspendable
                    )
                })
        });
    record.direction = if record.transaction_type == TransactionType::CoinJoin {
        TransactionDirection::CoinJoin
    } else if inputs.is_empty() {
        TransactionDirection::Incoming
    } else if !has_external && (has_ours || record.transaction_type == TransactionType::AssetLock) {
        TransactionDirection::Internal
    } else {
        TransactionDirection::Outgoing
    };
    record.input_details = inputs.into_values().collect();
    record.output_details = outputs.into_values().collect();
    let repaired = blob::encode(&record)?;
    if repaired != original {
        // Append-only: the first pre-repair blob is kept verbatim and never
        // replaced, so a wrong repair can always be undone.
        tx.execute(
            "INSERT OR IGNORE INTO core_transaction_record_originals (wallet_id, txid, record_blob) \
             SELECT wallet_id, txid, record_blob FROM core_transactions \
             WHERE wallet_id = ?1 AND txid = ?2",
            params![wallet_id.as_slice(), txid.as_byte_array().as_slice()],
        )?;
        tx.execute(
            "UPDATE core_transactions SET record_blob = ?1 WHERE wallet_id = ?2 AND txid = ?3",
            params![
                repaired,
                wallet_id.as_slice(),
                txid.as_byte_array().as_slice()
            ],
        )?;
    }
    Ok(())
}

pub(super) fn repair_history(tx: &Transaction<'_>) -> Result<(), WalletStorageError> {
    // Keys are collected first so drops never race the open cursor.
    let mut keys = Vec::new();
    {
        let mut stmt = tx.prepare_cached(
            "SELECT length(wallet_id), wallet_id, length(txid), txid \
             FROM core_transactions WHERE record_blob IS NOT NULL",
        )?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            blob::check_fixed_width(row.get(0)?, 32, "core_transactions.wallet_id")?;
            let wallet_id: Vec<u8> = row.get(1)?;
            let wallet_id = id32("core_transactions.wallet_id", &wallet_id)?;
            blob::check_fixed_width(row.get(2)?, 32, "core_transactions.txid")?;
            let txid: Vec<u8> = row.get(3)?;
            keys.push((wallet_id, Txid::from_slice(&txid)?));
        }
    }
    for (wallet_id, txid) in keys {
        match read_record(tx, &wallet_id, &txid) {
            Ok(Some(record)) => {
                index_record(tx, &wallet_id, &record)?;
                repair_record(tx, &wallet_id, record, network(tx, &wallet_id)?)?;
            }
            Ok(None) => {}
            Err(error) if is_unreadable(&error) => drop_for_resync(tx, &wallet_id, &txid, error)?,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use dashcore::address::Payload;
    use dashcore::{BlockHash, PubkeyHash, Transaction as CoreTransaction, TxIn, TxOut};
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::transaction_checking::BlockInfo;
    use key_wallet::Utxo;
    use platform_wallet::changeset::CoreChangeSet;
    use rusqlite::Connection;

    use super::*;
    use crate::sqlite::migrations::{self, rewind_to_v018};
    use crate::sqlite::schema::core_state;

    #[test]
    fn should_reject_oversize_owned_output_script_before_reading_it() {
        const WALLET: WalletId = [0xC2u8; 32];
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::run(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&WALLET[..]],
        )
        .unwrap();

        use rusqlite::limits::Limit;

        use crate::sqlite::conn::SQLITE_MAX_BLOB_BYTES;

        let outpoint = OutPoint::new(Txid::from_byte_array([0x42; 32]), 0);
        // Over the connection's length cap, so reading the column itself
        // fails: only a length-only pre-read reports it as oversize.
        let script = vec![0u8; SQLITE_MAX_BLOB_BYTES as usize + 1];
        conn.execute(
            "INSERT INTO core_utxos (wallet_id, outpoint, value, script, spent) \
             VALUES (?1, ?2, 0, ?3, 0)",
            params![
                &WALLET[..],
                blob::encode_outpoint(&outpoint).unwrap(),
                script
            ],
        )
        .unwrap();
        drop(script);
        conn.set_limit(Limit::SQLITE_LIMIT_LENGTH, SQLITE_MAX_BLOB_BYTES)
            .unwrap();
        let tx = conn.transaction().unwrap();

        let err = owned_output(&tx, &WALLET, &outpoint, dashcore::Network::Testnet).unwrap_err();

        assert!(
            matches!(err, WalletStorageError::BlobTooLarge { .. }),
            "got {err:?}"
        );
    }

    fn address(marker: u8) -> Address {
        Address::new(
            dashcore::Network::Testnet,
            Payload::PubkeyHash(PubkeyHash::from_byte_array([marker; 20])),
        )
    }

    fn utxo(outpoint: OutPoint, value: u64, address: Address) -> Utxo {
        Utxo {
            outpoint,
            txout: TxOut {
                value,
                script_pubkey: address.script_pubkey(),
            },
            address,
            height: 100,
            is_coinbase: false,
            is_confirmed: true,
            is_instantlocked: false,
            is_locked: false,
            is_trusted: false,
        }
    }

    /// A V018 database holding one oversize confirmed record for a wallet born
    /// at `birth_height`, synced to 900.
    fn seed_oversize_confirmed_record(birth_height: i64) -> (Connection, WalletId) {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::run(&mut conn).unwrap();
        let wallet_id = [0xC2u8; 32];
        let txid = Txid::from_byte_array([0x72; 32]);
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', ?2)",
            params![&wallet_id[..], birth_height],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO core_sync_state (wallet_id, last_processed_height, synced_height) \
             VALUES (?1, 900, 900)",
            params![&wallet_id[..]],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO core_transactions (wallet_id, txid, height, finalized, record_blob) \
             VALUES (?1, ?2, 300, 1, zeroblob(?3))",
            params![
                &wallet_id[..],
                txid.as_byte_array().as_slice(),
                i64::try_from(blob::BLOB_SIZE_LIMIT_BYTES + 1).unwrap()
            ],
        )
        .unwrap();
        rewind_to_v018(&conn);
        (conn, wallet_id)
    }

    /// `(record count, synced_height)` for `wallet_id`.
    fn records_and_synced_height(conn: &Connection, wallet_id: &WalletId) -> (i64, i64) {
        conn.query_row(
            "SELECT (SELECT count(*) FROM core_transactions WHERE wallet_id = ?1), \
                    (SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1)",
            params![&wallet_id[..]],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
    }

    /// An oversize confirmed record fails its length gate before its payload
    /// is read, and is dropped with a rescan from just below the birth height.
    #[test]
    fn should_drop_oversize_confirmed_record_for_resync() {
        let (mut conn, wallet_id) = seed_oversize_confirmed_record(50);

        migrations::run(&mut conn).unwrap();

        let (records, synced) = records_and_synced_height(&conn, &wallet_id);
        assert_eq!(records, 0, "the oversize record must be dropped");
        assert_eq!(
            synced, 49,
            "the rescan must restart just below the birth height"
        );
    }

    #[test]
    fn should_rescan_from_genesis_when_the_wallet_is_born_at_zero() {
        let (mut conn, wallet_id) = seed_oversize_confirmed_record(0);

        migrations::run(&mut conn).unwrap();

        assert_eq!(records_and_synced_height(&conn, &wallet_id), (0, 0));
    }

    /// A stored birth height outside `u32` fails the migration with a typed
    /// error, instead of overflowing, and leaves the record and sync state alone.
    #[test]
    fn should_reject_out_of_range_birth_height_before_scheduling_a_rescan() {
        for birth_height in [i64::MIN, -1, i64::from(u32::MAX) + 1, i64::MAX] {
            let (mut conn, wallet_id) = seed_oversize_confirmed_record(birth_height);

            let error = migrations::run(&mut conn).unwrap_err();

            assert!(
                format!("{error:?}").contains("wallets.birth_height"),
                "birth height {birth_height}: {error:?}"
            );
            assert_eq!(
                records_and_synced_height(&conn, &wallet_id),
                (1, 900),
                "birth height {birth_height}: the failed upgrade must roll back"
            );
        }
    }

    /// Pins V019's observable result on a V018-shaped database: the repaired
    /// record, the preserved original, the input index and the spent marks.
    #[test]
    fn should_pin_v019_repair_of_a_v018_database() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::run(&mut conn).unwrap();
        let wallet_id = [0xC1u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();
        let (own, change, contact, external) = (address(1), address(2), address(3), address(4));
        conn.execute(
            "INSERT INTO core_address_pool (wallet_id, account_type, account_index, pool_type, address_index, script) \
             VALUES (?1, 'dashpay_external', 0, 0, 0, ?2)",
            params![&wallet_id[..], contact.script_pubkey().as_bytes()],
        )
        .unwrap();
        let funding = OutPoint::new(Txid::from_byte_array([0x71; 32]), 0);
        let body = CoreTransaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: funding,
                ..Default::default()
            }],
            output: vec![
                TxOut {
                    value: 30_000,
                    script_pubkey: change.script_pubkey(),
                },
                TxOut {
                    value: 50_000,
                    script_pubkey: contact.script_pubkey(),
                },
                TxOut {
                    value: 15_000,
                    script_pubkey: external.script_pubkey(),
                },
            ],
            special_transaction_payload: None,
        };
        let txid = body.txid();
        // What an old build stored: the input was not known to be ours and
        // the contact's output was credited as received.
        let original = TransactionRecord::new(
            body,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::InBlock(BlockInfo::new(101, BlockHash::all_zeros(), 7)),
            TransactionType::Standard,
            TransactionDirection::Incoming,
            Vec::new(),
            vec![
                OutputDetail {
                    index: 0,
                    role: OutputRole::Change,
                    address: Some(change.clone()),
                    value: 30_000,
                },
                OutputDetail {
                    index: 1,
                    role: OutputRole::Received,
                    address: Some(contact),
                    value: 50_000,
                },
            ],
            80_000,
        );
        {
            let tx = conn.transaction().unwrap();
            core_state::apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    new_utxos: vec![
                        utxo(funding, 100_000, own.clone()),
                        utxo(OutPoint::new(txid, 0), 30_000, change.clone()),
                    ],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.execute(
                "INSERT OR REPLACE INTO core_transactions (wallet_id, txid, height, finalized, record_blob) \
                 VALUES (?1, ?2, 101, 1, ?3)",
                params![
                    &wallet_id[..],
                    txid.as_byte_array().as_slice(),
                    blob::encode(&original).unwrap()
                ],
            )
            .unwrap();
            rewind_to_v018(&tx);
            tx.commit().unwrap();
        }

        migrations::run(&mut conn).unwrap();

        let mut expected = original.clone();
        expected.input_details = vec![InputDetail {
            index: 0,
            value: 100_000,
            address: own,
        }];
        expected.output_details = vec![
            OutputDetail {
                index: 0,
                role: OutputRole::Change,
                address: Some(change),
                value: 30_000,
            },
            OutputDetail {
                index: 1,
                role: OutputRole::Sent,
                address: Some(address(3)),
                value: 50_000,
            },
        ];
        expected.net_amount = -70_000;
        expected.direction = TransactionDirection::Outgoing;
        let read_blob = |sql: &str| -> Vec<u8> {
            conn.query_row(
                sql,
                params![&wallet_id[..], txid.as_byte_array().as_slice()],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert_eq!(
            read_blob(
                "SELECT record_blob FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2"
            ),
            blob::encode(&expected).unwrap()
        );
        assert_eq!(
            read_blob(
                "SELECT record_blob FROM core_transaction_record_originals WHERE wallet_id = ?1 AND txid = ?2"
            ),
            blob::encode(&original).unwrap()
        );
        let (spent, spender): (bool, Option<Vec<u8>>) = conn
            .query_row(
                "SELECT spent, spent_in_txid FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
                params![&wallet_id[..], blob::encode_outpoint(&funding).unwrap()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(spent);
        assert_eq!(spender.as_deref(), Some(txid.as_byte_array().as_slice()));
        let indexed: i64 = conn
            .query_row(
                "SELECT count(*) FROM core_transaction_inputs WHERE wallet_id = ?1 AND txid = ?2 AND outpoint = ?3",
                params![
                    &wallet_id[..],
                    txid.as_byte_array().as_slice(),
                    blob::encode_outpoint(&funding).unwrap()
                ],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(indexed, 1);
    }
}
