//! Bincode-encoded AAD structs for the three contexts that authenticate
//! ciphertexts under `secrets/`: Tier-2 scheme-1 envelopes, vault entry
//! bodies, and the vault passphrase-verify token.
//!
//! Each struct is `Encode`-only — AAD is producer-side; the decoder
//! re-builds it from the surrounding context and bincode-encodes again
//! against [`WIRE_CONFIG`]. Pair-wise byte disjointness is guaranteed by
//! the three domain constants declared in [`super::config`] and pinned
//! empirically by the tests `tier2_and_entry_aad_byte_disjoint`,
//! `tier2_and_verify_aad_byte_disjoint`, and
//! `entry_and_verify_aad_byte_disjoint`.

use crate::secrets::file::crypto::SALT_LEN;
use crate::secrets::wire::kdf::KdfParamsEncoded;

/// AAD bound into every scheme-1 (password-protected) Tier-2 envelope.
/// Binds object identity (`wallet_id` + `label`) + header
/// (`envelope_version`, `scheme_discriminant`, `kdf`, `salt`) so any
/// in-place edit of those fields fails the AEAD tag.
///
/// `scheme_discriminant` is explicit (not inferred from a Rust enum
/// variant tag) so the AAD shape is stable under a future `Payload`
/// re-ordering.
#[derive(bincode::Encode)]
pub(crate) struct Tier2Aad<'a> {
    /// Domain tag — `TIER2_DOMAIN_V2`. Length-prefixed by bincode and
    /// byte-disjoint from `ENTRY_DOMAIN_V2` / `VERIFY_DOMAIN_V2` by
    /// content past the common prefix; pinned by the disjointness tests
    /// in [`super::aad::tests`].
    pub domain: &'static [u8],
    /// Envelope wire version (`ENVELOPE_VERSION`).
    pub envelope_version: u32,
    /// `0 = Unprotected`, `1 = Password`. Authenticates the scheme byte
    /// independently of the enum's bincode-derived tag.
    pub scheme_discriminant: u8,
    /// The exact bytes encoded into the envelope's `Payload::Password`
    /// body — AAD == body, so a wire-edited KDF header fails the tag.
    pub kdf: KdfParamsEncoded,
    /// Per-wrap CSPRNG salt.
    pub salt: [u8; SALT_LEN],
    /// 32-byte wallet correlation id (public, not secret).
    pub wallet_id: [u8; 32],
    /// Caller-allowlisted slot label.
    pub label: &'a str,
}

/// AAD bound into every vault entry's AEAD seal. Replaces the
/// hand-rolled `format::aad()` byte concatenation; binds slot identity
/// (`wallet_id` + `label`) at a stable `format_version`. A relocated
/// or version-rolled-back blob fails the tag.
#[derive(bincode::Encode)]
pub(crate) struct EntryAad<'a> {
    /// Domain tag — `ENTRY_DOMAIN_V2`.
    pub domain: &'static [u8],
    /// Vault `FORMAT_VERSION` (the compiled-in dispatch version,
    /// never the parsed JSON version).
    pub format_version: u32,
    /// 32-byte wallet correlation id.
    pub wallet_id: [u8; 32],
    /// Caller-allowlisted slot label.
    pub label: &'a str,
}

/// AAD bound into the vault passphrase-verify token's AEAD seal.
/// Binds salt + KDF header so a flipped salt or KDF-param shift fails
/// the token tag (surfaces as `WrongPassphrase` — a tampered header
/// also yields a different derived key).
#[derive(bincode::Encode)]
pub(crate) struct VerifyAad {
    /// Domain tag — `VERIFY_DOMAIN_V2`.
    pub domain: &'static [u8],
    /// Vault `FORMAT_VERSION`.
    pub format_version: u32,
    /// Vault-wide CSPRNG salt.
    pub salt: [u8; SALT_LEN],
    /// Vault-wide KDF parameters (the same wire image used by every
    /// scheme-1 Tier-2 envelope).
    pub kdf: KdfParamsEncoded,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::file::crypto::KdfParams;
    use crate::secrets::wire::config::{
        ENTRY_DOMAIN_V2, TIER2_DOMAIN_V2, VERIFY_DOMAIN_V2, WIRE_CONFIG,
    };

    fn floor_kdf() -> KdfParamsEncoded {
        KdfParamsEncoded::from(KdfParams::default_target())
    }

    fn tier2_with_domain(domain: &'static [u8]) -> Vec<u8> {
        let aad = Tier2Aad {
            domain,
            envelope_version: 1,
            scheme_discriminant: 1,
            kdf: floor_kdf(),
            salt: [0x77u8; SALT_LEN],
            wallet_id: [0x11u8; 32],
            label: "seed",
        };
        bincode::encode_to_vec(aad, WIRE_CONFIG).unwrap()
    }

    fn tier2_with_version(envelope_version: u32) -> Vec<u8> {
        let aad = Tier2Aad {
            domain: TIER2_DOMAIN_V2,
            envelope_version,
            scheme_discriminant: 1,
            kdf: floor_kdf(),
            salt: [0x77u8; SALT_LEN],
            wallet_id: [0x11u8; 32],
            label: "seed",
        };
        bincode::encode_to_vec(aad, WIRE_CONFIG).unwrap()
    }

    fn tier2_with_scheme(scheme_discriminant: u8) -> Vec<u8> {
        let aad = Tier2Aad {
            domain: TIER2_DOMAIN_V2,
            envelope_version: 1,
            scheme_discriminant,
            kdf: floor_kdf(),
            salt: [0x77u8; SALT_LEN],
            wallet_id: [0x11u8; 32],
            label: "seed",
        };
        bincode::encode_to_vec(aad, WIRE_CONFIG).unwrap()
    }

    fn entry(format_version: u32, wallet_id: [u8; 32], label: &str) -> Vec<u8> {
        let aad = EntryAad {
            domain: ENTRY_DOMAIN_V2,
            format_version,
            wallet_id,
            label,
        };
        bincode::encode_to_vec(aad, WIRE_CONFIG).unwrap()
    }

    fn verify(salt: [u8; SALT_LEN], kdf: KdfParamsEncoded) -> Vec<u8> {
        let aad = VerifyAad {
            domain: VERIFY_DOMAIN_V2,
            format_version: 1,
            salt,
            kdf,
        };
        bincode::encode_to_vec(aad, WIRE_CONFIG).unwrap()
    }

    /// Two byte strings where neither is a prefix of the other.
    fn assert_prefix_disjoint(a: &[u8], b: &[u8]) {
        assert!(
            !a.starts_with(b) && !b.starts_with(a),
            "prefix containment: a.len={} b.len={}",
            a.len(),
            b.len()
        );
    }

    /// TC-014 — Tier2Aad.domain is bincode-encoded.
    #[test]
    fn tier2_aad_domain_field_binds_bytes() {
        let a = tier2_with_domain(TIER2_DOMAIN_V2);
        let b = tier2_with_domain(b"PWSEV-TIER2-AAD-vX");
        assert_ne!(a, b);
        assert_prefix_disjoint(&a, &b);
    }

    /// TC-015 — Tier2Aad.envelope_version is bincode-encoded.
    #[test]
    fn tier2_aad_envelope_version_field_binds_bytes() {
        assert_ne!(tier2_with_version(1), tier2_with_version(2));
    }

    /// TC-016 — Tier2Aad.scheme_discriminant is bincode-encoded and
    /// explicit (not inferred from a Rust enum tag).
    #[test]
    fn tier2_aad_scheme_discriminant_field_binds_bytes() {
        assert_ne!(tier2_with_scheme(0), tier2_with_scheme(1));
    }

    /// TC-025 — Tier2Aad and EntryAad are byte-disjoint at the prefix.
    #[test]
    fn tier2_and_entry_aad_byte_disjoint() {
        let t = tier2_with_domain(TIER2_DOMAIN_V2);
        let e = entry(1, [0x11u8; 32], "seed");
        assert_prefix_disjoint(&t, &e);
    }

    /// TC-026 — Tier2Aad and VerifyAad are byte-disjoint at the prefix.
    #[test]
    fn tier2_and_verify_aad_byte_disjoint() {
        let t = tier2_with_domain(TIER2_DOMAIN_V2);
        let v = verify([0x77u8; SALT_LEN], floor_kdf());
        assert_prefix_disjoint(&t, &v);
    }

    /// TC-027 — EntryAad and VerifyAad are byte-disjoint at the prefix.
    /// Now backed by an explicit domain constant on top of the existing
    /// VERIFY_LABEL leading-NUL trick at the `format.rs` call site.
    #[test]
    fn entry_and_verify_aad_byte_disjoint() {
        let e = entry(1, [0u8; 32], "\0verify");
        let v = verify([0x77u8; SALT_LEN], floor_kdf());
        assert_prefix_disjoint(&e, &v);
    }

    /// TC-037 — EntryAad binds (format_version, wallet_id, label) and
    /// the label encoding carries its length prefix (`"a"+"b"` vs
    /// `"ab"` are distinct).
    #[test]
    fn entry_aad_binds_format_version_wallet_id_and_label() {
        let base = entry(1, [1u8; 32], "a");
        assert_ne!(base, entry(2, [1u8; 32], "a"));
        assert_ne!(base, entry(1, [2u8; 32], "a"));
        assert_ne!(base, entry(1, [1u8; 32], "b"));
        // Length-prefix sanity: "ab" must not equal the concatenation of
        // the encoding of "a" with the literal byte `b`.
        let ab = entry(1, [1u8; 32], "ab");
        let mut a_plus_b = base.clone();
        a_plus_b.extend_from_slice(b"b");
        assert_ne!(ab, a_plus_b);
    }

    /// TC-038 — VerifyAad binds salt + KDF; identical inputs produce
    /// identical bytes (determinism).
    #[test]
    fn verify_aad_binds_salt_and_kdf_params() {
        let salt = [7u8; SALT_LEN];
        let kdf = floor_kdf();
        let base = verify(salt, kdf);
        let mut salt2 = salt;
        salt2[0] ^= 0x01;
        assert_ne!(base, verify(salt2, kdf));

        let kdf_mkib = KdfParamsEncoded {
            m_kib: kdf.m_kib / 2,
            ..kdf
        };
        assert_ne!(base, verify(salt, kdf_mkib));
        let kdf_t = KdfParamsEncoded {
            t: kdf.t - 1,
            ..kdf
        };
        assert_ne!(base, verify(salt, kdf_t));

        // Determinism: identical inputs ⇒ identical bytes.
        assert_eq!(base, verify(salt, kdf));
    }
}
