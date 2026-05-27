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
//!   "entries": [
//!     { "label": "...", "nonce": "<24-byte hex>", "ciphertext": "<hex ct+tag>" }
//!   ]
//! }
//! ```
//!
//! Parsing is two-step: a lax [`VersionProbe`] reads `version` first
//! (tolerating future-version sibling fields), then — only for the
//! compiled-in [`FORMAT_VERSION`] — the strict [`VaultFile`] payload is
//! parsed. All byte fields are lowercase hex; Argon2 params are JSON
//! numbers.
//!
//! KDF params/salt are per-`wallet_id`. `verify_ct` is an AEAD seal of a
//! fixed constant under the header-derived key — a wrong passphrase
//! fails its tag, so a mismatched key is rejected before any entry is
//! written or read (no mixed-key corruption).

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

/// Parsed header (KDF params + salt + passphrase-verification token).
#[derive(Debug, Clone)]
pub(crate) struct Header {
    pub params: KdfParams,
    pub salt: [u8; SALT_LEN],
    pub verify_nonce: [u8; NONCE_LEN],
    pub verify_ct: Vec<u8>,
}

/// One decrypted-on-demand vault entry.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub label: String,
    pub nonce: [u8; NONCE_LEN],
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

    // Wired up by R-1/R-2 collapses in the follow-up commits; the unit
    // test below exercises both functions today.
    #[allow(dead_code)]
    pub(in crate::secrets::file) fn serialize<S, const N: usize>(
        bytes: &[u8; N],
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    #[allow(dead_code)]
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

/// Step-2 strict payload for the compiled-in [`FORMAT_VERSION`]. Fails
/// closed on any unknown field (C3).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VaultFile {
    version: u32,
    kdf: KdfDescriptor,
    #[serde(with = "hex_bytes")]
    salt: Vec<u8>,
    #[serde(with = "hex_bytes")]
    verify_nonce: Vec<u8>,
    #[serde(with = "hex_bytes")]
    verify_ct: Vec<u8>,
    entries: Vec<EntryRecord>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KdfDescriptor {
    id: u8,
    m_kib: u32,
    t: u32,
    p: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryRecord {
    label: String,
    #[serde(with = "hex_bytes")]
    nonce: Vec<u8>,
    #[serde(with = "hex_bytes")]
    ciphertext: Vec<u8>,
}

/// Serialize a full vault (header + entries) to JSON bytes. Contains
/// only salt/params (non-secret) + ciphertext — never plaintext.
pub(crate) fn serialize(header: &Header, entries: &[Entry]) -> Vec<u8> {
    let file = VaultFile {
        version: FORMAT_VERSION,
        kdf: KdfDescriptor {
            id: KDF_ID_ARGON2ID,
            m_kib: header.params.m_kib,
            t: header.params.t,
            p: header.params.p,
        },
        salt: header.salt.to_vec(),
        verify_nonce: header.verify_nonce.to_vec(),
        verify_ct: header.verify_ct.clone(),
        entries: entries
            .iter()
            .map(|e| EntryRecord {
                label: e.label.clone(),
                nonce: e.nonce.to_vec(),
                ciphertext: e.ciphertext.clone(),
            })
            .collect(),
    };
    // VaultFile carries only fixed-width arrays and owned Vecs that
    // serialize infallibly; a serializer error would be a logic bug.
    serde_json::to_vec(&file).expect("vault serialization is infallible")
}

/// Validate a hex-decoded byte field to a fixed-width array, rejecting a
/// wrong length as [`FileStoreError::MalformedVault`] rather than
/// panicking in `XNonce::from_slice` / `copy_from_slice`.
fn fixed<const N: usize>(bytes: &[u8]) -> Result<[u8; N], FileStoreError> {
    bytes.try_into().map_err(|_| FileStoreError::MalformedVault)
}

/// Parse a vault. Two-step: probe `version` (lax), then parse the strict
/// payload for the known version. Refuses unknown versions, unknown KDF
/// ids, and any malformed/short byte field — fail closed (SEC-REQ-2.2.9).
/// All `serde_json` errors are mapped to a static [`FileStoreError`] with
/// the source DISCARDED so input bytes can never leak into an error
/// string or log.
pub(crate) fn deserialize(buf: &[u8]) -> Result<(Header, Vec<Entry>), FileStoreError> {
    let probe: VersionProbe =
        serde_json::from_slice(buf).map_err(|_| FileStoreError::MalformedVault)?;
    if probe.version != FORMAT_VERSION {
        return Err(FileStoreError::VersionUnsupported {
            found: probe.version,
        });
    }

    let file: VaultFile =
        serde_json::from_slice(buf).map_err(|_| FileStoreError::MalformedVault)?;

    if file.kdf.id != KDF_ID_ARGON2ID {
        return Err(FileStoreError::MalformedVault);
    }

    let salt = fixed::<SALT_LEN>(&file.salt)?;
    let verify_nonce = fixed::<NONCE_LEN>(&file.verify_nonce)?;
    if file.verify_ct.len() < AEAD_TAG_LEN {
        return Err(FileStoreError::MalformedVault);
    }

    let mut entries = Vec::with_capacity(file.entries.len());
    for rec in file.entries {
        let nonce = fixed::<NONCE_LEN>(&rec.nonce)?;
        if rec.ciphertext.len() < AEAD_TAG_LEN {
            return Err(FileStoreError::MalformedVault);
        }
        entries.push(Entry {
            label: rec.label,
            nonce,
            ciphertext: rec.ciphertext,
        });
    }

    Ok((
        Header {
            params: KdfParams {
                m_kib: file.kdf.m_kib,
                t: file.kdf.t,
                p: file.kdf.p,
            },
            salt,
            verify_nonce,
            verify_ct: file.verify_ct,
        },
        entries,
    ))
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

    fn test_header() -> Header {
        Header {
            params: KdfParams::default_target(),
            salt: [7u8; SALT_LEN],
            verify_nonce: [5u8; NONCE_LEN],
            verify_ct: vec![0xCC; 34],
        }
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let header = test_header();
        let entries = vec![
            Entry {
                label: "bip39_mnemonic".into(),
                nonce: [3u8; NONCE_LEN],
                ciphertext: vec![1; AEAD_TAG_LEN + 4],
            },
            Entry {
                label: "bip32-seed".into(),
                nonce: [9u8; NONCE_LEN],
                ciphertext: vec![6; AEAD_TAG_LEN + 2],
            },
        ];
        let bytes = serialize(&header, &entries);
        let (h2, e2) = deserialize(&bytes).unwrap();
        assert_eq!(h2.params, header.params);
        assert_eq!(h2.salt, header.salt);
        assert_eq!(h2.verify_nonce, header.verify_nonce);
        assert_eq!(h2.verify_ct, header.verify_ct);
        assert_eq!(e2.len(), 2);
        assert_eq!(e2[0].label, "bip39_mnemonic");
        assert_eq!(e2[1].ciphertext, vec![6; AEAD_TAG_LEN + 2]);
    }

    #[test]
    fn serialized_form_is_json_with_version_and_lowercase_hex() {
        let bytes = serialize(&test_header(), &[]);
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
        let mut file: VaultFile = serde_json::from_slice(&serialize(&test_header(), &[])).unwrap();
        file.version = 999;
        let bytes = serde_json::to_vec(&file).unwrap();
        assert!(matches!(
            deserialize(&bytes),
            Err(FileStoreError::VersionUnsupported { found: 999 })
        ));
    }

    #[test]
    fn rejects_unknown_kdf_id() {
        let mut file: VaultFile = serde_json::from_slice(&serialize(&test_header(), &[])).unwrap();
        file.kdf.id = 7;
        let bytes = serde_json::to_vec(&file).unwrap();
        assert!(matches!(
            deserialize(&bytes),
            Err(FileStoreError::MalformedVault)
        ));
    }

    #[test]
    fn rejects_unknown_payload_field() {
        // A version-2 file with a stray sibling field must fail closed
        // (deny_unknown_fields on VaultFile, C3).
        let bytes = br#"{"version":2,"kdf":{"id":1,"m_kib":65536,"t":3,"p":1},"salt":"00","verify_nonce":"00","verify_ct":"00","entries":[],"rogue":true}"#;
        assert!(matches!(
            deserialize(bytes),
            Err(FileStoreError::MalformedVault)
        ));
    }

    #[test]
    fn wrong_length_nonce_yields_malformed_not_panic() {
        // A 1-byte nonce must not panic in copy_from_slice.
        let mut file: VaultFile = serde_json::from_slice(&serialize(&test_header(), &[])).unwrap();
        file.entries.push(EntryRecord {
            label: "seed".into(),
            nonce: vec![0u8; 1],
            ciphertext: vec![0u8; AEAD_TAG_LEN],
        });
        let bytes = serde_json::to_vec(&file).unwrap();
        assert!(matches!(
            deserialize(&bytes),
            Err(FileStoreError::MalformedVault)
        ));
    }

    #[test]
    fn wrong_length_salt_yields_malformed() {
        let mut file: VaultFile = serde_json::from_slice(&serialize(&test_header(), &[])).unwrap();
        file.salt = vec![0u8; SALT_LEN - 1];
        let bytes = serde_json::to_vec(&file).unwrap();
        assert!(matches!(
            deserialize(&bytes),
            Err(FileStoreError::MalformedVault)
        ));
    }

    #[test]
    fn short_ciphertext_below_tag_len_yields_malformed() {
        let mut file: VaultFile = serde_json::from_slice(&serialize(&test_header(), &[])).unwrap();
        file.entries.push(EntryRecord {
            label: "seed".into(),
            nonce: vec![0u8; NONCE_LEN],
            ciphertext: vec![0u8; AEAD_TAG_LEN - 1],
        });
        let bytes = serde_json::to_vec(&file).unwrap();
        assert!(matches!(
            deserialize(&bytes),
            Err(FileStoreError::MalformedVault)
        ));
    }

    #[test]
    fn short_verify_ct_below_tag_len_yields_malformed() {
        let mut file: VaultFile = serde_json::from_slice(&serialize(&test_header(), &[])).unwrap();
        file.verify_ct = vec![0u8; AEAD_TAG_LEN - 1];
        let bytes = serde_json::to_vec(&file).unwrap();
        assert!(matches!(
            deserialize(&bytes),
            Err(FileStoreError::MalformedVault)
        ));
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
