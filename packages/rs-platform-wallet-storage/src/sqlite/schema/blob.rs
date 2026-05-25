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

/// Hard cap on bincode-serde decode allocations. 16 MiB is two orders
/// of magnitude above any legitimate per-row payload we ship — a
/// hostile or corrupted backup with an inflated length prefix is
/// rejected before the allocator wakes up. Applied symmetrically to
/// encode + decode so we can't write a payload we'd then refuse.
pub const BLOB_SIZE_LIMIT_BYTES: usize = 16 * 1024 * 1024;

fn bounded_config() -> bincode::config::Configuration<
    bincode::config::LittleEndian,
    bincode::config::Varint,
    bincode::config::Limit<BLOB_SIZE_LIMIT_BYTES>,
> {
    bincode::config::standard().with_limit::<BLOB_SIZE_LIMIT_BYTES>()
}

/// Encode a serde-derived value into a `BLOB` payload.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, WalletStorageError> {
    Ok(bincode::serde::encode_to_vec(value, bounded_config())?)
}

/// Decode a `BLOB` payload back into a serde-derived value. Rejects
/// trailing bytes so a corrupt or forward-incompatible payload fails
/// loudly instead of decoding a stale prefix — mirroring the strict
/// length check in [`decode_outpoint`]. Also caps in-decode
/// allocations at [`BLOB_SIZE_LIMIT_BYTES`] so a crafted length
/// prefix can't OOM the host (CMT-006).
pub fn decode<T: DeserializeOwned>(blob: &[u8]) -> Result<T, WalletStorageError> {
    if blob.len() > BLOB_SIZE_LIMIT_BYTES {
        return Err(WalletStorageError::BlobTooLarge {
            len_bytes: blob.len(),
            limit_bytes: BLOB_SIZE_LIMIT_BYTES,
        });
    }
    let (value, consumed) = match bincode::serde::decode_from_slice(blob, bounded_config()) {
        Ok(v) => v,
        Err(bincode::error::DecodeError::LimitExceeded) => {
            return Err(WalletStorageError::BlobTooLarge {
                len_bytes: blob.len(),
                limit_bytes: BLOB_SIZE_LIMIT_BYTES,
            });
        }
        Err(other) => return Err(WalletStorageError::from(other)),
    };
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

    /// CMT-006: a blob larger than the per-row cap is rejected with a
    /// typed `BlobTooLarge`, not generic `BlobDecode` and not an OOM.
    /// We synthesize the oversize payload directly (the in-band limit
    /// would prevent encoding it through the helper).
    #[test]
    fn decode_rejects_oversize_blob_with_blob_too_large() {
        let oversize = vec![0u8; BLOB_SIZE_LIMIT_BYTES + 1];
        let res: Result<Vec<u8>, _> = decode(&oversize);
        match res {
            Err(WalletStorageError::BlobTooLarge {
                len_bytes,
                limit_bytes,
            }) => {
                assert_eq!(len_bytes, BLOB_SIZE_LIMIT_BYTES + 1);
                assert_eq!(limit_bytes, BLOB_SIZE_LIMIT_BYTES);
            }
            other => panic!("expected BlobTooLarge, got {other:?}"),
        }
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
