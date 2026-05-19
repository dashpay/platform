//! BLOB-column codec helpers.
//!
//! Thin error-mapping wrappers around `bincode::serde` so every
//! `_blob` column in the SQLite schema uses one encoding path. Schema
//! evolution is gated by the refinery migration version on the
//! database as a whole — there is no per-blob revision tag.
//!
//! [`encode_outpoint`] / [`decode_outpoint`] are a separate concern:
//! outpoints serve as primary-key fragments in typed columns, not as
//! blob payloads, and need a fixed on-disk layout for indexed lookups.

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::sqlite::error::WalletStorageError;

/// Encode a serde-derived value into a `BLOB` payload.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, WalletStorageError> {
    Ok(bincode::serde::encode_to_vec(
        value,
        bincode::config::standard(),
    )?)
}

/// Decode a `BLOB` payload back into a serde-derived value. Rejects
/// trailing bytes so a corrupt or forward-incompatible payload fails
/// loudly instead of decoding a stale prefix — mirroring the strict
/// length check in [`decode_outpoint`].
pub fn decode<T: DeserializeOwned>(blob: &[u8]) -> Result<T, WalletStorageError> {
    let (value, consumed) = bincode::serde::decode_from_slice(blob, bincode::config::standard())?;
    if consumed != blob.len() {
        return Err(WalletStorageError::blob_decode(
            "unexpected trailing bytes in blob payload",
        ));
    }
    Ok(value)
}

/// Encode a `dashcore::OutPoint` (txid + vout) as 36 bytes.
pub fn encode_outpoint(op: &dashcore::OutPoint) -> [u8; 36] {
    let mut out = [0u8; 36];
    out[..32].copy_from_slice(op.txid.as_ref());
    out[32..].copy_from_slice(&op.vout.to_le_bytes());
    out
}

/// Decode a 36-byte outpoint.
pub fn decode_outpoint(bytes: &[u8]) -> Result<dashcore::OutPoint, WalletStorageError> {
    use dashcore::hashes::Hash;
    if bytes.len() != 36 {
        return Err(WalletStorageError::blob_decode(
            "outpoint must be exactly 36 bytes",
        ));
    }
    let txid = dashcore::Txid::from_slice(&bytes[..32])?;
    let mut vout_bytes = [0u8; 4];
    vout_bytes.copy_from_slice(&bytes[32..]);
    Ok(dashcore::OutPoint {
        txid,
        vout: u32::from_le_bytes(vout_bytes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Dummy {
        a: u32,
        b: String,
    }

    #[test]
    fn encode_decode_roundtrip() {
        let value = Dummy {
            a: 42,
            b: "hello".into(),
        };
        let blob = encode(&value).unwrap();
        let decoded: Dummy = decode(&blob).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn decode_rejects_trailing_bytes() {
        let value = Dummy {
            a: 7,
            b: "world".into(),
        };
        let mut blob = encode(&value).unwrap();
        blob.push(0x00);
        let res: Result<Dummy, _> = decode(&blob);
        assert!(
            matches!(res, Err(WalletStorageError::BlobDecode { .. })),
            "expected BlobDecode on trailing bytes, got {res:?}"
        );
    }

    #[test]
    fn outpoint_roundtrip() {
        use dashcore::hashes::Hash;
        let op = dashcore::OutPoint {
            txid: dashcore::Txid::from_byte_array([7u8; 32]),
            vout: 9,
        };
        let bytes = encode_outpoint(&op);
        assert_eq!(decode_outpoint(&bytes).unwrap(), op);
    }
}
