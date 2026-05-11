//! `identities` table writer.

use rusqlite::{params, Connection, Transaction};

use platform_wallet::changeset::IdentityChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::SqlitePersisterError;
use crate::sqlite::schema::blob::BlobWriter;

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &IdentityChangeSet,
) -> Result<(), SqlitePersisterError> {
    for (id, entry) in &cs.identities {
        let mut w = BlobWriter::new();
        w.u64(entry.balance);
        w.u64(entry.revision);
        w.opt_u32(entry.identity_index);
        let blob = w.finish();
        tx.execute(
            "INSERT INTO identities (wallet_id, wallet_index, identity_id, entry_blob, tombstoned) \
             VALUES (?1, ?2, ?3, ?4, 0) \
             ON CONFLICT(wallet_id, identity_id) DO UPDATE SET \
                wallet_index = excluded.wallet_index, \
                entry_blob = excluded.entry_blob, \
                tombstoned = 0",
            params![
                wallet_id.as_slice(),
                entry.identity_index.map(|i| i as i64),
                id.as_slice(),
                blob,
            ],
        )?;
    }
    for id in &cs.removed {
        tx.execute(
            "UPDATE identities SET tombstoned = 1 WHERE wallet_id = ?1 AND identity_id = ?2",
            params![wallet_id.as_slice(), id.as_slice()],
        )?;
    }
    Ok(())
}

/// Insert a stub identity row so identity_keys / dashpay_profiles can
/// reference it via the FK trigger. Used by tests that exercise
/// identity_keys persistence without going through full identity flow.
pub fn ensure_exists(
    conn: &Connection,
    wallet_id: &WalletId,
    identity_id: &[u8; 32],
) -> Result<(), SqlitePersisterError> {
    conn.execute(
        "INSERT OR IGNORE INTO identities \
            (wallet_id, wallet_index, identity_id, entry_blob, tombstoned) \
         VALUES (?1, NULL, ?2, ?3, 0)",
        params![
            wallet_id.as_slice(),
            &identity_id[..],
            BlobWriter::new().finish(),
        ],
    )?;
    Ok(())
}
