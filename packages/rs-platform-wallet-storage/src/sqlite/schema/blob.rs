//! BLOB-column codec helpers.
//!
//! Thin error-mapping wrappers around `bincode::serde` so every
//! `_blob` column in the SQLite schema uses one encoding path. Schema
//! evolution is gated by the refinery migration version on the
//! database as a whole — there is no per-blob revision tag.
//!
//! [`encode_outpoint`] / [`decode_outpoint`] encode a `dashcore::OutPoint`
//! the same way — via bincode-serde — for the `outpoint` PRIMARY KEY
//! columns (`core_utxos`, `asset_locks`). The bytes are a stable but not
//! fixed-length key; both columns are used for exact-match PK lookups, so
//! variable width is fine (no range scans or byte-order dependence).

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
/// loudly instead of decoding a stale prefix. Also caps in-decode
/// allocations at [`BLOB_SIZE_LIMIT_BYTES`] so a crafted length prefix
/// can't OOM the host.
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

/// Encode a `dashcore::OutPoint` for an `outpoint` PRIMARY KEY column.
/// Uses the same bincode-serde path as every other column — a stable
/// (not fixed-length) key, which the exact-match PK lookups don't mind.
pub fn encode_outpoint(op: &dashcore::OutPoint) -> Result<Vec<u8>, WalletStorageError> {
    encode(op)
}

/// Decode an outpoint key produced by [`encode_outpoint`]. Rejects
/// malformed or trailing bytes with a typed [`WalletStorageError`] via
/// the shared [`decode`] path.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn decode_outpoint(bytes: &[u8]) -> Result<dashcore::OutPoint, WalletStorageError> {
    decode(bytes)
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

    /// A blob larger than the per-row cap is rejected with a typed
    /// `BlobTooLarge`, not generic `BlobDecode` and not an OOM. We
    /// synthesize the oversize payload directly (the in-band limit would
    /// prevent encoding it through the helper).
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
        let bytes = encode_outpoint(&op).unwrap();
        assert_eq!(decode_outpoint(&bytes).unwrap(), op);
    }

    /// A non-zero vout round-trips too — bincode varint-encodes the vout,
    /// so the key length is not fixed but decoding recovers the value.
    #[test]
    fn outpoint_roundtrip_large_vout() {
        use dashcore::hashes::Hash;
        let op = dashcore::OutPoint {
            txid: dashcore::Txid::from_byte_array([0xABu8; 32]),
            vout: u32::MAX,
        };
        let bytes = encode_outpoint(&op).unwrap();
        assert_eq!(decode_outpoint(&bytes).unwrap(), op);
    }

    /// A truncated / malformed outpoint key is a typed decode error, not
    /// a panic — replaces the old fixed-36-byte length check. A 4-byte
    /// input is too short for the 32-byte txid prefix, so bincode fails
    /// deterministically with `BincodeDecode` (UnexpectedEnd) before the
    /// trailing-bytes check.
    #[test]
    fn decode_outpoint_rejects_malformed_bytes() {
        let res = decode_outpoint(&[0x01u8; 4]);
        assert!(
            matches!(res, Err(WalletStorageError::BincodeDecode { .. })),
            "a 4-byte payload must fail as BincodeDecode, got {res:?}"
        );
    }
}
