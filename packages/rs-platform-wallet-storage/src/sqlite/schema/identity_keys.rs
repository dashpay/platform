//! `identity_keys` table writer (PUBLIC material only — see NFR-10).

use rusqlite::{params, Transaction};

use platform_wallet::changeset::IdentityKeysChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::SqlitePersisterError;
use crate::sqlite::schema::blob::BlobWriter;

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &IdentityKeysChangeSet,
) -> Result<(), SqlitePersisterError> {
    for ((identity_id, key_id), entry) in &cs.upserts {
        // Encode the DPP `IdentityPublicKey` via its `Encode` impl from
        // `dpp` (it implements bincode 2 Encode/Decode).
        let pk_blob = encode_public_key(&entry.public_key)?;
        let derivation_blob = entry.derivation_indices.map(|d| {
            let mut w = BlobWriter::new();
            w.u32(d.identity_index);
            w.u32(d.key_index);
            w.finish()
        });
        tx.execute(
            "INSERT INTO identity_keys \
                (wallet_id, identity_id, key_id, public_key_blob, public_key_hash, derivation_blob) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(wallet_id, identity_id, key_id) DO UPDATE SET \
                public_key_blob = excluded.public_key_blob, \
                public_key_hash = excluded.public_key_hash, \
                derivation_blob = excluded.derivation_blob",
            params![
                wallet_id.as_slice(),
                identity_id.as_slice(),
                *key_id as i64,
                pk_blob,
                &entry.public_key_hash[..],
                derivation_blob,
            ],
        )?;
    }
    for (identity_id, key_id) in &cs.removed {
        tx.execute(
            "DELETE FROM identity_keys \
             WHERE wallet_id = ?1 AND identity_id = ?2 AND key_id = ?3",
            params![wallet_id.as_slice(), identity_id.as_slice(), *key_id as i64],
        )?;
    }
    Ok(())
}

fn encode_public_key(
    key: &dpp::identity::IdentityPublicKey,
) -> Result<Vec<u8>, SqlitePersisterError> {
    use bincode::config::standard;
    bincode::encode_to_vec(key, standard()).map_err(SqlitePersisterError::serialization)
}
