//! Argon2id KDF + XChaCha20-Poly1305 AEAD (SEC-REQ-2.2.1–2.2.8).
//!
//! `pub(crate)` only — no crypto primitive escapes the `secrets` tree.

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use getrandom::getrandom;

use super::super::secret::SecretBytes;
use super::error::FileStoreError;

/// Argon2 parameter floors (SEC-REQ-2.2.2) — derivation MUST NOT use
/// anything weaker; a header declaring less is refused.
pub(crate) const ARGON2_MIN_M_KIB: u32 = 19_456;
pub(crate) const ARGON2_MIN_T: u32 = 2;
pub(crate) const ARGON2_P: u32 = 1;

/// Shipped defaults for new vaults (SEC-REQ-2.2.2 SHOULD target:
/// 64 MiB, t≥3).
pub(crate) const ARGON2_DEFAULT_M_KIB: u32 = 65_536;
pub(crate) const ARGON2_DEFAULT_T: u32 = 3;

/// CSPRNG salt width (≥16 required; we use 32 — SEC-REQ-2.2.3).
pub(crate) const SALT_LEN: usize = 32;
/// XChaCha20-Poly1305 nonce width (SEC-REQ-2.2.6).
pub(crate) const NONCE_LEN: usize = 24;
/// Derived AEAD key width.
pub(crate) const KEY_LEN: usize = 32;

/// Fill `buf` with CSPRNG bytes (`OsRng` via `getrandom`).
pub(crate) fn random_bytes(buf: &mut [u8]) -> Result<(), FileStoreError> {
    getrandom(buf).map_err(|_| FileStoreError::KdfFailure)
}

/// Argon2id parameters as stored in / read from a vault header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KdfParams {
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

impl KdfParams {
    /// The shipped default for new vaults.
    pub(crate) fn default_target() -> Self {
        Self {
            m_kib: ARGON2_DEFAULT_M_KIB,
            t: ARGON2_DEFAULT_T,
            p: ARGON2_P,
        }
    }

    /// Reject params below the floors (a downgraded header) before any
    /// derivation runs (SEC-REQ-2.2.2).
    pub(crate) fn enforce_floors(&self) -> Result<(), FileStoreError> {
        if self.m_kib < ARGON2_MIN_M_KIB || self.t < ARGON2_MIN_T || self.p != ARGON2_P {
            return Err(FileStoreError::KdfFailure);
        }
        Ok(())
    }
}

/// Derive a 32-byte AEAD key from `passphrase` + `salt` with Argon2id.
/// Output lands directly in a [`SecretBytes`] (SEC-REQ-2.2.4).
pub(crate) fn derive_key(
    passphrase: &[u8],
    salt: &[u8],
    params: KdfParams,
) -> Result<SecretBytes, FileStoreError> {
    params.enforce_floors()?;
    let argon_params = Params::new(params.m_kib, params.t, params.p, Some(KEY_LEN))
        .map_err(|_| FileStoreError::KdfFailure)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);
    let mut key = SecretBytes::zeroed(KEY_LEN);
    argon
        .hash_password_into(passphrase, salt, key.expose_secret_mut())
        .map_err(|_| FileStoreError::KdfFailure)?;
    Ok(key)
}

/// Encrypt `plaintext` under `key` with a fresh random nonce, binding
/// `aad`. Returns `(nonce, ciphertext_with_tag)` (SEC-REQ-2.2.5/.6/.7).
pub(crate) fn seal(
    key: &SecretBytes,
    aad: &[u8],
    plaintext: &[u8],
) -> Result<([u8; NONCE_LEN], Vec<u8>), FileStoreError> {
    let cipher = XChaCha20Poly1305::new_from_slice(key.expose_secret())
        .map_err(|_| FileStoreError::KdfFailure)?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    random_bytes(&mut nonce_bytes)?;
    let nonce = XNonce::from_slice(&nonce_bytes);
    let ct = cipher
        .encrypt(
            nonce,
            chacha20poly1305::aead::Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| FileStoreError::Decrypt)?;
    Ok((nonce_bytes, ct))
}

/// Decrypt `ciphertext` under `key`/`nonce`/`aad`. On tag failure
/// returns [`FileStoreError::Decrypt`] and **no** plaintext — the
/// combined (non-detached) API never materializes unverified bytes at
/// our boundary (SEC-REQ-2.2.8, CWE-347, the RUSTSEC-2023-0096 lesson).
pub(crate) fn open(
    key: &SecretBytes,
    nonce: &[u8; NONCE_LEN],
    aad: &[u8],
    ciphertext: &[u8],
) -> Result<SecretBytes, FileStoreError> {
    let cipher = XChaCha20Poly1305::new_from_slice(key.expose_secret())
        .map_err(|_| FileStoreError::KdfFailure)?;
    let nonce = XNonce::from_slice(nonce);
    let pt = cipher
        .decrypt(
            nonce,
            chacha20poly1305::aead::Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| FileStoreError::Decrypt)?;
    Ok(SecretBytes::new(pt))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floors_reject_weak_params() {
        assert!(KdfParams {
            m_kib: 1024,
            t: 2,
            p: 1
        }
        .enforce_floors()
        .is_err());
        assert!(KdfParams {
            m_kib: ARGON2_MIN_M_KIB,
            t: 1,
            p: 1
        }
        .enforce_floors()
        .is_err());
        assert!(KdfParams {
            m_kib: ARGON2_MIN_M_KIB,
            t: 2,
            p: 2
        }
        .enforce_floors()
        .is_err());
        assert!(KdfParams::default_target().enforce_floors().is_ok());
    }

    #[test]
    fn seal_open_roundtrip_with_floor_params() {
        // Floor params keep the test fast; production uses the default
        // target (64 MiB) which is too slow for a unit test.
        let params = KdfParams {
            m_kib: ARGON2_MIN_M_KIB,
            t: ARGON2_MIN_T,
            p: ARGON2_P,
        };
        let mut salt = [0u8; SALT_LEN];
        random_bytes(&mut salt).unwrap();
        let key = derive_key(b"correct horse", &salt, params).unwrap();
        let aad = b"v1|wallet|label";
        let (nonce, ct) = seal(&key, aad, b"top secret seed").unwrap();
        let pt = open(&key, &nonce, aad, &ct).unwrap();
        assert_eq!(pt.expose_secret(), b"top secret seed");
    }

    #[test]
    fn wrong_aad_fails_with_no_plaintext() {
        let params = KdfParams {
            m_kib: ARGON2_MIN_M_KIB,
            t: ARGON2_MIN_T,
            p: ARGON2_P,
        };
        let salt = [9u8; SALT_LEN];
        let key = derive_key(b"pw", &salt, params).unwrap();
        let (nonce, ct) = seal(&key, b"slot-A", b"seed").unwrap();
        let err = open(&key, &nonce, b"slot-B", &ct).unwrap_err();
        assert!(matches!(err, FileStoreError::Decrypt));
    }

    #[test]
    fn wrong_key_fails() {
        let params = KdfParams {
            m_kib: ARGON2_MIN_M_KIB,
            t: ARGON2_MIN_T,
            p: ARGON2_P,
        };
        let salt = [1u8; SALT_LEN];
        let k1 = derive_key(b"right", &salt, params).unwrap();
        let k2 = derive_key(b"wrong", &salt, params).unwrap();
        let (nonce, ct) = seal(&k1, b"aad", b"seed").unwrap();
        assert!(matches!(
            open(&k2, &nonce, b"aad", &ct),
            Err(FileStoreError::Decrypt)
        ));
    }

    #[test]
    fn nonces_are_unique_across_seals() {
        let params = KdfParams {
            m_kib: ARGON2_MIN_M_KIB,
            t: ARGON2_MIN_T,
            p: ARGON2_P,
        };
        let key = derive_key(b"pw", &[2u8; SALT_LEN], params).unwrap();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..256 {
            let (nonce, _) = seal(&key, b"aad", b"x").unwrap();
            assert!(seen.insert(nonce), "nonce reuse across put");
        }
    }
}
