//! Tier-2 envelope wire format — bincode-encoded `Envelope` / `Payload`
//! plus the [`wrap`] / [`wrap_with_params`] / [`unwrap`] API.
//!
//! Encoder lives here; the decoder (and the strict fail-closed dispatch
//! table) is filled in by T-3. Every byte that crosses the AEAD seam is
//! produced by `bincode::encode_to_vec` against [`WIRE_CONFIG`], so a
//! future config drift surfaces in the golden-vector tests, not in
//! silently corrupted blobs.

use crate::secrets::error::SecretStoreError;
use crate::secrets::file::crypto::{self, KdfParams, NONCE_LEN, SALT_LEN};
use crate::secrets::secret::{SecretBytes, SecretString};
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

/// Wrap `plaintext` for `(wallet_id, label)` using the shipped default
/// Argon2 target when a password is supplied.
///
/// `None` → an unprotected (scheme-0) envelope; `Some(pw)` → a scheme-1
/// envelope sealed under `pw`. A blank password is rejected at enrol
/// (`SecretStoreError::BlankPassphrase`).
///
/// Returns the envelope inside a zeroizing [`SecretBytes`].
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

/// [`wrap`] with explicit Argon2 `params` (tests use floor params for
/// speed). `params` is ignored when `password` is `None`.
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
        let envelope = Envelope {
            version: ENVELOPE_VERSION,
            payload: Payload::Unprotected(plaintext.to_vec()),
        };
        return Ok(SecretBytes::new(encode_envelope(&envelope)));
    };

    // Reject a blank object password BEFORE any salt / derive.
    if pw.is_blank() {
        return Err(SecretStoreError::BlankPassphrase);
    }

    let mut salt = [0u8; SALT_LEN];
    crypto::random_bytes(&mut salt)?;
    let key = crypto::derive_key(pw, &salt, params)?;
    let kdf = KdfParamsEncoded::from(params);
    let aad = encode_tier2_aad(wallet_id, label, kdf, &salt);
    let (nonce, ciphertext) = crypto::seal(&key, &aad, plaintext)?;

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
/// the encoder and the (T-3) decoder so the two cannot disagree.
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
    if pw.is_blank() {
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
    use crate::secrets::file::crypto::{ARGON2_MIN_M_KIB, ARGON2_MIN_T, ARGON2_P};
    use crate::secrets::file::format::KDF_ID_ARGON2ID;

    /// Captured once from the runtime encoder; a subsequent CI failure
    /// here means a wire-format drift to investigate, NOT to "fix" by
    /// re-generating the constant.
    ///
    /// Decoding: 0x01 envelope.version=1, 0x00 Payload::Unprotected,
    /// 0x05 Vec<u8> length=5, "hello".
    const SCHEME0_GOLDEN_HEX: &str = "01000568656c6c6f";

    /// scheme-1 deterministic golden: wid=[0;32], label="seed",
    /// pw="pw", plaintext="hello", floor params, salt=[0x11;32],
    /// nonce=[0x22;24]. Bytes: version + Payload::Password tag +
    /// kdf(id,m_kib,t,p as varints) + salt[32] + nonce[24] +
    /// ciphertext-with-tag length + ciphertext+tag(21B).
    const SCHEME1_GOLDEN_HEX: &str = "010101fb4c000201111111111111111111111111111111111111111111111111111111111111111122222222222222222222222222222222222222222222222215e2ffdf3f0476b6bfb99b4f71b3039ff965132b92f0";

    fn wid(b: u8) -> WalletId {
        WalletId::from([b; 32])
    }

    fn pw(s: &str) -> SecretString {
        SecretString::new(s)
    }

    fn floor() -> KdfParams {
        KdfParams {
            id: KDF_ID_ARGON2ID,
            m_kib: ARGON2_MIN_M_KIB,
            t: ARGON2_MIN_T,
            p: ARGON2_P,
        }
    }

    /// TC-033 — blank object password rejected at enrol (wrap-side).
    /// The unwrap-side blank-pw guard lives on a sibling branch.
    #[test]
    fn blank_object_password_rejected_at_wrap() {
        for blank in [SecretString::empty(), pw(""), pw("   "), pw("\t\n")] {
            let err =
                wrap_with_params(&wid(1), "seed", Some(&blank), b"seed", floor()).unwrap_err();
            assert!(
                matches!(err, SecretStoreError::BlankPassphrase),
                "got {err:?}"
            );
        }
    }

    /// TC-034 — plaintext cap accept at MAX_PLAINTEXT_LEN, reject at
    /// +1, for both schemes.
    #[test]
    fn plaintext_cap_accept_then_reject() {
        let at_cap = vec![0x5Au8; MAX_PLAINTEXT_LEN];
        let over = vec![0x5Au8; MAX_PLAINTEXT_LEN + 1];

        // Scheme 0
        assert!(wrap(&wid(1), "seed", None, &at_cap).is_ok());
        assert!(matches!(
            wrap(&wid(1), "seed", None, &over).unwrap_err(),
            SecretStoreError::SecretTooLarge { found, max }
                if found == MAX_PLAINTEXT_LEN + 1 && max == MAX_PLAINTEXT_LEN
        ));

        // Scheme 1 — cap check fires before any derivation.
        let p = pw("pw");
        assert!(matches!(
            wrap_with_params(&wid(1), "seed", Some(&p), &over, floor()).unwrap_err(),
            SecretStoreError::SecretTooLarge { found, max }
                if found == MAX_PLAINTEXT_LEN + 1 && max == MAX_PLAINTEXT_LEN
        ));

        // Scheme-0 enveloped bytes for an at-cap plaintext fit the backend cap.
        let enveloped = wrap(&wid(1), "seed", None, &at_cap).unwrap();
        assert!(enveloped.len() <= MAX_SECRET_LEN);
    }

    /// TC-035 (size-budget half) — scheme-1 accepts plaintext at the
    /// exact MAX_PLAINTEXT_LEN boundary; the enveloped bytes fit the
    /// backend cap. The round-trip half lands in T-3.
    #[test]
    fn scheme1_at_cap_envelope_fits_backend_cap() {
        let p = pw("pw");
        let pt = vec![0x5Au8; MAX_PLAINTEXT_LEN];
        let blob = wrap_with_params(&wid(1), "seed", Some(&p), &pt, floor()).unwrap();
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
        let blob = wrap(&WalletId::from([0u8; 32]), "seed", None, b"hello").unwrap();
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
            floor(),
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
            floor(),
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
}
