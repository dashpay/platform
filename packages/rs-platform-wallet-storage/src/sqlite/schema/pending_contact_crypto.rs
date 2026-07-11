//! `pending_contact_crypto` deferred-crypto queue writer + reader.
//!
//! The signerless background sweep enqueues contact-crypto ops it can't perform
//! without a signer; they're drained when one is available. The queue is
//! persisted (a restore-from-Keychain is exactly when it's needed) as add/clear
//! deltas on the changeset. Each row stores the bincode-serde
//! [`PendingContactCrypto`] in `payload`; the `(owner, contact, kind)` columns
//! mirror its dedup key for keyed deletes + the CHECK on `kind`. Holds nothing
//! sensitive — only ciphertext + public key indices ever reach here.

// `BTreeMap` + `Connection` are used only by the test/helper-gated reader.
#[cfg(test)]
use std::collections::BTreeMap;

#[cfg(test)]
use rusqlite::Connection;
use rusqlite::{params, Transaction};

use platform_wallet::changeset::{
    PendingContactCrypto, PendingContactCryptoKey, PendingContactCryptoKind,
};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

/// TEXT-column domain for `pending_contact_crypto.kind`. Single source of truth
/// shared with the migration's CHECK clause and [`kind_db_label`]; pinned equal
/// to the writer's codomain by `kind_labels_match_enum`.
pub const KIND_LABELS: &[&str] = &[
    "register_receiving",
    "register_external",
    "contact_info_decrypt",
    "auto_accept",
];

fn kind_db_label(kind: PendingContactCryptoKind) -> &'static str {
    match kind {
        PendingContactCryptoKind::RegisterReceiving => "register_receiving",
        PendingContactCryptoKind::RegisterExternal => "register_external",
        PendingContactCryptoKind::ContactInfoDecrypt => "contact_info_decrypt",
        PendingContactCryptoKind::AutoAccept => "auto_accept",
    }
}

/// Apply one round of queue deltas in `tx`: upsert `added` (by the
/// `(owner, contact, kind)` dedup key, latest payload wins) and delete
/// `cleared`. Mirrors `accounts::apply_registrations`' upsert shape.
pub fn apply_pending_contact_crypto(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    added: &[PendingContactCrypto],
    cleared: &[PendingContactCryptoKey],
) -> Result<(), WalletStorageError> {
    if !added.is_empty() {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO pending_contact_crypto \
                (wallet_id, owner_identity_id, contact_id, kind, payload, enqueued_at_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(wallet_id, owner_identity_id, contact_id, kind) DO UPDATE SET \
                payload = excluded.payload, enqueued_at_ms = excluded.enqueued_at_ms",
        )?;
        for entry in added {
            let payload = blob::encode(entry)?;
            let owner = entry.owner_identity_id.to_buffer();
            let contact = entry.contact_id.to_buffer();
            let enqueued = i64::try_from(entry.enqueued_at_ms).unwrap_or(i64::MAX);
            stmt.execute(params![
                wallet_id.as_slice(),
                owner.as_slice(),
                contact.as_slice(),
                kind_db_label(entry.op.kind()),
                payload,
                enqueued,
            ])?;
        }
    }
    if !cleared.is_empty() {
        let mut stmt = tx.prepare_cached(
            "DELETE FROM pending_contact_crypto \
             WHERE wallet_id = ?1 AND owner_identity_id = ?2 AND contact_id = ?3 AND kind = ?4",
        )?;
        for key in cleared {
            let owner = key.owner_identity_id.to_buffer();
            let contact = key.contact_id.to_buffer();
            stmt.execute(params![
                wallet_id.as_slice(),
                owner.as_slice(),
                contact.as_slice(),
                kind_db_label(key.kind),
            ])?;
        }
    }
    Ok(())
}

/// Every wallet's deferred-crypto queue, grouped by `wallet_id`, decoded from
/// the `payload` blob.
///
/// The production consumer is the `load()` restore into each identity's
/// each identity's `DashPayState.pending_contact_crypto`, fanned out by `owner_identity_id`
/// (this reader returns entries grouped by `wallet_id`; the restore must apply
/// the wallet's identities BEFORE routing each entry to its owner's queue, or an
/// entry whose owner isn't resident yet is dropped). It is blocked on the
/// upstream per-wallet state restore (`LOAD_UNIMPLEMENTED: ClientStartState::wallets`
/// — see `persister.rs`). Until that lands this reader is exercised only by the
/// round-trip test, so it is `cfg(test)`-gated to keep both the lib and the
/// `__test-helpers` builds dead-code-clean; widen to
/// `any(test, feature = "__test-helpers")` when the load restore consumes it.
#[cfg(test)]
pub(crate) fn all_pending_contact_crypto(
    conn: &Connection,
) -> Result<BTreeMap<WalletId, Vec<PendingContactCrypto>>, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT wallet_id, payload FROM pending_contact_crypto \
         ORDER BY wallet_id, enqueued_at_ms",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?))
    })?;
    let mut out: BTreeMap<WalletId, Vec<PendingContactCrypto>> = BTreeMap::new();
    for r in rows {
        let (wid_bytes, payload) = r?;
        let wallet_id = <[u8; 32]>::try_from(wid_bytes.as_slice()).map_err(|_| {
            WalletStorageError::InvalidWalletIdLength {
                actual: wid_bytes.len(),
            }
        })?;
        let entry: PendingContactCrypto = blob::decode(&payload)?;
        out.entry(wallet_id).or_default().push(entry);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `KIND_LABELS` (the migration CHECK domain) must equal the set of labels
    /// `kind_db_label` can emit — so a new op kind can't slip past the CHECK.
    #[test]
    fn kind_labels_match_enum() {
        use std::collections::BTreeSet;
        let mapped: BTreeSet<&str> = [
            PendingContactCryptoKind::RegisterReceiving,
            PendingContactCryptoKind::RegisterExternal,
            PendingContactCryptoKind::ContactInfoDecrypt,
            PendingContactCryptoKind::AutoAccept,
        ]
        .into_iter()
        .map(kind_db_label)
        .collect();
        let labels: BTreeSet<&str> = KIND_LABELS.iter().copied().collect();
        assert_eq!(
            mapped, labels,
            "KIND_LABELS must equal the kind_db_label codomain"
        );
    }

    /// Persist two queue entries, read them back, clear one, read again —
    /// proving the add-delta upsert, the keyed clear-delta delete, and the
    /// payload round-trip all work against the real migrated schema.
    #[test]
    fn round_trip_persists_and_clears_queue() {
        use crate::sqlite::migrations;
        use crate::sqlite::schema::wallet_meta;
        use dpp::prelude::Identifier;
        use platform_wallet::changeset::PendingContactCryptoOp;
        use rusqlite::Connection;

        let mut conn = Connection::open_in_memory().unwrap();
        migrations::run(&mut conn).unwrap();
        let wallet_id: WalletId = [7u8; 32];
        wallet_meta::ensure_exists(&conn, &wallet_id).unwrap();

        let owner = Identifier::from([0xAAu8; 32]);
        let contact = Identifier::from([0xBBu8; 32]);
        let receiving = PendingContactCrypto {
            owner_identity_id: owner,
            contact_id: contact,
            op: PendingContactCryptoOp::RegisterReceiving,
            enqueued_at_ms: 1,
        };
        let external = PendingContactCrypto {
            owner_identity_id: owner,
            contact_id: contact,
            op: PendingContactCryptoOp::RegisterExternal {
                encrypted_public_key: vec![1, 2, 3],
                our_decryption_key_index: 0,
                contact_encryption_key_index: 0,
            },
            enqueued_at_ms: 2,
        };

        // Persist both add-deltas.
        {
            let tx = conn.transaction().unwrap();
            apply_pending_contact_crypto(
                &tx,
                &wallet_id,
                &[receiving.clone(), external.clone()],
                &[],
            )
            .unwrap();
            tx.commit().unwrap();
        }
        let loaded = all_pending_contact_crypto(&conn).unwrap();
        assert_eq!(
            loaded.get(&wallet_id).map(|v| v.len()),
            Some(2),
            "both enqueued entries persist and read back"
        );

        // Clear the receiving entry; the external one remains.
        {
            let tx = conn.transaction().unwrap();
            apply_pending_contact_crypto(&tx, &wallet_id, &[], &[receiving.key()]).unwrap();
            tx.commit().unwrap();
        }
        let remaining = all_pending_contact_crypto(&conn)
            .unwrap()
            .get(&wallet_id)
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            remaining.len(),
            1,
            "the clear-delta removes exactly one entry"
        );
        assert!(
            matches!(
                remaining[0].op,
                PendingContactCryptoOp::RegisterExternal { .. }
            ),
            "the external entry survives the receiving-entry clear"
        );
    }
}
