//! Wallet accounting repaired from historical owned outputs, independent of live UTXOs.

use std::collections::{BTreeMap, HashSet};

use dashcore::hashes::Hash;
use dashcore::{Address, OutPoint, ScriptBuf, Txid};
use key_wallet::managed_account::transaction_record::{
    InputDetail, OutputDetail, OutputRole, TransactionDirection, TransactionRecord,
};
use key_wallet::transaction_checking::{TransactionContext, TransactionType};
use platform_wallet::changeset::CoreChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;
use rusqlite::{params, Connection, Transaction};

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
    if previous.transaction != incoming.transaction {
        return Err(WalletStorageError::TransactionBodyConflict {
            wallet_id: *wallet_id,
            txid: incoming.txid,
        });
    }
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
            .is_some_and(|old| matches!(old.role, OutputRole::Received | OutputRole::Change))
            && !matches!(detail.role, OutputRole::Received | OutputRole::Change);
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
    let mut stmt = tx.prepare_cached(
        "SELECT value, length(script), script FROM core_utxos \
         WHERE wallet_id = ?1 AND outpoint = ?2 AND is_sweep_placeholder = 0",
    )?;
    let mut rows = stmt.query(params![
        wallet_id.as_slice(),
        blob::encode_outpoint(outpoint)?
    ])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let value = i64_to_u64("core_utxos.value", row.get(0)?)?;
    blob::check_size(row.get(1)?)?;
    let script: Vec<u8> = row.get(2)?;
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
    record.direction = repaired_direction(
        record.transaction_type,
        !inputs.is_empty(),
        has_ours,
        has_external,
    );
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

/// Direction of a repaired record. The Swift SDK's
/// `PersistentTransaction.reconciledAccounting` applies the same rule; keep
/// both in step (each side tests the same case table).
///
/// `has_external` counts every output that is neither ours nor an OP_RETURN
/// burn, so an asset lock is internal only when nothing leaves the wallet.
fn repaired_direction(
    transaction_type: TransactionType,
    spends_ours: bool,
    has_ours: bool,
    has_external: bool,
) -> TransactionDirection {
    if transaction_type == TransactionType::CoinJoin {
        TransactionDirection::CoinJoin
    } else if !spends_ours {
        TransactionDirection::Incoming
    } else if !has_external && (has_ours || transaction_type == TransactionType::AssetLock) {
        TransactionDirection::Internal
    } else {
        TransactionDirection::Outgoing
    }
}

#[cfg(test)]
mod tests {
    use dashcore::address::Payload;
    use dashcore::{PubkeyHash, Transaction as CoreTransaction, TxOut};
    use key_wallet::account::{AccountType, StandardAccountType};

    use super::*;

    const WALLET_ID: WalletId = [0x5Au8; 32];

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

    /// Shared with the Swift SDK's `TransactionAccountingTests` direction table.
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
                repaired_direction(kind, spends_ours, has_ours, has_external),
                expected,
                "{kind:?} spends_ours={spends_ours} has_ours={has_ours} has_external={has_external}"
            );
        }
    }
}
