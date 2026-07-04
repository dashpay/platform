//! `identity_keys` table writer. Stores PUBLIC key material only — no
//! signing-key bytes ever reach this table.
//!
//! `IdentityKeyEntry`'s `public_key: dpp::IdentityPublicKey` uses
//! `#[serde(tag = "$formatVersion")]` on the parent enum, which
//! bincode-serde rejects (it requires `deserialize_any`). The other
//! fields are plain serde-compatible types. To keep the
//! "one blob per row" property we transcribe the entry into a wire
//! shape where the public key is bincode-2-native-encoded (the dpp
//! types derive `Encode`/`Decode`) and the surrounding fields ride
//! the bincode-serde encoder. The shape is documented on the
//! `IdentityKeyWire` struct below.

use rusqlite::{params, Transaction};
use serde::{Deserialize, Serialize};

use dpp::identity::KeyID;
// Used only by the test-gated `into_entry` and the unit tests below.
#[cfg(any(test, feature = "__test-helpers"))]
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    IdentityKeyDerivationIndices, IdentityKeyEntry, IdentityKeysChangeSet,
};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

/// On-disk wire shape for `IdentityKeyEntry`. The `public_key` field
/// is pre-encoded via bincode 2's native `Encode/Decode` impls on
/// `dpp::IdentityPublicKey` so bincode-serde doesn't trip on dpp's
/// `serde(tag = ...)` representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IdentityKeyWire {
    identity_id: Identifier,
    key_id: KeyID,
    public_key_bincode: Vec<u8>,
    public_key_hash: [u8; 20],
    wallet_id: Option<[u8; 32]>,
    derivation_indices: Option<IdentityKeyDerivationIndices>,
}

impl IdentityKeyWire {
    fn from_entry(entry: &IdentityKeyEntry) -> Result<Self, WalletStorageError> {
        let pk = bincode::encode_to_vec(&entry.public_key, bincode::config::standard())?;
        Ok(Self {
            identity_id: entry.identity_id,
            key_id: entry.key_id,
            public_key_bincode: pk,
            public_key_hash: entry.public_key_hash,
            wallet_id: entry.wallet_id,
            derivation_indices: entry.derivation_indices,
        })
    }

    #[cfg(any(test, feature = "__test-helpers"))]
    fn into_entry(self) -> Result<IdentityKeyEntry, WalletStorageError> {
        let (public_key, consumed): (IdentityPublicKey, usize) =
            bincode::decode_from_slice(&self.public_key_bincode, bincode::config::standard())?;
        // Consistent with the outer blob::decode trailing-byte guard: a
        // valid-prefix + trailing-garbage payload that bincode's decoder
        // happily accepts (it stops after the typed length) is corruption
        // or forward-schema drift — refuse it.
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

/// `identity_keys` is keyed by `(identity_id, key_id)`; the parent FK
/// targets `identities(identity_id)`. The caller-supplied [`WalletId`]
/// scopes cross-checks against the entry's own `wallet_id` field so
/// the entry-blob and the typed columns stay aligned.
pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &IdentityKeysChangeSet,
) -> Result<(), WalletStorageError> {
    if !cs.upserts.is_empty() {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO identity_keys \
                (identity_id, key_id, public_key_blob, public_key_hash) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(identity_id, key_id) DO UPDATE SET \
                public_key_blob = excluded.public_key_blob, \
                public_key_hash = excluded.public_key_hash",
        )?;
        for ((identity_id, key_id), entry) in &cs.upserts {
            // Reject any disagreement between the map key / outer
            // wallet_id (informational scope) and the entry fields
            // (what the serialized blob carries) so the two
            // representations of a row can never diverge on disk.
            if entry.identity_id != *identity_id || entry.key_id != *key_id {
                return Err(WalletStorageError::IdentityKeyEntryMismatch);
            }
            // Sentinel scope ("no parent wallet known") requires the
            // entry's wallet_id to also be `None`; a real entry
            // wallet_id under sentinel scope would silently file the
            // key under the wrong parenting. Non-sentinel scope
            // requires the entry's wallet_id (when set) to match
            // exactly.
            let scope_is_sentinel = wallet_id.iter().all(|b| *b == 0);
            match (scope_is_sentinel, entry.wallet_id) {
                (true, Some(_)) => return Err(WalletStorageError::IdentityKeyEntryMismatch),
                (false, Some(entry_wallet_id)) if entry_wallet_id != *wallet_id => {
                    return Err(WalletStorageError::IdentityKeyEntryMismatch);
                }
                _ => {}
            }
            let wire = IdentityKeyWire::from_entry(entry)?;
            let entry_blob = blob::encode(&wire)?;
            stmt.execute(params![
                identity_id.as_slice(),
                i64::from(*key_id),
                entry_blob,
                &entry.public_key_hash[..],
            ])?;
        }
    }
    if !cs.removed.is_empty() {
        let mut stmt =
            tx.prepare_cached("DELETE FROM identity_keys WHERE identity_id = ?1 AND key_id = ?2")?;
        for (identity_id, key_id) in &cs.removed {
            stmt.execute(params![identity_id.as_slice(), i64::from(*key_id)])?;
        }
    }
    Ok(())
}

/// Decode an `identity_keys.public_key_blob` cell back to the entry.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn decode_entry(payload: &[u8]) -> Result<IdentityKeyEntry, WalletStorageError> {
    let wire: IdentityKeyWire = blob::decode(payload)?;
    wire.into_entry()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;

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
        let mut pk_bincode = bincode::encode_to_vec(&pk, bincode::config::standard()).unwrap();
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
