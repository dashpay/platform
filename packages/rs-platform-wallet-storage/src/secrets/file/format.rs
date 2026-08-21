//! Versioned, self-describing vault format + canonical AAD.
//!
//! The vault is one `serde_json` document: a single salt / KDF block at
//! the top, then a map keyed by `wallet_id` (lowercase hex) and `label`.
//!
//! ```json
//! {
//!   "version": 1,
//!   "kdf": { "id": 1, "m_kib": 65536, "t": 3, "p": 1 },
//!   "salt": "<32-byte lowercase hex>",
//!   "verify_nonce": "<24-byte lowercase hex>",
//!   "verify_ct": "<lowercase hex of AEAD(VERIFY_CONSTANT)>",
//!   "wallets": {
//!     "<wallet-id-hex>": {
//!       "<label>": { "nonce": "<24-byte hex>", "ciphertext": "<hex ct+tag>" }
//!     }
//!   }
//! }
//! ```
//!
//! Nested `BTreeMap`s give O(log n) lookup and a JSON-object shape that
//! excludes duplicate `(wallet_id, label)` pairs by construction on the
//! WRITE side. On the READ side, a hand-edited document with duplicate JSON
//! keys is not rejected — `serde_json` collapses duplicates last-wins into
//! the `BTreeMap`. That is benign: every entry's ciphertext is AEAD-sealed
//! with its `(wallet_id, label)` bound as AAD, so a collapsed or reordered
//! structure can never surface bytes that don't authenticate against the
//! surviving key (a forged duplicate fails its tag as `Corruption`).
//!
//! Parsing is two-step: a lax [`VersionProbe`] reads `version` first
//! (tolerating future-version siblings), then the strict [`Vault`]
//! payload is parsed only for the compiled-in [`FORMAT_VERSION`].
//!
//! `verify_ct` is an AEAD seal of a fixed constant under the
//! header-derived key, so a wrong passphrase fails its tag and a
//! mismatched key is rejected before any entry is touched (no mixed-key
//! corruption). The verify-token AAD is not bound to any wallet id, so it
//! validates the store-wide passphrase once per op.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::crypto::{KdfParams, NONCE_LEN, SALT_LEN};
use crate::secrets::error::SecretStoreError;
use crate::secrets::wire::aad::{EntryAad, VerifyAad};
use crate::secrets::wire::config::{ENTRY_DOMAIN_V2, VERIFY_DOMAIN_V2, WIRE_CONFIG};
use crate::secrets::wire::kdf::KdfParamsEncoded;

pub(crate) const FORMAT_VERSION: u32 = 1;
pub(crate) const KDF_ID_ARGON2ID: u8 = 1;

/// Fixed plaintext sealed under the header key to form the passphrase-
/// verification token. Its only purpose is the AEAD tag check; the
/// value itself is not secret.
pub(crate) const VERIFY_CONSTANT: &[u8] = b"PWSVAULT-VERIFY-v1";

/// Minimum AEAD ciphertext length: the Poly1305 tag is always present
/// even for an empty plaintext, so any `verify_ct`/`ciphertext` shorter
/// than this is structurally impossible and rejected.
const AEAD_TAG_LEN: usize = 16;

/// The full parsed vault, serializing directly to the on-disk wire form.
/// `hex_array` validates fixed-width fields at the serde seam, and
/// `serde_json` preserves field order, so the byte layout is stable.
/// `deny_unknown_fields` fails closed on a stray sibling; forward-compat
/// dispatch runs through [`VersionProbe`] before this strict parse.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Vault {
    pub version: u32,
    pub kdf: KdfParams,
    #[serde(with = "hex_array")]
    pub salt: [u8; SALT_LEN],
    #[serde(with = "hex_array")]
    pub verify_nonce: [u8; NONCE_LEN],
    #[serde(with = "hex_bytes")]
    pub verify_ct: Vec<u8>,
    /// Outer key = `wallet_id` lowercase hex; inner key = `label`. A
    /// `BTreeMap` for both layers guarantees stable iteration order and
    /// the JSON-object shape that excludes duplicates by construction.
    pub wallets: BTreeMap<String, BTreeMap<String, EntryBody>>,
}

/// One vault entry body, keyed by `label` in the owning `BTreeMap` (so
/// the label is the map key, not a field). `hex_array` validates `nonce`'s
/// width at parse; `deny_unknown_fields` fails closed on a stray sibling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EntryBody {
    #[serde(with = "hex_array")]
    pub nonce: [u8; NONCE_LEN],
    #[serde(with = "hex_bytes")]
    pub ciphertext: Vec<u8>,
}

/// Canonical AAD binding a vault entry's ciphertext to its slot:
/// `domain ‖ format_version ‖ wallet_id ‖ label`, bincode-encoded
/// against [`WIRE_CONFIG`]. A blob moved to another slot, or one
/// version-rolled-back, fails the tag.
///
/// Determinism invariant: AAD is built solely from this typed triple,
/// never from serialized JSON bytes or key order. `format_version` is
/// always the compiled-in [`FORMAT_VERSION`]; the JSON `version` field
/// is a dispatch gate only and is never routed into AAD.
pub(crate) fn aad(format_version: u32, wallet_id: &[u8; 32], label: &str) -> Vec<u8> {
    bincode::encode_to_vec(
        EntryAad {
            domain: ENTRY_DOMAIN_V2,
            format_version,
            wallet_id: *wallet_id,
            label,
        },
        WIRE_CONFIG,
    )
    .expect("EntryAad encode is infallible")
}

/// AAD for the verify-token: bincode-encoded `VerifyAad` binding the
/// vault-wide salt + KDF header against the verify domain tag. A
/// tampered header yields a different AAD AND a different derived key,
/// so the token surfaces `WrongPassphrase`.
pub(crate) fn verify_aad(format_version: u32, salt: &[u8; SALT_LEN], kdf: &KdfParams) -> Vec<u8> {
    bincode::encode_to_vec(
        VerifyAad {
            domain: VERIFY_DOMAIN_V2,
            format_version,
            salt: *salt,
            kdf: KdfParamsEncoded::from(*kdf),
        },
        WIRE_CONFIG,
    )
    .expect("VerifyAad encode is infallible")
}

/// Serde helpers encoding `Vec<u8>` as lowercase hex. Hex is already a
/// crate dependency, deterministic, and avoids adding `base64`. The
/// encoding sits outside the AEAD envelope and the AAD, so it has no
/// cryptographic bearing.
mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(bytes))
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        hex::decode(&s).map_err(serde::de::Error::custom)
    }
}

/// Const-generic companion to [`hex_bytes`] for fixed-width fields. The
/// `[u8; N]` target moves length validation into the serde seam: a
/// wrong-length blob is rejected at parse with an error naming the
/// offending size and the expected `N`.
pub(super) mod hex_array {
    use serde::{de::Error as DeError, Deserialize, Deserializer, Serializer};

    pub(in crate::secrets::file) fn serialize<S, const N: usize>(
        bytes: &[u8; N],
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub(in crate::secrets::file) fn deserialize<'de, D, const N: usize>(
        deserializer: D,
    ) -> Result<[u8; N], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s)
            .map_err(|e| D::Error::custom(format!("invalid hex (expected {N} bytes): {e}")))?;
        if bytes.len() != N {
            let expected = format!("{N} bytes (hex-encoded)");
            return Err(D::Error::invalid_length(bytes.len(), &expected.as_str()));
        }
        let mut out = [0u8; N];
        out.copy_from_slice(&bytes);
        Ok(out)
    }
}

/// Step-1 probe: read ONLY `version`, tolerating unknown siblings so a
/// future vN file can be dispatched on. MUST NOT use `deny_unknown_fields`.
#[derive(Deserialize)]
struct VersionProbe {
    version: u32,
}

/// Serialize a vault to JSON bytes — salt/params + ciphertext only, never
/// plaintext.
pub(crate) fn serialize(vault: &Vault) -> Vec<u8> {
    // Vault holds only fixed arrays and owned Vecs; serialization is
    // infallible, so an error would be a logic bug.
    serde_json::to_vec(vault).expect("vault serialization is infallible")
}

/// Parse a vault: probe `version` (lax), then parse the strict payload
/// for the known version. Fails closed on unknown versions and malformed
/// fields. `serde_json` errors are mapped to a static
/// [`SecretStoreError`] with the source DISCARDED so input bytes never
/// leak. Unknown KDF ids / out-of-range Argon2 params are caught later at
/// `KdfParams::enforce_bounds`.
pub(crate) fn deserialize(buf: &[u8]) -> Result<Vault, SecretStoreError> {
    // INTENTIONAL: the 2x parse (probe + strict) over the 128MiB-capped,
    // lock-gated local file is accepted for forward-version dispatch.
    // INTENTIONAL: relies on serde_json's default recursion limit (128)
    // for deep-nesting DoS safety — MUST NOT disable it or use from_reader.
    let probe: VersionProbe =
        serde_json::from_slice(buf).map_err(|_| SecretStoreError::MalformedVault)?;
    if probe.version != FORMAT_VERSION {
        return Err(SecretStoreError::VersionUnsupported {
            found: probe.version,
        });
    }

    let vault: Vault = serde_json::from_slice(buf).map_err(|_| SecretStoreError::MalformedVault)?;

    if vault.verify_ct.len() < AEAD_TAG_LEN {
        return Err(SecretStoreError::MalformedVault);
    }

    // Validate wallet-id and label keys at parse: the serde shape allows
    // any string, so a bogus key would otherwise surface only at the
    // first put/get/delete. Reject the whole vault on the first offender.
    for (wallet_hex, entries) in &vault.wallets {
        super::decode_wallet_id_hex(wallet_hex)?;
        for (label, body) in entries {
            super::super::validate::validated_label(label)
                .map_err(|_| SecretStoreError::InvalidLabel)?;
            if body.ciphertext.len() < AEAD_TAG_LEN {
                return Err(SecretStoreError::MalformedVault);
            }
        }
    }

    Ok(vault)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_vault(wallets: BTreeMap<String, BTreeMap<String, EntryBody>>) -> Vault {
        Vault {
            version: FORMAT_VERSION,
            kdf: KdfParams::default_target(),
            salt: [7u8; SALT_LEN],
            verify_nonce: [5u8; NONCE_LEN],
            verify_ct: vec![0xCC; 34],
            wallets,
        }
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let mut entries = BTreeMap::new();
        entries.insert(
            "bip39_mnemonic".to_string(),
            EntryBody {
                nonce: [3u8; NONCE_LEN],
                ciphertext: vec![1; AEAD_TAG_LEN + 4],
            },
        );
        entries.insert(
            "bip32-seed".to_string(),
            EntryBody {
                nonce: [9u8; NONCE_LEN],
                ciphertext: vec![6; AEAD_TAG_LEN + 2],
            },
        );
        let mut wallets = BTreeMap::new();
        wallets.insert(hex::encode([1u8; 32]), entries);
        let vault = test_vault(wallets);
        let bytes = serialize(&vault);
        let back = deserialize(&bytes).unwrap();
        assert_eq!(back.kdf, vault.kdf);
        assert_eq!(back.salt, vault.salt);
        assert_eq!(back.verify_nonce, vault.verify_nonce);
        assert_eq!(back.verify_ct, vault.verify_ct);
        assert_eq!(back.wallets.len(), 1);
        let only = &back.wallets[&hex::encode([1u8; 32])];
        assert_eq!(only.len(), 2);
        assert!(only.contains_key("bip39_mnemonic"));
        assert_eq!(only["bip32-seed"].ciphertext, vec![6; AEAD_TAG_LEN + 2]);
    }

    #[test]
    fn serialized_form_is_json_with_version_and_lowercase_hex() {
        let bytes = serialize(&test_vault(BTreeMap::new()));
        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(s.starts_with('{'), "vault is a JSON object: {s}");
        assert!(s.contains("\"version\":1"));
        assert!(s.contains("\"wallets\":{}"));
        // Salt is 0x07 * 32 → lowercase hex, never uppercase.
        assert!(s.contains(&"07".repeat(SALT_LEN)));
        assert!(!s.contains("0C0C"), "hex must be lowercase");
    }

    #[test]
    fn rejects_non_json_and_unknown_version() {
        assert!(matches!(
            deserialize(b"NOPENOPE...."),
            Err(SecretStoreError::MalformedVault)
        ));
        let mut vault = test_vault(BTreeMap::new());
        vault.version = 999;
        let bytes = serialize(&vault);
        assert!(matches!(
            deserialize(&bytes),
            Err(SecretStoreError::VersionUnsupported { found: 999 })
        ));
    }

    #[test]
    fn deserialize_accepts_unknown_kdf_id_and_bounds_check_rejects_later() {
        // Unknown algo ids ride through parse; the gate lives solely at
        // `KdfParams::enforce_bounds` (called on every `derive_key`).
        let mut vault = test_vault(BTreeMap::new());
        vault.kdf.id = 7;
        let bytes = serialize(&vault);
        let parsed = deserialize(&bytes).expect("parse must accept unknown id");
        assert_eq!(parsed.kdf.id, 7);
        assert!(matches!(
            parsed.kdf.enforce_bounds(),
            Err(SecretStoreError::KdfFailure)
        ));
    }

    #[test]
    fn rejects_unknown_payload_field() {
        // A version-1 file with a stray sibling field must fail closed
        // (deny_unknown_fields on Vault, C3).
        let bytes = br#"{"version":1,"kdf":{"id":1,"m_kib":65536,"t":3,"p":1},"salt":"00","verify_nonce":"00","verify_ct":"00","wallets":{},"rogue":true}"#;
        assert!(matches!(
            deserialize(bytes),
            Err(SecretStoreError::MalformedVault)
        ));
    }

    #[test]
    fn wrong_length_nonce_yields_malformed_not_panic() {
        // A 1-byte nonce must not panic — hex_array rejects it at the
        // serde seam before the runtime [u8; NONCE_LEN] is ever filled.
        let mut v: serde_json::Value =
            serde_json::from_slice(&serialize(&test_vault(BTreeMap::new()))).unwrap();
        v["wallets"] = serde_json::json!({
            hex::encode([1u8; 32]): {
                "seed": {
                    // 2 hex chars = 1 byte, well below NONCE_LEN (24).
                    "nonce": "00",
                    "ciphertext": "0".repeat(AEAD_TAG_LEN * 2),
                }
            }
        });
        let bytes = serde_json::to_vec(&v).unwrap();
        assert!(matches!(
            deserialize(&bytes),
            Err(SecretStoreError::MalformedVault)
        ));
    }

    #[test]
    fn wrong_length_salt_yields_malformed() {
        let mut v: serde_json::Value =
            serde_json::from_slice(&serialize(&test_vault(BTreeMap::new()))).unwrap();
        v["salt"] = serde_json::json!("0".repeat((SALT_LEN - 1) * 2));
        let bytes = serde_json::to_vec(&v).unwrap();
        assert!(matches!(
            deserialize(&bytes),
            Err(SecretStoreError::MalformedVault)
        ));
    }

    #[test]
    fn short_ciphertext_below_tag_len_yields_malformed() {
        // The AEAD-tag-length floor is a post-parse structural check on
        // the deserialized entries — wire-malformed nonce widths can't
        // even reach this point thanks to hex_array.
        let mut v: serde_json::Value =
            serde_json::from_slice(&serialize(&test_vault(BTreeMap::new()))).unwrap();
        v["wallets"] = serde_json::json!({
            hex::encode([1u8; 32]): {
                "seed": {
                    "nonce": "0".repeat(NONCE_LEN * 2),
                    "ciphertext": "0".repeat((AEAD_TAG_LEN - 1) * 2),
                }
            }
        });
        let bytes = serde_json::to_vec(&v).unwrap();
        assert!(matches!(
            deserialize(&bytes),
            Err(SecretStoreError::MalformedVault)
        ));
    }

    #[test]
    fn short_verify_ct_below_tag_len_yields_malformed() {
        let mut vault = test_vault(BTreeMap::new());
        vault.verify_ct = vec![0u8; AEAD_TAG_LEN - 1];
        let bytes = serialize(&vault);
        assert!(matches!(
            deserialize(&bytes),
            Err(SecretStoreError::MalformedVault)
        ));
    }

    #[test]
    fn entry_wire_shape_is_byte_identical_with_hand_crafted_json() {
        // Parsing serialize() output through the wire-typed Vault and
        // re-serializing must reproduce the same bytes — proves the
        // nested-map wire format is stable.
        let mut entries = BTreeMap::new();
        entries.insert(
            "bip39_mnemonic".to_string(),
            EntryBody {
                nonce: [0x11; NONCE_LEN],
                ciphertext: vec![0x22; AEAD_TAG_LEN + 8],
            },
        );
        let mut wallets = BTreeMap::new();
        wallets.insert(hex::encode([0xAAu8; 32]), entries);
        let vault = test_vault(wallets);
        let bytes = serialize(&vault);
        let parsed: Vault = serde_json::from_slice(&bytes).unwrap();
        let again = serde_json::to_vec(&parsed).unwrap();
        assert_eq!(bytes, again, "wire round-trip must be byte-identical");

        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(s.starts_with("{\"version\":1,\"kdf\":{"));
        assert!(s.contains(&format!("\"{}\"", "aa".repeat(32))));
        assert!(s.contains("\"bip39_mnemonic\":{"));
        assert!(s.contains(&format!("\"nonce\":\"{}\"", "11".repeat(NONCE_LEN))));
        assert!(s.contains("\"ciphertext\":\""));
    }

    #[test]
    fn duplicate_label_in_wire_is_collapsed_by_object_semantics() {
        // BTreeMap-backed entries means the on-disk shape is a JSON
        // object — a duplicate key is impossible by JSON semantics,
        // and serde collapses it on parse.
        let wid_hex = hex::encode([1u8; 32]);
        let bytes = format!(
            r#"{{"version":1,"kdf":{{"id":1,"m_kib":65536,"t":3,"p":1}},"salt":"0000000000000000000000000000000000000000000000000000000000000007","verify_nonce":"050505050505050505050505050505050505050505050505","verify_ct":"cccccccccccccccccccccccccccccccccccc","wallets":{{"{wid_hex}":{{"seed":{{"nonce":"010101010101010101010101010101010101010101010101","ciphertext":"00000000000000000000000000000000aa"}},"seed":{{"nonce":"020202020202020202020202020202020202020202020202","ciphertext":"00000000000000000000000000000000bb"}}}}}}}}"#
        );
        let parsed = deserialize(bytes.as_bytes()).expect("dup-key JSON parses to single entry");
        let wallet = &parsed.wallets[&wid_hex];
        assert_eq!(wallet.len(), 1);
        let body = &wallet["seed"];
        assert!(body.nonce == [1u8; NONCE_LEN] || body.nonce == [2u8; NONCE_LEN]);
    }

    #[test]
    fn hex_array_round_trips_and_validates_length() {
        #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
        struct Probe(#[serde(with = "hex_array")] [u8; 4]);

        let p = Probe([0xDE, 0xAD, 0xBE, 0xEF]);
        let s = serde_json::to_string(&p).unwrap();
        assert_eq!(s, "\"deadbeef\"");
        let back: Probe = serde_json::from_str(&s).unwrap();
        assert_eq!(back, p);

        let err = serde_json::from_str::<Probe>("\"deadbe\"").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("4 bytes"), "missing expected length: {msg}");
        assert!(msg.contains('3'), "missing actual length: {msg}");

        let err = serde_json::from_str::<Probe>("\"zzzzzzzz\"").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("expected 4 bytes"), "bad msg: {msg}");
    }

    #[test]
    fn deserialize_rejects_non_hex_wallet_id_key() {
        // A non-hex outer key must be rejected at parse, not surface
        // later at put/get/delete.
        let mut entries = BTreeMap::new();
        entries.insert(
            "seed".to_string(),
            EntryBody {
                nonce: [0u8; NONCE_LEN],
                ciphertext: vec![0xCC; AEAD_TAG_LEN],
            },
        );
        let mut wallets = BTreeMap::new();
        wallets.insert("not-a-hex".to_string(), entries);
        let bytes = serialize(&test_vault(wallets));
        assert!(matches!(
            deserialize(&bytes),
            Err(SecretStoreError::MalformedVault)
        ));
    }

    #[test]
    fn deserialize_rejects_short_wallet_id_key() {
        // 32-hex chars is half the required width; reject at parse.
        let mut entries = BTreeMap::new();
        entries.insert(
            "seed".to_string(),
            EntryBody {
                nonce: [0u8; NONCE_LEN],
                ciphertext: vec![0xCC; AEAD_TAG_LEN],
            },
        );
        let mut wallets = BTreeMap::new();
        wallets.insert("ab".repeat(16), entries);
        let bytes = serialize(&test_vault(wallets));
        assert!(matches!(
            deserialize(&bytes),
            Err(SecretStoreError::MalformedVault)
        ));
    }

    #[test]
    fn deserialize_rejects_traversal_label() {
        // A label that would not survive `validated_label` (path
        // traversal attempt) must fail at parse, not at the first get.
        let mut entries = BTreeMap::new();
        entries.insert(
            "../escape".to_string(),
            EntryBody {
                nonce: [0u8; NONCE_LEN],
                ciphertext: vec![0xCC; AEAD_TAG_LEN],
            },
        );
        let mut wallets = BTreeMap::new();
        wallets.insert(hex::encode([1u8; 32]), entries);
        let bytes = serialize(&test_vault(wallets));
        assert!(matches!(
            deserialize(&bytes),
            Err(SecretStoreError::InvalidLabel)
        ));
    }

    #[test]
    fn deserialize_rejects_oversize_label() {
        // 65 chars busts the 1..=64 allowlist bound.
        let mut entries = BTreeMap::new();
        entries.insert(
            "a".repeat(65),
            EntryBody {
                nonce: [0u8; NONCE_LEN],
                ciphertext: vec![0xCC; AEAD_TAG_LEN],
            },
        );
        let mut wallets = BTreeMap::new();
        wallets.insert(hex::encode([1u8; 32]), entries);
        let bytes = serialize(&test_vault(wallets));
        assert!(matches!(
            deserialize(&bytes),
            Err(SecretStoreError::InvalidLabel)
        ));
    }

    #[test]
    fn malformed_error_renders_no_input_bytes() {
        // A parse failure must never echo the offending input.
        let needle = "SUPERSECRETNEEDLE";
        let evil = format!("{{\"version\": \"{needle}\"}}");
        let err = deserialize(evil.as_bytes()).unwrap_err();
        let rendered = format!("{err} {err:?}");
        assert!(
            !rendered.contains(needle),
            "error leaked input bytes: {rendered}"
        );
    }

    /// A parse of mutated bytes must be a clean `Ok` or a typed error
    /// variant — never a panic / abort.
    fn assert_deserialize_outcome_is_typed(bytes: &[u8]) {
        let res = std::panic::catch_unwind(|| deserialize(bytes));
        let parsed = res.expect("deserialize must never panic on hostile input");
        match parsed {
            Ok(_)
            | Err(SecretStoreError::MalformedVault)
            | Err(SecretStoreError::VersionUnsupported { .. })
            | Err(SecretStoreError::InvalidLabel) => {}
            Err(other) => panic!("unexpected error variant from parser: {other:?}"),
        }
    }

    /// Deterministic byte-level fuzz: flip bytes and truncate at every
    /// offset of a valid vault, asserting the parser stays fail-closed and
    /// never panics. Fixed seed, no proptest dependency.
    #[test]
    fn parser_is_fuzz_resistant_to_byte_mutation() {
        let mut entries = BTreeMap::new();
        entries.insert(
            "bip39_mnemonic".to_string(),
            EntryBody {
                nonce: [0x33; NONCE_LEN],
                ciphertext: vec![0x44; AEAD_TAG_LEN + 16],
            },
        );
        let mut wallets = BTreeMap::new();
        wallets.insert(hex::encode([0xABu8; 32]), entries);
        let valid = serialize(&test_vault(wallets));

        // The pristine vault parses.
        assert!(deserialize(&valid).is_ok());

        // xorshift32 — deterministic, std-only.
        let mut state: u32 = 0x1234_5678;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };

        for _ in 0..2_000 {
            let mut buf = valid.clone();
            // Flip 1..=4 random bytes.
            let flips = 1 + (next() % 4) as usize;
            for _ in 0..flips {
                let idx = (next() as usize) % buf.len();
                buf[idx] ^= (next() & 0xFF) as u8;
            }
            assert_deserialize_outcome_is_typed(&buf);
        }

        // Truncation at every offset — a short read must never panic.
        for cut in 0..valid.len() {
            assert_deserialize_outcome_is_typed(&valid[..cut]);
        }
    }

    /// Structural fuzz: hostile shapes a byte-flip rarely hits (oversized
    /// KDF params, deep nesting, bad labels, wrong-width hex). Each must be
    /// a typed error or a valid Ok, never a panic. Inflated KDF params
    /// parse Ok by design (the bounds gate lives at `derive_key`).
    #[test]
    fn parser_is_fuzz_resistant_to_structural_mutation() {
        let base: serde_json::Value =
            serde_json::from_slice(&serialize(&test_vault(BTreeMap::new()))).unwrap();
        let wid_owned = hex::encode([1u8; 32]);
        let wid = wid_owned.as_str();
        let good_nonce = "0".repeat(NONCE_LEN * 2);
        let good_ct = "0".repeat((AEAD_TAG_LEN + 1) * 2);

        let mut cases: Vec<serde_json::Value> = Vec::new();

        // Oversized / absurd KDF params.
        for (k, v) in [
            ("m_kib", serde_json::json!(u32::MAX)),
            ("t", serde_json::json!(u32::MAX)),
            ("p", serde_json::json!(u32::MAX)),
            ("id", serde_json::json!(255)),
        ] {
            let mut c = base.clone();
            c["kdf"][k] = v;
            cases.push(c);
        }

        // Deep nesting in the wallets map (well past the type's depth).
        {
            let mut nested = serde_json::json!(0);
            for _ in 0..512 {
                nested = serde_json::json!([nested]);
            }
            let mut c = base.clone();
            c["wallets"] = nested;
            cases.push(c);
        }

        // Hostile labels and key shapes.
        for label in ["\0null", "../escape", &"a".repeat(65), "has space"] {
            let mut c = base.clone();
            c["wallets"] = serde_json::json!({ wid: { label: { "nonce": good_nonce.as_str(), "ciphertext": good_ct.as_str() } } });
            cases.push(c);
        }

        // Wrong-width hex and oversized declared sizes.
        for (nonce, ct) in [
            ("00", good_ct.as_str()),                       // short nonce
            (good_nonce.as_str(), "00"),                    // short ciphertext
            (&"0".repeat(NONCE_LEN * 4), good_ct.as_str()), // over-wide nonce
            ("zz", good_ct.as_str()),                       // non-hex nonce
        ] {
            let mut c = base.clone();
            c["wallets"] =
                serde_json::json!({ wid: { "seed": { "nonce": nonce, "ciphertext": ct } } });
            cases.push(c);
        }

        // Non-hex / wrong-length outer wallet-id key.
        for bad_wid in ["not-hex", &"aa".repeat(8), &"AB".repeat(32)] {
            let mut c = base.clone();
            c["wallets"] = serde_json::json!({ bad_wid: { "seed": { "nonce": good_nonce.as_str(), "ciphertext": good_ct.as_str() } } });
            cases.push(c);
        }

        // Header fields (salt / verify_nonce / verify_ct): empty / short /
        // over-wide / non-hex must each be a typed error, never a panic.
        let over_wide = "0".repeat(SALT_LEN * 4);
        for field in ["salt", "verify_nonce", "verify_ct"] {
            for bad in ["", "00", over_wide.as_str(), "zz"] {
                let mut c = base.clone();
                c[field] = serde_json::json!(bad);
                cases.push(c);
            }
        }

        for c in cases {
            let bytes = serde_json::to_vec(&c).unwrap();
            assert_deserialize_outcome_is_typed(&bytes);
        }
    }
}
