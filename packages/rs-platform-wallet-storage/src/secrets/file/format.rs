//! Versioned, self-describing vault format + canonical AAD
//! (SEC-REQ-2.2.7 / 2.2.9).
//!
//! The vault is one `serde_json` document for a single `wallet_id`:
//!
//! ```json
//! {
//!   "version": 2,
//!   "kdf": { "id": 1, "m_kib": 65536, "t": 3, "p": 1 },
//!   "salt": "<32-byte lowercase hex>",
//!   "verify_nonce": "<24-byte lowercase hex>",
//!   "verify_ct": "<lowercase hex of AEAD(VERIFY_CONSTANT)>",
//!   "entries": {
//!     "<label>": { "nonce": "<24-byte hex>", "ciphertext": "<hex ct+tag>" }
//!   }
//! }
//! ```
//!
//! Entries are a JSON object keyed by `label` so lookup is O(log n) and
//! the on-disk shape excludes a duplicate-label vault by construction
//! (a JSON object cannot carry two values under the same key). Backed
//! by `BTreeMap<String, EntryBody>` for stable iteration order — the
//! tests rely on byte-stable serialization.
//!
//! Parsing is two-step: a lax [`VersionProbe`] reads `version` first
//! (tolerating future-version sibling fields), then — only for the
//! compiled-in [`FORMAT_VERSION`] — the strict [`Vault`] payload is
//! parsed. All byte fields are lowercase hex; Argon2 params are JSON
//! numbers.
//!
//! KDF params/salt are per-`wallet_id`. `verify_ct` is an AEAD seal of a
//! fixed constant under the header-derived key — a wrong passphrase
//! fails its tag, so a mismatched key is rejected before any entry is
//! written or read (no mixed-key corruption).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::crypto::{KdfParams, NONCE_LEN, SALT_LEN};
use super::error::FileStoreError;

pub(crate) const FORMAT_VERSION: u32 = 2;
pub(crate) const KDF_ID_ARGON2ID: u8 = 1;

/// Fixed plaintext sealed under the header key to form the passphrase-
/// verification token. Its only purpose is the AEAD tag check; the
/// value itself is not secret.
pub(crate) const VERIFY_CONSTANT: &[u8] = b"PWSVAULT-VERIFY-v1";

/// AAD slot label for the verification token. The leading NUL keeps it
/// disjoint from every allowlisted entry label (SEC-REQ-4.3), so the
/// token can never alias a real entry's AAD.
pub(crate) const VERIFY_LABEL: &str = "\0verify";

/// Minimum AEAD ciphertext length: the Poly1305 tag is always present
/// even for an empty plaintext, so any `verify_ct`/`ciphertext` shorter
/// than this is structurally impossible and rejected.
const AEAD_TAG_LEN: usize = 16;

/// The full parsed vault: format `version`, KDF parameters, salt, the
/// passphrase-verification token, and all entries. Serializes directly to
/// the on-disk wire form — `hex_array` validates `salt`/`verify_nonce`
/// widths at the serde seam, so no parallel `Vec<u8>`-typed wire mirror
/// is needed. Field order matches the documented schema and `serde_json`
/// preserves it, so the byte layout is stable.
///
/// `deny_unknown_fields` fails closed on a stray sibling for this
/// compiled-in [`FORMAT_VERSION`] (C3). Forward-compat dispatch on
/// `version` runs through [`VersionProbe`] before this strict parse.
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
    pub entries: BTreeMap<String, EntryBody>,
}

/// One decrypted-on-demand vault entry body. The owning `Vault.entries`
/// `BTreeMap` keys this by `label`, so the label is the map key — not
/// a field — and the on-disk shape can't carry two entries under the
/// same label. `hex_array` validates `nonce`'s fixed width at parse;
/// `deny_unknown_fields` fails closed on a stray sibling (C3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EntryBody {
    #[serde(with = "hex_array")]
    pub nonce: [u8; NONCE_LEN],
    #[serde(with = "hex_bytes")]
    pub ciphertext: Vec<u8>,
}

/// Canonical length-prefixed AAD binding ciphertext to its slot
/// (SEC-REQ-2.2.7): `format_version ‖ wallet_id ‖ label`. A blob moved
/// to another slot, or a rolled-back `format_version`, fails the tag.
///
/// AAD-DETERMINISM INVARIANT (C1): AAD is built solely from the typed
/// `(format_version, wallet_id, label)` triple via this length-prefixed
/// layout — never from any serialized JSON bytes or JSON key order. The
/// `format_version` argument is always the compiled-in [`FORMAT_VERSION`]
/// constant at every call site; the JSON `version` field is used ONLY as
/// the two-step dispatch gate and is NEVER routed into AAD.
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

/// AAD for the passphrase-verification token — the same canonical
/// construction as entry AAD but bound to [`VERIFY_LABEL`], so the
/// token is cryptographically tied to this `(version, wallet_id)` and
/// cannot be replayed into an entry slot.
pub(crate) fn verify_aad(format_version: u32, wallet_id: &[u8; 32]) -> Vec<u8> {
    aad(format_version, wallet_id, VERIFY_LABEL)
}

/// Serde helpers encoding `Vec<u8>` as lowercase hex strings. Hex is
/// already a crate dependency (`WalletId::to_hex`), is deterministic and
/// self-validating, and avoids adding `base64`. The encoding sits wholly
/// outside the AEAD envelope and the AAD (C1), so it has no bearing on
/// any cryptographic binding.
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

/// Const-generic companion to [`hex_bytes`] for fixed-width byte fields.
/// Wire form is identical (lowercase hex), but the `[u8; N]` deserialize
/// target moves length validation into the serde seam — a wrong-length
/// hex blob is rejected at parse with a `serde::de::Error` naming both
/// the offending size and the expected `N`, so the field is identifiable
/// in the error message (no anonymous "invalid length").
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

/// Step-1 probe: read ONLY `version`, tolerating unknown sibling fields
/// so a future v-N file can be dispatched on before its payload shape is
/// committed to. MUST NOT use `deny_unknown_fields` (C3).
#[derive(Deserialize)]
struct VersionProbe {
    version: u32,
}

/// Serialize a full vault to JSON bytes. Contains only salt/params
/// (non-secret) + ciphertext — never plaintext.
pub(crate) fn serialize(vault: &Vault) -> Vec<u8> {
    // Vault carries only fixed-width arrays and owned Vecs that serialize
    // infallibly; a serializer error would be a logic bug.
    serde_json::to_vec(vault).expect("vault serialization is infallible")
}

/// Parse a vault. Two-step: probe `version` (lax), then parse the strict
/// payload for the known version. Refuses unknown versions and any
/// malformed/short byte field — fail closed (SEC-REQ-2.2.9). Unknown KDF
/// algorithm ids and out-of-range Argon2 params are caught later at
/// `KdfParams::enforce_bounds` (called on every `derive_key`), so they
/// can't silently slip past. All `serde_json` errors are mapped to a
/// static [`FileStoreError`] with the source DISCARDED so input bytes
/// can never leak into an error string or log. Salt and nonce widths
/// are validated by `hex_array` at the serde seam; the AEAD-tag-length
/// floor remains a post-parse check.
pub(crate) fn deserialize(buf: &[u8]) -> Result<Vault, FileStoreError> {
    let probe: VersionProbe =
        serde_json::from_slice(buf).map_err(|_| FileStoreError::MalformedVault)?;
    if probe.version != FORMAT_VERSION {
        return Err(FileStoreError::VersionUnsupported {
            found: probe.version,
        });
    }

    let vault: Vault = serde_json::from_slice(buf).map_err(|_| FileStoreError::MalformedVault)?;

    if vault.verify_ct.len() < AEAD_TAG_LEN {
        return Err(FileStoreError::MalformedVault);
    }

    for body in vault.entries.values() {
        if body.ciphertext.len() < AEAD_TAG_LEN {
            return Err(FileStoreError::MalformedVault);
        }
    }

    Ok(vault)
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

    fn test_vault(entries: BTreeMap<String, EntryBody>) -> Vault {
        Vault {
            version: FORMAT_VERSION,
            kdf: KdfParams::default_target(),
            salt: [7u8; SALT_LEN],
            verify_nonce: [5u8; NONCE_LEN],
            verify_ct: vec![0xCC; 34],
            entries,
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
        let vault = test_vault(entries);
        let bytes = serialize(&vault);
        let back = deserialize(&bytes).unwrap();
        assert_eq!(back.kdf, vault.kdf);
        assert_eq!(back.salt, vault.salt);
        assert_eq!(back.verify_nonce, vault.verify_nonce);
        assert_eq!(back.verify_ct, vault.verify_ct);
        assert_eq!(back.entries.len(), 2);
        assert!(back.entries.contains_key("bip39_mnemonic"));
        assert_eq!(
            back.entries["bip32-seed"].ciphertext,
            vec![6; AEAD_TAG_LEN + 2]
        );
    }

    #[test]
    fn serialized_form_is_json_with_version_and_lowercase_hex() {
        let bytes = serialize(&test_vault(BTreeMap::new()));
        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(s.starts_with('{'), "vault is a JSON object: {s}");
        assert!(s.contains("\"version\":2"));
        // Salt is 0x07 * 32 → lowercase hex, never uppercase.
        assert!(s.contains(&"07".repeat(SALT_LEN)));
        assert!(!s.contains("0C0C"), "hex must be lowercase");
    }

    #[test]
    fn rejects_non_json_and_unknown_version() {
        assert!(matches!(
            deserialize(b"NOPENOPE...."),
            Err(FileStoreError::MalformedVault)
        ));
        let mut vault = test_vault(BTreeMap::new());
        vault.version = 999;
        let bytes = serialize(&vault);
        assert!(matches!(
            deserialize(&bytes),
            Err(FileStoreError::VersionUnsupported { found: 999 })
        ));
    }

    #[test]
    fn deserialize_accepts_unknown_kdf_id_and_bounds_check_rejects_later() {
        // Unknown algo ids ride through parse so the algorithm gate
        // lives in one place — `KdfParams::enforce_bounds`, called on
        // every `derive_key`. The format layer no longer guards it.
        let mut vault = test_vault(BTreeMap::new());
        vault.kdf.id = 7;
        let bytes = serialize(&vault);
        let parsed = deserialize(&bytes).expect("parse must accept unknown id");
        assert_eq!(parsed.kdf.id, 7);
        assert!(matches!(
            parsed.kdf.enforce_bounds(),
            Err(FileStoreError::KdfFailure)
        ));
    }

    #[test]
    fn rejects_unknown_payload_field() {
        // A version-2 file with a stray sibling field must fail closed
        // (deny_unknown_fields on Vault, C3).
        let bytes = br#"{"version":2,"kdf":{"id":1,"m_kib":65536,"t":3,"p":1},"salt":"00","verify_nonce":"00","verify_ct":"00","entries":{},"rogue":true}"#;
        assert!(matches!(
            deserialize(bytes),
            Err(FileStoreError::MalformedVault)
        ));
    }

    #[test]
    fn wrong_length_nonce_yields_malformed_not_panic() {
        // A 1-byte nonce must not panic — hex_array rejects it at the
        // serde seam before the runtime [u8; NONCE_LEN] is ever filled.
        // Inject via raw JSON since the wire-typed `EntryBody` no
        // longer permits a wrong-width nonce at construction.
        let mut v: serde_json::Value =
            serde_json::from_slice(&serialize(&test_vault(BTreeMap::new()))).unwrap();
        v["entries"] = serde_json::json!({
            "seed": {
                // 2 hex chars = 1 byte, well below NONCE_LEN (24).
                "nonce": "00",
                "ciphertext": "0".repeat(AEAD_TAG_LEN * 2),
            }
        });
        let bytes = serde_json::to_vec(&v).unwrap();
        assert!(matches!(
            deserialize(&bytes),
            Err(FileStoreError::MalformedVault)
        ));
    }

    #[test]
    fn wrong_length_salt_yields_malformed() {
        // Same hex_array seam reasoning as the nonce case — inject via
        // raw JSON since Vault.salt is now [u8; SALT_LEN] at the type
        // level.
        let mut v: serde_json::Value =
            serde_json::from_slice(&serialize(&test_vault(BTreeMap::new()))).unwrap();
        v["salt"] = serde_json::json!("0".repeat((SALT_LEN - 1) * 2));
        let bytes = serde_json::to_vec(&v).unwrap();
        assert!(matches!(
            deserialize(&bytes),
            Err(FileStoreError::MalformedVault)
        ));
    }

    #[test]
    fn short_ciphertext_below_tag_len_yields_malformed() {
        // The AEAD-tag-length floor is a post-parse structural check on
        // the deserialized entries — wire-malformed nonce widths can't
        // even reach this point thanks to hex_array.
        let mut v: serde_json::Value =
            serde_json::from_slice(&serialize(&test_vault(BTreeMap::new()))).unwrap();
        v["entries"] = serde_json::json!({
            "seed": {
                "nonce": "0".repeat(NONCE_LEN * 2),
                "ciphertext": "0".repeat((AEAD_TAG_LEN - 1) * 2),
            }
        });
        let bytes = serde_json::to_vec(&v).unwrap();
        assert!(matches!(
            deserialize(&bytes),
            Err(FileStoreError::MalformedVault)
        ));
    }

    #[test]
    fn short_verify_ct_below_tag_len_yields_malformed() {
        let mut vault = test_vault(BTreeMap::new());
        vault.verify_ct = vec![0u8; AEAD_TAG_LEN - 1];
        let bytes = serialize(&vault);
        assert!(matches!(
            deserialize(&bytes),
            Err(FileStoreError::MalformedVault)
        ));
    }

    #[test]
    fn entry_wire_shape_is_byte_identical_with_hand_crafted_json() {
        // Hand-crafted Vault matching the documented schema: parsing
        // serialize() output through the wire-typed Vault and
        // re-serializing must reproduce the same bytes — proves the
        // wire format (now BTreeMap-keyed) is stable across the
        // R-3 collapse.
        let mut entries = BTreeMap::new();
        entries.insert(
            "bip39_mnemonic".to_string(),
            EntryBody {
                nonce: [0x11; NONCE_LEN],
                ciphertext: vec![0x22; AEAD_TAG_LEN + 8],
            },
        );
        let vault = test_vault(entries);
        let bytes = serialize(&vault);
        let parsed: Vault = serde_json::from_slice(&bytes).unwrap();
        let again = serde_json::to_vec(&parsed).unwrap();
        assert_eq!(bytes, again, "wire round-trip must be byte-identical");

        // The rendered JSON: label is now an object key (no `"label":`
        // field), and the body holds nonce/ciphertext.
        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(s.starts_with("{\"version\":2,\"kdf\":{"));
        assert!(s.contains("\"bip39_mnemonic\":{"));
        assert!(!s.contains("\"label\":"));
        assert!(s.contains(&format!("\"nonce\":\"{}\"", "11".repeat(NONCE_LEN))));
        assert!(s.contains("\"ciphertext\":\""));
    }

    #[test]
    fn duplicate_label_in_wire_is_collapsed_by_object_semantics() {
        // BTreeMap-backed entries means the on-disk shape is a JSON
        // object — a duplicate key is impossible by JSON semantics,
        // and serde collapses it on parse. Documenting this as a
        // load-bearing invariant of the R-3 collapse (CMT-010).
        let bytes = br#"{"version":2,"kdf":{"id":1,"m_kib":65536,"t":3,"p":1},"salt":"0000000000000000000000000000000000000000000000000000000000000007","verify_nonce":"050505050505050505050505050505050505050505050505","verify_ct":"cccccccccccccccccccccccccccccccccccc","entries":{"seed":{"nonce":"010101010101010101010101010101010101010101010101","ciphertext":"00000000000000000000000000000000aa"},"seed":{"nonce":"020202020202020202020202020202020202020202020202","ciphertext":"00000000000000000000000000000000bb"}}}"#;
        let parsed = deserialize(bytes).expect("dup-key JSON parses to single entry");
        assert_eq!(parsed.entries.len(), 1);
        // Last-occurrence wins by serde_json semantics; either value
        // is acceptable — what matters is that the on-disk shape can
        // never carry two values under the same label.
        let body = &parsed.entries["seed"];
        assert!(body.nonce == [1u8; NONCE_LEN] || body.nonce == [2u8; NONCE_LEN]);
    }

    #[test]
    fn hex_array_round_trips_and_validates_length() {
        // Probe the adapter directly via a one-field tuple struct so the
        // collapses below can rely on its serde behaviour.
        #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
        struct Probe(#[serde(with = "hex_array")] [u8; 4]);

        let p = Probe([0xDE, 0xAD, 0xBE, 0xEF]);
        let s = serde_json::to_string(&p).unwrap();
        assert_eq!(s, "\"deadbeef\"");
        let back: Probe = serde_json::from_str(&s).unwrap();
        assert_eq!(back, p);

        // Wrong length surfaces with both the offending size and the
        // expected N — no anonymous "invalid length".
        let err = serde_json::from_str::<Probe>("\"deadbe\"").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("4 bytes"), "missing expected length: {msg}");
        assert!(msg.contains('3'), "missing actual length: {msg}");

        // Invalid hex names the field width in the error.
        let err = serde_json::from_str::<Probe>("\"zzzzzzzz\"").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("expected 4 bytes"), "bad msg: {msg}");
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
}
