//! `identity_keys` table writer (PUBLIC material only — see NFR-10).
//!
//! `IdentityKeyEntry`'s `public_key: dpp::IdentityPublicKey` uses
//! `#[serde(tag = "$formatVersion")]` on the parent enum, which
//! bincode-serde rejects (it requires `deserialize_any`). The other
//! fields are plain serde-compatible types. To keep the
//! "one blob per row" property we transcribe the entry into a wire
//! shape where the public key is bincode-2-native-encoded (the dpp
//! types derive `Encode`/`Decode`) and the surrounding fields ride
//! the bincode-serde encoder. The shape is documented at
//! [`IdentityKeyWire`].

use rusqlite::{params, Transaction};
use serde::{Deserialize, Serialize};

use dpp::identity::{IdentityPublicKey, KeyID};
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

    fn into_entry(self) -> Result<IdentityKeyEntry, WalletStorageError> {
        let (public_key, _): (IdentityPublicKey, usize) =
            bincode::decode_from_slice(&self.public_key_bincode, bincode::config::standard())?;
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

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &IdentityKeysChangeSet,
) -> Result<(), WalletStorageError> {
    for ((identity_id, key_id), entry) in &cs.upserts {
        let wire = IdentityKeyWire::from_entry(entry)?;
        let entry_blob = blob::encode(&wire)?;
        tx.execute(
            "INSERT INTO identity_keys \
                (wallet_id, identity_id, key_id, public_key_blob, public_key_hash, derivation_blob) \
             VALUES (?1, ?2, ?3, ?4, ?5, NULL) \
             ON CONFLICT(wallet_id, identity_id, key_id) DO UPDATE SET \
                public_key_blob = excluded.public_key_blob, \
                public_key_hash = excluded.public_key_hash, \
                derivation_blob = NULL",
            params![
                wallet_id.as_slice(),
                identity_id.as_slice(),
                i64::from(*key_id),
                entry_blob,
                &entry.public_key_hash[..],
            ],
        )?;
    }
    for (identity_id, key_id) in &cs.removed {
        tx.execute(
            "DELETE FROM identity_keys \
             WHERE wallet_id = ?1 AND identity_id = ?2 AND key_id = ?3",
            params![
                wallet_id.as_slice(),
                identity_id.as_slice(),
                i64::from(*key_id),
            ],
        )?;
    }
    Ok(())
}

/// Decode an `identity_keys.public_key_blob` cell back to the entry.
pub fn decode_entry(payload: &[u8]) -> Result<IdentityKeyEntry, WalletStorageError> {
    let wire: IdentityKeyWire = blob::decode(payload)?;
    wire.into_entry()
}
