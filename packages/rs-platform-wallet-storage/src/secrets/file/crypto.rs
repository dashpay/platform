//! Argon2id KDF + XChaCha20-Poly1305 AEAD.
//!
//! `pub(crate)` only — no crypto primitive escapes the `secrets` tree.

use argon2::{Algorithm, Argon2, Block, Params, Version};
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use getrandom::getrandom;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

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

/// Tier-2 envelope per-read ceiling — the strongest header
/// [`KdfParams::enforce_read_ceiling`] will derive under. **Wire-format
/// constants, not tunables**: a protected secret whose header this build
/// refuses is unrecoverable, so these may only ever be RAISED. The
/// `ARGON2_DEFAULT_*` write target is an ordinary tunable and must stay
/// at or below them.
pub(crate) const ARGON2_READ_MAX_M_KIB: u32 = 65_536;
pub(crate) const ARGON2_READ_MAX_T: u32 = 3;

/// The read ceiling must contain the write target, and both must sit
/// inside the [`KdfParams::enforce_bounds`] band. Violating the first
/// bricks reads in one of two directions, so it breaks the BUILD: unlike
/// a runtime guard there is no legitimate configuration in which it
/// fails. If this fires, RAISE the read ceiling — never delete it.
const _: () = {
    assert!(
        ARGON2_DEFAULT_M_KIB <= ARGON2_READ_MAX_M_KIB && ARGON2_DEFAULT_T <= ARGON2_READ_MAX_T,
        "Argon2 write target exceeds the Tier-2 read ceiling: raise ARGON2_READ_MAX_* to match, \
         or every freshly written envelope is refused by the build that wrote it"
    );
    assert!(
        ARGON2_READ_MAX_M_KIB >= ARGON2_MIN_M_KIB
            && ARGON2_READ_MAX_M_KIB <= ARGON2_MAX_M_KIB
            && ARGON2_READ_MAX_T >= ARGON2_MIN_T
            && ARGON2_READ_MAX_T <= ARGON2_MAX_T,
        "the Tier-2 read ceiling must sit inside the enforce_bounds band"
    );
};

/// CSPRNG salt width (≥16 required; we use 32).
pub(crate) const SALT_LEN: usize = 32;
/// XChaCha20-Poly1305 nonce width.
pub(crate) const NONCE_LEN: usize = 24;
/// Derived AEAD key width.
pub(crate) const KEY_LEN: usize = 32;

/// Fill `buf` with CSPRNG bytes (`OsRng` via `getrandom`). Backs the salt,
/// nonce, and key-material draws, so a failure is reported as
/// [`SecretStoreError::EntropyUnavailable`] — never `KdfFailure`, which
/// would misname the failing subsystem on the nonce/salt paths.
pub(crate) fn random_bytes(buf: &mut [u8]) -> Result<(), SecretStoreError> {
    getrandom(buf).map_err(|_| SecretStoreError::EntropyUnavailable)
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

    /// Tier-2 envelope read gate, tighter than [`enforce_bounds`]: bounds a
    /// forged header at the shipped cost instead of the 1 GiB / 16-pass DoS
    /// band. Deliberately asymmetric with the FILE VAULT header, which
    /// `file::derive_and_verify` accepts across the whole band — a vault is a
    /// local artefact its owner may harden at will, whereas an envelope's
    /// cost is paid on every read by whoever holds the object password.
    ///
    /// Gated on the wire-stable `ARGON2_READ_MAX_*` rather than
    /// `default_target()` so lowering the shipped write target can never
    /// orphan an already-enrolled secret.
    ///
    /// [`enforce_bounds`]: KdfParams::enforce_bounds
    pub(crate) fn enforce_read_ceiling(&self) -> Result<(), SecretStoreError> {
        if self.m_kib > ARGON2_READ_MAX_M_KIB || self.t > ARGON2_READ_MAX_T {
            return Err(SecretStoreError::KdfFailure);
        }
        Ok(())
    }

    /// The componentwise-stronger of two parameter sets; `id`/`p` are fixed
    /// crate-wide, so only `m_kib`/`t` vary. Lets a rekey carry a hardened
    /// vault header forward instead of overwriting it with the handle's own
    /// target, while a raised default still upgrades an old vault.
    pub(crate) fn max_strength(self, other: Self) -> Self {
        Self {
            m_kib: self.m_kib.max(other.m_kib),
            t: self.t.max(other.t),
            ..self
        }
    }
}

/// Caller-owned Argon2 working memory, wiped before it is released.
///
/// The block matrix is key-equivalent — its seed blocks derive from
/// `H0` and the last pass's final block is hashed straight into the
/// output tag — but argon2 0.5.3's `zeroize` feature covers only
/// `initial_hash`/`blockhash`. [`Argon2::hash_password_into`] allocates
/// the matrix internally and drops it intact, so this crate owns it
/// instead and passes it to `hash_password_into_with_memory`.
struct ScopedBlocks(Vec<Block>);

impl ScopedBlocks {
    fn new(block_count: usize) -> Self {
        Self(vec![Block::default(); block_count])
    }

    /// Zeroize every block IN PLACE, keeping the vector's length — a
    /// length-clearing `Vec::zeroize` would make the wipe unobservable to
    /// `argon2_block_matrix_is_wiped_before_release`, and an unverifiable
    /// security fix is not one.
    fn wipe(&mut self) {
        for block in &mut self.0 {
            block.zeroize();
        }
    }
}

impl AsMut<[Block]> for ScopedBlocks {
    fn as_mut(&mut self) -> &mut [Block] {
        &mut self.0
    }
}

impl Drop for ScopedBlocks {
    fn drop(&mut self) {
        self.wipe();
    }
}

/// Derive a 32-byte AEAD key from `passphrase` + `salt` with Argon2id,
/// landing directly in a [`SecretBytes`]. Takes `&SecretString` so the
/// bare-byte passphrase view lives only inside this function.
///
/// The Argon2 block matrix is caller-owned ([`ScopedBlocks`]), so it is
/// wiped on every exit including the error path. Residual against A5
/// (swap / core-dump while unlocked): that matrix is ordinary heap, not
/// `mlock`ed — a guarded allocation of up to `m_kib` does not fit the
/// locked-memory budget in `secrets/file/mod.rs`. Accepted deliberately.
pub(crate) fn derive_key(
    passphrase: &SecretString,
    salt: &[u8; SALT_LEN],
    params: KdfParams,
) -> Result<SecretBytes, SecretStoreError> {
    // Bounds MUST gate first so an inflated m_kib never reaches the allocator.
    params.enforce_bounds()?;
    let argon_params = Params::new(params.m_kib, params.t, params.p, Some(KEY_LEN))
        .map_err(|_| SecretStoreError::KdfFailure)?;
    let block_count = argon_params.block_count();
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);
    let mut key = SecretBytes::zeroed(KEY_LEN);
    let mut blocks = ScopedBlocks::new(block_count);
    argon
        .hash_password_into_with_memory(
            passphrase.expose_secret().as_bytes(),
            salt,
            key.expose_secret_mut(),
            &mut blocks,
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

    /// **Vault-opening invariant — do not "fix" by updating an expected
    /// value.** Caller-owned working memory changed only WHERE the Argon2
    /// matrix lives, never what it derives. Should this ever diverge from
    /// argon2's own `hash_password_into`, every existing vault and every
    /// enrolled Tier-2 secret stops opening, reported as a wrong
    /// passphrase, with no recovery path.
    #[test]
    fn derive_key_matches_upstream_reference_derivation() {
        const PW: &str = "correct horse battery";
        let salt = [0x5Au8; SALT_LEN];
        let params = KdfParams::floor_target();
        let derived = derive_key(&SecretString::new(PW), &salt, params).unwrap();

        let argon_params = Params::new(params.m_kib, params.t, params.p, Some(KEY_LEN)).unwrap();
        let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);
        let mut reference = [0u8; KEY_LEN];
        argon
            .hash_password_into(PW.as_bytes(), &salt, &mut reference)
            .unwrap();

        assert_eq!(
            derived.expose_secret(),
            &reference[..],
            "caller-owned Argon2 memory changed the derived key"
        );
        reference.zeroize();
    }

    /// `max_strength` ratchets each axis independently: a handle's target
    /// can raise a weak header, never lower a hardened one.
    #[test]
    fn max_strength_ratchets_each_axis_independently() {
        let floor = KdfParams::floor_target();
        let target = KdfParams::default_target();
        assert_eq!(floor.max_strength(target), target);
        assert_eq!(target.max_strength(floor), target);

        let wide = KdfParams {
            m_kib: ARGON2_MAX_M_KIB,
            ..floor
        };
        let slow = KdfParams {
            t: ARGON2_MAX_T,
            ..floor
        };
        assert_eq!(
            wide.max_strength(slow),
            KdfParams {
                m_kib: ARGON2_MAX_M_KIB,
                t: ARGON2_MAX_T,
                ..floor
            }
        );
    }

    /// The Tier-2 read ceiling is its own wire-format bound, not a mirror
    /// of the shipped write target: exactly-at-ceiling derives, one step
    /// over on either axis is refused.
    #[test]
    fn read_ceiling_accepts_its_bound_and_refuses_above_it() {
        let at_ceiling = KdfParams {
            m_kib: ARGON2_READ_MAX_M_KIB,
            t: ARGON2_READ_MAX_T,
            ..KdfParams::default_target()
        };
        assert!(at_ceiling.enforce_read_ceiling().is_ok());
        assert!(matches!(
            KdfParams {
                m_kib: ARGON2_READ_MAX_M_KIB + 1,
                ..at_ceiling
            }
            .enforce_read_ceiling(),
            Err(SecretStoreError::KdfFailure)
        ));
        assert!(matches!(
            KdfParams {
                t: ARGON2_READ_MAX_T + 1,
                ..at_ceiling
            }
            .enforce_read_ceiling(),
            Err(SecretStoreError::KdfFailure)
        ));
        // No build may ship a write target its own read path refuses; the
        // const assert enforces it, this pins the behaviour.
        assert!(KdfParams::default_target().enforce_read_ceiling().is_ok());
        assert!(KdfParams::floor_target().enforce_read_ceiling().is_ok());
    }

    /// The block matrix is key-equivalent state, so `ScopedBlocks` must
    /// leave none of it behind. Filled through argon2's public
    /// `fill_memory`, so the wipe is proven against REAL derived material
    /// rather than a synthetic pattern.
    #[test]
    fn argon2_block_matrix_is_wiped_before_release() {
        // Deliberately tiny (8 KiB): this exercises ScopedBlocks, not the
        // production cost parameters.
        let params = Params::new(8, 1, 1, Some(KEY_LEN)).unwrap();
        let block_count = params.block_count();
        let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut blocks = ScopedBlocks::new(block_count);
        argon
            .fill_memory(b"passphrase", &[7u8; SALT_LEN], &mut blocks)
            .unwrap();

        let nonzero_words = |b: &[Block]| {
            b.iter()
                .flat_map(|block| {
                    let words: &[u64] = block.as_ref();
                    words.iter()
                })
                .filter(|w| **w != 0)
                .count()
        };
        assert!(
            nonzero_words(blocks.as_mut()) > 0,
            "fixture must hold real derived state before the wipe"
        );

        blocks.wipe();

        assert_eq!(
            nonzero_words(blocks.as_mut()),
            0,
            "Argon2 working memory survived the wipe"
        );
    }

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
