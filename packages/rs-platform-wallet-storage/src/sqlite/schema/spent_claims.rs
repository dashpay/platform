//! Authoritative engine claims, independent of historical TXO spend stamps.

use dashcore::hashes::Hash;
use dashcore::{OutPoint, Txid};
use platform_wallet::changeset::SpentClaimBatch;
use platform_wallet::wallet::platform_wallet::WalletId;
use rusqlite::{params, Connection, Transaction};

use super::blob;
use crate::sqlite::error::WalletStorageError;

/// Apply each engine batch in order within its causing event's transaction.
pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    batches: &[SpentClaimBatch],
) -> Result<(), WalletStorageError> {
    let mut delete =
        tx.prepare_cached("DELETE FROM core_spent_claims WHERE wallet_id = ?1 AND outpoint = ?2")?;
    let mut upsert = tx.prepare_cached("INSERT INTO core_spent_claims(wallet_id, outpoint, claimant) VALUES (?1, ?2, ?3) ON CONFLICT(wallet_id, outpoint) DO UPDATE SET claimant = excluded.claimant")?;
    for batch in batches {
        for (outpoint, claimant) in &batch.claimed {
            upsert.execute(params![
                wallet_id.as_slice(),
                blob::encode_outpoint(outpoint)?,
                claimant.map(|txid| txid.to_byte_array())
            ])?;
        }
        for outpoint in &batch.released {
            delete.execute(params![
                wallet_id.as_slice(),
                blob::encode_outpoint(outpoint)?
            ])?;
        }
    }
    Ok(())
}

/// Load complete claims using one bounded query, including unknown-owner outpoints.
pub fn load(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<Vec<(OutPoint, Option<Txid>)>, WalletStorageError> {
    let mut stmt = conn.prepare("SELECT length(outpoint), outpoint, length(claimant), claimant FROM core_spent_claims WHERE wallet_id = ?1 ORDER BY outpoint")?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    let mut claims = Vec::new();
    while let Some(row) = rows.next()? {
        blob::check_size(row.get(0)?)?;
        let outpoint: Vec<u8> = row.get(1)?;
        let claimant = if let Some(length) = row.get::<_, Option<i64>>(2)? {
            blob::check_fixed_width(length, 32, "core_spent_claims.claimant")?;
            let bytes: Vec<u8> = row.get(3)?;
            Some(Txid::from_slice(&bytes)?)
        } else {
            None
        };
        claims.push((blob::decode_outpoint(&outpoint)?, claimant));
    }
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::{migrations, schema::wallets};

    #[test]
    fn should_not_certify_legacy_wallets_from_metadata_or_partial_claim_batches() {
        use platform_wallet::changeset::WalletMetadataEntry;
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::run(&mut conn).unwrap();
        let wallet_id = [9; 32];
        conn.execute(
            "INSERT INTO wallets(wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![wallet_id.as_slice()],
        )
        .unwrap();
        let tx = conn.transaction().unwrap();
        wallets::upsert(
            &tx,
            &wallet_id,
            &WalletMetadataEntry {
                network: key_wallet::Network::Testnet,
                wallet_group_id: [0; 32],
                birth_height: 0,
            },
        )
        .unwrap();
        apply(
            &tx,
            &wallet_id,
            &[SpentClaimBatch {
                claimed: vec![(OutPoint::new(Txid::from_byte_array([2; 32]), 0), None)],
                released: vec![],
            }],
        )
        .unwrap();
        tx.commit().unwrap();
        assert!(!wallets::fetch(&conn, &wallet_id).unwrap().unwrap().2);
    }

    #[test]
    fn should_apply_ordered_claim_transfers_releases_and_unknown_claims_atomically() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::run(&mut conn).unwrap();
        let wallet_id = [1; 32];
        wallets::ensure_exists(&conn, &wallet_id).unwrap();
        let outpoint = OutPoint::new(Txid::from_byte_array([2; 32]), 0);
        let a = Txid::from_byte_array([3; 32]);
        let b = Txid::from_byte_array([4; 32]);
        let claims = |claimant| SpentClaimBatch {
            claimed: vec![(outpoint, claimant)],
            released: vec![],
        };
        let tx = conn.transaction().unwrap();
        apply(&tx, &wallet_id, &[claims(Some(a)), claims(Some(b))]).unwrap();
        assert_eq!(load(&tx, &wallet_id).unwrap(), vec![(outpoint, Some(b))]);
        apply(
            &tx,
            &wallet_id,
            &[SpentClaimBatch {
                claimed: vec![(outpoint, Some(a))],
                released: vec![outpoint],
            }],
        )
        .unwrap();
        assert!(load(&tx, &wallet_id).unwrap().is_empty());
        apply(
            &tx,
            &wallet_id,
            &[
                SpentClaimBatch {
                    claimed: vec![],
                    released: vec![outpoint],
                },
                claims(None),
                claims(None),
            ],
        )
        .unwrap();
        assert_eq!(load(&tx, &wallet_id).unwrap(), vec![(outpoint, None)]);
        tx.commit().unwrap();
        let tx = conn.transaction().unwrap();
        apply(
            &tx,
            &wallet_id,
            &[SpentClaimBatch {
                claimed: vec![],
                released: vec![outpoint],
            }],
        )
        .unwrap();
        assert!(load(&tx, &wallet_id).unwrap().is_empty());
        tx.rollback().unwrap();
        assert_eq!(load(&conn, &wallet_id).unwrap(), vec![(outpoint, None)]);
    }
}
