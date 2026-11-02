//! Bounded reading of Core's wire encoding. Hashes are 32 bytes in wire order,
//! the reverse of the order Core's RPC displays them in.
use crate::Error;
use sha2::{Digest, Sha256};

/// A double-SHA256 hash in wire byte order.
pub type Hash = [u8; 32];

/// Result of every check in this crate.
pub type Result<T> = core::result::Result<T, Error>;

/// Core's double SHA-256.
pub fn sha256d(bytes: &[u8]) -> Hash {
    Sha256::digest(Sha256::digest(bytes)).into()
}

pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    pub(crate) fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(len).ok_or(Error("length overflow"))?;
        let bytes = self
            .bytes
            .get(self.pos..end)
            .ok_or(Error("truncated input"))?;
        self.pos = end;
        Ok(bytes)
    }

    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        self.take(N)?
            .try_into()
            .map_err(|_| Error("truncated input"))
    }

    pub(crate) fn u8(&mut self) -> Result<u8> {
        Ok(self.array::<1>()?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub(crate) fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    pub(crate) fn finish(self) -> Result<()> {
        if self.pos == self.bytes.len() {
            Ok(())
        } else {
            Err(Error("trailing bytes"))
        }
    }

    /// Core's CompactSize: canonical encoding only, and at most `max`.
    pub(crate) fn compact(&mut self, max: u64) -> Result<usize> {
        let n = match self.u8()? {
            253 => {
                let n = u64::from(self.u16()?);
                if n < 253 {
                    return Err(Error("noncanonical CompactSize"));
                }
                n
            }
            254 => {
                let n = u64::from(self.u32()?);
                if n <= 0xffff {
                    return Err(Error("noncanonical CompactSize"));
                }
                n
            }
            255 => {
                let n = self.u64()?;
                if n <= 0xffff_ffff {
                    return Err(Error("noncanonical CompactSize"));
                }
                n
            }
            n => u64::from(n),
        };
        if n > max {
            return Err(Error("CompactSize limit"));
        }
        usize::try_from(n).map_err(|_| Error("length overflow"))
    }

    /// A blob with a 32-bit little-endian length prefix, at most `max` bytes.
    pub(crate) fn blob(&mut self, max: usize) -> Result<&'a [u8]> {
        let n = self.u32()? as usize;
        if n > max {
            return Err(Error("blob limit"));
        }
        self.take(n)
    }
}

pub(crate) fn write_blob(out: &mut Vec<u8>, bytes: &[u8]) -> Result<()> {
    let n = u32::try_from(bytes.len()).map_err(|_| Error("blob limit"))?;
    out.extend(n.to_le_bytes());
    out.extend(bytes);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_reject_noncanonical_and_oversized_compact_sizes() {
        assert_eq!(Reader::new(&[252]).compact(252), Ok(252));
        assert_eq!(Reader::new(&[253, 253, 0]).compact(253), Ok(253));
        assert!(Reader::new(&[253, 252, 0]).compact(u64::MAX).is_err());
        assert!(Reader::new(&[254, 0xff, 0xff, 0, 0])
            .compact(u64::MAX)
            .is_err());
        assert!(Reader::new(&[255, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0])
            .compact(u64::MAX)
            .is_err());
        assert!(Reader::new(&[253, 253, 0]).compact(252).is_err());
    }

    #[test]
    fn should_reject_truncated_blobs_and_trailing_bytes() {
        assert!(Reader::new(&[2, 0, 0, 0, 1]).blob(16).is_err());
        assert!(Reader::new(&[2, 0, 0, 0, 1, 2]).blob(1).is_err());
        let mut reader = Reader::new(&[1, 0, 0, 0, 7, 8]);
        assert_eq!(reader.blob(16), Ok(&[7u8][..]));
        assert!(reader.finish().is_err());
    }
}
