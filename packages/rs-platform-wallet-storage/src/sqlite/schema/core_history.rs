//! Wallet accounting repaired from historical owned outputs, independent of live UTXOs.

use std::collections::{BTreeMap, HashSet};

use dashcore::hashes::Hash;
use dashcore::{Address, OutPoint, ScriptBuf, Txid};
use key_wallet::managed_account::transaction_record::{
    InputDetail, OutputDetail, OutputRole, TransactionRecord,
};
use key_wallet::transaction_checking::TransactionContext;
use platform_wallet::changeset::{is_owned, wallet_accounting, CoreChangeSet};
use platform_wallet::wallet::platform_wallet::WalletId;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use super::accounts::DASHPAY_EXTERNAL_LABEL;
use super::{blob, core_state, wallets};
use crate::sqlite::error::WalletStorageError;
use crate::sqlite::load_ctx::LoadCtx;
use crate::sqlite::util::safe_cast::i64_to_u64;

/// Preserve proven ownership when a partial account snapshot replaces the wallet record.
pub(super) fn preserve_known_details(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    incoming: &TransactionRecord,
) -> Result<TransactionRecord, WalletStorageError> {
    let mut merged = incoming.clone();
    let Some(previous) = prior_record(tx, wallet_id, &incoming.txid)? else {
        return Ok(merged);
    };
    // Compare only txid-committed content: a peer may attach BIP144 witnesses
    // to a known txid, and that must neither conflict nor replace the stored body.
    if previous.transaction.txid() != incoming.transaction.txid() {
        return Err(WalletStorageError::TransactionBodyConflict {
            wallet_id: *wallet_id,
            txid: incoming.txid,
        });
    }
    merged.transaction = previous.transaction;
    let mut inputs: BTreeMap<_, _> = previous
        .input_details
        .into_iter()
        .map(|d| (d.index, d))
        .collect();
    for detail in &incoming.input_details {
        inputs.insert(detail.index, detail.clone());
    }
    let mut outputs: BTreeMap<_, _> = previous
        .output_details
        .into_iter()
        .map(|d| (d.index, d))
        .collect();
    for detail in &incoming.output_details {
        let keep_previous = outputs
            .get(&detail.index)
            .is_some_and(|old| is_owned(old.role))
            && !is_owned(detail.role);
        if !keep_previous {
            outputs.insert(detail.index, detail.clone());
        }
    }
    merged.input_details = inputs.into_values().collect();
    merged.output_details = outputs.into_values().collect();
    Ok(merged)
}

/// Read a stored record strictly: corrupt history is an error, never skipped.
fn prior_record(
    conn: &Connection,
    wallet_id: &WalletId,
    txid: &Txid,
) -> Result<Option<TransactionRecord>, WalletStorageError> {
    core_state::get_tx_record(conn, wallet_id, txid, &LoadCtx::strict())
}

/// Index raw inputs independently of when their ownership becomes known.
pub(super) fn index_record(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    record: &TransactionRecord,
) -> Result<(), WalletStorageError> {
    let mut stmt = tx.prepare_cached(
        "INSERT OR IGNORE INTO core_transaction_inputs (wallet_id, txid, outpoint) \
         VALUES (?1, ?2, ?3)",
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

/// Repair changed records and consumers of newly materialized historical outputs.
pub(super) fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &CoreChangeSet,
) -> Result<(), WalletStorageError> {
    let mut affected: HashSet<Txid> = cs.records.iter().map(|r| r.txid).collect();
    let mut consumers = tx.prepare_cached(
        "SELECT txid FROM core_transaction_inputs WHERE wallet_id = ?1 AND outpoint = ?2",
    )?;
    for utxo in cs.new_utxos.iter().chain(&cs.spent_utxos) {
        affected.insert(utxo.outpoint.txid);
        let mut rows = consumers.query(params![
            wallet_id.as_slice(),
            blob::encode_outpoint(&utxo.outpoint)?
        ])?;
        while let Some(row) = rows.next()? {
            let bytes: Vec<u8> = row.get(0)?;
            affected.insert(Txid::from_slice(&bytes)?);
        }
    }
    if affected.is_empty() {
        return Ok(());
    }
    let network = network(tx, wallet_id)?;
    for txid in affected {
        repair_record(tx, wallet_id, &txid, network)?;
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
pub(crate) fn contact_only_script(
    conn: &Connection,
    wallet_id: &WalletId,
    script: &[u8],
) -> Result<bool, WalletStorageError> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM core_address_pool WHERE wallet_id = ?1 AND script = ?2) \
         AND NOT EXISTS(SELECT 1 FROM core_address_pool \
             WHERE wallet_id = ?1 AND script = ?2 AND account_type != ?3)",
        params![wallet_id.as_slice(), script, DASHPAY_EXTERNAL_LABEL],
        |r| r.get(0),
    )?)
}

fn repair_record(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    txid: &Txid,
    network: dashcore::Network,
) -> Result<(), WalletStorageError> {
    let Some(mut record) = prior_record(tx, wallet_id, txid)? else {
        return Ok(());
    };
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
                // TODO(release-repair-spends-after-reorg): release rows whose
                // `spent_in_txid` spender is reorged out and never re-mined;
                // needs verification of how upstream downgrades a stored
                // record's context on reorg.
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
    record.input_details = inputs.into_values().collect();
    record.output_details = outputs.into_values().collect();
    // The live projection computes the same accounting, so repair never
    // flips a row it just wrote; only the overflow policy differs.
    let (net, direction) = wallet_accounting(&record);
    record.net_amount = i64::try_from(net).map_err(|_| WalletStorageError::NetAmountOverflow {
        wallet_id: *wallet_id,
        txid: *txid,
        value: net,
    })?;
    record.direction = direction;
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

#[cfg(test)]
mod tests {
    use dashcore::address::Payload;
    use dashcore::{PubkeyHash, Transaction as CoreTransaction, TxOut};
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::transaction_record::TransactionDirection;
    use key_wallet::transaction_checking::TransactionType;
    use platform_wallet::changeset::wallet_direction;
    use platform_wallet::test_support::fold_wallet_records;

    use super::*;

    const WALLET_ID: WalletId = [0x5Au8; 32];

    #[test]
    fn should_reject_oversize_owned_output_script_before_reading_it() {
        const WALLET: WalletId = WALLET_ID;
        let mut conn = wallet_db("testnet");

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

    fn wallet_db(network: &str) -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        // The schema CHECK rejects unknown labels; a corrupt or newer file may still carry one.
        conn.pragma_update(None, "ignore_check_constraints", true)
            .unwrap();
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, ?2, 0)",
            params![&WALLET_ID[..], network],
        )
        .unwrap();
        conn
    }

    fn record(output_values: &[u64], received: &[u64]) -> TransactionRecord {
        let script = Address::new(
            dashcore::Network::Testnet,
            Payload::PubkeyHash(PubkeyHash::from_byte_array([7; 20])),
        )
        .script_pubkey();
        let body = CoreTransaction {
            version: 1,
            lock_time: 0,
            input: Vec::new(),
            output: output_values
                .iter()
                .map(|&value| TxOut {
                    value,
                    script_pubkey: script.clone(),
                })
                .collect(),
            special_transaction_payload: None,
        };
        let details = received
            .iter()
            .enumerate()
            .map(|(index, &value)| OutputDetail {
                index: index as u32,
                role: OutputRole::Received,
                address: None,
                value,
            })
            .collect();
        TransactionRecord::new(
            body,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Incoming,
            Vec::new(),
            details,
            0,
        )
    }

    fn store(conn: &Connection, record: &TransactionRecord) {
        conn.execute(
            "INSERT INTO core_transactions (wallet_id, txid, finalized, record_blob) \
             VALUES (?1, ?2, 0, ?3)",
            params![
                &WALLET_ID[..],
                record.txid.as_byte_array().as_slice(),
                blob::encode(record).unwrap()
            ],
        )
        .unwrap();
    }

    #[test]
    fn should_report_a_body_conflict_for_the_same_txid_with_another_body() {
        let mut conn = wallet_db("testnet");
        let stored = record(&[1_000], &[]);
        store(&conn, &stored);
        let mut incoming = record(&[2_000], &[]);
        incoming.txid = stored.txid;

        let tx = conn.transaction().unwrap();
        let err = preserve_known_details(&tx, &WALLET_ID, &incoming).unwrap_err();

        assert!(
            matches!(
                err,
                WalletStorageError::TransactionBodyConflict { wallet_id, txid }
                    if wallet_id == WALLET_ID && txid == stored.txid
            ),
            "got {err:?}"
        );
    }

    /// A peer can serve a known txid with BIP144 witnesses attached; the txid
    /// does not commit to them, so the stored body stands and the flush goes on.
    #[test]
    fn should_keep_the_stored_body_when_a_same_txid_body_differs_only_in_witness() {
        let mut conn = wallet_db("testnet");
        let mut stored = record(&[1_000], &[]);
        stored.transaction.input.push(dashcore::TxIn::default());
        stored.txid = stored.transaction.txid();
        store(&conn, &stored);
        let mut incoming = stored.clone();
        incoming.transaction.input[0].witness = dashcore::Witness::from_slice(&[[0xAB]]);
        assert_eq!(incoming.transaction.txid(), stored.txid);
        assert_ne!(incoming.transaction, stored.transaction);

        let tx = conn.transaction().unwrap();
        let merged = preserve_known_details(&tx, &WALLET_ID, &incoming).unwrap();

        assert_eq!(merged.transaction, stored.transaction);
    }

    #[test]
    fn should_report_an_unknown_wallet_network_label() {
        let mut conn = wallet_db("moonnet");
        let tx = conn.transaction().unwrap();

        let err = network(&tx, &WALLET_ID).unwrap_err();

        assert!(
            matches!(
                &err,
                WalletStorageError::UnknownWalletNetwork { wallet_id, label }
                    if *wallet_id == WALLET_ID && label == "moonnet"
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn should_report_a_net_amount_that_does_not_fit_i64() {
        let mut conn = wallet_db("testnet");
        let stored = record(&[u64::MAX, u64::MAX], &[u64::MAX, u64::MAX]);
        store(&conn, &stored);
        let tx = conn.transaction().unwrap();

        let err =
            repair_record(&tx, &WALLET_ID, &stored.txid, dashcore::Network::Testnet).unwrap_err();

        assert!(
            matches!(
                err,
                WalletStorageError::NetAmountOverflow { wallet_id, txid, value }
                    if wallet_id == WALLET_ID
                        && txid == stored.txid
                        && value == 2 * i128::from(u64::MAX)
            ),
            "got {err:?}"
        );
    }

    /// Shared with the Swift SDK's `TransactionAccountingTests` direction
    /// table; pins the rule repair takes from `platform_wallet`.
    #[test]
    fn should_classify_repaired_direction_like_the_swift_sdk() {
        use TransactionDirection::{CoinJoin, Incoming, Internal, Outgoing};
        use TransactionType::{AssetLock, Standard};
        // (type, spends ours, has owned output, has external output, expected)
        let cases = [
            (Standard, true, true, false, Internal),
            (Standard, true, true, true, Outgoing),
            (Standard, true, false, false, Outgoing),
            (AssetLock, true, false, false, Internal),
            (AssetLock, true, true, false, Internal),
            (AssetLock, true, false, true, Outgoing),
            (Standard, false, true, false, Incoming),
            (TransactionType::CoinJoin, true, true, false, CoinJoin),
        ];
        for (kind, spends_ours, has_ours, has_external, expected) in cases {
            assert_eq!(
                wallet_direction(kind, spends_ours, has_ours, has_external),
                expected,
                "{kind:?} spends_ours={spends_ours} has_ours={has_ours} has_external={has_external}"
            );
        }
    }

    const FUNDING: u64 = 100_000_000;
    const LOCK_CREDIT: u64 = 60_000_000;
    const LOCK_FEE: u64 = 1_000;

    fn address(byte: u8) -> Address {
        Address::new(
            dashcore::Network::Testnet,
            Payload::PubkeyHash(PubkeyHash::from_byte_array([byte; 20])),
        )
    }

    fn bip44() -> AccountType {
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        }
    }

    /// The funding account's record upstream emits for a spend of one
    /// wallet coin worth [`FUNDING`], classified the way upstream
    /// `record_transaction` does. `outputs` are `(script, value, role)`.
    fn funding_slice(
        kind: TransactionType,
        outputs: &[(ScriptBuf, u64, OutputRole)],
    ) -> TransactionRecord {
        let body = CoreTransaction {
            version: 3,
            lock_time: 0,
            input: vec![dashcore::TxIn {
                previous_output: OutPoint {
                    txid: Txid::from_byte_array([0x11; 32]),
                    vout: 0,
                },
                ..Default::default()
            }],
            output: outputs
                .iter()
                .map(|(script, value, _)| TxOut {
                    value: *value,
                    script_pubkey: script.clone(),
                })
                .collect(),
            special_transaction_payload: None,
        };
        let details: Vec<OutputDetail> = outputs
            .iter()
            .enumerate()
            .map(|(index, (script, value, role))| OutputDetail {
                index: index as u32,
                role: *role,
                address: Address::from_script(script, dashcore::Network::Testnet).ok(),
                value: *value,
            })
            .collect();
        let owned: u64 = details
            .iter()
            .filter(|d| is_owned(d.role))
            .map(|d| d.value)
            .sum();
        let has_sent = details.iter().any(|d| d.role == OutputRole::Sent);
        let direction = if !has_sent && owned > 0 {
            TransactionDirection::Internal
        } else {
            TransactionDirection::Outgoing
        };
        TransactionRecord::new(
            body,
            bip44(),
            TransactionContext::Mempool,
            kind,
            direction,
            vec![InputDetail {
                index: 0,
                value: FUNDING,
                address: address(1),
            }],
            details,
            owned as i64 - FUNDING as i64,
        )
    }

    /// The keys account's thin marker for an asset lock: `Internal`, no
    /// details, net `+credit` (the OP_RETURN output's value).
    fn keys_marker(funding: &TransactionRecord) -> TransactionRecord {
        let credit = funding.transaction.output[0].value;
        TransactionRecord::new(
            funding.transaction.clone(),
            AccountType::AssetLockAddressTopUp,
            TransactionContext::Mempool,
            TransactionType::AssetLock,
            TransactionDirection::Internal,
            Vec::new(),
            Vec::new(),
            credit as i64,
        )
    }

    /// The live projection and the SQLite repair must agree on a row's
    /// net and direction, or the row flips each time storage repairs
    /// what the live path just wrote. Covers the asset-lock shapes (no
    /// change, change, paying an external output) and a plain
    /// cross-account transfer.
    #[test]
    fn should_repair_live_folded_records_without_changing_their_accounting() {
        let burn = || ScriptBuf::new_op_return(&[]);
        let change = FUNDING - LOCK_CREDIT - LOCK_FEE;
        // With no change, everything but the fee is burned into credits.
        let lock_no_change = funding_slice(
            TransactionType::AssetLock,
            &[(burn(), FUNDING - LOCK_FEE, OutputRole::Unspendable)],
        );
        let lock_change = funding_slice(
            TransactionType::AssetLock,
            &[
                (burn(), LOCK_CREDIT, OutputRole::Unspendable),
                (address(2).script_pubkey(), change, OutputRole::Change),
            ],
        );
        let lock_external = funding_slice(
            TransactionType::AssetLock,
            &[
                (burn(), LOCK_CREDIT, OutputRole::Unspendable),
                (
                    address(2).script_pubkey(),
                    change - 5_000,
                    OutputRole::Change,
                ),
                (address(9).script_pubkey(), 5_000, OutputRole::Sent),
            ],
        );
        // Output 0 lands on a second account of the same wallet, which the
        // funding account's local view can only call `Sent`.
        let transfer = funding_slice(
            TransactionType::Standard,
            &[(
                address(3).script_pubkey(),
                FUNDING - LOCK_FEE,
                OutputRole::Sent,
            )],
        );
        let mut transfer_receiver = transfer.clone();
        transfer_receiver.account_type = AccountType::Standard {
            index: 1,
            standard_account_type: StandardAccountType::BIP44Account,
        };
        transfer_receiver.direction = TransactionDirection::Incoming;
        transfer_receiver.input_details.clear();
        transfer_receiver.output_details[0].role = OutputRole::Received;
        transfer_receiver.net_amount = (FUNDING - LOCK_FEE) as i64;

        let lock_net = -((LOCK_CREDIT + LOCK_FEE) as i64);
        let cases = [
            (
                "asset lock, no change",
                vec![lock_no_change.clone(), keys_marker(&lock_no_change)],
                TransactionDirection::Internal,
                -(FUNDING as i64),
            ),
            (
                "asset lock, change",
                vec![lock_change.clone(), keys_marker(&lock_change)],
                TransactionDirection::Internal,
                lock_net,
            ),
            (
                "asset lock, no change, funding slice alone",
                vec![lock_no_change.clone()],
                TransactionDirection::Internal,
                -(FUNDING as i64),
            ),
            (
                "asset lock paying an external output",
                vec![lock_external.clone(), keys_marker(&lock_external)],
                TransactionDirection::Outgoing,
                lock_net - 5_000,
            ),
            (
                "cross-account transfer",
                vec![transfer, transfer_receiver],
                TransactionDirection::Internal,
                -(LOCK_FEE as i64),
            ),
        ];
        for (name, mut records, direction, net) in cases {
            fold_wallet_records(&mut records);
            assert_eq!(records.len(), 1, "{name}");
            let live = &records[0];
            assert_eq!(live.direction, direction, "{name}: live direction");
            assert_eq!(live.net_amount, net, "{name}: live net");

            let mut conn = wallet_db("testnet");
            store(&conn, live);
            let tx = conn.transaction().unwrap();
            repair_record(&tx, &WALLET_ID, &live.txid, dashcore::Network::Testnet).unwrap();
            let repaired = prior_record(&tx, &WALLET_ID, &live.txid).unwrap().unwrap();

            assert_eq!(
                repaired.direction, live.direction,
                "{name}: repaired direction"
            );
            assert_eq!(repaired.net_amount, live.net_amount, "{name}: repaired net");
        }
    }
}
