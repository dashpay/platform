//! BLOB-column codec helpers.
//!
//! Every `_blob` column on disk is laid out as `<u8 schema-rev>
//! || <bincode-serde body>`. The schema-rev tag lets a future
//! migration add new encoders without losing existing rows. Today
//! only one revision exists.
//!
//! The body uses `bincode::serde::encode_to_vec` /
//! `decode_from_slice` with `bincode::config::standard()` against
//! the platform-wallet changeset types (serde-derived via the
//! `platform-wallet/serde` feature).
//!
//! [`encode_outpoint`] / [`decode_outpoint`] live here too because
//! they're a typed-column helper, not a blob — outpoints serve as
//! primary-key fragments.

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::sqlite::error::SqlitePersisterError;

/// Schema-revision tag prepended to every blob.
pub const BLOB_REV: u8 = 1;

/// Encode a serde-derived value into a `BLOB` payload.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, SqlitePersisterError> {
    let body = bincode::serde::encode_to_vec(value, bincode::config::standard())
        .map_err(SqlitePersisterError::serialization)?;
    let mut out = Vec::with_capacity(1 + body.len());
    out.push(BLOB_REV);
    out.extend_from_slice(&body);
    Ok(out)
}

/// Decode a `BLOB` payload back into a serde-derived value.
pub fn decode<T: DeserializeOwned>(blob: &[u8]) -> Result<T, SqlitePersisterError> {
    let Some((&rev, body)) = blob.split_first() else {
        return Err(SqlitePersisterError::serialization("empty blob"));
    };
    if rev != BLOB_REV {
        return Err(SqlitePersisterError::serialization(format!(
            "unknown blob schema revision: {rev}"
        )));
    }
    let (value, _) = bincode::serde::decode_from_slice(body, bincode::config::standard())
        .map_err(SqlitePersisterError::serialization)?;
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
pub fn decode_outpoint(bytes: &[u8]) -> Result<dashcore::OutPoint, SqlitePersisterError> {
    use dashcore::hashes::Hash;
    if bytes.len() != 36 {
        return Err(SqlitePersisterError::serialization(
            "outpoint must be exactly 36 bytes",
        ));
    }
    let txid = dashcore::Txid::from_slice(&bytes[..32])
        .map_err(|e| SqlitePersisterError::serialization(format!("txid decode: {e}")))?;
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
        assert_eq!(blob[0], BLOB_REV);
        let decoded: Dummy = decode(&blob).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn decode_rejects_unknown_rev() {
        let bad = [99u8, 0, 0, 0];
        let err = decode::<Dummy>(&bad).unwrap_err().to_string();
        assert!(err.contains("unknown blob schema revision: 99"), "{err}");
    }

    #[test]
    fn decode_rejects_empty_blob() {
        let err = decode::<Dummy>(&[]).unwrap_err().to_string();
        assert!(err.contains("empty blob"), "{err}");
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
