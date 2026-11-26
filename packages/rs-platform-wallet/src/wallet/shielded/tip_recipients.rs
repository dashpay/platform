//! The recipient a wallet last confirmed for each DPNS name it tipped: the
//! identity the name resolved to and the shielded address its profile
//! published. A tip review compares a fresh resolution against it, so a name
//! that now resolves to another identity or address is shown as changed
//! before the user pays it.
//!
//! One row per wallet and normalized DPNS label, in the shielded store's
//! recovery connection. A shielded Clear keeps the rows (they describe what
//! the user confirmed, not scanned state); removing the wallet drops them.

use rusqlite::{params, Connection, OptionalExtension};

use crate::wallet::platform_wallet::WalletId;

/// The identity id and raw Orchard address of a confirmed recipient.
pub(super) type ConfirmedTipRecipient = ([u8; 32], [u8; 43]);

pub(super) fn create_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS shielded_tip_recipients (
            wallet_id   BLOB NOT NULL,
            label       TEXT NOT NULL,
            identity_id BLOB NOT NULL,
            address     BLOB NOT NULL,
            PRIMARY KEY (wallet_id, label)
        )",
        [],
    )?;
    Ok(())
}

/// The stored identity id and address last confirmed for `label` from
/// `wallet_id`, as the row holds them: the caller checks their widths.
pub(super) fn confirmed(
    conn: &Connection,
    wallet_id: WalletId,
    label: &str,
) -> rusqlite::Result<Option<(Vec<u8>, Vec<u8>)>> {
    conn.query_row(
        "SELECT identity_id, address FROM shielded_tip_recipients \
         WHERE wallet_id = ?1 AND label = ?2",
        params![wallet_id.as_slice(), label],
        |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?)),
    )
    .optional()
}

/// Keep `recipient` as the one confirmed for `label` from `wallet_id`,
/// replacing an earlier one.
pub(super) fn confirm(
    conn: &Connection,
    wallet_id: WalletId,
    label: &str,
    recipient: &ConfirmedTipRecipient,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO shielded_tip_recipients \
         (wallet_id, label, identity_id, address) VALUES (?1, ?2, ?3, ?4)",
        params![
            wallet_id.as_slice(),
            label,
            recipient.0.as_slice(),
            recipient.1.as_slice()
        ],
    )?;
    Ok(())
}

/// Drop every recipient `wallet_id` confirmed, as the wallet is removed.
pub(super) fn purge_wallet(conn: &Connection, wallet_id: WalletId) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM shielded_tip_recipients WHERE wallet_id = ?1",
        params![wallet_id.as_slice()],
    )?;
    Ok(())
}
