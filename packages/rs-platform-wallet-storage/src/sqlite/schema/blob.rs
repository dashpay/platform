//! Tiny self-describing binary encoder for blob columns.
//!
//! Upstream changeset types (`TransactionRecord`, `InstantLock`,
//! `Transaction`, etc.) do not derive `serde`, so we cannot bincode
//! them. Instead we encode the subset of fields the persister needs
//! using a fixed-shape layout per logical record kind. Each blob starts
//! with a `u8` schema-revision tag so future migrations can rewrite
//! in-place.
//!
//! The layout is deliberately minimal: little-endian integers, length-
//! prefixed byte strings, no padding, no embedded type info. Each
//! call-site documents the field order it expects.

use std::io::{Cursor, Read};

/// Schema-rev tag prepended to every blob.
pub const BLOB_REV: u8 = 1;

/// Builder for a blob payload.
pub struct BlobWriter {
    buf: Vec<u8>,
}

impl BlobWriter {
    pub fn new() -> Self {
        let mut buf = Vec::with_capacity(64);
        buf.push(BLOB_REV);
        Self { buf }
    }

    pub fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn bool(&mut self, v: bool) {
        self.buf.push(v as u8);
    }

    pub fn bytes(&mut self, v: &[u8]) {
        let len = v.len() as u64;
        self.buf.extend_from_slice(&len.to_le_bytes());
        self.buf.extend_from_slice(v);
    }

    pub fn opt_bytes(&mut self, v: Option<&[u8]>) {
        match v {
            None => self.buf.push(0),
            Some(b) => {
                self.buf.push(1);
                self.bytes(b);
            }
        }
    }

    pub fn opt_u32(&mut self, v: Option<u32>) {
        match v {
            None => self.buf.push(0),
            Some(x) => {
                self.buf.push(1);
                self.u32(x);
            }
        }
    }

    pub fn opt_u64(&mut self, v: Option<u64>) {
        match v {
            None => self.buf.push(0),
            Some(x) => {
                self.buf.push(1);
                self.u64(x);
            }
        }
    }

    pub fn str(&mut self, v: &str) {
        self.bytes(v.as_bytes());
    }

    pub fn finish(self) -> Vec<u8> {
        self.buf
    }
}

impl Default for BlobWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Reader for a blob payload. Methods return `Err` on truncation /
/// schema-rev mismatch.
pub struct BlobReader<'a> {
    inner: Cursor<&'a [u8]>,
}

impl<'a> BlobReader<'a> {
    pub fn new(buf: &'a [u8]) -> Result<Self, BlobError> {
        let mut r = Self {
            inner: Cursor::new(buf),
        };
        let rev = r.u8()?;
        if rev != BLOB_REV {
            return Err(BlobError::UnknownRev(rev));
        }
        Ok(r)
    }

    pub fn u8(&mut self) -> Result<u8, BlobError> {
        let mut b = [0u8; 1];
        self.inner
            .read_exact(&mut b)
            .map_err(|_| BlobError::Truncated)?;
        Ok(b[0])
    }

    pub fn u32(&mut self) -> Result<u32, BlobError> {
        let mut b = [0u8; 4];
        self.inner
            .read_exact(&mut b)
            .map_err(|_| BlobError::Truncated)?;
        Ok(u32::from_le_bytes(b))
    }

    pub fn u64(&mut self) -> Result<u64, BlobError> {
        let mut b = [0u8; 8];
        self.inner
            .read_exact(&mut b)
            .map_err(|_| BlobError::Truncated)?;
        Ok(u64::from_le_bytes(b))
    }

    pub fn bool(&mut self) -> Result<bool, BlobError> {
        Ok(self.u8()? != 0)
    }

    pub fn bytes(&mut self) -> Result<Vec<u8>, BlobError> {
        let len = self.u64()? as usize;
        let mut out = vec![0u8; len];
        self.inner
            .read_exact(&mut out)
            .map_err(|_| BlobError::Truncated)?;
        Ok(out)
    }

    pub fn opt_bytes(&mut self) -> Result<Option<Vec<u8>>, BlobError> {
        let tag = self.u8()?;
        match tag {
            0 => Ok(None),
            1 => Ok(Some(self.bytes()?)),
            other => Err(BlobError::BadOptionTag(other)),
        }
    }

    pub fn opt_u32(&mut self) -> Result<Option<u32>, BlobError> {
        let tag = self.u8()?;
        match tag {
            0 => Ok(None),
            1 => Ok(Some(self.u32()?)),
            other => Err(BlobError::BadOptionTag(other)),
        }
    }

    pub fn opt_u64(&mut self) -> Result<Option<u64>, BlobError> {
        let tag = self.u8()?;
        match tag {
            0 => Ok(None),
            1 => Ok(Some(self.u64()?)),
            other => Err(BlobError::BadOptionTag(other)),
        }
    }

    pub fn str(&mut self) -> Result<String, BlobError> {
        let bytes = self.bytes()?;
        String::from_utf8(bytes).map_err(|_| BlobError::BadUtf8)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BlobError {
    #[error("blob truncated")]
    Truncated,
    #[error("unknown blob schema revision: {0}")]
    UnknownRev(u8),
    #[error("bad option tag: {0}")]
    BadOptionTag(u8),
    #[error("bad UTF-8 in blob string")]
    BadUtf8,
}

/// Encode the `dashcore::OutPoint` (txid + vout) as 36 bytes.
pub fn encode_outpoint(op: &dashcore::OutPoint) -> [u8; 36] {
    let mut out = [0u8; 36];
    out[..32].copy_from_slice(op.txid.as_ref());
    out[32..].copy_from_slice(&op.vout.to_le_bytes());
    out
}

/// Decode a 36-byte outpoint.
pub fn decode_outpoint(bytes: &[u8]) -> Result<dashcore::OutPoint, BlobError> {
    use dashcore::hashes::Hash;
    if bytes.len() != 36 {
        return Err(BlobError::Truncated);
    }
    let txid = dashcore::Txid::from_slice(&bytes[..32]).map_err(|_| BlobError::Truncated)?;
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

    #[test]
    fn roundtrip_writer_reader() {
        let mut w = BlobWriter::new();
        w.u32(42);
        w.u64(123456789012345);
        w.bool(true);
        w.bytes(b"hello");
        w.opt_u32(None);
        w.opt_u32(Some(7));
        w.str("hi");
        let buf = w.finish();

        let mut r = BlobReader::new(&buf).unwrap();
        assert_eq!(r.u32().unwrap(), 42);
        assert_eq!(r.u64().unwrap(), 123456789012345);
        assert!(r.bool().unwrap());
        assert_eq!(r.bytes().unwrap(), b"hello");
        assert_eq!(r.opt_u32().unwrap(), None);
        assert_eq!(r.opt_u32().unwrap(), Some(7));
        assert_eq!(r.str().unwrap(), "hi");
    }

    #[test]
    fn writer_starts_with_rev() {
        let w = BlobWriter::new();
        assert_eq!(w.buf[0], BLOB_REV);
    }

    #[test]
    fn reader_rejects_unknown_rev() {
        let buf = [99u8, 0];
        let err = match BlobReader::new(&buf) {
            Ok(_) => panic!("expected rejection"),
            Err(e) => e,
        };
        assert!(matches!(err, BlobError::UnknownRev(99)));
    }
}
