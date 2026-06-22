//! Tier-2 opt-in per-object password envelope (backend-independent).
//!
//! Sits ABOVE [`SecretStore`](crate::secrets::SecretStore), over both the
//! `File` vault and `Os` keyring arms: the backend stores opaque bytes,
//! and a chosen critical object (a seed wallet, a single privkey) can be
//! wrapped under an extra, user-supplied **object password** before it
//! ever reaches the backend. Reading a protected object then needs BOTH
//! backend access AND the password — the first control that survives a
//! full backend compromise (the keychain scraped, the vault stolen and its
//! passphrase cracked).
//!
//! # Wire format (self-describing, authenticated)
//!
//! ```text
//! magic   b"PWSEV"   (5)
//! version u8 = 1      (ENVELOPE_VERSION — independent of the vault FORMAT_VERSION)
//! scheme  u8          (0 = unprotected passthrough, 1 = argon2id-xchacha password)
//! ── scheme 0 ──  payload: raw secret bytes
//! ── scheme 1 ──  kdf(id u8 ‖ m_kib u32 LE ‖ t u32 LE ‖ p u32 LE)  (13)
//!                 ‖ salt[32] ‖ nonce[24] ‖ ciphertext+tag
//! ```
//!
//! The header proves what the blob **is**, never what the caller
//! **expected** — that expectation lives solely in the caller's `Some/None`
//! password argument (see [`unwrap`]'s strict, fail-closed table). The
//! self-description is a convenience for `NeedsPassword`/`WrongPassword`/
//! version UX, **not** the security boundary.
//!
//! ## Reused, never reinvented
//! - KDF: [`crypto::derive_key`] (Argon2id) with a fresh 32-byte salt; the
//!   param **ceiling is enforced BEFORE derivation** on the
//!   attacker-controllable header ([`KdfParams::enforce_bounds`]).
//! - AEAD: [`crypto::seal`]/[`crypto::open`] (XChaCha20-Poly1305), fresh
//!   per-wrap nonce; a tag failure maps to
//!   [`SecretStoreError::WrongPassword`] with no plaintext.
//! - AAD binds `domain ‖ magic ‖ version ‖ scheme ‖ kdf ‖ salt ‖ wallet_id
//!   ‖ label`, mirroring [`format::aad`]/[`format::verify_aad`] so a
//!   relocated/confused blob fails the tag.
//!
//! No bespoke crypto.
//!
//! [`format::aad`]: super::file::format::aad
//! [`format::verify_aad`]: super::file::format::verify_aad

use std::sync::Once;

use super::error::SecretStoreError;
use super::file::crypto::{self, KdfParams, NONCE_LEN, SALT_LEN};
use super::secret::{SecretBytes, SecretString};
use super::validate::WalletId;
use super::MAX_SECRET_LEN;

/// 5-byte sentinel marking a Tier-2 envelope. A decrypted entry NOT
/// starting with this is a legacy magic-less raw value (see [`unwrap`]).
pub(crate) const MAGIC: &[u8; 5] = b"PWSEV";

/// Envelope wire version — bumped only on a breaking layout change, and
/// independent of the vault `FORMAT_VERSION` (the envelope rides inside the
/// entry bytes, identical over File/Os).
pub(crate) const ENVELOPE_VERSION: u8 = 1;

/// Scheme 0: unprotected passthrough — payload is the raw secret.
pub(crate) const SCHEME_UNPROTECTED: u8 = 0;
/// Scheme 1: Argon2id + XChaCha20-Poly1305 under an object password.
pub(crate) const SCHEME_PASSWORD: u8 = 1;

/// Domain-separation tag leading the scheme-1 AAD, so a Tier-2 tag can
/// never be confused with the vault's own verify/entry AAD.
const TIER2_DOMAIN: &[u8] = b"PWSEV-TIER2-AAD-v1";

/// Fixed header: `magic ‖ version ‖ scheme`.
const HEADER_LEN: usize = MAGIC.len() + 2;
/// Encoded KDF-params field: `id u8 ‖ m_kib u32 ‖ t u32 ‖ p u32`.
const KDF_FIELD_LEN: usize = 1 + 4 + 4 + 4;
/// Poly1305 tag length — present even for empty plaintext.
const AEAD_TAG_LEN: usize = 16;
/// Smallest valid scheme-1 body (kdf ‖ salt ‖ nonce ‖ bare tag).
const MIN_SCHEME1_BODY: usize = KDF_FIELD_LEN + SALT_LEN + NONCE_LEN + AEAD_TAG_LEN;

/// Fixed, bounded envelope overhead (`magic 5 + version 1 + scheme 1 + kdf
/// 13 + salt 32 + nonce 24 + tag 16 = 92`), rounded up to 128 for headroom
/// (future header fields / versions). Used to derive the plaintext cap.
pub(crate) const MAX_ENVELOPE_OVERHEAD: usize = 128;

/// Plaintext cap at the envelope boundary: `MAX_SECRET_LEN −
/// MAX_ENVELOPE_OVERHEAD`. Capping the **plaintext** (uniformly for both
/// schemes) keeps the user-visible limit stable AND guarantees the
/// enveloped bytes always fit the backend vault's own `MAX_SECRET_LEN`
/// `put_bytes` cap. Re-exported at
/// [`crate::secrets`] as the documented, stable user-facing cap.
pub const MAX_PLAINTEXT_LEN: usize = MAX_SECRET_LEN - MAX_ENVELOPE_OVERHEAD;

/// Wrap `plaintext` for `(wallet_id, label)` using the shipped default
/// Argon2 target (64 MiB / t=3) when a password is supplied.
///
/// `None` → an unprotected (scheme-0) envelope; `Some(pw)` → a scheme-1
/// envelope sealed under `pw`. A blank password is rejected at enrol
/// ([`SecretStoreError::BlankPassphrase`]).
///
/// Returns the envelope inside a zeroizing [`SecretBytes`]: a scheme-0
/// envelope embeds the raw plaintext, so the wire bytes are handled as
/// sensitive (mlock'd, wiped on drop) by construction — symmetric with
/// [`unwrap`]'s return.
pub(crate) fn wrap(
    wallet_id: &WalletId,
    label: &str,
    password: Option<&SecretString>,
    plaintext: &[u8],
) -> Result<SecretBytes, SecretStoreError> {
    wrap_with_params(
        wallet_id,
        label,
        password,
        plaintext,
        KdfParams::default_target(),
    )
}

/// [`wrap`] with explicit Argon2 `params` (tests use the floor params for
/// speed; production uses [`KdfParams::default_target`]). `params` is
/// ignored when `password` is `None`.
pub(crate) fn wrap_with_params(
    wallet_id: &WalletId,
    label: &str,
    password: Option<&SecretString>,
    plaintext: &[u8],
    params: KdfParams,
) -> Result<SecretBytes, SecretStoreError> {
    // Cap the PLAINTEXT (before overhead) uniformly for both schemes so the
    // enveloped bytes always fit the backend cap and the limit is stable.
    if plaintext.len() > MAX_PLAINTEXT_LEN {
        return Err(SecretStoreError::SecretTooLarge {
            found: plaintext.len(),
            max: MAX_PLAINTEXT_LEN,
        });
    }

    let Some(pw) = password else {
        // Scheme 0: magic ‖ version ‖ scheme ‖ raw payload.
        let mut out = Vec::with_capacity(HEADER_LEN + plaintext.len());
        out.extend_from_slice(MAGIC);
        out.push(ENVELOPE_VERSION);
        out.push(SCHEME_UNPROTECTED);
        out.extend_from_slice(plaintext);
        // `SecretBytes::new` moves `out` into a zeroizing, mlock'd buffer
        // (no copy) — the scheme-0 plaintext never lives in a bare Vec.
        return Ok(SecretBytes::new(out));
    };

    // Reject a blank object password BEFORE any derivation.
    if pw.is_blank() {
        return Err(SecretStoreError::BlankPassphrase);
    }

    // Fresh per-object salt so the same password on two objects yields
    // different keys and precomputation is defeated.
    let mut salt = [0u8; SALT_LEN];
    crypto::random_bytes(&mut salt)?;
    // `derive_key` enforces the param bounds before allocating.
    let key = crypto::derive_key(pw, &salt, params)?;
    let aad = scheme1_aad(&params, &salt, wallet_id.as_bytes(), label);
    let (nonce, ciphertext) = crypto::seal(&key, &aad, plaintext)?;

    let mut out =
        Vec::with_capacity(HEADER_LEN + KDF_FIELD_LEN + SALT_LEN + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.push(ENVELOPE_VERSION);
    out.push(SCHEME_PASSWORD);
    out.extend_from_slice(&encode_kdf(&params));
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(SecretBytes::new(out))
}

/// Unwrap `blob` for `(wallet_id, label)`, applying the **strict,
/// fail-closed** read. The "expected-protected" bit is
/// the caller's assertion, surfaced solely by `password`, and is NEVER
/// inferred from the blob's scheme byte.
///
/// | `password` | stored blob | result |
/// |---|---|---|
/// | `Some(pw)` | valid scheme-1 | secret, or [`WrongPassword`] on tag fail |
/// | `Some(pw)` | scheme-0 **or** magic-less (legacy raw) | [`ExpectedProtectedButUnsealed`] |
/// | `Some(pw)` | scheme-1 but too short | [`Corruption`] (sealed-but-broken) |
/// | `Some/None` | magic present, unknown version/scheme | [`UnsupportedEnvelopeVersion`] |
/// | `None` | valid scheme-1 | [`NeedsPassword`] (never ciphertext) |
/// | `None` | scheme-0 | secret |
/// | `None` | magic-less (legacy raw) | secret (+ one-time warn; re-wrapped on next write) |
/// | `None` | magic present but truncated header | [`Corruption`] |
///
/// The load-bearing row is `Some(pw)` + non-envelope ⇒
/// [`ExpectedProtectedButUnsealed`]: with a password in hand, a
/// non-protected blob can only mean a strip → refuse, return no bytes.
///
/// [`WrongPassword`]: SecretStoreError::WrongPassword
/// [`ExpectedProtectedButUnsealed`]: SecretStoreError::ExpectedProtectedButUnsealed
/// [`Corruption`]: SecretStoreError::Corruption
/// [`UnsupportedEnvelopeVersion`]: SecretStoreError::UnsupportedEnvelopeVersion
/// [`NeedsPassword`]: SecretStoreError::NeedsPassword
pub(crate) fn unwrap(
    wallet_id: &WalletId,
    label: &str,
    password: Option<&SecretString>,
    blob: &[u8],
) -> Result<SecretBytes, SecretStoreError> {
    // Magic-less ⇒ a legacy unprotected raw value (scheme-0-equivalent),
    // (legacy-tolerant read-path: a None read returns it, a Some(pw) read refuses).
    if !blob.starts_with(MAGIC) {
        return match password {
            None => {
                warn_legacy_once();
                Ok(SecretBytes::from_slice(blob))
            }
            // Caller asserted protection but found a magic-less raw value:
            // a strip/downgrade ⇒ FAIL CLOSED. Never returns bytes.
            Some(_) => Err(SecretStoreError::ExpectedProtectedButUnsealed),
        };
    }

    // Magic present but truncated before version+scheme: a broken envelope.
    if blob.len() < HEADER_LEN {
        return Err(SecretStoreError::Corruption);
    }

    let version = blob[MAGIC.len()];
    if version != ENVELOPE_VERSION {
        // Fail closed regardless of password — an unparseable future format
        // can be neither safely unwrapped nor treated as scheme-0.
        return Err(SecretStoreError::UnsupportedEnvelopeVersion { found: version });
    }

    let scheme = blob[MAGIC.len() + 1];
    let body = &blob[HEADER_LEN..];
    match scheme {
        SCHEME_UNPROTECTED => match password {
            None => Ok(SecretBytes::from_slice(body)),
            // Strip: caller expected protection, blob is unprotected.
            Some(_) => Err(SecretStoreError::ExpectedProtectedButUnsealed),
        },
        SCHEME_PASSWORD => match password {
            None => Err(SecretStoreError::NeedsPassword),
            Some(pw) => unwrap_scheme1(wallet_id, label, pw, body),
        },
        // Unknown scheme under a known version ⇒ forward-incompatible
        // layout; report the (known) version byte. Fail closed.
        _ => Err(SecretStoreError::UnsupportedEnvelopeVersion { found: version }),
    }
}

/// Decrypt a scheme-1 body. The KDF params, salt, and nonce are all read
/// from the (attacker-controllable) header; the param **ceiling is
/// enforced before** [`crypto::derive_key`] allocates, and every
/// header field that feeds key/AAD is bound into the AAD so any in-place
/// edit fails the tag.
fn unwrap_scheme1(
    wallet_id: &WalletId,
    label: &str,
    password: &SecretString,
    body: &[u8],
) -> Result<SecretBytes, SecretStoreError> {
    if body.len() < MIN_SCHEME1_BODY {
        // The scheme byte says protected, but the body cannot hold a sealed
        // payload — corrupt, not a strip.
        return Err(SecretStoreError::Corruption);
    }
    let kdf = decode_kdf(&body[..KDF_FIELD_LEN]);
    // Gate the inflated/unknown header BEFORE any derivation/alloc.
    kdf.enforce_bounds()?;

    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&body[KDF_FIELD_LEN..KDF_FIELD_LEN + SALT_LEN]);
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&body[KDF_FIELD_LEN + SALT_LEN..KDF_FIELD_LEN + SALT_LEN + NONCE_LEN]);
    let ciphertext = &body[KDF_FIELD_LEN + SALT_LEN + NONCE_LEN..];

    let aad = scheme1_aad(&kdf, &salt, wallet_id.as_bytes(), label);
    let key = crypto::derive_key(password, &salt, kdf)?;
    match crypto::open(&key, &nonce, &aad, ciphertext) {
        Ok(plaintext) => Ok(plaintext),
        // Tag failure (wrong password, relocated blob, or header tamper):
        // no plaintext is ever materialized (CWE-347).
        Err(SecretStoreError::Decrypt) => Err(SecretStoreError::WrongPassword),
        Err(e) => Err(e),
    }
}

/// Build the scheme-1 AAD binding object identity + header,
/// length-prefixed for the variable fields, mirroring
/// [`format::aad`](super::file::format::aad)/`verify_aad`.
fn scheme1_aad(
    kdf: &KdfParams,
    salt: &[u8; SALT_LEN],
    wallet_id: &[u8; 32],
    label: &str,
) -> Vec<u8> {
    let lb = label.as_bytes();
    let mut v = Vec::with_capacity(
        TIER2_DOMAIN.len()
            + MAGIC.len()
            + 2
            + KDF_FIELD_LEN
            + 4
            + SALT_LEN
            + 4
            + wallet_id.len()
            + 4
            + lb.len(),
    );
    v.extend_from_slice(TIER2_DOMAIN);
    v.extend_from_slice(MAGIC);
    v.push(ENVELOPE_VERSION);
    v.push(SCHEME_PASSWORD);
    v.extend_from_slice(&encode_kdf(kdf));
    v.extend_from_slice(&(salt.len() as u32).to_le_bytes());
    v.extend_from_slice(salt);
    v.extend_from_slice(&(wallet_id.len() as u32).to_le_bytes());
    v.extend_from_slice(wallet_id);
    v.extend_from_slice(&(lb.len() as u32).to_le_bytes());
    v.extend_from_slice(lb);
    v
}

/// Encode KDF params to the fixed 13-byte header field (LE).
fn encode_kdf(kdf: &KdfParams) -> [u8; KDF_FIELD_LEN] {
    let mut out = [0u8; KDF_FIELD_LEN];
    out[0] = kdf.id;
    out[1..5].copy_from_slice(&kdf.m_kib.to_le_bytes());
    out[5..9].copy_from_slice(&kdf.t.to_le_bytes());
    out[9..13].copy_from_slice(&kdf.p.to_le_bytes());
    out
}

/// Decode the fixed 13-byte KDF header field. Out-of-range values are
/// caught downstream by [`KdfParams::enforce_bounds`].
fn decode_kdf(b: &[u8]) -> KdfParams {
    debug_assert_eq!(b.len(), KDF_FIELD_LEN);
    KdfParams {
        id: b[0],
        m_kib: u32::from_le_bytes([b[1], b[2], b[3], b[4]]),
        t: u32::from_le_bytes([b[5], b[6], b[7], b[8]]),
        p: u32::from_le_bytes([b[9], b[10], b[11], b[12]]),
    }
}

/// Emit a single process-lifetime warning that a legacy magic-less entry
/// was read. Carries no secret (the message is static).
fn warn_legacy_once() {
    static WARN: Once = Once::new();
    WARN.call_once(|| {
        tracing::warn!(
            "read a legacy unprotected secret entry with no envelope header; \
             it will be re-wrapped on the next write"
        );
    });
}

#[cfg(test)]
mod tests {
    use subtle::ConstantTimeEq;

    use super::super::file::crypto::{
        ARGON2_MAX_M_KIB, ARGON2_MAX_T, ARGON2_MIN_M_KIB, ARGON2_MIN_T, ARGON2_P,
    };
    use super::super::file::format::KDF_ID_ARGON2ID;
    use super::*;

    // Wire offsets into a scheme-1 envelope (for surgical tampering).
    const O_VERSION: usize = 5;
    const O_SCHEME: usize = 6;
    const O_KDF: usize = HEADER_LEN; // 7
    const O_ID: usize = O_KDF; // 7
    const O_MKIB: usize = O_KDF + 1; // 8
    const O_T: usize = O_KDF + 5; // 12
    const O_SALT: usize = O_KDF + KDF_FIELD_LEN; // 20
    const O_NONCE: usize = O_SALT + SALT_LEN; // 52

    fn wid(b: u8) -> WalletId {
        WalletId::from([b; 32])
    }

    /// Argon2id floor params — fast enough for unit tests.
    fn floor() -> KdfParams {
        KdfParams {
            id: KDF_ID_ARGON2ID,
            m_kib: ARGON2_MIN_M_KIB,
            t: ARGON2_MIN_T,
            p: ARGON2_P,
        }
    }

    fn pw(s: &str) -> SecretString {
        SecretString::new(s)
    }

    /// Wrap and expose the envelope as a `Vec<u8>` for byte-level
    /// inspection/mutation in tests (the production `wrap` returns a
    /// zeroizing `SecretBytes`).
    fn wrap_bytes(
        w: &WalletId,
        label: &str,
        password: Option<&SecretString>,
        pt: &[u8],
    ) -> Vec<u8> {
        wrap(w, label, password, pt)
            .unwrap()
            .expose_secret()
            .to_vec()
    }

    /// [`wrap_bytes`] with explicit (floor) params, for the scheme-1 tests.
    fn wrap_p(
        w: &WalletId,
        label: &str,
        password: Option<&SecretString>,
        pt: &[u8],
        params: KdfParams,
    ) -> Vec<u8> {
        wrap_with_params(w, label, password, pt, params)
            .unwrap()
            .expose_secret()
            .to_vec()
    }

    /// scheme-0 passthrough round-trip; the wrapped form leads
    /// with magic, version=1, scheme=0, then the raw payload.
    #[test]
    fn scheme0_passthrough_round_trip() {
        let secret = b"top secret seed bytes";
        let blob = wrap_bytes(&wid(1), "seed", None, secret);
        assert!(blob.starts_with(MAGIC));
        assert_eq!(blob[O_VERSION], 1);
        assert_eq!(blob[O_SCHEME], 0);
        assert_eq!(&blob[HEADER_LEN..], secret);
        let got = unwrap(&wid(1), "seed", None, &blob).unwrap();
        assert_eq!(got.expose_secret(), secret);
    }

    /// scheme-1 round-trip; header records the argon2id id, a
    /// 32-byte fresh salt and 24-byte nonce, ct != pt, and two wraps of the
    /// same secret/pw differ in salt+nonce (no reuse).
    #[test]
    fn scheme1_round_trip_and_fresh_salt_nonce() {
        let secret = b"correct horse battery staple seed";
        let p = pw("hunter2-but-better");
        let blob = wrap_p(&wid(7), "seed", Some(&p), secret, floor());
        assert!(blob.starts_with(MAGIC));
        assert_eq!(blob[O_VERSION], 1);
        assert_eq!(blob[O_SCHEME], 1);
        assert_eq!(blob[O_ID], KDF_ID_ARGON2ID);
        // ciphertext differs from plaintext.
        assert_ne!(&blob[O_NONCE + NONCE_LEN..], secret);

        let got = unwrap(&wid(7), "seed", Some(&p), &blob).unwrap();
        assert_eq!(got.expose_secret(), secret);

        let blob2 = wrap_p(&wid(7), "seed", Some(&p), secret, floor());
        assert_ne!(
            &blob[O_SALT..O_SALT + SALT_LEN],
            &blob2[O_SALT..O_SALT + SALT_LEN],
            "salt must be fresh per wrap"
        );
        assert_ne!(
            &blob[O_NONCE..O_NONCE + NONCE_LEN],
            &blob2[O_NONCE..O_NONCE + NONCE_LEN],
            "nonce must be fresh per wrap"
        );
    }

    /// Wrong object password → WrongPassword, no plaintext.
    #[test]
    fn wrong_password_fails_closed() {
        let blob = wrap_p(&wid(1), "seed", Some(&pw("right")), b"seed", floor());
        let err = unwrap(&wid(1), "seed", Some(&pw("wrong")), &blob).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::WrongPassword),
            "got {err:?}"
        );
    }

    /// Identity AAD — a protected blob unwrapped at any
    /// other (wallet, label) fails the tag; same-identity still succeeds.
    #[test]
    fn relocation_across_identity_is_rejected() {
        let p = pw("pw");
        let blob = wrap_p(&wid(0xA), "labelA", Some(&p), b"seed", floor());
        for (w, l) in [(0xB, "labelB"), (0xA, "labelB"), (0xB, "labelA")] {
            let err = unwrap(&wid(w), l, Some(&p), &blob).unwrap_err();
            assert!(
                matches!(err, SecretStoreError::WrongPassword),
                "relocation to ({w:#x},{l}) must fail, got {err:?}"
            );
        }
        let ok = unwrap(&wid(0xA), "labelA", Some(&p), &blob).unwrap();
        assert_eq!(ok.expose_secret(), b"seed");
    }

    /// Per-field header tamper. Unknown KDF id is rejected by
    /// `enforce_bounds` (KdfFailure) before derive; in-bounds KDF shifts,
    /// salt, and nonce all fail the AEAD tag (WrongPassword) — never the
    /// plaintext.
    #[test]
    fn header_tamper_fails_closed_per_field() {
        let p = pw("pw");
        let base = wrap_p(&wid(1), "seed", Some(&p), b"seed", floor());

        // kdf.id → 7 (unknown) ⇒ KdfFailure (bounds reject pre-derive).
        let mut b = base.clone();
        b[O_ID] = 7;
        assert!(matches!(
            unwrap(&wid(1), "seed", Some(&p), &b).unwrap_err(),
            SecretStoreError::KdfFailure
        ));

        // kdf.m_kib → a different IN-BOUNDS value ⇒ WrongPassword (AAD + key).
        let mut b = base.clone();
        b[O_MKIB..O_MKIB + 4].copy_from_slice(&(ARGON2_MIN_M_KIB + 1024).to_le_bytes());
        assert!(matches!(
            unwrap(&wid(1), "seed", Some(&p), &b).unwrap_err(),
            SecretStoreError::WrongPassword
        ));

        // kdf.t → a different IN-BOUNDS value ⇒ WrongPassword.
        let mut b = base.clone();
        b[O_T..O_T + 4].copy_from_slice(&(ARGON2_MIN_T + 1).to_le_bytes());
        assert!(matches!(
            unwrap(&wid(1), "seed", Some(&p), &b).unwrap_err(),
            SecretStoreError::WrongPassword
        ));

        // salt[0] flip ⇒ WrongPassword (wrong key + AAD-bound salt).
        let mut b = base.clone();
        b[O_SALT] ^= 1;
        assert!(matches!(
            unwrap(&wid(1), "seed", Some(&p), &b).unwrap_err(),
            SecretStoreError::WrongPassword
        ));

        // nonce[0] flip ⇒ WrongPassword (nonce feeds decrypt ⇒ tag fail).
        let mut b = base;
        b[O_NONCE] ^= 1;
        assert!(matches!(
            unwrap(&wid(1), "seed", Some(&p), &b).unwrap_err(),
            SecretStoreError::WrongPassword
        ));
    }

    /// An inflated KDF param on a forged header is
    /// rejected by `enforce_bounds` BEFORE `derive_key` allocates — the
    /// ~4 TiB allocation never happens (the test would OOM if it did). The
    /// exact ceilings remain valid params.
    #[test]
    fn kdf_ceiling_enforced_before_derivation() {
        let p = pw("pw");
        let base = wrap_p(&wid(1), "seed", Some(&p), b"seed", floor());

        // m_kib = u32::MAX ⇒ KdfFailure, no allocation.
        let mut b = base.clone();
        b[O_MKIB..O_MKIB + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            unwrap(&wid(1), "seed", Some(&p), &b).unwrap_err(),
            SecretStoreError::KdfFailure
        ));

        // t = ARGON2_MAX_T + 1 ⇒ KdfFailure.
        let mut b = base;
        b[O_T..O_T + 4].copy_from_slice(&(ARGON2_MAX_T + 1).to_le_bytes());
        assert!(matches!(
            unwrap(&wid(1), "seed", Some(&p), &b).unwrap_err(),
            SecretStoreError::KdfFailure
        ));

        // The exact ceilings are accepted by the bounds check (no derive
        // here — a 1 GiB Argon2 run is not a unit-test concern).
        assert!(KdfParams {
            id: KDF_ID_ARGON2ID,
            m_kib: ARGON2_MAX_M_KIB,
            t: ARGON2_MAX_T,
            p: ARGON2_P,
        }
        .enforce_bounds()
        .is_ok());
    }

    /// A blank object password is rejected at enrol; nothing
    /// is sealed.
    #[test]
    fn blank_object_password_rejected_at_enrol() {
        for blank in [SecretString::empty(), pw(""), pw("   "), pw("\t\n")] {
            let err =
                wrap_with_params(&wid(1), "seed", Some(&blank), b"seed", floor()).unwrap_err();
            assert!(
                matches!(err, SecretStoreError::BlankPassphrase),
                "got {err:?}"
            );
        }
    }

    /// The plaintext is capped at `MAX_PLAINTEXT_LEN` (`MAX_SECRET_LEN −
    /// MAX_ENVELOPE_OVERHEAD`), uniform across schemes, so plaintext +
    /// overhead always fits the backend's own `MAX_SECRET_LEN` cap. Accept
    /// at the cap, reject at cap+1 with `max = MAX_PLAINTEXT_LEN`.
    #[test]
    fn plaintext_size_cap_at_envelope_boundary() {
        let at_cap = vec![0x5Au8; MAX_PLAINTEXT_LEN];
        let over = vec![0x5Au8; MAX_PLAINTEXT_LEN + 1];

        // Unprotected (scheme 0): cap accepted, +1 rejected.
        assert!(wrap(&wid(1), "seed", None, &at_cap).is_ok());
        assert!(matches!(
            wrap(&wid(1), "seed", None, &over).unwrap_err(),
            SecretStoreError::SecretTooLarge { found, max }
                if found == MAX_PLAINTEXT_LEN + 1 && max == MAX_PLAINTEXT_LEN
        ));

        // Protected (scheme 1): same cap (checked before any derivation).
        let p = pw("pw");
        assert!(matches!(
            wrap_with_params(&wid(1), "seed", Some(&p), &over, floor()).unwrap_err(),
            SecretStoreError::SecretTooLarge { found, max }
                if found == MAX_PLAINTEXT_LEN + 1 && max == MAX_PLAINTEXT_LEN
        ));
        // The enveloped bytes for an at-cap plaintext fit the backend cap.
        let enveloped = wrap(&wid(1), "seed", None, &at_cap).unwrap();
        assert!(enveloped.len() <= MAX_SECRET_LEN);
    }

    /// Scheme-1 accepts a plaintext of EXACTLY `MAX_PLAINTEXT_LEN` (the
    /// accept boundary), round-trips it, and the enveloped bytes still fit
    /// the backend's `MAX_SECRET_LEN` cap.
    #[test]
    fn scheme1_accepts_plaintext_at_exact_cap() {
        let p = pw("pw");
        let pt = vec![0x5Au8; MAX_PLAINTEXT_LEN];
        let blob = wrap_with_params(&wid(1), "seed", Some(&p), &pt, floor()).unwrap();
        assert!(
            blob.len() <= MAX_SECRET_LEN,
            "enveloped bytes exceed backend cap"
        );
        let got = unwrap(&wid(1), "seed", Some(&p), blob.expose_secret()).unwrap();
        assert_eq!(got.expose_secret(), &pt[..]);
    }

    /// Value rollback is intentionally NOT defended: an older valid scheme-1
    /// envelope still decrypts cleanly under the current password. Pinned so
    /// a future reader does not mistake the strict read for rollback
    /// protection (anti-rollback would need a monotonic anchor in the
    /// consumer's integrity-protected metadata).
    #[test]
    fn value_rollback_is_not_defended() {
        let p = pw("pw");
        let old_blob = wrap_with_params(&wid(1), "seed", Some(&p), b"OLD-VALUE", floor()).unwrap();
        // A newer value is written under the same identity + password …
        let _new_blob = wrap_with_params(&wid(1), "seed", Some(&p), b"NEW-VALUE", floor()).unwrap();
        // … yet "restoring" the OLD envelope still decrypts cleanly.
        let restored = unwrap(&wid(1), "seed", Some(&p), old_blob.expose_secret()).unwrap();
        assert_eq!(
            restored.expose_secret(),
            b"OLD-VALUE",
            "older envelope still decrypts: value rollback is a known, undefended residual"
        );
    }

    /// magic/version discrimination: a magic-less blob is a legacy raw
    /// value — returned on a `None` read (with a one-time warning), refused
    /// fail-closed on `Some(pw)` so the strict rule holds. A magic-present
    /// blob with an unknown version fails closed both ways; truncated-
    /// after-magic is corruption.
    #[test]
    fn magic_and_version_discrimination() {
        let p = pw("pw");
        // (a) Magic-less / wrong magic.
        let legacy = b"NOTPWSEV raw legacy seed bytes".to_vec();
        // None ⇒ legacy raw bytes (adopted contingency; NOT Corruption).
        let got = unwrap(&wid(1), "seed", None, &legacy).unwrap();
        assert_eq!(got.expose_secret(), &legacy[..]);
        // Some(pw) ⇒ strip/downgrade ⇒ fail closed.
        assert!(matches!(
            unwrap(&wid(1), "seed", Some(&p), &legacy).unwrap_err(),
            SecretStoreError::ExpectedProtectedButUnsealed
        ));

        // (b) Magic present but truncated below the header ⇒ Corruption.
        let mut trunc = MAGIC.to_vec();
        trunc.push(ENVELOPE_VERSION); // no scheme byte
        assert!(matches!(
            unwrap(&wid(1), "seed", None, &trunc).unwrap_err(),
            SecretStoreError::Corruption
        ));

        // (c) Magic OK but version = 2 ⇒ UnsupportedEnvelopeVersion{2},
        // regardless of password.
        let mut v2 = wrap_bytes(&wid(1), "seed", None, b"x");
        v2[O_VERSION] = 2;
        for arg in [None, Some(&p)] {
            assert!(matches!(
                unwrap(&wid(1), "seed", arg, &v2).unwrap_err(),
                SecretStoreError::UnsupportedEnvelopeVersion { found: 2 }
            ));
        }

        // (d) Magic+version OK but unknown scheme = 9 ⇒ fail closed.
        let mut s9 = wrap_bytes(&wid(1), "seed", None, b"x");
        s9[O_SCHEME] = 9;
        assert!(matches!(
            unwrap(&wid(1), "seed", None, &s9).unwrap_err(),
            SecretStoreError::UnsupportedEnvelopeVersion { found: 1 }
        ));
    }

    /// Non-vacuity helper for the strict read (used here and by the store
    /// tests): a scheme-0 blob carrying `secret` DOES decode under `None`.
    #[test]
    fn scheme0_some_password_fails_closed_strip() {
        let blob = wrap_bytes(&wid(1), "seed", None, b"attacker-seed");
        // None ⇒ it WOULD decode to the (attacker) bytes…
        assert_eq!(
            unwrap(&wid(1), "seed", None, &blob)
                .unwrap()
                .expose_secret(),
            b"attacker-seed"
        );
        // …but Some(pw) ⇒ ExpectedProtectedButUnsealed, no bytes.
        assert!(matches!(
            unwrap(&wid(1), "seed", Some(&pw("pw")), &blob).unwrap_err(),
            SecretStoreError::ExpectedProtectedButUnsealed
        ));
    }

    /// `ct_eq` sanity: a round-tripped secret matches the original under a
    /// constant-time compare (no `==` on secret bytes).
    #[test]
    fn round_trip_is_constant_time_equal() {
        let p = pw("pw");
        let original = SecretBytes::from_slice(b"seed material");
        let blob = wrap_p(&wid(1), "seed", Some(&p), original.expose_secret(), floor());
        let got = unwrap(&wid(1), "seed", Some(&p), &blob).unwrap();
        assert!(bool::from(got.ct_eq(&original)));
    }

    /// Deterministic byte-level fuzz. Every mutant unwrap is a
    /// clean `Ok` or a TYPED `SecretStoreError` — never a panic, never
    /// plaintext from a tag-failing branch. The `None` path (no Argon2
    /// derivation) runs the full 2000 mutants + every truncation; the
    /// `Some(pw)` path — each mutant of which may trigger a real Argon2
    /// derive — runs a representative subset so the suite stays fast while
    /// still exercising the derive/open code path.
    #[test]
    fn fuzz_byte_mutation_never_panics() {
        let p = pw("fuzz-pw");
        let valid = wrap_p(&wid(0xAB), "seed", Some(&p), b"seed-bytes", floor());
        // The pristine envelope unwraps.
        assert_eq!(
            unwrap(&wid(0xAB), "seed", Some(&p), &valid)
                .unwrap()
                .expose_secret(),
            b"seed-bytes"
        );

        // xorshift32 — deterministic, std-only.
        let mut state: u32 = 0x9E37_79B9;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };

        let assert_typed = |arg: Option<&SecretString>, buf: &[u8]| {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                unwrap(&wid(0xAB), "seed", arg, buf)
            }))
            .expect("unwrap must never panic on hostile input");
            match res {
                Ok(_)
                | Err(SecretStoreError::Corruption)
                | Err(SecretStoreError::WrongPassword)
                | Err(SecretStoreError::NeedsPassword)
                | Err(SecretStoreError::ExpectedProtectedButUnsealed)
                | Err(SecretStoreError::UnsupportedEnvelopeVersion { .. })
                | Err(SecretStoreError::KdfFailure) => {}
                Err(other) => panic!("unexpected error variant: {other:?}"),
            }
        };

        for i in 0..2_000 {
            let mut buf = valid.clone();
            let flips = 1 + (next() % 4) as usize;
            for _ in 0..flips {
                let idx = (next() as usize) % buf.len();
                buf[idx] ^= (next() & 0xFF) as u8;
            }
            // None path every iteration (cheap, no derive).
            assert_typed(None, &buf);
            // Some path on a representative subset (each may derive Argon2).
            if i % 16 == 0 {
                assert_typed(Some(&p), &buf);
            }
        }

        // Truncation at every offset — a short read must never panic.
        for cut in 0..valid.len() {
            assert_typed(None, &valid[..cut]);
            assert_typed(Some(&p), &valid[..cut]);
        }
    }
}
