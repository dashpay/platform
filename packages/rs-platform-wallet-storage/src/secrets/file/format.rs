//! Versioned, self-describing vault format + canonical AAD
//! (SEC-REQ-2.2.7 / 2.2.9).
//!
//! ```text
//! MAGIC          9  b"PWSVAULT1"
//! format_version u32 LE  (= 1)
//! kdf_id         u8   (1 = Argon2id)
//! m_kib          u32 LE
//! t              u32 LE
//! p              u32 LE
//! salt_len       u8   (= 32)
//! salt           32
//! ── header ends ──
//! entries, each: label_len u16 LE | label | nonce 24 | ct_len u32 LE | ct+tag
//! ```
//!
//! The whole file is one logical map for a single `wallet_id`; KDF
//! params/salt are therefore per-wallet.

use super::super::error::SecretStoreError;
use super::crypto::{KdfParams, NONCE_LEN, SALT_LEN};

pub(crate) const MAGIC: &[u8; 9] = b"PWSVAULT1";
pub(crate) const FORMAT_VERSION: u32 = 1;
pub(crate) const KDF_ID_ARGON2ID: u8 = 1;

/// Parsed header (KDF params + salt).
#[derive(Debug, Clone)]
pub(crate) struct Header {
    pub params: KdfParams,
    pub salt: [u8; SALT_LEN],
}

/// One decrypted-on-demand vault entry.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub label: String,
    pub nonce: [u8; NONCE_LEN],
    pub ciphertext: Vec<u8>,
}

/// Canonical length-prefixed AAD binding ciphertext to its slot
/// (SEC-REQ-2.2.7): `format_version ‖ wallet_id ‖ label`. A blob moved
/// to another slot, or a rolled-back `format_version`, fails the tag.
pub(crate) fn aad(format_version: u32, wallet_id: &[u8; 32], label: &str) -> Vec<u8> {
    let lb = label.as_bytes();
    let mut v = Vec::with_capacity(4 + 4 + 32 + 4 + lb.len());
    v.extend_from_slice(&format_version.to_le_bytes());
    v.extend_from_slice(&(wallet_id.len() as u32).to_le_bytes());
    v.extend_from_slice(wallet_id);
    v.extend_from_slice(&(lb.len() as u32).to_le_bytes());
    v.extend_from_slice(lb);
    v
}

/// Serialize a full vault (header + entries) to bytes. Contains only
/// salt/params (non-secret) + ciphertext — never plaintext.
pub(crate) fn serialize(header: &Header, entries: &[Entry]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    out.push(KDF_ID_ARGON2ID);
    out.extend_from_slice(&header.params.m_kib.to_le_bytes());
    out.extend_from_slice(&header.params.t.to_le_bytes());
    out.extend_from_slice(&header.params.p.to_le_bytes());
    out.push(SALT_LEN as u8);
    out.extend_from_slice(&header.salt);
    for e in entries {
        let lb = e.label.as_bytes();
        out.extend_from_slice(&(lb.len() as u16).to_le_bytes());
        out.extend_from_slice(lb);
        out.extend_from_slice(&e.nonce);
        out.extend_from_slice(&(e.ciphertext.len() as u32).to_le_bytes());
        out.extend_from_slice(&e.ciphertext);
    }
    out
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], SecretStoreError> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or(SecretStoreError::MalformedVault)?;
        let s = self
            .buf
            .get(self.pos..end)
            .ok_or(SecretStoreError::MalformedVault)?;
        self.pos = end;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, SecretStoreError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, SecretStoreError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, SecretStoreError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

/// Parse a vault. Refuses unknown magic/version (fail closed,
/// SEC-REQ-2.2.9); parameter floors are enforced later at derive time.
pub(crate) fn deserialize(buf: &[u8]) -> Result<(Header, Vec<Entry>), SecretStoreError> {
    let mut r = Reader { buf, pos: 0 };
    if r.take(MAGIC.len())? != MAGIC {
        return Err(SecretStoreError::MalformedVault);
    }
    let version = r.u32()?;
    if version != FORMAT_VERSION {
        return Err(SecretStoreError::VersionUnsupported { found: version });
    }
    if r.u8()? != KDF_ID_ARGON2ID {
        return Err(SecretStoreError::MalformedVault);
    }
    let m_kib = r.u32()?;
    let t = r.u32()?;
    let p = r.u32()?;
    let salt_len = r.u8()? as usize;
    if salt_len != SALT_LEN {
        return Err(SecretStoreError::MalformedVault);
    }
    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(r.take(SALT_LEN)?);

    let mut entries = Vec::new();
    while r.pos < buf.len() {
        let label_len = r.u16()? as usize;
        let label = std::str::from_utf8(r.take(label_len)?)
            .map_err(|_| SecretStoreError::MalformedVault)?
            .to_string();
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(r.take(NONCE_LEN)?);
        let ct_len = r.u32()? as usize;
        let ciphertext = r.take(ct_len)?.to_vec();
        entries.push(Entry {
            label,
            nonce,
            ciphertext,
        });
    }
    Ok((
        Header {
            params: KdfParams { m_kib, t, p },
            salt,
        },
        entries,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aad_binds_slot() {
        let w = [1u8; 32];
        assert_ne!(aad(1, &w, "a"), aad(1, &w, "b"));
        assert_ne!(aad(1, &w, "a"), aad(2, &w, "a"));
        assert_ne!(aad(1, &w, "a"), aad(1, &[2u8; 32], "a"));
        // Length-prefix defeats `"a"+"bc"` vs `"ab"+"c"` ambiguity.
        assert_ne!(aad(1, &w, "ab"), {
            let mut v = aad(1, &w, "a");
            v.extend_from_slice(b"b");
            v
        });
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let header = Header {
            params: KdfParams::default_target(),
            salt: [7u8; SALT_LEN],
        };
        let entries = vec![
            Entry {
                label: "bip39_mnemonic".into(),
                nonce: [3u8; NONCE_LEN],
                ciphertext: vec![1, 2, 3, 4],
            },
            Entry {
                label: "bip32-seed".into(),
                nonce: [9u8; NONCE_LEN],
                ciphertext: vec![5, 6],
            },
        ];
        let bytes = serialize(&header, &entries);
        let (h2, e2) = deserialize(&bytes).unwrap();
        assert_eq!(h2.params, header.params);
        assert_eq!(h2.salt, header.salt);
        assert_eq!(e2.len(), 2);
        assert_eq!(e2[0].label, "bip39_mnemonic");
        assert_eq!(e2[1].ciphertext, vec![5, 6]);
    }

    #[test]
    fn rejects_bad_magic_and_unknown_version() {
        assert!(matches!(
            deserialize(b"NOPENOPE...."),
            Err(SecretStoreError::MalformedVault)
        ));
        let mut bytes = serialize(
            &Header {
                params: KdfParams::default_target(),
                salt: [0u8; SALT_LEN],
            },
            &[],
        );
        let v = MAGIC.len();
        bytes[v..v + 4].copy_from_slice(&999u32.to_le_bytes());
        assert!(matches!(
            deserialize(&bytes),
            Err(SecretStoreError::VersionUnsupported { found: 999 })
        ));
    }

    #[test]
    fn rejects_truncated() {
        let bytes = serialize(
            &Header {
                params: KdfParams::default_target(),
                salt: [0u8; SALT_LEN],
            },
            &[],
        );
        assert!(matches!(
            deserialize(&bytes[..bytes.len() - 5]),
            Err(SecretStoreError::MalformedVault)
        ));
    }
}
