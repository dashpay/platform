//! `invitations` table writer + reader (DIP-13 DashPay invitations).
//!
//! Every field maps to an explicit column (the entry is all-primitive), so a
//! row reconstructs an [`InvitationEntry`] directly — no lifecycle blob. No key
//! material is stored: the voucher key is re-derived from `funding_index`.

use rusqlite::{params, Transaction};

use platform_wallet::changeset::{InvitationChangeSet, InvitationStatus};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

// Imports used only by the test-gated reader below.
#[cfg(any(test, feature = "__test-helpers"))]
use {
    dashcore::OutPoint, platform_wallet::changeset::InvitationEntry, rusqlite::Connection,
    std::collections::BTreeMap,
};

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &InvitationChangeSet,
) -> Result<(), WalletStorageError> {
    if !cs.invitations.is_empty() {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO invitations \
                (wallet_id, outpoint, status, funding_index, amount_duffs, expiry_unix, created_at_secs, has_inviter) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
             ON CONFLICT(wallet_id, outpoint) DO UPDATE SET \
                status = excluded.status, \
                funding_index = excluded.funding_index, \
                amount_duffs = excluded.amount_duffs, \
                expiry_unix = excluded.expiry_unix, \
                created_at_secs = excluded.created_at_secs, \
                has_inviter = excluded.has_inviter",
        )?;
        for (op, entry) in &cs.invitations {
            let op_bytes = blob::encode_outpoint(op)?;
            stmt.execute(params![
                wallet_id.as_slice(),
                &op_bytes[..],
                status_str(&entry.status),
                i64::from(entry.funding_index),
                crate::sqlite::util::safe_cast::u64_to_i64(
                    "invitations.amount_duffs",
                    entry.amount_duffs,
                )?,
                i64::from(entry.expiry_unix),
                i64::from(entry.created_at_secs),
                i64::from(entry.has_inviter),
            ])?;
        }
    }
    if !cs.removed.is_empty() {
        let mut stmt =
            tx.prepare_cached("DELETE FROM invitations WHERE wallet_id = ?1 AND outpoint = ?2")?;
        for op in &cs.removed {
            let op_bytes = blob::encode_outpoint(op)?;
            stmt.execute(params![wallet_id.as_slice(), &op_bytes[..]])?;
        }
    }
    Ok(())
}

/// Single source of truth for the `invitations.status` TEXT-column domain.
/// The `CHECK (status IN …)` in `migrations/V003__invitations.rs` must list
/// exactly these values.
pub(crate) fn status_str(s: &InvitationStatus) -> &'static str {
    match s {
        InvitationStatus::Created => "created",
        InvitationStatus::Claimed => "claimed",
        InvitationStatus::Reclaimed => "reclaimed",
    }
}

#[cfg(any(test, feature = "__test-helpers"))]
fn status_from_str(s: &str) -> Result<InvitationStatus, WalletStorageError> {
    match s {
        "created" => Ok(InvitationStatus::Created),
        "claimed" => Ok(InvitationStatus::Claimed),
        "reclaimed" => Ok(InvitationStatus::Reclaimed),
        _ => Err(WalletStorageError::blob_decode(
            "unknown invitations.status value in row",
        )),
    }
}

/// Read every invitation row for a wallet, keyed by outpoint. Test/round-trip
/// helper (the production load path does not re-hydrate invitations into the
/// Rust manager; the Swift SwiftData mirror is the UI source).
#[cfg(any(test, feature = "__test-helpers"))]
pub fn read_all(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<BTreeMap<OutPoint, InvitationEntry>, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT outpoint, status, funding_index, amount_duffs, expiry_unix, created_at_secs, has_inviter \
         FROM invitations WHERE wallet_id = ?1",
    )?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        Ok((
            row.get::<_, Vec<u8>>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
        ))
    })?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (op_bytes, status, funding_index, amount, expiry, created_at, has_inviter) = row?;
        let out_point = blob::decode_outpoint(&op_bytes)?;
        out.insert(
            out_point,
            InvitationEntry {
                out_point,
                funding_index: funding_index as u32,
                amount_duffs: amount as u64,
                expiry_unix: expiry as u32,
                created_at_secs: created_at as u32,
                has_inviter: has_inviter != 0,
                status: status_from_str(&status)?,
            },
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::hashes::Hash;

    fn entry(vout: u32, status: InvitationStatus) -> InvitationEntry {
        InvitationEntry {
            out_point: OutPoint::new(dashcore::Txid::from_byte_array([vout as u8; 32]), vout),
            funding_index: vout,
            amount_duffs: 100_000 + u64::from(vout),
            expiry_unix: 1_800_000_000,
            created_at_secs: 1_799_913_600,
            has_inviter: vout.is_multiple_of(2),
            status,
        }
    }

    #[test]
    fn apply_then_read_round_trips_and_upserts_and_removes() {
        let wallet_id: WalletId = [0x11; 32];
        let mut conn = Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        // The FK requires the wallet_metadata row to exist.
        conn.execute(
            "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        // Insert two.
        let e0 = entry(0, InvitationStatus::Created);
        let e1 = entry(1, InvitationStatus::Created);
        let mut cs = InvitationChangeSet::default();
        cs.invitations.insert(e0.out_point, e0.clone());
        cs.invitations.insert(e1.out_point, e1.clone());
        {
            let tx = conn.transaction().unwrap();
            apply(&tx, &wallet_id, &cs).unwrap();
            tx.commit().unwrap();
        }
        let got = read_all(&conn, &wallet_id).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[&e0.out_point], e0);

        // Upsert e0 → Claimed, remove e1.
        let mut cs2 = InvitationChangeSet::default();
        let e0b = entry(0, InvitationStatus::Claimed);
        cs2.invitations.insert(e0b.out_point, e0b.clone());
        cs2.removed.insert(e1.out_point);
        {
            let tx = conn.transaction().unwrap();
            apply(&tx, &wallet_id, &cs2).unwrap();
            tx.commit().unwrap();
        }
        let got = read_all(&conn, &wallet_id).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[&e0.out_point].status, InvitationStatus::Claimed);
    }
}
