//! Argon2id KDF + XChaCha20-Poly1305 AEAD.
//!
//! `pub(crate)` only — no crypto primitive escapes the `secrets` tree.

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use getrandom::getrandom;
use serde::{Deserialize, Serialize};

use super::super::secret::{SecretBytes, SecretString};
use super::format::KDF_ID_ARGON2ID;
use crate::secrets::error::SecretStoreError;

/// Argon2 parameter floors — derivation MUST NOT use anything weaker; a
/// header declaring less is refused.
pub(crate) const ARGON2_MIN_M_KIB: u32 = 19_456;
pub(crate) const ARGON2_MIN_T: u32 = 2;
pub(crate) const ARGON2_P: u32 = 1;

/// Argon2 parameter ceilings. Vault `kdf` params are attacker-
/// controllable JSON, so an oversized `m_kib`/`t` would let a crafted
/// vault force a multi-GiB allocation or an unbounded-time derivation (a
/// DoS) before any tag check. 1 GiB memory and 16 passes bound the cost
/// well above the shipped default (64 MiB, t=3) yet far below an
/// exhaustion threshold.
pub(crate) const ARGON2_MAX_M_KIB: u32 = 1_048_576;
pub(crate) const ARGON2_MAX_T: u32 = 16;

/// Shipped defaults for new vaults (64 MiB, t≥3).
pub(crate) const ARGON2_DEFAULT_M_KIB: u32 = 65_536;
pub(crate) const ARGON2_DEFAULT_T: u32 = 3;

/// CSPRNG salt width (≥16 required; we use 32).
pub(crate) const SALT_LEN: usize = 32;
/// XChaCha20-Poly1305 nonce width.
pub(crate) const NONCE_LEN: usize = 24;
/// Derived AEAD key width.
pub(crate) const KEY_LEN: usize = 32;

/// Fill `buf` with CSPRNG bytes (`OsRng` via `getrandom`).
pub(crate) fn random_bytes(buf: &mut [u8]) -> Result<(), SecretStoreError> {
    getrandom(buf).map_err(|_| SecretStoreError::KdfFailure)
}

/// Argon2id parameters as stored in / read from the vault. Serializes
/// directly to the on-disk `kdf` object — `id` discriminates the KDF
/// algorithm (only [`KDF_ID_ARGON2ID`] is accepted today), validated
/// alongside the parameter ranges in [`KdfParams::enforce_bounds`].
/// `deny_unknown_fields` fails closed on a stray sibling (C3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct KdfParams {
    pub id: u8,
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

impl KdfParams {
    /// The shipped default for new vaults.
    pub(crate) fn default_target() -> Self {
        Self {
            id: KDF_ID_ARGON2ID,
            m_kib: ARGON2_DEFAULT_M_KIB,
            t: ARGON2_DEFAULT_T,
            p: ARGON2_P,
        }
    }

    /// Reject params outside the accepted bounds before any derivation
    /// or allocation runs. The lower bound refuses a downgraded vault;
    /// the upper bound refuses an inflated vault from an
    /// attacker-controllable JSON file that would otherwise force a
    /// huge allocation / unbounded derivation ahead of any tag check.
    /// An unknown algorithm `id` is also a bounds failure — Argon2id is
    /// the only KDF family this version supports.
    pub(crate) fn enforce_bounds(&self) -> Result<(), SecretStoreError> {
        if self.id != KDF_ID_ARGON2ID
            || self.m_kib < ARGON2_MIN_M_KIB
            || self.t < ARGON2_MIN_T
            || self.p != ARGON2_P
            || self.m_kib > ARGON2_MAX_M_KIB
            || self.t > ARGON2_MAX_T
        {
            return Err(SecretStoreError::KdfFailure);
        }
        Ok(())
    }
}

/// Derive a 32-byte AEAD key from `passphrase` + `salt` with Argon2id.
/// Output lands directly in a [`SecretBytes`].
///
/// Takes `&SecretString` directly so the bare-byte view of the
/// passphrase lives only inside this function — callers can no
/// longer accidentally hand a `&[u8]` (e.g. by holding a stray
/// `expose_secret().as_bytes()` longer than intended) into KDF input.
pub(crate) fn derive_key(
    passphrase: &SecretString,
    salt: &[u8],
    params: KdfParams,
) -> Result<SecretBytes, SecretStoreError> {
    // Bounds MUST gate before Params::new / hash_password_into so an
    // inflated m_kib never reaches the allocator.
    params.enforce_bounds()?;
    let argon_params = Params::new(params.m_kib, params.t, params.p, Some(KEY_LEN))
        .map_err(|_| SecretStoreError::KdfFailure)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);
    let mut key = SecretBytes::zeroed(KEY_LEN);
    argon
        .hash_password_into(
            passphrase.expose_secret().as_bytes(),
            salt,
            key.expose_secret_mut(),
        )
        .map_err(|_| SecretStoreError::KdfFailure)?;
    Ok(key)
}

/// Encrypt `plaintext` under `key` with a fresh random nonce, binding
/// `aad`. Returns `(nonce, ciphertext_with_tag)`.
pub(crate) fn seal(
    key: &SecretBytes,
    aad: &[u8],
    plaintext: &[u8],
) -> Result<([u8; NONCE_LEN], Vec<u8>), SecretStoreError> {
    let cipher = XChaCha20Poly1305::new_from_slice(key.expose_secret())
        .map_err(|_| SecretStoreError::KdfFailure)?;
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
        // Encrypt-path failure (XChaCha20-Poly1305 only fails here when
        // the plaintext exceeds the construction's length limit), so it is
        // not a decryption concern; keep it on the same write-oriented
        // variant the cipher-construction failure above uses.
        .map_err(|_| SecretStoreError::KdfFailure)?;
    Ok((nonce_bytes, ct))
}

/// Decrypt `ciphertext` under `key`/`nonce`/`aad`. On tag failure
/// returns [`SecretStoreError::Decrypt`] and **no** plaintext — the
/// combined (non-detached) API never materializes unverified bytes at
/// our boundary (CWE-347, RUSTSEC-2023-0096).
pub(crate) fn open(
    key: &SecretBytes,
    nonce: &[u8; NONCE_LEN],
    aad: &[u8],
    ciphertext: &[u8],
) -> Result<SecretBytes, SecretStoreError> {
    let cipher = XChaCha20Poly1305::new_from_slice(key.expose_secret())
        .map_err(|_| SecretStoreError::KdfFailure)?;
    let nonce = XNonce::from_slice(nonce);
    let pt = cipher
        .decrypt(
            nonce,
            chacha20poly1305::aead::Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| SecretStoreError::Decrypt)?;
    Ok(SecretBytes::new(pt))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Argon2id floor params — fast enough for unit tests; production
    /// runs at the default target (64 MiB).
    fn floor_params() -> KdfParams {
        KdfParams {
            id: KDF_ID_ARGON2ID,
            m_kib: ARGON2_MIN_M_KIB,
            t: ARGON2_MIN_T,
            p: ARGON2_P,
        }
    }

    #[test]
    fn floors_reject_weak_params() {
        let base = floor_params();
        assert!(KdfParams {
            m_kib: 1024,
            ..base
        }
        .enforce_bounds()
        .is_err());
        assert!(KdfParams { t: 1, ..base }.enforce_bounds().is_err());
        assert!(KdfParams { p: 2, ..base }.enforce_bounds().is_err());
        assert!(KdfParams::default_target().enforce_bounds().is_ok());
    }

    #[test]
    fn ceilings_reject_inflated_params() {
        // An attacker-controllable JSON kdf cannot force a huge
        // allocation or unbounded derivation.
        let base = floor_params();
        assert!(KdfParams {
            m_kib: u32::MAX,
            ..base
        }
        .enforce_bounds()
        .is_err());
        assert!(KdfParams {
            m_kib: ARGON2_MAX_M_KIB + 1,
            ..base
        }
        .enforce_bounds()
        .is_err());
        assert!(KdfParams {
            t: ARGON2_MAX_T + 1,
            ..base
        }
        .enforce_bounds()
        .is_err());
        // The exact ceilings are accepted.
        assert!(KdfParams {
            m_kib: ARGON2_MAX_M_KIB,
            t: ARGON2_MAX_T,
            ..base
        }
        .enforce_bounds()
        .is_ok());
    }

    #[test]
    fn unknown_kdf_id_is_rejected_at_bounds_check() {
        // Defence-in-depth: even with floor-valid m_kib/t/p, an unknown
        // algorithm id is refused before any derivation runs.
        let bad = KdfParams {
            id: 7,
            ..floor_params()
        };
        assert!(matches!(
            bad.enforce_bounds(),
            Err(SecretStoreError::KdfFailure)
        ));
        assert!(matches!(
            derive_key(&SecretString::new("pw"), &[0u8; SALT_LEN], bad),
            Err(SecretStoreError::KdfFailure)
        ));
    }

    #[test]
    fn derive_key_rejects_inflated_m_kib_before_allocating() {
        // u32::MAX m_kib must error fast (enforce_bounds) and never reach
        // the multi-GiB allocator. A real allocation of ~4 TiB would OOM
        // the test, so reaching here at all proves the ceiling fired
        // first.
        let err = derive_key(
            &SecretString::new("pw"),
            &[0u8; SALT_LEN],
            KdfParams {
                m_kib: u32::MAX,
                ..floor_params()
            },
        )
        .unwrap_err();
        assert!(matches!(err, SecretStoreError::KdfFailure));
    }

    #[test]
    fn seal_open_roundtrip_with_floor_params() {
        let mut salt = [0u8; SALT_LEN];
        random_bytes(&mut salt).unwrap();
        let key = derive_key(&SecretString::new("correct horse"), &salt, floor_params()).unwrap();
        let aad = b"v1|wallet|label";
        let (nonce, ct) = seal(&key, aad, b"top secret seed").unwrap();
        let pt = open(&key, &nonce, aad, &ct).unwrap();
        assert_eq!(pt.expose_secret(), b"top secret seed");
    }

    #[test]
    fn wrong_aad_fails_with_no_plaintext() {
        let key = derive_key(&SecretString::new("pw"), &[9u8; SALT_LEN], floor_params()).unwrap();
        let (nonce, ct) = seal(&key, b"slot-A", b"seed").unwrap();
        let err = open(&key, &nonce, b"slot-B", &ct).unwrap_err();
        assert!(matches!(err, SecretStoreError::Decrypt));
    }

    #[test]
    fn wrong_key_fails() {
        let salt = [1u8; SALT_LEN];
        let k1 = derive_key(&SecretString::new("right"), &salt, floor_params()).unwrap();
        let k2 = derive_key(&SecretString::new("wrong"), &salt, floor_params()).unwrap();
        let (nonce, ct) = seal(&k1, b"aad", b"seed").unwrap();
        assert!(matches!(
            open(&k2, &nonce, b"aad", &ct),
            Err(SecretStoreError::Decrypt)
        ));
    }

    #[test]
    fn nonces_are_unique_across_seals() {
        let key = derive_key(&SecretString::new("pw"), &[2u8; SALT_LEN], floor_params()).unwrap();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..256 {
            let (nonce, _) = seal(&key, b"aad", b"x").unwrap();
            assert!(seen.insert(nonce), "nonce reuse across put");
        }
    }
}
