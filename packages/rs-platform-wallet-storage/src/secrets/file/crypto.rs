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
/// controllable JSON, so without a cap an oversized `m_kib`/`t` could
/// force a multi-GiB allocation or unbounded derivation (DoS) before any
/// tag check. 1 GiB / 16 passes is well above the default, far below
/// exhaustion.
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

/// Argon2id parameters stored in the on-disk `kdf` object. `id`
/// discriminates the algorithm (only [`KDF_ID_ARGON2ID`] today),
/// validated with the parameter ranges in [`KdfParams::enforce_bounds`].
/// `deny_unknown_fields` fails closed on a stray sibling.
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

    /// The fastest configuration [`enforce_bounds`] still accepts — the
    /// enforced floor itself, and the ONE definition of "fastest legal
    /// Argon2id params" in this crate. Every test call site and
    /// [`SecretStore::file_mock`] derive at it, so a suite does not pay the
    /// 64 MiB target per call (#4111).
    ///
    /// # Panics
    ///
    /// Panics unless the build has `debug_assertions` on, or is this crate's
    /// own test harness. This is the single choke point for weak-but-legal
    /// params, so guarding it here covers EVERY caller uniformly — the mock
    /// constructors inherit it rather than repeating the check.
    ///
    /// `cfg!(debug_assertions)` is a runtime *value*, not a `debug_assert!`:
    /// it is evaluated in every profile, so the check is present precisely in
    /// the optimized build it defends. If `test-util` ever reaches a release
    /// build (feature unification), any path to floor params stops loudly
    /// instead of silently yielding weak crypto. A `const {}` assert would
    /// instead break the BUILD, taking `--release --all-features` down with
    /// it; the refusal belongs at the call, not at every consumer's compile.
    ///
    /// [`enforce_bounds`]: KdfParams::enforce_bounds
    /// [`SecretStore::file_mock`]: crate::secrets::SecretStore::file_mock
    #[cfg(any(test, feature = "test-util"))]
    #[expect(
        clippy::assertions_on_constants,
        reason = "build-configuration guard: folds to `panic!` iff test-util reached a release build"
    )]
    pub(crate) fn floor_target() -> Self {
        assert!(
            cfg!(debug_assertions) || cfg!(test),
            "KdfParams::floor_target is the Argon2id FLOOR and is test-only, but this build \
             has debug_assertions off — the `test-util` feature reached a release build \
             (likely via feature unification). Refusing to hand back weak-crypto params."
        );
        Self {
            id: KDF_ID_ARGON2ID,
            m_kib: ARGON2_MIN_M_KIB,
            t: ARGON2_MIN_T,
            p: ARGON2_P,
        }
    }

    /// Reject out-of-bounds params before any derivation/allocation: the
    /// lower bound refuses a downgraded vault, the upper bound an inflated
    /// one (huge allocation / unbounded derivation ahead of any tag
    /// check). An unknown algorithm `id` also fails — Argon2id is the only
    /// supported family.
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

/// Derive a 32-byte AEAD key from `passphrase` + `salt` with Argon2id,
/// landing directly in a [`SecretBytes`]. Takes `&SecretString` so the
/// bare-byte passphrase view lives only inside this function.
///
/// Zeroization residual: argon2 0.5.3's `zeroize` feature wipes
/// `initial_hash` / `blockhash` but NOT the bulk `Block` matrix (up to
/// `m_kib` of derived state). Accepted residual against A5 (swap /
/// core-dump while unlocked); closing it needs an upstream fix.
pub(crate) fn derive_key(
    passphrase: &SecretString,
    salt: &[u8; SALT_LEN],
    params: KdfParams,
) -> Result<SecretBytes, SecretStoreError> {
    // Bounds MUST gate first so an inflated m_kib never reaches the allocator.
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
        .map_err(|_| SecretStoreError::Encrypt)?;
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
        // AEAD write-side failure (only when plaintext exceeds the length
        // limit), not a key-derivation one.
        .map_err(|_| SecretStoreError::Encrypt)?;
    Ok((nonce_bytes, ct))
}

/// Like [`seal`] but takes a caller-supplied `nonce` instead of pulling
/// from the CSPRNG. **Test-only** — golden-vector / size-budget tests
/// need byte-deterministic ciphertext output. Production code MUST use
/// [`seal`] so nonces stay unique (XChaCha20-Poly1305 nonce reuse leaks
/// the keystream).
#[cfg(test)]
pub(crate) fn seal_with_nonce(
    key: &SecretBytes,
    nonce_bytes: [u8; NONCE_LEN],
    aad: &[u8],
    plaintext: &[u8],
) -> Result<([u8; NONCE_LEN], Vec<u8>), SecretStoreError> {
    let cipher = XChaCha20Poly1305::new_from_slice(key.expose_secret())
        .map_err(|_| SecretStoreError::Encrypt)?;
    let nonce = XNonce::from_slice(&nonce_bytes);
    let ct = cipher
        .encrypt(
            nonce,
            chacha20poly1305::aead::Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| SecretStoreError::Encrypt)?;
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
        .map_err(|_| SecretStoreError::Encrypt)?;
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

    // Compile-time guard: argon2's `impl Zeroize for Block` is feature-
    // gated, so this fails to build if `argon2/zeroize` is ever dropped.
    static_assertions::assert_impl_all!(argon2::Block: zeroize::Zeroize);

    #[test]
    fn floors_reject_weak_params() {
        let base = KdfParams::floor_target();
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
        let base = KdfParams::floor_target();
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
            ..KdfParams::floor_target()
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
        // u32::MAX m_kib must error via enforce_bounds before the ~4 TiB
        // allocation — which would OOM the test if it ever ran.
        let err = derive_key(
            &SecretString::new("pw"),
            &[0u8; SALT_LEN],
            KdfParams {
                m_kib: u32::MAX,
                ..KdfParams::floor_target()
            },
        )
        .unwrap_err();
        assert!(matches!(err, SecretStoreError::KdfFailure));
    }

    #[test]
    fn seal_open_roundtrip_with_floor_params() {
        let mut salt = [0u8; SALT_LEN];
        random_bytes(&mut salt).unwrap();
        let key = derive_key(
            &SecretString::new("correct horse"),
            &salt,
            KdfParams::floor_target(),
        )
        .unwrap();
        let aad = b"v1|wallet|label";
        let (nonce, ct) = seal(&key, aad, b"top secret seed").unwrap();
        let pt = open(&key, &nonce, aad, &ct).unwrap();
        assert_eq!(pt.expose_secret(), b"top secret seed");
    }

    #[test]
    fn wrong_aad_fails_with_no_plaintext() {
        let key = derive_key(
            &SecretString::new("pw"),
            &[9u8; SALT_LEN],
            KdfParams::floor_target(),
        )
        .unwrap();
        let (nonce, ct) = seal(&key, b"slot-A", b"seed").unwrap();
        let err = open(&key, &nonce, b"slot-B", &ct).unwrap_err();
        assert!(matches!(err, SecretStoreError::Decrypt));
    }

    #[test]
    fn wrong_key_fails() {
        let salt = [1u8; SALT_LEN];
        let k1 = derive_key(
            &SecretString::new("right"),
            &salt,
            KdfParams::floor_target(),
        )
        .unwrap();
        let k2 = derive_key(
            &SecretString::new("wrong"),
            &salt,
            KdfParams::floor_target(),
        )
        .unwrap();
        let (nonce, ct) = seal(&k1, b"aad", b"seed").unwrap();
        assert!(matches!(
            open(&k2, &nonce, b"aad", &ct),
            Err(SecretStoreError::Decrypt)
        ));
    }

    #[test]
    fn nonces_are_unique_across_seals() {
        let key = derive_key(
            &SecretString::new("pw"),
            &[2u8; SALT_LEN],
            KdfParams::floor_target(),
        )
        .unwrap();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..256 {
            let (nonce, _) = seal(&key, b"aad", b"x").unwrap();
            assert!(seen.insert(nonce), "nonce reuse across put");
        }
    }
}
