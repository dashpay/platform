//! Tier-2 envelope wire format — bincode-encoded `Envelope` / `Payload`
//! plus the [`wrap_with_params`] / [`unwrap`] API.
//!
//! Every byte that crosses the AEAD seam is produced by
//! `bincode::encode_to_vec` against [`WIRE_CONFIG`], so a future config
//! drift surfaces in the golden-vector tests, not in silently corrupted
//! blobs. Decoding goes through [`DECODE_CONFIG`] — the same
//! configuration with a byte limit, so a hostile blob declaring a
//! multi-GiB length prefix is rejected before any allocation.

use bincode::config::{BigEndian, Configuration, Limit, Varint};
use zeroize::Zeroize;

use crate::secrets::error::SecretStoreError;
use crate::secrets::file::crypto::{self, KdfParams, NONCE_LEN, SALT_LEN};
use crate::secrets::secret::{SecretBytes, SecretString, MAX_PASSPHRASE_LEN};
use crate::secrets::validate::WalletId;
use crate::secrets::wire::aad::Tier2Aad;
use crate::secrets::wire::config::{ENVELOPE_VERSION, TIER2_DOMAIN_V2, WIRE_CONFIG};
use crate::secrets::wire::kdf::KdfParamsEncoded;
use crate::secrets::MAX_SECRET_LEN;

/// On-disk Tier-2 wire envelope. The whole struct is bincode-encoded
/// in one call; a wire-edited `version` is gated to
/// `SecretStoreError::UnsupportedEnvelopeVersion` before dispatch.
#[derive(bincode::Encode, bincode::Decode, Debug, PartialEq, Eq)]
pub(crate) struct Envelope {
    /// Envelope wire version (`ENVELOPE_VERSION`).
    pub version: u32,
    /// Tagged payload selecting unprotected vs password-protected.
    pub payload: Payload,
}

/// Tagged payload: scheme-0 ships the plaintext as-is (the backend's
/// own at-rest crypto is the only defence); scheme-1 ships the AEAD
/// triple under an object-password-derived key.
#[derive(bincode::Encode, bincode::Decode, Debug, PartialEq, Eq)]
pub(crate) enum Payload {
    /// Scheme 0 — unprotected passthrough; the bytes are the secret.
    Unprotected(Vec<u8>),
    /// Scheme 1 — sealed under an Argon2id-derived key with
    /// XChaCha20-Poly1305. The AAD bound at seal time is
    /// [`crate::secrets::wire::aad::Tier2Aad`].
    Password {
        /// Argon2 parameters used to derive the key.
        kdf: KdfParamsEncoded,
        /// Per-wrap CSPRNG salt fed into Argon2.
        salt: [u8; SALT_LEN],
        /// Per-wrap CSPRNG nonce fed into XChaCha20-Poly1305.
        nonce: [u8; NONCE_LEN],
        /// Ciphertext + 16-byte Poly1305 tag.
        ciphertext: Vec<u8>,
    },
}

impl Payload {
    /// Zeroize the heap buffer this payload carries.
    ///
    /// `bincode` decodes `Unprotected` into an ordinary unguarded `Vec` —
    /// the raw secret, outside every control `secrets::guarded` provides.
    /// The success path launders it through `SecretBytes::new`; every early
    /// return in [`unwrap`] wipes instead — through this method before the
    /// payload is destructured, directly on the moved buffer after — or the
    /// secret is left on the heap for a later re-read, a core dump, or swap.
    fn wipe(&mut self) {
        // In place rather than `Vec::zeroize`, which clears to length 0 and
        // would hide the wipe from `payload_wipe_clears_both_variants`.
        match self {
            Payload::Unprotected(plaintext) => plaintext.as_mut_slice().zeroize(),
            Payload::Password { ciphertext, .. } => ciphertext.as_mut_slice().zeroize(),
        }
    }
}

/// Upper bound on the bincode-encoded envelope overhead over its
/// plaintext (header + KDF + salt + nonce + AEAD tag + bincode framing).
/// Pinned by a runtime cross-check in `tests::max_envelope_overhead_matches_runtime`
/// so any bincode-config drift surfaces immediately. The smallest
/// scheme-1 envelope (empty plaintext sealed → 16-byte tag) measures
/// 81 bytes; rounded up to the next 16-byte boundary that satisfies a
/// 16-byte safety margin (81 + 16 = 97 → 112) for headroom against a
/// future header field.
pub(crate) const MAX_ENVELOPE_OVERHEAD: usize = 112;

/// Plaintext cap at the envelope boundary: `MAX_SECRET_LEN −
/// MAX_ENVELOPE_OVERHEAD`. Capping the plaintext (uniformly for both
/// schemes) keeps the user-visible limit stable AND guarantees the
/// enveloped bytes always fit the backend vault's own `MAX_SECRET_LEN`
/// `put_bytes` cap.
pub const MAX_PLAINTEXT_LEN: usize = MAX_SECRET_LEN - MAX_ENVELOPE_OVERHEAD;

/// Decode-side budget: caps the bytes the bincode decoder will consume
/// from a single envelope. Equal to the on-disk cap.
const DECODE_BUDGET: usize = MAX_SECRET_LEN + MAX_ENVELOPE_OVERHEAD;

/// Bincode decode config — derived from [`WIRE_CONFIG`] but with a
/// [`DECODE_BUDGET`] byte limit applied.
///
/// **Asymmetric on purpose, security-positive deviation from
/// design-brief NF2** (which locks the wire config to
/// `with_no_limit()`). The deviation exists for hostile-decode
/// hardening: an attacker-controlled length prefix in the blob would
/// otherwise drive `Vec::with_capacity` to a multi-GiB allocation
/// before any tag check. With `Limit<N>`, bincode refuses the
/// allocation up front and the unwrap fails closed as `Corruption`.
///
/// The encoder retains [`WIRE_CONFIG`] (no limit) because AAD and
/// envelope encoding are producer-only — every input is library-owned
/// and bounded by `MAX_PLAINTEXT_LEN`, so a limit there has no
/// security benefit and would be a foot-gun against legitimate
/// at-cap secrets.
const DECODE_CONFIG: Configuration<BigEndian, Varint, Limit<DECODE_BUDGET>> =
    WIRE_CONFIG.with_limit::<DECODE_BUDGET>();

/// Wrap `plaintext` for `(wallet_id, label)` under Argon2 `params`.
///
/// `None` → an unprotected (scheme-0) envelope, and `params` is unused;
/// `Some(pw)` → a scheme-1 envelope sealed under `pw`. A sub-floor password
/// is rejected at enrol (`SecretStoreError::BlankPassphrase`).
///
/// Callers pass [`KdfParams::default_target`]; a mock store
/// ([`SecretStore::file_mock`]) passes the floor instead (#4111).
///
/// Returns the envelope inside a zeroizing [`SecretBytes`].
///
/// [`SecretStore::file_mock`]: crate::secrets::SecretStore::file_mock
pub(crate) fn wrap_with_params(
    wallet_id: &WalletId,
    label: &str,
    password: Option<&SecretString>,
    plaintext: &[u8],
    params: KdfParams,
) -> Result<SecretBytes, SecretStoreError> {
    // Cap the PLAINTEXT (before overhead) uniformly for both schemes so
    // the enveloped bytes always fit the backend cap.
    if plaintext.len() > MAX_PLAINTEXT_LEN {
        return Err(SecretStoreError::SecretTooLarge {
            found: plaintext.len(),
            max: MAX_PLAINTEXT_LEN,
        });
    }

    let Some(pw) = password else {
        // The scheme-0 plaintext copy rides the envelope in the clear. Encode,
        // then wipe that copy before it drops — the returned SecretBytes is the
        // only retained copy and zeroizes itself.
        let mut envelope = Envelope {
            version: ENVELOPE_VERSION,
            payload: Payload::Unprotected(plaintext.to_vec()),
        };
        let encoded = encode_envelope(&envelope);
        if let Payload::Unprotected(ref mut bytes) = envelope.payload {
            bytes.zeroize();
        }
        return Ok(SecretBytes::new(encoded));
    };

    // Reject an out-of-range object password before any salt or derivation.
    if pw.is_below_minimum_passphrase_len() {
        return Err(SecretStoreError::BlankPassphrase);
    }
    if pw.exceeds_maximum_passphrase_len() {
        return Err(SecretStoreError::PassphraseTooLong {
            found: pw.len(),
            max: MAX_PASSPHRASE_LEN,
        });
    }

    let mut salt = [0u8; SALT_LEN];
    crypto::random_bytes(&mut salt)?;
    let kdf = KdfParamsEncoded::from(params);
    let aad = encode_tier2_aad(wallet_id, label, kdf, &salt);
    // Scoped so the derived key's guarded page is released before the
    // envelope buffer is allocated; the two are never both needed.
    let (nonce, ciphertext) = {
        let key = crypto::derive_key(pw, &salt, params)?;
        crypto::seal(&key, &aad, plaintext)?
    };

    let envelope = Envelope {
        version: ENVELOPE_VERSION,
        payload: Payload::Password {
            kdf,
            salt,
            nonce,
            ciphertext,
        },
    };
    Ok(SecretBytes::new(encode_envelope(&envelope)))
}

/// Bincode-encode the scheme-1 AAD against [`WIRE_CONFIG`]. Shared by
/// [`wrap_with_params`] and [`unwrap_password_payload`] so the encode
/// and decode AADs cannot drift apart.
pub(crate) fn encode_tier2_aad(
    wallet_id: &WalletId,
    label: &str,
    kdf: KdfParamsEncoded,
    salt: &[u8; SALT_LEN],
) -> Vec<u8> {
    let aad = Tier2Aad {
        domain: TIER2_DOMAIN_V2,
        envelope_version: ENVELOPE_VERSION,
        scheme_discriminant: 1,
        kdf,
        salt: *salt,
        wallet_id: *wallet_id.as_bytes(),
        label,
    };
    // AAD encode is infallible — every field is owned/borrowed bincode-
    // Encode-able. A failure would be a logic bug.
    bincode::encode_to_vec(aad, WIRE_CONFIG).expect("Tier2Aad encode is infallible")
}

/// Bincode-encode the whole envelope. Wrapping in `SecretBytes::new`
/// keeps the (possibly plaintext-bearing) scheme-0 buffer zeroizing.
fn encode_envelope(envelope: &Envelope) -> Vec<u8> {
    bincode::encode_to_vec(envelope, WIRE_CONFIG).expect("Envelope encode is infallible")
}

/// Unwrap `blob` for `(wallet_id, label)`, applying the strict
/// fail-closed read.
///
/// `password` carries the caller's protection assertion — never the
/// blob's scheme byte. Decode errors (truncated, garbage bytes, unknown
/// enum tag) collapse to `Corruption`; an envelope version this build
/// does not recognise yields `UnsupportedEnvelopeVersion` ahead of
/// dispatch.
///
/// | `password` | `payload` | result |
/// |---|---|---|
/// | `Some(pw)` | `Password { .. }` | the secret, or `WrongPassword` on tag fail |
/// | `Some(pw)` | `Unprotected(_)` | `ExpectedProtectedButUnsealed` (strip/downgrade) |
/// | `None`     | `Password { .. }` | `NeedsPassword` (never ciphertext) |
/// | `None`     | `Unprotected(pt)` | the secret |
pub(crate) fn unwrap(
    wallet_id: &WalletId,
    label: &str,
    password: Option<&SecretString>,
    blob: &[u8],
) -> Result<SecretBytes, SecretStoreError> {
    let (mut envelope, consumed) = bincode::decode_from_slice::<Envelope, _>(blob, DECODE_CONFIG)
        .map_err(|_| SecretStoreError::Corruption)?;
    // Trailing bytes after a valid decode are a truncation/extension
    // probe — fail closed. Both pre-dispatch refusals wipe first: the
    // decoded payload may already hold a scheme-0 plaintext.
    if consumed != blob.len() {
        envelope.payload.wipe();
        return Err(SecretStoreError::Corruption);
    }

    if envelope.version != ENVELOPE_VERSION {
        envelope.payload.wipe();
        return Err(SecretStoreError::UnsupportedEnvelopeVersion {
            found: envelope.version,
        });
    }

    match (envelope.payload, password) {
        (Payload::Unprotected(mut plaintext), None) => {
            // Enforce the same cap the wrap side applies.  DECODE_BUDGET is
            // larger than MAX_PLAINTEXT_LEN (by MAX_ENVELOPE_OVERHEAD), so a
            // tampered blob can pass the bincode budget check yet exceed the
            // application-level plaintext ceiling; reject it here.
            if plaintext.len() > MAX_PLAINTEXT_LEN {
                let found = plaintext.len();
                plaintext.as_mut_slice().zeroize();
                return Err(SecretStoreError::SecretTooLarge {
                    found,
                    max: MAX_PLAINTEXT_LEN,
                });
            }
            Ok(SecretBytes::new(plaintext))
        }
        // Caller asserted protection but blob is unprotected: strip /
        // downgrade — fail closed, never return the bytes, and never leave
        // them on the heap either.
        (Payload::Unprotected(mut plaintext), Some(_)) => {
            plaintext.as_mut_slice().zeroize();
            Err(SecretStoreError::ExpectedProtectedButUnsealed)
        }
        (Payload::Password { .. }, None) => Err(SecretStoreError::NeedsPassword),
        (
            Payload::Password {
                kdf,
                salt,
                nonce,
                ciphertext,
            },
            Some(pw),
        ) => unwrap_password_payload(wallet_id, label, pw, kdf, salt, nonce, &ciphertext),
    }
}

/// Decrypt a `Payload::Password` body. The KDF params, salt and nonce
/// come from the (attacker-controllable) envelope; `enforce_bounds` AND
/// the stricter wire-stable per-read ceiling gate the params BEFORE
/// `derive_key` allocates.
fn unwrap_password_payload(
    wallet_id: &WalletId,
    label: &str,
    password: &SecretString,
    kdf_encoded: KdfParamsEncoded,
    salt: [u8; SALT_LEN],
    nonce: [u8; NONCE_LEN],
    ciphertext: &[u8],
) -> Result<SecretBytes, SecretStoreError> {
    // (a0) Mirror wrap's floor on read so a backend-write attacker cannot
    // plant a weakly sealed envelope for an accidentally weak caller input.
    if password.is_below_minimum_passphrase_len() {
        return Err(SecretStoreError::BlankPassphrase);
    }
    // (a1) Mirror wrap's ceiling too. A re-protect holds the old password,
    // the new one and the resident vault passphrase at once, so all three
    // must be bounded for the budget at `MAX_SECRET_LEN` to hold. Refusing
    // here locks nobody out: wrap applies the same ceiling, so no
    // legitimately enrolled entry can need a longer password.
    if password.exceeds_maximum_passphrase_len() {
        return Err(SecretStoreError::PassphraseTooLong {
            found: password.len(),
            max: MAX_PASSPHRASE_LEN,
        });
    }
    // (a) Wider Argon2 floors/ceilings — refuses an inflated header
    // before any allocation.
    let kdf = KdfParams::try_from(kdf_encoded)?;
    // (b) Per-read ceiling tighter than `enforce_bounds`: a header
    // declaring more memory OR more time than `ARGON2_READ_MAX_*` is
    // refused before `derive_key` allocates, closing the gap between the
    // 1 GiB / 16-pass DoS band and the shipped cost. Gated on wire-stable
    // constants, NOT `default_target()`: a read ceiling tied to a tunable
    // would orphan every enrolled secret the day the shipped default is
    // lowered for a low-RAM host, with no recovery path.
    kdf.enforce_read_ceiling()?;
    // (c) AAD binds identity + header — the same bytes the encoder
    // produced, by construction.
    let aad = encode_tier2_aad(wallet_id, label, kdf_encoded, &salt);
    let key = crypto::derive_key(password, &salt, kdf)?;
    match crypto::open(&key, &nonce, &aad, ciphertext) {
        Ok(plaintext) => {
            // A wrap-produced blob is always within MAX_PLAINTEXT_LEN; a
            // plaintext that exceeds the cap after successful AEAD
            // authentication must have been produced by a different build or
            // is tampered — fail closed.
            if plaintext.len() > MAX_PLAINTEXT_LEN {
                return Err(SecretStoreError::SecretTooLarge {
                    found: plaintext.len(),
                    max: MAX_PLAINTEXT_LEN,
                });
            }
            Ok(plaintext)
        }
        // Tag failure (wrong password, relocated blob, header tamper):
        // no plaintext ever materialises (CWE-347).
        Err(SecretStoreError::Decrypt) => Err(SecretStoreError::WrongPassword),
        Err(e) => Err(e),
    }
}

/// Test-only deterministic encoder: takes pre-supplied `salt` and
/// `nonce` instead of pulling from the CSPRNG, so golden-vector tests
/// produce reproducible bytes. Production callers MUST use
/// [`wrap_with_params`].
#[cfg(test)]
pub(crate) fn wrap_with_params_for_test(
    wallet_id: &WalletId,
    label: &str,
    pw: &SecretString,
    plaintext: &[u8],
    params: KdfParams,
    salt: [u8; SALT_LEN],
    nonce: [u8; NONCE_LEN],
) -> Result<SecretBytes, SecretStoreError> {
    if plaintext.len() > MAX_PLAINTEXT_LEN {
        return Err(SecretStoreError::SecretTooLarge {
            found: plaintext.len(),
            max: MAX_PLAINTEXT_LEN,
        });
    }
    if pw.is_below_minimum_passphrase_len() {
        return Err(SecretStoreError::BlankPassphrase);
    }
    let key = crypto::derive_key(pw, &salt, params)?;
    let kdf = KdfParamsEncoded::from(params);
    let aad = encode_tier2_aad(wallet_id, label, kdf, &salt);
    let (nonce, ciphertext) = crypto::seal_with_nonce(&key, nonce, &aad, plaintext)?;
    let envelope = Envelope {
        version: ENVELOPE_VERSION,
        payload: Payload::Password {
            kdf,
            salt,
            nonce,
            ciphertext,
        },
    };
    Ok(SecretBytes::new(encode_envelope(&envelope)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::file::crypto::{ARGON2_MIN_M_KIB, ARGON2_MIN_T};

    /// Captured once from the runtime encoder; a subsequent CI failure
    /// here means a wire-format drift to investigate, NOT to "fix" by
    /// re-generating the constant.
    ///
    /// Decoding: 0x01 envelope.version=1, 0x00 Payload::Unprotected,
    /// 0x05 Vec<u8> length=5, "hello".
    const SCHEME0_GOLDEN_HEX: &str = "01000568656c6c6f";

    /// scheme-1 deterministic golden: wid=[0;32], label="seed",
    /// pw="pw------", plaintext="hello", floor params, salt=[0x11;32],
    /// nonce=[0x22;24]. Bytes: version + Payload::Password tag +
    /// kdf(id,m_kib,t,p as varints) + salt[32] + nonce[24] +
    /// ciphertext-with-tag length + ciphertext+tag(21B).
    const SCHEME1_GOLDEN_HEX: &str = "010101fb4c0002011111111111111111111111111111111111111111111111111111111111111111222222222222222222222222222222222222222222222222152b9c8f5632a7bad30ca908db231f1aa2c897982352";

    fn wid(b: u8) -> WalletId {
        WalletId::from([b; 32])
    }

    /// Pad nonblank fixture labels to the production length floor.
    fn pw(s: &str) -> SecretString {
        if s.trim().is_empty() {
            return SecretString::new(s);
        }
        let mut value = s.to_owned();
        while value.trim().len() < crate::secrets::MIN_PASSPHRASE_LEN {
            value.push('-');
        }
        SecretString::new(value)
    }

    /// TC-033 — blank object password rejected at enrol (wrap-side).
    #[test]
    fn blank_object_password_rejected_at_wrap() {
        for blank in [SecretString::empty(), pw(""), pw("   "), pw("\t\n")] {
            let err = wrap_with_params(
                &wid(1),
                "seed",
                Some(&blank),
                b"seed",
                KdfParams::floor_target(),
            )
            .unwrap_err();
            assert!(
                matches!(err, SecretStoreError::BlankPassphrase),
                "got {err:?}"
            );
        }
    }

    /// Symmetric guard on the read side: a `Some(blank)` password reaching
    /// `unwrap_password_payload` is refused with `BlankPassphrase` BEFORE
    /// any KDF or AEAD work — never `WrongPassword`, never `Decrypt`,
    /// never plaintext. Pins the contract that closes the asymmetry where
    /// a backend-write attacker could plant a scheme-1 envelope sealed
    /// under the blank password and have a caller that accidentally
    /// forwards `Some(SecretString::empty())` accept attacker-controlled
    /// plaintext.
    #[test]
    fn unwrap_password_payload_rejects_some_blank_password() {
        let blob = scheme1_blob(&pw("good"));
        let err = unwrap(&wid(1), "seed", Some(&SecretString::empty()), &blob).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::BlankPassphrase),
            "blank object password must be refused before KDF/AEAD, got {err:?}"
        );
    }

    /// The object-password ceiling is enforced symmetrically, wrap and
    /// unwrap. Both sides matter: a re-protect holds the old password, the
    /// new one and the resident vault passphrase at once, so all three
    /// must be bounded for `MAX_SECRET_LEN`'s budget to hold. Enforcing it
    /// on read locks nobody out, because wrap refuses to enrol one.
    #[test]
    fn object_password_cap_accept_then_reject_on_both_sides() {
        let at_cap = SecretString::new("p".repeat(MAX_PASSPHRASE_LEN));
        let over = SecretString::new("p".repeat(MAX_PASSPHRASE_LEN + 1));

        let blob = wrap_with_params(
            &wid(1),
            "seed",
            Some(&at_cap),
            b"seed",
            KdfParams::floor_target(),
        )
        .expect("a password exactly at the cap must be accepted");
        assert_eq!(
            unwrap(&wid(1), "seed", Some(&at_cap), blob.expose_secret())
                .unwrap()
                .expose_secret(),
            b"seed"
        );

        let wrap_err = wrap_with_params(
            &wid(1),
            "seed",
            Some(&over),
            b"seed",
            KdfParams::floor_target(),
        )
        .unwrap_err();
        assert!(
            matches!(wrap_err, SecretStoreError::PassphraseTooLong { found, max }
                if found == MAX_PASSPHRASE_LEN + 1 && max == MAX_PASSPHRASE_LEN),
            "got {wrap_err:?}"
        );

        // Refused before any KDF or AEAD work, so never as WrongPassword.
        let unwrap_err = unwrap(&wid(1), "seed", Some(&over), blob.expose_secret()).unwrap_err();
        assert!(
            matches!(unwrap_err, SecretStoreError::PassphraseTooLong { .. }),
            "got {unwrap_err:?}"
        );
    }

    /// TC-034 — plaintext cap accept at MAX_PLAINTEXT_LEN, reject at
    /// +1, for both schemes.
    #[test]
    fn plaintext_cap_accept_then_reject() {
        let at_cap = vec![0x5Au8; MAX_PLAINTEXT_LEN];
        let over = vec![0x5Au8; MAX_PLAINTEXT_LEN + 1];

        // Scheme 0 — `params` is unused on the unprotected path.
        assert!(
            wrap_with_params(&wid(1), "seed", None, &at_cap, KdfParams::floor_target()).is_ok()
        );
        assert!(matches!(
            wrap_with_params(&wid(1), "seed", None, &over, KdfParams::floor_target()).unwrap_err(),
            SecretStoreError::SecretTooLarge { found, max }
                if found == MAX_PLAINTEXT_LEN + 1 && max == MAX_PLAINTEXT_LEN
        ));

        // Scheme 1 — cap check fires before any derivation.
        let p = pw("pw");
        assert!(matches!(
            wrap_with_params(&wid(1), "seed", Some(&p), &over, KdfParams::floor_target()).unwrap_err(),
            SecretStoreError::SecretTooLarge { found, max }
                if found == MAX_PLAINTEXT_LEN + 1 && max == MAX_PLAINTEXT_LEN
        ));

        // Scheme-0 enveloped bytes for an at-cap plaintext fit the backend cap.
        let enveloped =
            wrap_with_params(&wid(1), "seed", None, &at_cap, KdfParams::floor_target()).unwrap();
        assert!(enveloped.len() <= MAX_SECRET_LEN);
    }

    /// TC-035 (size-budget half) — scheme-1 accepts plaintext at the
    /// exact MAX_PLAINTEXT_LEN boundary; the enveloped bytes fit the
    /// backend cap. The round-trip half is `scheme1_at_cap_round_trips_within_backend_cap`.
    #[test]
    fn scheme1_at_cap_envelope_fits_backend_cap() {
        let p = pw("pw");
        let pt = vec![0x5Au8; MAX_PLAINTEXT_LEN];
        let blob =
            wrap_with_params(&wid(1), "seed", Some(&p), &pt, KdfParams::floor_target()).unwrap();
        assert!(
            blob.len() <= MAX_SECRET_LEN,
            "enveloped bytes ({} B) exceed backend cap ({} B)",
            blob.len(),
            MAX_SECRET_LEN
        );
    }

    /// TC-028 — golden hex vector for the scheme-0 wire bytes. Any
    /// bincode-config drift (endianness, varint mode, limit) trips this.
    #[test]
    fn scheme0_golden_vector_matches_const() {
        let blob = wrap_with_params(
            &WalletId::from([0u8; 32]),
            "seed",
            None,
            b"hello",
            KdfParams::floor_target(),
        )
        .unwrap();
        let actual = hex::encode(blob.expose_secret());
        assert_eq!(actual, SCHEME0_GOLDEN_HEX);
    }

    /// TC-029 — golden hex vector for the scheme-1 wire bytes, produced
    /// via the deterministic encoder seam.
    #[test]
    fn scheme1_golden_vector_matches_const() {
        let blob = wrap_with_params_for_test(
            &WalletId::from([0u8; 32]),
            "seed",
            &pw("pw"),
            b"hello",
            KdfParams::floor_target(),
            [0x11u8; SALT_LEN],
            [0x22u8; NONCE_LEN],
        )
        .unwrap();
        let actual = hex::encode(blob.expose_secret());
        assert_eq!(actual, SCHEME1_GOLDEN_HEX);
    }

    /// Minimum overhead within budget AND the budget not absurdly above
    /// the actual encoding — bound on both sides so the constant stays
    /// honest as the wire shape evolves.
    const SAFETY_MARGIN: usize = 16;

    /// TC-030 — `MAX_ENVELOPE_OVERHEAD` cross-checks the runtime
    /// bincode encoding of the smallest possible scheme-1 envelope
    /// (empty plaintext sealed → ciphertext == 16-byte AEAD tag).
    #[test]
    fn max_envelope_overhead_matches_runtime() {
        let blob = wrap_with_params_for_test(
            &WalletId::from([0u8; 32]),
            "seed",
            &pw("pw"),
            b"",
            KdfParams::floor_target(),
            [0x11u8; SALT_LEN],
            [0x22u8; NONCE_LEN],
        )
        .unwrap();
        let actual = blob.len();
        assert!(
            actual + SAFETY_MARGIN <= MAX_ENVELOPE_OVERHEAD,
            "overhead {} + margin {} exceeds const {}",
            actual,
            SAFETY_MARGIN,
            MAX_ENVELOPE_OVERHEAD
        );
        assert!(
            MAX_ENVELOPE_OVERHEAD - actual < 64,
            "MAX_ENVELOPE_OVERHEAD {} is more than 64 B above the runtime measurement {} — tighten it",
            MAX_ENVELOPE_OVERHEAD,
            actual
        );
    }

    // ===== Decoder: dispatch / wire-flip / fuzz / property =====

    use crate::secrets::file::crypto::{
        ARGON2_MAX_M_KIB, ARGON2_MAX_T, ARGON2_READ_MAX_M_KIB, ARGON2_READ_MAX_T,
    };
    use crate::secrets::wire::config::WIRE_CONFIG;
    use subtle::ConstantTimeEq;

    /// Decode a real envelope so wire-flip tests can mutate one field
    /// and re-encode.
    fn decode(blob: &[u8]) -> Envelope {
        bincode::decode_from_slice::<Envelope, _>(blob, WIRE_CONFIG)
            .unwrap()
            .0
    }

    fn encode(envelope: &Envelope) -> Vec<u8> {
        bincode::encode_to_vec(envelope, WIRE_CONFIG).unwrap()
    }

    /// Build a fresh scheme-1 envelope (under wid(1)/"seed"/pw=`p`) and
    /// hand back the bytes for mutation tests.
    fn scheme1_blob(p: &SecretString) -> Vec<u8> {
        wrap_with_params(&wid(1), "seed", Some(p), b"seed", KdfParams::floor_target())
            .unwrap()
            .expose_secret()
            .to_vec()
    }

    /// TC-001 — scheme-0 round-trip preserves plaintext.
    #[test]
    fn scheme0_round_trip_preserves_plaintext() {
        let blob = wrap_with_params(
            &wid(1),
            "seed",
            None,
            b"top secret seed bytes",
            KdfParams::floor_target(),
        )
        .unwrap();
        let got = unwrap(&wid(1), "seed", None, blob.expose_secret()).unwrap();
        assert_eq!(got.expose_secret(), b"top secret seed bytes");
    }

    /// TC-002 — scheme-1 round-trip preserves plaintext.
    #[test]
    fn scheme1_round_trip_preserves_plaintext() {
        let p = pw("hunter2");
        let blob = wrap_with_params(
            &wid(7),
            "seed",
            Some(&p),
            b"correct horse battery staple",
            KdfParams::floor_target(),
        )
        .unwrap();
        assert_ne!(blob.expose_secret(), b"correct horse battery staple");
        let got = unwrap(&wid(7), "seed", Some(&p), blob.expose_secret()).unwrap();
        assert_eq!(got.expose_secret(), b"correct horse battery staple");
    }

    /// TC-003 — scheme-1 produces a fresh salt + nonce per wrap.
    #[test]
    fn scheme1_uses_fresh_salt_and_nonce_per_wrap() {
        let p = pw("pw");
        let a = scheme1_blob(&p);
        let b = scheme1_blob(&p);
        let (sa, na) = match decode(&a).payload {
            Payload::Password { salt, nonce, .. } => (salt, nonce),
            _ => panic!("scheme-1 wrap must yield Password"),
        };
        let (sb, nb) = match decode(&b).payload {
            Payload::Password { salt, nonce, .. } => (salt, nonce),
            _ => panic!("scheme-1 wrap must yield Password"),
        };
        assert_ne!(sa, sb, "salt must be fresh per wrap");
        assert_ne!(na, nb, "nonce must be fresh per wrap");
    }

    /// TC-004 — wrong object password yields WrongPassword.
    #[test]
    fn wrong_password_fails_closed() {
        let blob = scheme1_blob(&pw("right"));
        let err = unwrap(&wid(1), "seed", Some(&pw("wrong")), &blob).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::WrongPassword),
            "got {err:?}"
        );
    }

    /// Mutate the `Payload::Password` body in-place via decode → patch
    /// → encode. Returns the new blob.
    fn mutate_scheme1(
        blob: &[u8],
        patch: impl FnOnce(&mut KdfParamsEncoded, &mut [u8; SALT_LEN], &mut [u8; NONCE_LEN]),
    ) -> Vec<u8> {
        let mut env = decode(blob);
        match env.payload {
            Payload::Password {
                ref mut kdf,
                ref mut salt,
                ref mut nonce,
                ..
            } => patch(kdf, salt, nonce),
            _ => panic!("mutate_scheme1 expects a Password payload"),
        }
        encode(&env)
    }

    /// TC-005 — wire-flip of kdf.m_kib (in-bounds shift) yields WrongPassword.
    #[test]
    fn wire_flip_kdf_m_kib_fails_closed() {
        let p = pw("pw");
        let blob = scheme1_blob(&p);
        let tampered = mutate_scheme1(&blob, |kdf, _, _| {
            kdf.m_kib = ARGON2_MIN_M_KIB + 1024;
        });
        let err = unwrap(&wid(1), "seed", Some(&p), &tampered).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::WrongPassword),
            "got {err:?}"
        );
    }

    /// TC-006 — wire-flip of kdf.t (in-bounds shift) yields WrongPassword.
    #[test]
    fn wire_flip_kdf_t_fails_closed() {
        let p = pw("pw");
        let blob = scheme1_blob(&p);
        let tampered = mutate_scheme1(&blob, |kdf, _, _| {
            kdf.t = ARGON2_MIN_T + 1;
        });
        let err = unwrap(&wid(1), "seed", Some(&p), &tampered).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::WrongPassword),
            "got {err:?}"
        );
    }

    /// TC-007 — wire-flip of kdf.id to an unknown value is rejected by
    /// `enforce_bounds` BEFORE `derive_key` allocates.
    #[test]
    fn wire_flip_kdf_id_unknown_rejected_pre_derive() {
        let p = pw("pw");
        let blob = scheme1_blob(&p);
        let tampered = mutate_scheme1(&blob, |kdf, _, _| {
            kdf.id = 7;
        });
        let err = unwrap(&wid(1), "seed", Some(&p), &tampered).unwrap_err();
        assert!(matches!(err, SecretStoreError::KdfFailure), "got {err:?}");
    }

    /// TC-008 — wire-flip of salt[0] yields WrongPassword.
    #[test]
    fn wire_flip_salt_fails_closed() {
        let p = pw("pw");
        let blob = scheme1_blob(&p);
        let tampered = mutate_scheme1(&blob, |_, salt, _| {
            salt[0] ^= 0x01;
        });
        let err = unwrap(&wid(1), "seed", Some(&p), &tampered).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::WrongPassword),
            "got {err:?}"
        );
    }

    /// TC-009 — wire-flip of nonce[0] yields WrongPassword.
    #[test]
    fn wire_flip_nonce_fails_closed() {
        let p = pw("pw");
        let blob = scheme1_blob(&p);
        let tampered = mutate_scheme1(&blob, |_, _, nonce| {
            nonce[0] ^= 0x01;
        });
        let err = unwrap(&wid(1), "seed", Some(&p), &tampered).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::WrongPassword),
            "got {err:?}"
        );
    }

    /// TC-010 — re-binding the unwrap to a different wallet_id rejects.
    #[test]
    fn relocation_across_wallet_id_rejected() {
        let p = pw("pw");
        let blob = wrap_with_params(
            &wid(0xA),
            "seed",
            Some(&p),
            b"seed",
            KdfParams::floor_target(),
        )
        .unwrap();
        let err = unwrap(&wid(0xB), "seed", Some(&p), blob.expose_secret()).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::WrongPassword),
            "got {err:?}"
        );
    }

    /// TC-011 — re-binding the unwrap to a different label rejects.
    #[test]
    fn relocation_across_label_rejected() {
        let p = pw("pw");
        let blob = wrap_with_params(
            &wid(1),
            "labelA",
            Some(&p),
            b"seed",
            KdfParams::floor_target(),
        )
        .unwrap();
        let err = unwrap(&wid(1), "labelB", Some(&p), blob.expose_secret()).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::WrongPassword),
            "got {err:?}"
        );
    }

    /// TC-012 — wire-flip of envelope.version (via re-encode) is gated
    /// to UnsupportedEnvelopeVersion before AAD bind.
    #[test]
    fn wire_flip_version_rejected_pre_aad() {
        let blob = scheme1_blob(&pw("pw"));
        let mut env = decode(&blob);
        env.version = 2;
        let tampered = encode(&env);
        let err = unwrap(&wid(1), "seed", Some(&pw("pw")), &tampered).unwrap_err();
        assert!(
            matches!(
                err,
                SecretStoreError::UnsupportedEnvelopeVersion { found: 2 }
            ),
            "got {err:?}"
        );
    }

    /// TC-013 — forged `Payload::Unprotected` with ciphertext bytes +
    /// `Some(pw)` redirects to ExpectedProtectedButUnsealed.
    #[test]
    fn wire_flip_scheme_dispatch_redirects_safely() {
        let env = Envelope {
            version: ENVELOPE_VERSION,
            payload: Payload::Unprotected(vec![0xDEu8; 32]),
        };
        let blob = encode(&env);
        let err = unwrap(&wid(1), "seed", Some(&pw("pw")), &blob).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::ExpectedProtectedButUnsealed),
            "got {err:?}"
        );
    }

    /// TC-017 — truncated blob (< minimum envelope length) yields
    /// Corruption.
    #[test]
    fn truncated_blob_yields_corruption() {
        let blob = scheme1_blob(&pw("pw"));
        let cut = blob.len() / 2;
        let err = unwrap(&wid(1), "seed", Some(&pw("pw")), &blob[..cut]).unwrap_err();
        assert!(matches!(err, SecretStoreError::Corruption), "got {err:?}");
    }

    /// TC-018 — random-byte blob yields Corruption (both arms).
    #[test]
    fn random_garbage_yields_corruption() {
        let garbage = b"NOTANEVELOPE.........................";
        let err = unwrap(&wid(1), "seed", None, garbage).unwrap_err();
        assert!(matches!(err, SecretStoreError::Corruption), "got {err:?}");
        let err = unwrap(&wid(1), "seed", Some(&pw("pw")), garbage).unwrap_err();
        assert!(matches!(err, SecretStoreError::Corruption), "got {err:?}");
    }

    /// TC-019 — a manually-built envelope at version=2 fails closed
    /// regardless of password.
    #[test]
    fn unsupported_version_rejected_for_any_password() {
        let env = Envelope {
            version: 2,
            payload: Payload::Unprotected(b"x".to_vec()),
        };
        let blob = encode(&env);
        for arg in [None, Some(&pw("pw"))] {
            let err = unwrap(&wid(1), "seed", arg, &blob).unwrap_err();
            assert!(
                matches!(
                    err,
                    SecretStoreError::UnsupportedEnvelopeVersion { found: 2 }
                ),
                "got {err:?}"
            );
        }
    }

    /// A version above 255 must surface its FULL `u32`, not a truncated `u8`
    /// (`300 as u8 == 44` would alias a different version in diagnostics).
    #[test]
    fn unsupported_version_preserves_full_u32() {
        let env = Envelope {
            version: 300,
            payload: Payload::Unprotected(b"x".to_vec()),
        };
        let blob = encode(&env);
        let err = unwrap(&wid(1), "seed", None, &blob).unwrap_err();
        assert!(
            matches!(
                err,
                SecretStoreError::UnsupportedEnvelopeVersion { found: 300 }
            ),
            "version must not be truncated to u8, got {err:?}"
        );
    }

    /// TC-020 — a hand-crafted byte stream with an unknown payload
    /// enum tag yields Corruption (bincode's natural fail-closed).
    #[test]
    fn unknown_scheme_discriminant_yields_corruption() {
        // envelope.version = 1 (varint = 0x01) then a Payload enum tag
        // of 7 (varint = 0x07) — the two-variant enum decode rejects.
        let blob = [0x01u8, 0x07];
        let err = unwrap(&wid(1), "seed", None, &blob).unwrap_err();
        assert!(matches!(err, SecretStoreError::Corruption), "got {err:?}");
    }

    /// TC-021 — Some(pw) + scheme-0 yields ExpectedProtectedButUnsealed.
    #[test]
    fn some_pw_on_scheme0_fails_closed() {
        let blob = wrap_with_params(
            &wid(1),
            "seed",
            None,
            b"attacker-seed",
            KdfParams::floor_target(),
        )
        .unwrap();
        let err = unwrap(&wid(1), "seed", Some(&pw("pw")), blob.expose_secret()).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::ExpectedProtectedButUnsealed),
            "got {err:?}"
        );
    }

    /// TC-022 — None + scheme-1 yields NeedsPassword.
    #[test]
    fn none_pw_on_scheme1_yields_needs_password() {
        let blob = scheme1_blob(&pw("pw"));
        let err = unwrap(&wid(1), "seed", None, &blob).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::NeedsPassword),
            "got {err:?}"
        );
    }

    /// TC-023 — inflated KDF param rejected by `enforce_bounds` before
    /// `derive_key` allocates (a ~4 TiB allocation would OOM the test).
    #[test]
    fn kdf_enforce_bounds_rejects_before_derive() {
        let p = pw("pw");
        let blob = scheme1_blob(&p);
        let tampered = mutate_scheme1(&blob, |kdf, _, _| {
            kdf.m_kib = u32::MAX;
        });
        let err = unwrap(&wid(1), "seed", Some(&p), &tampered).unwrap_err();
        assert!(matches!(err, SecretStoreError::KdfFailure), "got {err:?}");

        let tampered = mutate_scheme1(&blob, |kdf, _, _| {
            kdf.t = ARGON2_MAX_T + 1;
        });
        let err = unwrap(&wid(1), "seed", Some(&p), &tampered).unwrap_err();
        assert!(matches!(err, SecretStoreError::KdfFailure), "got {err:?}");
    }

    /// TC-024 — the per-read ceiling rejects an envelope whose `m_kib`
    /// exceeds `ARGON2_READ_MAX_M_KIB` even when still inside
    /// `enforce_bounds`. Catches inflated headers BEFORE `derive_key`.
    #[test]
    fn per_read_ceiling_rejects_inflated_header() {
        let p = pw("pw");
        let blob = scheme1_blob(&p);
        let bumped = ARGON2_READ_MAX_M_KIB * 2;
        // Sanity: the bumped value stays inside the wider enforce_bounds
        // ceiling, so only the per-read gate can refuse it.
        assert!(bumped <= ARGON2_MAX_M_KIB);
        let tampered = mutate_scheme1(&blob, |kdf, _, _| {
            kdf.m_kib = bumped;
        });
        let err = unwrap(&wid(1), "seed", Some(&p), &tampered).unwrap_err();
        assert!(matches!(err, SecretStoreError::KdfFailure), "got {err:?}");
    }

    /// Sibling to TC-024 on the `t` axis — the per-read ceiling rejects
    /// an envelope whose `t` exceeds `ARGON2_READ_MAX_T` even when still
    /// inside `enforce_bounds` (`ARGON2_MAX_T = 16`). Closes the CPU-axis
    /// gap that would otherwise let a forged header run Argon2 at 5.3×
    /// the shipped iteration count.
    #[test]
    fn kdf_t_ceiling_fires_before_derive() {
        let p = pw("pw");
        let blob = scheme1_blob(&p);
        let bumped_t = ARGON2_READ_MAX_T + 1;
        // Sanity: the bumped t stays inside the wider enforce_bounds
        // ceiling, so only the per-read gate can refuse it.
        assert!(bumped_t <= ARGON2_MAX_T);
        let tampered = mutate_scheme1(&blob, |kdf, _, _| {
            // Keep m_kib at the ceiling so the m_kib gate cannot fire —
            // t must be the sole reason this rejects.
            kdf.m_kib = ARGON2_READ_MAX_M_KIB;
            kdf.t = bumped_t;
        });
        let err = unwrap(&wid(1), "seed", Some(&p), &tampered).unwrap_err();
        assert!(matches!(err, SecretStoreError::KdfFailure), "got {err:?}");
    }

    /// Every early return in `unwrap` routes through `Payload::wipe`, so
    /// the wipe itself is the thing worth pinning: a decoded scheme-0
    /// payload is the raw secret on an unguarded heap `Vec`.
    #[test]
    fn payload_wipe_clears_both_variants() {
        let mut unprotected = Payload::Unprotected(b"seed material".to_vec());
        unprotected.wipe();
        match &unprotected {
            Payload::Unprotected(bytes) => {
                assert_eq!(bytes.len(), b"seed material".len(), "length must survive");
                assert!(bytes.iter().all(|b| *b == 0), "plaintext survived the wipe");
            }
            Payload::Password { .. } => unreachable!("variant must not change"),
        }

        let mut protected = Payload::Password {
            kdf: KdfParamsEncoded::from(KdfParams::floor_target()),
            salt: [1u8; SALT_LEN],
            nonce: [2u8; NONCE_LEN],
            ciphertext: vec![0xAB; 32],
        };
        protected.wipe();
        match &protected {
            Payload::Password { ciphertext, .. } => {
                assert!(
                    ciphertext.iter().all(|b| *b == 0),
                    "ciphertext survived the wipe"
                );
            }
            Payload::Unprotected(_) => unreachable!("variant must not change"),
        }
    }

    /// Trailing bytes appended after a valid envelope are rejected as
    /// `Corruption` — defends against a truncation/extension probe.
    #[test]
    fn decode_rejects_trailing_garbage() {
        let p = pw("pw");
        let blob = scheme1_blob(&p);
        let mut extended = blob.clone();
        extended.extend_from_slice(&[0xFFu8; 16]);
        let err = unwrap(&wid(1), "seed", Some(&p), &extended).unwrap_err();
        assert!(matches!(err, SecretStoreError::Corruption), "got {err:?}");

        // The same blob without the suffix still unwraps cleanly —
        // proves the rejection is on the trailing bytes, not the
        // envelope itself.
        let ok = unwrap(&wid(1), "seed", Some(&p), &blob).unwrap();
        assert_eq!(ok.expose_secret(), b"seed");
    }

    /// TC-031 — round-tripped secret matches the original under a
    /// constant-time compare.
    #[test]
    fn round_trip_is_constant_time_equal() {
        let p = pw("pw");
        let original = SecretBytes::from_slice(b"seed material");
        let blob = wrap_with_params(
            &wid(1),
            "seed",
            Some(&p),
            original.expose_secret(),
            KdfParams::floor_target(),
        )
        .unwrap();
        let got = unwrap(&wid(1), "seed", Some(&p), blob.expose_secret()).unwrap();
        assert!(bool::from(got.ct_eq(&original)));
    }

    /// TC-035 (round-trip half) — scheme-1 at exact MAX_PLAINTEXT_LEN
    /// round-trips and the enveloped bytes fit the backend cap.
    #[test]
    fn scheme1_at_cap_round_trips_within_backend_cap() {
        let p = pw("pw");
        let pt = vec![0x5Au8; MAX_PLAINTEXT_LEN];
        let blob =
            wrap_with_params(&wid(1), "seed", Some(&p), &pt, KdfParams::floor_target()).unwrap();
        assert!(blob.len() <= MAX_SECRET_LEN);
        let got = unwrap(&wid(1), "seed", Some(&p), blob.expose_secret()).unwrap();
        assert_eq!(got.expose_secret(), &pt[..]);
    }

    /// TC-037 — scheme-0 decode rejects an oversize plaintext (> MAX_PLAINTEXT_LEN)
    /// even though the blob fits within DECODE_BUDGET.  A tampered blob that encodes
    /// a plaintext between MAX_PLAINTEXT_LEN+1 and DECODE_BUDGET would otherwise
    /// bypass the wrap-side cap on the decode path.
    #[test]
    fn scheme0_decode_rejects_oversize_plaintext() {
        // Build a scheme-0 envelope with plaintext = MAX_PLAINTEXT_LEN + 1 bytes.
        // encode_envelope bypasses the wrap-side cap (it is a raw encoder), so this
        // creates the exact tampered-blob scenario.
        let oversized = vec![0x5Au8; MAX_PLAINTEXT_LEN + 1];
        let env = Envelope {
            version: ENVELOPE_VERSION,
            payload: Payload::Unprotected(oversized.clone()),
        };
        let blob = encode(&env);
        // Confirm the blob fits within DECODE_BUDGET (otherwise the test proves nothing).
        assert!(
            blob.len() <= DECODE_BUDGET,
            "test blob must fit within DECODE_BUDGET to prove the plaintext check fires"
        );
        let err = unwrap(&wid(1), "seed", None, &blob).unwrap_err();
        assert!(
            matches!(
                err,
                SecretStoreError::SecretTooLarge { found, max }
                    if found == MAX_PLAINTEXT_LEN + 1 && max == MAX_PLAINTEXT_LEN
            ),
            "expected SecretTooLarge on oversized scheme-0 plaintext, got {err:?}"
        );
    }

    /// TC-038 — scheme-0 decode accepts a plaintext at exactly MAX_PLAINTEXT_LEN.
    #[test]
    fn scheme0_decode_accepts_at_cap_plaintext() {
        let at_cap = vec![0x5Au8; MAX_PLAINTEXT_LEN];
        let env = Envelope {
            version: ENVELOPE_VERSION,
            payload: Payload::Unprotected(at_cap.clone()),
        };
        let blob = encode(&env);
        let got = unwrap(&wid(1), "seed", None, &blob).unwrap();
        assert_eq!(got.expose_secret(), &at_cap[..]);
    }

    /// TC-036 — value rollback is intentionally NOT defended.
    #[test]
    fn value_rollback_is_not_defended() {
        let p = pw("pw");
        let old = wrap_with_params(
            &wid(1),
            "seed",
            Some(&p),
            b"OLD-VALUE",
            KdfParams::floor_target(),
        )
        .unwrap();
        let _new = wrap_with_params(
            &wid(1),
            "seed",
            Some(&p),
            b"NEW-VALUE",
            KdfParams::floor_target(),
        )
        .unwrap();
        let got = unwrap(&wid(1), "seed", Some(&p), old.expose_secret()).unwrap();
        assert_eq!(got.expose_secret(), b"OLD-VALUE");
    }

    /// TC-032 — random byte mutations and truncations never panic;
    /// every outcome is a permitted typed variant.
    #[test]
    fn fuzz_byte_mutation_and_truncation_never_panics() {
        let p = pw("fuzz-pw");
        let valid = scheme1_blob(&p);
        // Pristine envelope unwraps cleanly.
        assert_eq!(
            unwrap(&wid(1), "seed", Some(&p), &valid)
                .unwrap()
                .expose_secret(),
            b"seed"
        );

        let mut state: u32 = 0x9E37_79B9;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };

        let assert_typed = |arg: Option<&SecretString>, buf: &[u8]| {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                unwrap(&wid(1), "seed", arg, buf)
            }))
            .expect("unwrap must never panic on hostile input");
            match res {
                Ok(_)
                | Err(SecretStoreError::Corruption)
                | Err(SecretStoreError::WrongPassword)
                | Err(SecretStoreError::NeedsPassword)
                | Err(SecretStoreError::ExpectedProtectedButUnsealed)
                | Err(SecretStoreError::UnsupportedEnvelopeVersion { .. })
                | Err(SecretStoreError::KdfFailure)
                // A mutated scheme-0 blob can decode a plaintext Vec<u8> that
                // exceeds MAX_PLAINTEXT_LEN while still fitting DECODE_BUDGET.
                | Err(SecretStoreError::SecretTooLarge { .. }) => {}
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
            // Some path on a representative subset (each may derive).
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

    // TC-040 — proptest: no single-byte flip surfaces the plaintext.
    // Minimises to the offset that breaks coverage if one exists.
    proptest::proptest! {
        #[test]
        fn prop_single_byte_flip_never_yields_plaintext(
            (offset, mask) in (0usize..200usize, 1u8..=255u8),
        ) {
            // Re-built per case so the proptest harness can shrink
            // independently of the host RNG.
            let plaintext: &[u8] = b"goldfinch";
            let p = pw("pw");
            let valid = wrap_with_params(&wid(1), "seed", Some(&p), plaintext, KdfParams::floor_target())
                .unwrap()
                .expose_secret()
                .to_vec();
            if offset >= valid.len() {
                // Out-of-bounds offset → skip via prop_assume so proptest
                // shrinks toward in-bounds offsets.
                proptest::prop_assume!(offset < valid.len());
            }
            let mut buf = valid.clone();
            buf[offset] ^= mask;
            match unwrap(&wid(1), "seed", Some(&p), &buf) {
                Ok(secret) => {
                    proptest::prop_assert_ne!(
                        secret.expose_secret(),
                        plaintext,
                        "single-byte flip at offset {} surfaced the plaintext",
                        offset
                    );
                }
                Err(_) => { /* any typed error is fine */ }
            }
        }
    }
}
