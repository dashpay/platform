//! `identity_keys` table writer. Stores PUBLIC key material only — no
//! signing-key bytes reach this table.
//!
//! `IdentityKeyEntry.public_key`'s `#[serde(tag = ...)]` enum is rejected by
//! bincode-serde (needs `deserialize_any`), so `IdentityKeyWire` pre-encodes
//! the key with bincode's native `Encode`/`Decode` and rides the surrounding
//! fields on the serde encoder, keeping one blob per row.

use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};

use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyID;
use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    IdentityKeyDerivationIndices, IdentityKeyEntry, IdentityKeysChangeSet,
};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

/// On-disk wire shape for `IdentityKeyEntry`, with `public_key_bincode`
/// holding the natively-encoded key (see module docs).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IdentityKeyWire {
    identity_id: Identifier,
    key_id: KeyID,
    public_key_bincode: Vec<u8>,
    public_key_hash: [u8; 20],
    wallet_id: Option<[u8; 32]>,
    derivation_indices: Option<IdentityKeyDerivationIndices>,
}

// PUBLIC material only reaching `entry_blob`: the wire shape carries
// bincode-encoded public keys + public-key hashes. No private bytes.
crate::sqlite::schema::blob::impl_persistable_blob!(IdentityKeyWire);

impl IdentityKeyWire {
    fn from_entry(entry: &IdentityKeyEntry) -> Result<Self, WalletStorageError> {
        let pk = bincode::encode_to_vec(&entry.public_key, blob::bounded_config())?;
        Ok(Self {
            identity_id: entry.identity_id,
            key_id: entry.key_id,
            public_key_bincode: pk,
            public_key_hash: entry.public_key_hash,
            wallet_id: entry.wallet_id,
            derivation_indices: entry.derivation_indices,
        })
    }

    fn into_entry(self) -> Result<IdentityKeyEntry, WalletStorageError> {
        let (public_key, consumed): (IdentityPublicKey, usize) =
            bincode::decode_from_slice(&self.public_key_bincode, blob::bounded_config())?;
        // Reject a valid-prefix + trailing-garbage payload (bincode stops
        // after the typed length); mirrors the outer blob::decode guard.
        if consumed != self.public_key_bincode.len() {
            return Err(WalletStorageError::blob_decode(
                "unexpected trailing bytes in identity_keys.public_key_bincode",
            ));
        }
        Ok(IdentityKeyEntry {
            identity_id: self.identity_id,
            key_id: self.key_id,
            public_key,
            public_key_hash: self.public_key_hash,
            wallet_id: self.wallet_id,
            derivation_indices: self.derivation_indices,
        })
    }
}

/// SQLite's extended result code for a foreign-key constraint failure
/// (`SQLITE_CONSTRAINT_FOREIGNKEY`). Discriminated numerically — never by
/// matching the driver's message text.
const SQLITE_CONSTRAINT_FOREIGNKEY: i32 = 787;

/// Re-map an FK violation on the `identity_keys` upsert to a typed error
/// naming the wallet and identity involved.
///
/// The table has two FK parents, so a violation could in principle be the
/// missing-`wallets` one; the compound `identities(wallet_id, identity_id)`
/// parent is by far the likelier cause and the one worth naming, since a
/// cross-wallet key write is the failure this guard exists to catch.
fn map_fk_violation(
    err: rusqlite::Error,
    wallet_id: &WalletId,
    identity_id: &Identifier,
) -> WalletStorageError {
    match &err {
        rusqlite::Error::SqliteFailure(e, _)
            if e.code == rusqlite::ErrorCode::ConstraintViolation
                && e.extended_code == SQLITE_CONSTRAINT_FOREIGNKEY =>
        {
            WalletStorageError::IdentityKeyWalletMismatch {
                wallet_id: *wallet_id,
                identity_id: identity_id.to_buffer(),
                source: Box::new(err),
            }
        }
        _ => WalletStorageError::from(err),
    }
}

/// Keyed by `(identity_id, key_id)` — matching the domain changeset, since
/// an identity has exactly one owning wallet — with an FK to `wallets` and a
/// compound FK to `identities(wallet_id, identity_id)` — so a key can only be
/// filed under the wallet that owns the identity. The typed `wallet_id` column
/// comes from the flush scope; the entry's own `wallet_id` (when set) is
/// cross-checked against it so the typed columns and the blob stay aligned.
/// A cross-wallet write is refused with
/// [`WalletStorageError::IdentityKeyWalletMismatch`].
pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &IdentityKeysChangeSet,
) -> Result<(), WalletStorageError> {
    if !cs.upserts.is_empty() {
        // `derivation_blob` is always NULL (reserved); derivation_indices ride
        // inside the IdentityKeyWire blob, the source of truth.
        //
        // `wallet_id = excluded.wallet_id` is load-bearing — do NOT drop it
        // as a redundant self-assignment. The primary key is
        // `(identity_id, key_id)`, so a foreign wallet writing an existing
        // key arrives as a CONFLICT, not an INSERT, and resolves to this
        // UPDATE. An UPDATE that leaves `wallet_id` untouched violates no
        // foreign key, so without this assignment the compound FK never
        // fires on the cross-wallet path — the write returns Ok and the
        // foreign wallet's key material silently replaces the owner's
        // under the owner's own scope. Assigning the column re-states the
        // incoming scope, which is what the FK checks.
        // Pinned by `identity_key_upsert_cannot_overwrite_another_wallets_key`.
        let mut stmt = tx.prepare_cached(
            "INSERT INTO identity_keys \
                (wallet_id, identity_id, key_id, public_key_blob, public_key_hash, derivation_blob) \
             VALUES (?1, ?2, ?3, ?4, ?5, NULL) \
             ON CONFLICT(identity_id, key_id) DO UPDATE SET \
                public_key_blob = excluded.public_key_blob, \
                public_key_hash = excluded.public_key_hash, \
                derivation_blob = NULL, \
                wallet_id = excluded.wallet_id",
        )?;
        for ((identity_id, key_id), entry) in &cs.upserts {
            // Typed columns and blob fields must agree so a row can never
            // diverge on disk.
            if entry.identity_id != *identity_id || entry.key_id != *key_id {
                return Err(WalletStorageError::IdentityKeyEntryMismatch);
            }
            if let Some(entry_wallet_id) = entry.wallet_id {
                if entry_wallet_id != *wallet_id {
                    return Err(WalletStorageError::IdentityKeyEntryMismatch);
                }
            }
            let wire = IdentityKeyWire::from_entry(entry)?;
            let entry_blob = blob::encode(&wire)?;
            stmt.execute(params![
                wallet_id.as_slice(),
                identity_id.as_slice(),
                i64::from(*key_id),
                entry_blob,
                &entry.public_key_hash[..],
            ])
            .map_err(|e| map_fk_violation(e, wallet_id, identity_id))?;
        }
    }
    if !cs.removed.is_empty() {
        // `(identity_id, key_id)` alone now identifies the row, so
        // `wallet_id = ?1` is no longer part of the key — it is kept
        // deliberately as a scope GUARD, mirroring the `identities`
        // tombstone: one wallet's `removed` set must not delete another
        // wallet's key. Keeping it makes a cross-scope delete a no-op;
        // dropping it would make that delete succeed, which is a
        // destructive way to be permissive.
        let mut stmt = tx.prepare_cached(
            "DELETE FROM identity_keys \
             WHERE wallet_id = ?1 AND identity_id = ?2 AND key_id = ?3",
        )?;
        for (identity_id, key_id) in &cs.removed {
            stmt.execute(params![
                wallet_id.as_slice(),
                identity_id.as_slice(),
                i64::from(*key_id),
            ])?;
        }
    }
    Ok(())
}

/// Decode an `identity_keys.public_key_blob` cell back to the entry.
pub fn decode_entry(payload: &[u8]) -> Result<IdentityKeyEntry, WalletStorageError> {
    let wire: IdentityKeyWire = blob::decode(payload)?;
    wire.into_entry()
}

/// Read every `identity_keys` row for `wallet_id` back into a keyless
/// [`IdentityKeysChangeSet`] (PUBLIC material only — the blob is an
/// `IdentityPublicKey`; private keys are NOT stored or read here).
///
/// Keyed by `(identity_id, key_id)`; `removed` is always empty (deletes
/// reach storage as `DELETE`s, never as rows). Any row whose blob fails
/// to decode is a hard, typed [`WalletStorageError`] — corruption is
/// never silently dropped.
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<IdentityKeysChangeSet, WalletStorageError> {
    let mut cs = IdentityKeysChangeSet::default();
    let mut stmt = conn.prepare(
        "SELECT identity_id, key_id, length(public_key_blob), public_key_blob \
         FROM identity_keys WHERE wallet_id = ?1",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    while let Some(row) = rows.next()? {
        let identity_id_bytes: Vec<u8> = row.get(0)?;
        let key_id: i64 = row.get(1)?;
        blob::check_size(row.get::<_, i64>(2)?)?;
        let payload: Vec<u8> = row.get(3)?;
        let id32 = super::id32("identity_keys.identity_id", &identity_id_bytes)?;
        let identity_id = Identifier::from(id32);
        let key_id: KeyID =
            crate::sqlite::util::safe_cast::i64_to_u32("identity_keys.key_id", key_id)?;
        let entry = decode_entry(&payload)?;
        // Cross-check the decoded blob against the typed columns it was
        // selected by (mirrors `accounts`/`asset_locks` readers): a row whose
        // blob names a different identity / key / wallet than its indexed
        // columns is corruption, never silently mis-keyed into the map.
        // `public_key.id()` is verified too — it becomes the DPP
        // signing-selection map key via `add_public_key`, so a mismatch would
        // file the key under a wrong KeyID rather than being caught here.
        if entry.identity_id != identity_id
            || entry.key_id != key_id
            || entry.public_key.id() != key_id
        {
            return Err(WalletStorageError::IdentityKeyEntryMismatch);
        }
        if let Some(entry_wallet_id) = entry.wallet_id {
            if entry_wallet_id != *wallet_id {
                return Err(WalletStorageError::IdentityKeyEntryMismatch);
            }
        }
        cs.upserts.insert((identity_id, key_id), entry);
    }
    Ok(cs)
}

/// Build an outer `public_key_blob` payload whose inner `public_key_bincode`
/// field contains a crafted byte sequence that causes the inner
/// `blob::bounded_config()` decode to fail. Used by the blob-gate integration
/// test to prove the inner decode is bounded end-to-end.
///
/// The outer blob is well within [`blob::BLOB_SIZE_LIMIT_BYTES`];
/// only the *inner* decode path is stressed.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn crafted_entry_blob_with_bad_pk_bincode_for_test() -> Vec<u8> {
    // 0xFC followed by four 0xFF bytes is bincode's 5-byte varint encoding
    // for u32::MAX (4 294 967 295). Decoded as the first value inside
    // IdentityPublicKey, this either triggers LimitExceeded (4 GB read
    // attempt > 16 MiB bound) or an InvalidVariant — either way the decode
    // fails without OOM-allocating.
    let wire = IdentityKeyWire {
        identity_id: dpp::prelude::Identifier::from([0xAAu8; 32]),
        key_id: 0,
        public_key_bincode: vec![0xFCu8, 0xFF, 0xFF, 0xFF, 0xFF],
        public_key_hash: [0u8; 20],
        wallet_id: None,
        derivation_indices: None,
    };
    // Intentionally unbounded outer encode — test setup only, not a
    // production path.
    bincode::serde::encode_to_vec(&wire, bincode::config::standard())
        .expect("test helper outer encode must not fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;

    /// In-memory connection with the full schema applied.
    fn migrated_conn() -> rusqlite::Connection {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        conn
    }

    /// A valid `IdentityPublicKey` for building wire blobs in tests.
    fn sample_public_key() -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![2u8; 33]),
            disabled_at: None,
        })
    }

    /// Insert an `identity_keys` row with a fully-formed wire blob, first
    /// staging the `wallets` + `identities` FK parents it depends on.
    fn insert_key_row(
        conn: &Connection,
        wallet: &[u8; 32],
        typed_identity: &[u8; 32],
        wire: &IdentityKeyWire,
    ) {
        conn.execute(
            "INSERT OR IGNORE INTO wallets (wallet_id, network, birth_height) \
             VALUES (?1, 'testnet', 0)",
            params![&wallet[..]],
        )
        .unwrap();
        crate::sqlite::schema::identities::ensure_exists(conn, wallet, typed_identity).unwrap();
        let entry_blob = blob::encode(wire).unwrap();
        conn.execute(
            "INSERT INTO identity_keys \
                (wallet_id, identity_id, key_id, public_key_blob, public_key_hash, derivation_blob) \
             VALUES (?1, ?2, 0, ?3, ?4, NULL)",
            params![&wallet[..], &typed_identity[..], entry_blob, &[0u8; 20][..]],
        )
        .unwrap();
    }

    /// `load_state`'s `key_id` boundary cast is a `u32` conversion
    /// (`KeyID = u32`), so an out-of-`u32`-range column must surface
    /// `IntegerOverflow` stamped `SafeCastTarget::U32` — naming `u64` would
    /// misdirect an operator to the wrong boundary.
    #[test]
    fn load_state_key_id_overflow_reports_u32_target() {
        let conn = migrated_conn();
        let wallet = [0x77u8; 32];
        let typed_identity = [0x88u8; 32];
        conn.execute(
            "INSERT OR IGNORE INTO wallets (wallet_id, network, birth_height) \
             VALUES (?1, 'testnet', 0)",
            params![&wallet[..]],
        )
        .unwrap();
        crate::sqlite::schema::identities::ensure_exists(&conn, &wallet, &typed_identity).unwrap();
        let wire = IdentityKeyWire {
            identity_id: Identifier::from(typed_identity),
            key_id: 0,
            public_key_bincode: bincode::encode_to_vec(sample_public_key(), blob::bounded_config())
                .unwrap(),
            public_key_hash: [0u8; 20],
            wallet_id: None,
            derivation_indices: None,
        };
        let entry_blob = blob::encode(&wire).unwrap();
        // key_id column set beyond u32::MAX so the i64->u32 cast overflows.
        conn.execute(
            "INSERT INTO identity_keys \
                (wallet_id, identity_id, key_id, public_key_blob, public_key_hash, derivation_blob) \
             VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
            params![
                &wallet[..],
                &typed_identity[..],
                i64::from(u32::MAX) + 1,
                entry_blob,
                &[0u8; 20][..]
            ],
        )
        .unwrap();

        let err = load_state(&conn, &wallet).expect_err("key_id overflow must fail");
        assert!(
            matches!(
                err,
                WalletStorageError::IntegerOverflow {
                    field: "identity_keys.key_id",
                    target: crate::sqlite::util::safe_cast::SafeCastTarget::U32,
                    ..
                }
            ),
            "expected IntegerOverflow with U32 target for key_id, got {err:?}"
        );
    }

    /// `load_state` rejects a row whose decoded blob names a different
    /// `identity_id` than its typed column — corruption is a hard, typed
    /// error rather than a silent mis-key into the upsert map.
    #[test]
    fn load_state_rejects_identity_id_column_mismatch() {
        let conn = migrated_conn();
        let wallet = [0x11u8; 32];
        let typed_identity = [0xBBu8; 32];
        let wire = IdentityKeyWire {
            identity_id: Identifier::from([0xAAu8; 32]), // disagrees with the column
            key_id: 0,
            public_key_bincode: bincode::encode_to_vec(sample_public_key(), blob::bounded_config())
                .unwrap(),
            public_key_hash: [0u8; 20],
            wallet_id: None,
            derivation_indices: None,
        };
        insert_key_row(&conn, &wallet, &typed_identity, &wire);

        let err = load_state(&conn, &wallet).expect_err("identity_id mismatch must fail");
        assert!(
            matches!(err, WalletStorageError::IdentityKeyEntryMismatch),
            "expected IdentityKeyEntryMismatch, got {err:?}"
        );
    }

    /// `load_state` rejects a row whose decoded blob carries a `wallet_id`
    /// different from the wallet scope the typed column is read under.
    #[test]
    fn load_state_rejects_wallet_id_blob_mismatch() {
        let conn = migrated_conn();
        let wallet = [0x22u8; 32];
        let typed_identity = [0xCCu8; 32];
        let wire = IdentityKeyWire {
            identity_id: Identifier::from(typed_identity), // matches the column
            key_id: 0,
            public_key_bincode: bincode::encode_to_vec(sample_public_key(), blob::bounded_config())
                .unwrap(),
            public_key_hash: [0u8; 20],
            wallet_id: Some([0xDDu8; 32]), // disagrees with the read scope
            derivation_indices: None,
        };
        insert_key_row(&conn, &wallet, &typed_identity, &wire);

        let err = load_state(&conn, &wallet).expect_err("wallet_id mismatch must fail");
        assert!(
            matches!(err, WalletStorageError::IdentityKeyEntryMismatch),
            "expected IdentityKeyEntryMismatch, got {err:?}"
        );
    }

    /// `load_state` rejects a row whose inner `IdentityPublicKey.id()`
    /// disagrees with the typed `key_id` column. That inner id becomes the
    /// DPP signing-selection map key via `add_public_key`, so a mismatch
    /// must hard-error, not file the key under the wrong KeyID.
    #[test]
    fn load_state_rejects_public_key_id_mismatch() {
        let conn = migrated_conn();
        let wallet = [0x33u8; 32];
        let typed_identity = [0xEEu8; 32];
        // Inner key carries id=5, but insert_key_row stamps the key_id column 0.
        let mismatched_pk = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 5,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![2u8; 33]),
            disabled_at: None,
        });
        let wire = IdentityKeyWire {
            identity_id: Identifier::from(typed_identity),
            key_id: 0, // agrees with the typed column
            public_key_bincode: bincode::encode_to_vec(&mismatched_pk, blob::bounded_config())
                .unwrap(),
            public_key_hash: [0u8; 20],
            wallet_id: None,
            derivation_indices: None,
        };
        insert_key_row(&conn, &wallet, &typed_identity, &wire);

        let err = load_state(&conn, &wallet).expect_err("public_key.id() mismatch must fail");
        assert!(
            matches!(err, WalletStorageError::IdentityKeyEntryMismatch),
            "expected IdentityKeyEntryMismatch, got {err:?}"
        );
    }

    /// A `public_key_bincode` payload whose IdentityPublicKey prefix is
    /// valid but carries trailing garbage is refused at decode time
    /// rather than silently dropping the trailing bytes.
    #[test]
    fn into_entry_rejects_trailing_bytes_in_public_key_bincode() {
        let pk = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![2u8; 33]),
            disabled_at: None,
        });
        let mut pk_bincode = bincode::encode_to_vec(&pk, blob::bounded_config()).unwrap();
        pk_bincode.push(0xFF); // trailing garbage past the typed length

        let wire = IdentityKeyWire {
            identity_id: dpp::prelude::Identifier::from([0xAA; 32]),
            key_id: 0,
            public_key_bincode: pk_bincode,
            public_key_hash: [0u8; 20],
            wallet_id: None,
            derivation_indices: None,
        };
        let err = wire.into_entry().expect_err("trailing bytes must error");
        assert!(
            matches!(err, WalletStorageError::BlobDecode { .. }),
            "expected BlobDecode for trailing-byte garbage, got {err:?}"
        );
    }

    /// `IdentityKeyEntry` carries no key material by construction
    /// (derive-sign-destroy removed the carried scalar; the client derives it
    /// on demand from the keychain), so the "no key material at rest outside
    /// the keychain" guarantee is enforced at the type level and the wire
    /// shape only has the breadcrumb metadata to preserve. Pins that a
    /// `from_entry` → `into_entry` round-trip keeps the `(wallet_id,
    /// derivation_indices)` breadcrumb intact.
    #[test]
    fn wire_round_trip_preserves_breadcrumb_metadata() {
        let pk = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![2u8; 33]),
            disabled_at: None,
        });
        let entry = IdentityKeyEntry {
            identity_id: dpp::prelude::Identifier::from([0xAA; 32]),
            key_id: 0,
            public_key: pk,
            public_key_hash: [0x11; 20],
            wallet_id: Some([0x9A; 32]),
            derivation_indices: Some(IdentityKeyDerivationIndices {
                identity_index: 1,
                key_index: 2,
            }),
        };

        let wire = IdentityKeyWire::from_entry(&entry).expect("encode wire");
        let restored = wire.into_entry().expect("decode wire");

        // The breadcrumb metadata survives the round-trip.
        assert_eq!(restored.wallet_id, entry.wallet_id);
        assert_eq!(restored.derivation_indices, entry.derivation_indices);
    }
}
