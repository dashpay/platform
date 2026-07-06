//! BLOB-column codec helpers: thin `bincode::serde` wrappers so every
//! `_blob` column uses one encoding path. Schema evolution is gated by the
//! refinery migration version — no per-blob revision tag.
//!
//! [`encode_outpoint`] / [`decode_outpoint`] encode `dashcore::OutPoint`
//! the same way for the `outpoint` PK columns. The key is variable-width,
//! which is fine for the exact-match PK lookups (no range scans).

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::sqlite::error::WalletStorageError;

/// Sealed-trait machinery enforcing the no-key-material-in-DB invariant at
/// the type level: only types opting in via [`impl_persistable_blob!`] can
/// reach [`encode`].
pub(crate) mod sealed {
    /// `pub(crate)` supertrait of [`PersistableBlob`] — downstream cannot
    /// name it, so the trait is sealed.
    pub trait Sealed {}
}

/// Marker for types allowed into a `_blob` column. Sealed via
/// [`sealed::Sealed`] so adding a (possibly key-bearing) type to the
/// persistence path is an explicit, reviewable `impl` rather than a silent
/// `T: Serialize` slip.
pub trait PersistableBlob: Serialize + sealed::Sealed {}

/// Seal-and-mark a type for [`blob::encode`](encode).
macro_rules! impl_persistable_blob {
    ($($t:ty),+ $(,)?) => {
        $(
            impl $crate::sqlite::schema::blob::sealed::Sealed for $t {}
            impl $crate::sqlite::schema::blob::PersistableBlob for $t {}
        )+
    };
}
pub(crate) use impl_persistable_blob;

/// Hard cap on bincode-serde allocations, applied symmetrically to encode +
/// decode so a crafted length prefix can't OOM the host. Shares the crate-root
/// [`SIZE_LIMIT_BYTES`](crate::SIZE_LIMIT_BYTES) with the KV value cap.
pub const BLOB_SIZE_LIMIT_BYTES: usize = crate::SIZE_LIMIT_BYTES;

pub(crate) fn bounded_config() -> bincode::config::Configuration<
    bincode::config::LittleEndian,
    bincode::config::Varint,
    bincode::config::Limit<BLOB_SIZE_LIMIT_BYTES>,
> {
    bincode::config::standard().with_limit::<BLOB_SIZE_LIMIT_BYTES>()
}

/// Gate a variable-width blob column BEFORE materializing the `Vec<u8>`.
/// `len` is the value of `length(<col>)` selected in the same row.
/// Returns [`WalletStorageError::BlobTooLarge`] when `len` exceeds the cap.
pub(crate) fn check_size(len: i64) -> Result<(), WalletStorageError> {
    let len_usize = usize::try_from(len).unwrap_or(usize::MAX);
    if len_usize > BLOB_SIZE_LIMIT_BYTES {
        return Err(WalletStorageError::BlobTooLarge {
            len_bytes: len_usize,
            limit_bytes: BLOB_SIZE_LIMIT_BYTES,
        });
    }
    Ok(())
}

/// Gate a fixed-width blob column BEFORE materializing the `Vec<u8>`.
/// Oversize (`len` past the cap) surfaces as [`WalletStorageError::BlobTooLarge`];
/// any other deviation from `expected` as [`WalletStorageError::BlobDecode`].
pub(crate) fn check_fixed_width(
    len: i64,
    expected: usize,
    col: &'static str,
) -> Result<(), WalletStorageError> {
    check_size(len)?;
    if usize::try_from(len).unwrap_or(usize::MAX) != expected {
        return Err(WalletStorageError::blob_decode(col));
    }
    Ok(())
}

/// Encode a [`PersistableBlob`] value into a `BLOB` payload. The sealed bound
/// (not a bare `T: Serialize`) guards against unreviewed types reaching a
/// `_blob` column.
pub fn encode<T: PersistableBlob>(value: &T) -> Result<Vec<u8>, WalletStorageError> {
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

// An outpoint is a PUBLIC (txid, vout) reference — never key material.
impl_persistable_blob!(dashcore::OutPoint);

/// Encode a `dashcore::OutPoint` for an `outpoint` PRIMARY KEY column via the
/// shared [`encode`] path.
pub fn encode_outpoint(op: &dashcore::OutPoint) -> Result<Vec<u8>, WalletStorageError> {
    encode(op)
}

/// Decode an outpoint key produced by [`encode_outpoint`]. Rejects
/// malformed or trailing bytes with a typed [`WalletStorageError`] via
/// the shared [`decode`] path.
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
    impl_persistable_blob!(Dummy);

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

    /// A truncated outpoint key is a typed decode error, not a panic: a
    /// 4-byte input is too short for the 32-byte txid prefix, so bincode
    /// fails deterministically with `BincodeDecode` (UnexpectedEnd).
    #[test]
    fn decode_outpoint_rejects_malformed_bytes() {
        let res = decode_outpoint(&[0x01u8; 4]);
        assert!(
            matches!(res, Err(WalletStorageError::BincodeDecode { .. })),
            "a 4-byte payload must fail as BincodeDecode, got {res:?}"
        );
    }
}
