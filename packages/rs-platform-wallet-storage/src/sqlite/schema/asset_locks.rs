//! `asset_locks` table writer + reader.
//!
//! Each row carries the lifecycle status as a string column plus a
//! self-describing blob for the rest (transaction hex, account/identity
//! indices, amount, optional proof bytes). The blob layout is documented
//! in [`encode`] / [`decode`].

use std::collections::BTreeMap;

use dashcore::consensus::{Decodable, Encodable};
use dashcore::OutPoint;
use rusqlite::{params, Connection, Transaction};

use platform_wallet::changeset::{AssetLockChangeSet, AssetLockEntry};
use platform_wallet::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::SqlitePersisterError;
use crate::sqlite::schema::blob::{decode_outpoint, encode_outpoint, BlobReader, BlobWriter};

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &AssetLockChangeSet,
) -> Result<(), SqlitePersisterError> {
    for (op, entry) in &cs.asset_locks {
        let op_bytes = encode_outpoint(op);
        let blob = encode(entry)?;
        tx.execute(
            "INSERT INTO asset_locks \
                (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(wallet_id, outpoint) DO UPDATE SET \
                status = excluded.status, \
                account_index = excluded.account_index, \
                identity_index = excluded.identity_index, \
                amount_duffs = excluded.amount_duffs, \
                lifecycle_blob = excluded.lifecycle_blob",
            params![
                wallet_id.as_slice(),
                &op_bytes[..],
                status_str(&entry.status),
                entry.account_index as i64,
                entry.identity_index as i64,
                entry.amount_duffs as i64,
                blob,
            ],
        )?;
    }
    for op in &cs.removed {
        let op_bytes = encode_outpoint(op);
        tx.execute(
            "DELETE FROM asset_locks WHERE wallet_id = ?1 AND outpoint = ?2",
            params![wallet_id.as_slice(), &op_bytes[..]],
        )?;
    }
    Ok(())
}

fn status_str(s: &AssetLockStatus) -> &'static str {
    match s {
        AssetLockStatus::Built => "built",
        AssetLockStatus::Broadcast => "broadcast",
        AssetLockStatus::InstantSendLocked => "is_locked",
        AssetLockStatus::ChainLocked => "chain_locked",
    }
}

fn parse_status(s: &str) -> Result<AssetLockStatus, SqlitePersisterError> {
    Ok(match s {
        "built" => AssetLockStatus::Built,
        "broadcast" => AssetLockStatus::Broadcast,
        "is_locked" => AssetLockStatus::InstantSendLocked,
        "chain_locked" => AssetLockStatus::ChainLocked,
        other => {
            return Err(SqlitePersisterError::serialization(format!(
                "unknown asset_lock status: {other}"
            )))
        }
    })
}

/// Serialise an `AssetLockEntry` into the `lifecycle_blob` column.
fn encode(entry: &AssetLockEntry) -> Result<Vec<u8>, SqlitePersisterError> {
    let mut w = BlobWriter::new();
    // funding_type is a tiny enum — encode as a u8.
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    let funding_tag: u8 = match entry.funding_type {
        AssetLockFundingType::IdentityRegistration => 0,
        AssetLockFundingType::IdentityTopUp => 1,
        AssetLockFundingType::IdentityTopUpNotBound => 2,
        AssetLockFundingType::IdentityInvitation => 3,
        AssetLockFundingType::AssetLockAddressTopUp => 4,
        AssetLockFundingType::AssetLockShieldedAddressTopUp => 5,
    };
    w.u8(funding_tag);
    // Transaction — consensus-encoded.
    let mut tx_bytes = Vec::new();
    entry
        .transaction
        .consensus_encode(&mut tx_bytes)
        .map_err(SqlitePersisterError::serialization)?;
    w.bytes(&tx_bytes);
    // Optional proof bytes (bincode-encoded via dpp).
    use bincode::config::standard;
    let proof_bytes: Option<Vec<u8>> = if let Some(proof) = &entry.proof {
        Some(
            bincode::encode_to_vec(proof, standard())
                .map_err(SqlitePersisterError::serialization)?,
        )
    } else {
        None
    };
    w.opt_bytes(proof_bytes.as_deref());
    Ok(w.finish())
}

fn decode(
    blob: &[u8],
    out_point: OutPoint,
    status: AssetLockStatus,
    account_index: u32,
    identity_index: u32,
    amount_duffs: u64,
) -> Result<AssetLockEntry, SqlitePersisterError> {
    let mut r = BlobReader::new(blob).map_err(SqlitePersisterError::serialization)?;
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    let funding_tag = r.u8().map_err(SqlitePersisterError::serialization)?;
    let funding_type = match funding_tag {
        0 => AssetLockFundingType::IdentityRegistration,
        1 => AssetLockFundingType::IdentityTopUp,
        2 => AssetLockFundingType::IdentityTopUpNotBound,
        3 => AssetLockFundingType::IdentityInvitation,
        4 => AssetLockFundingType::AssetLockAddressTopUp,
        5 => AssetLockFundingType::AssetLockShieldedAddressTopUp,
        other => {
            return Err(SqlitePersisterError::serialization(format!(
                "unknown funding type tag: {other}"
            )))
        }
    };
    let tx_bytes = r.bytes().map_err(SqlitePersisterError::serialization)?;
    let transaction = dashcore::Transaction::consensus_decode(&mut tx_bytes.as_slice())
        .map_err(SqlitePersisterError::serialization)?;
    let proof_bytes = r.opt_bytes().map_err(SqlitePersisterError::serialization)?;
    use bincode::config::standard;
    let proof = match proof_bytes {
        None => None,
        Some(b) => {
            let (decoded, _): (dpp::prelude::AssetLockProof, usize) =
                bincode::decode_from_slice(&b, standard())
                    .map_err(SqlitePersisterError::serialization)?;
            Some(decoded)
        }
    };
    Ok(AssetLockEntry {
        out_point,
        transaction,
        account_index,
        funding_type,
        identity_index,
        amount_duffs,
        status,
        proof,
    })
}

/// Return non-`Used` asset locks per wallet, bucketed by account index.
/// All four `AssetLockStatus` variants are considered "active" because
/// the changeset removes consumed locks via the `removed` set rather
/// than flagging them — by the time a lock is gone from the changeset
/// it should be gone from the table too.
pub fn list_active(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<BTreeMap<u32, BTreeMap<OutPoint, TrackedAssetLock>>, SqlitePersisterError> {
    let mut stmt = conn.prepare(
        "SELECT outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob \
         FROM asset_locks WHERE wallet_id = ?1",
    )?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        let op_bytes: Vec<u8> = row.get(0)?;
        let status: String = row.get(1)?;
        let account_index: i64 = row.get(2)?;
        let identity_index: i64 = row.get(3)?;
        let amount: i64 = row.get(4)?;
        let blob: Vec<u8> = row.get(5)?;
        Ok((
            op_bytes,
            status,
            account_index,
            identity_index,
            amount,
            blob,
        ))
    })?;
    let mut out: BTreeMap<u32, BTreeMap<OutPoint, TrackedAssetLock>> = BTreeMap::new();
    for r in rows {
        let (op_bytes, status_s, account_index, identity_index, amount, blob) = r?;
        let outpoint = decode_outpoint(&op_bytes).map_err(SqlitePersisterError::serialization)?;
        let status = parse_status(&status_s)?;
        let entry = decode(
            &blob,
            outpoint,
            status.clone(),
            account_index as u32,
            identity_index as u32,
            amount as u64,
        )?;
        let tracked = TrackedAssetLock {
            out_point: entry.out_point,
            transaction: entry.transaction,
            account_index: entry.account_index,
            funding_type: entry.funding_type,
            identity_index: entry.identity_index,
            amount: entry.amount_duffs,
            status: entry.status,
            proof: entry.proof,
        };
        out.entry(account_index as u32)
            .or_default()
            .insert(outpoint, tracked);
    }
    Ok(out)
}
