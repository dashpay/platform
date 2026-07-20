//! Shared decoder for the identity add/register public-key BLOB.
//!
//! The Kotlin side encodes rich key rows (a `keyId`, the three DPP role bytes,
//! an optional contract-bounds section, and the compressed public key) into an
//! opaque `byte[]`. This module parses that wire format back into owned
//! [`DecodedPubkeyRow`]s whose buffers back the raw pointers of an
//! [`IdentityPubkeyFFI`] for the duration of one synchronous FFI call.
//!
//! ## One pure parser, two JNI seams
//!
//! [`parse_pubkey_rows`] is a pure `&[u8]` → `Vec<DecodedPubkeyRow>` function
//! (no `JNIEnv`, no `JByteArray`) so its round-trip and edge-case behaviour is
//! covered by ordinary Rust unit tests rather than JNI-environment-dependent
//! ones. Two thin JNI adapters wrap it:
//!
//! - [`decode_update_pubkeys_blob`] backs the identity-UPDATE add-key path
//!   (`TransactionsNative.updateIdentity`). A null / empty blob is a
//!   legitimate "no keys to add" — an update may be disable-only. **No**
//!   key-ID-0 or single-MASTER invariant applies here: an ordinary add-key
//!   update neither includes nor should include key 0.
//! - [`decode_registration_pubkeys_blob`] backs every identity-CREATE path
//!   (the four registration JNI exports). It additionally requires ≥1 row,
//!   rejects duplicate key IDs, and enforces the wallet's key-ID-0 =
//!   MASTER + AUTHENTICATION convention (`MASTER_KEY_INDEX == 0`; see
//!   `platform-wallet`'s `identity_handle.rs`). Those extra checks are
//!   registration-only and deliberately absent from the update decoder.
//!
//! ## Strictness (wire-skew detection)
//!
//! The parser rejects truncated input, **trailing bytes**, invalid DPP-role /
//! bounds-kind / boolean discriminants, interior-NUL document types, and
//! negative key IDs. The opaque payload shape is not self-describing, so an
//! old Kotlin artifact paired with a newer native library (or the reverse)
//! would otherwise be silently misparsed rather than rejected. Failing loud on
//! any structural surprise turns that skew into a clean error instead of a key
//! registered with the wrong semantics.

use crate::support::throw_sdk_exception;
use jni::objects::JByteArray;
use jni::JNIEnv;
use platform_wallet_ffi::identity_registration_with_signer::IdentityPubkeyFFI;
use std::ffi::CString;
use std::ptr;

/// DPP `Purpose::AUTHENTICATION` discriminant byte.
const PURPOSE_AUTHENTICATION: u8 = 0;
/// DPP `SecurityLevel::MASTER` discriminant byte.
const SECURITY_LEVEL_MASTER: u8 = 0;

/// One decoded add/register key row with owned buffers backing the FFI
/// pointers an [`IdentityPubkeyFFI`] borrows.
///
/// Fields are crate-visible so the registration invariant checks and the unit
/// tests can read them; call sites should still build their FFI view through
/// [`DecodedPubkeyRow::to_ffi`] so pointer creation and the borrow contract
/// stay in one place.
///
/// Every field is public-key / role metadata — no private key material — so
/// `Debug` (used by the round-trip tests) leaks nothing sensitive.
#[derive(Debug)]
pub(crate) struct DecodedPubkeyRow {
    pub(crate) key_id: u32,
    pub(crate) key_type: u8,
    pub(crate) purpose: u8,
    pub(crate) security_level: u8,
    pub(crate) read_only: bool,
    pub(crate) contract_bounds_kind: u8,
    pub(crate) pubkey_bytes: Vec<u8>,
    pub(crate) contract_bounds_id: Option<[u8; 32]>,
    pub(crate) contract_bounds_document_type: Option<CString>,
}

impl DecodedPubkeyRow {
    /// Build the borrowed [`IdentityPubkeyFFI`] view of this row.
    ///
    /// Every pointer references a buffer owned by `self`, so the returned
    /// struct must not outlive it — keep the owning `DecodedPubkeyRow` (or the
    /// `Vec` holding it) alive across the whole FFI call that reads the view.
    pub(crate) fn to_ffi(&self) -> IdentityPubkeyFFI {
        IdentityPubkeyFFI {
            key_id: self.key_id,
            key_type: self.key_type,
            purpose: self.purpose,
            security_level: self.security_level,
            pubkey_bytes: self.pubkey_bytes.as_ptr(),
            pubkey_len: self.pubkey_bytes.len(),
            read_only: self.read_only,
            contract_bounds_kind: self.contract_bounds_kind,
            contract_bounds_id: self
                .contract_bounds_id
                .as_ref()
                .map_or(ptr::null(), |b| b.as_ptr()),
            contract_bounds_document_type: self
                .contract_bounds_document_type
                .as_ref()
                .map_or(ptr::null(), |c| c.as_ptr()),
        }
    }
}

/// Parse the rich add/register public-key BLOB into owned rows.
///
/// Layout (all integers big-endian), one row per key:
/// ```text
/// u32 row_count
/// repeat row_count times:
///   u32  key_id
///   u8   key_type          (DPP KeyType discriminant, 0 = ECDSA_SECP256K1)
///   u8   purpose           (DPP Purpose discriminant, 0 = AUTHENTICATION)
///   u8   security_level    (DPP SecurityLevel discriminant, 0 = MASTER)
///   u8   read_only         (0 / 1 — any other byte is rejected)
///   u8   contract_bounds_kind (0 none, 1 SingleContract, 2 SingleContractDocumentType)
///   u16  pubkey_len
///   u8[pubkey_len]  pubkey_bytes  (compressed pubkey, or 20-byte HASH160)
///   if contract_bounds_kind != 0:
///     u8[32] contract_bounds_id
///   if contract_bounds_kind == 2:
///     u16 doc_type_len, u8[doc_type_len] doc_type (UTF-8)
/// ```
///
/// Strict: returns `Err` on truncation, trailing bytes, a negative key ID
/// (`writeInt` is signed on the Kotlin side, so a set sign bit is a bug), an
/// invalid `read_only` or `contract_bounds_kind` byte, or an interior NUL in a
/// document type. It does **not** validate the DPP role bytes against DPP's
/// structural rules — that stays server-side — nor does it apply any
/// registration-only invariant; the registration seam layers those on top.
pub(crate) fn parse_pubkey_rows(bytes: &[u8]) -> Result<Vec<DecodedPubkeyRow>, String> {
    let mut cursor = 0usize;
    let read = |cursor: &mut usize, n: usize| -> Option<&[u8]> {
        let end = cursor.checked_add(n)?;
        if end > bytes.len() {
            return None;
        }
        let s = &bytes[*cursor..end];
        *cursor = end;
        Some(s)
    };

    let count_bytes = read(&mut cursor, 4).ok_or("pubkey blob truncated (row count)")?;
    let count = u32::from_be_bytes([
        count_bytes[0],
        count_bytes[1],
        count_bytes[2],
        count_bytes[3],
    ]) as usize;
    // Length-before-allocation guard: each row is at least an 11-byte fixed
    // header, so a header claiming more rows than the remaining payload can
    // possibly hold is malformed — prevents a huge `with_capacity` abort from a
    // raw-JNI blob.
    if count
        .checked_mul(11)
        .is_none_or(|need| bytes.len() - cursor < need)
    {
        return Err(format!(
            "pubkey blob claims {count} rows but the body is too short"
        ));
    }

    let mut rows = Vec::with_capacity(count);
    for i in 0..count {
        let fixed = read(&mut cursor, 4 + 1 + 1 + 1 + 1 + 1 + 2)
            .ok_or_else(|| format!("pubkey blob truncated at row {i} header"))?;
        let key_id = u32::from_be_bytes([fixed[0], fixed[1], fixed[2], fixed[3]]);
        // The Kotlin encoder writes this field with writeInt (signed); a set
        // sign bit means a negative key id crossed the boundary.
        if key_id > i32::MAX as u32 {
            return Err(format!("pubkey blob row {i} keyId must be non-negative"));
        }
        let key_type = fixed[4];
        let purpose = fixed[5];
        let security_level = fixed[6];
        let read_only = match fixed[7] {
            0 => false,
            1 => true,
            other => {
                return Err(format!(
                    "pubkey blob row {i} readOnly must be 0 or 1, got {other}"
                ));
            }
        };
        let contract_bounds_kind = fixed[8];
        if contract_bounds_kind > 2 {
            return Err(format!(
                "pubkey blob row {i} contractBoundsKind must be 0, 1 or 2, got {contract_bounds_kind}"
            ));
        }
        let pubkey_len = u16::from_be_bytes([fixed[9], fixed[10]]) as usize;
        let pubkey_bytes = read(&mut cursor, pubkey_len)
            .ok_or_else(|| format!("pubkey blob truncated at row {i} pubkey"))?
            .to_vec();

        let mut contract_bounds_id: Option<[u8; 32]> = None;
        let mut contract_bounds_document_type: Option<CString> = None;
        if contract_bounds_kind != 0 {
            let id_bytes = read(&mut cursor, 32)
                .ok_or_else(|| format!("pubkey blob truncated at row {i} contractBoundsId"))?;
            let mut id = [0u8; 32];
            id.copy_from_slice(id_bytes);
            contract_bounds_id = Some(id);

            if contract_bounds_kind == 2 {
                let dt_len_bytes = read(&mut cursor, 2)
                    .ok_or_else(|| format!("pubkey blob truncated at row {i} docTypeLen"))?;
                let dt_len = u16::from_be_bytes([dt_len_bytes[0], dt_len_bytes[1]]) as usize;
                let dt_bytes = read(&mut cursor, dt_len)
                    .ok_or_else(|| format!("pubkey blob truncated at row {i} docType"))?;
                contract_bounds_document_type = Some(
                    CString::new(dt_bytes.to_vec())
                        .map_err(|_| format!("pubkey blob row {i} docType had an interior NUL"))?,
                );
            }
        }

        rows.push(DecodedPubkeyRow {
            key_id,
            key_type,
            purpose,
            security_level,
            read_only,
            contract_bounds_kind,
            pubkey_bytes,
            contract_bounds_id,
            contract_bounds_document_type,
        });
    }

    // Reject trailing bytes: the payload is not self-describing, so leftover
    // bytes mean the writer and reader disagree on the layout (a wire-format
    // skew) — surface it rather than silently ignoring the tail.
    if cursor != bytes.len() {
        return Err(format!(
            "pubkey blob has {} trailing byte(s) after {count} row(s)",
            bytes.len() - cursor
        ));
    }

    Ok(rows)
}

/// Read a Java `byte[]` into owned bytes, or `None` when the array is null.
fn convert_or_null(env: &mut JNIEnv, arr: &JByteArray) -> Option<Vec<u8>> {
    match env.convert_byte_array(arr) {
        Ok(b) => Some(b),
        Err(_) => {
            let _ = env.exception_clear();
            None
        }
    }
}

/// Decode the identity-UPDATE add-keys BLOB (the `updateIdentity` path).
///
/// A null or empty blob is a legitimate "no keys to add" (an update may be
/// disable-only), so it maps to an empty row list rather than an error. Any
/// non-empty blob must parse strictly. Throws + returns `None` on a malformed
/// blob. No registration-only invariant is applied — see the module docs.
pub(crate) fn decode_update_pubkeys_blob(
    env: &mut JNIEnv,
    arr: &JByteArray,
) -> Option<Vec<DecodedPubkeyRow>> {
    let Some(bytes) = convert_or_null(env, arr) else {
        // A null blob is a legitimate "no keys to add".
        return Some(Vec::new());
    };
    if bytes.is_empty() {
        return Some(Vec::new());
    }
    match parse_pubkey_rows(&bytes) {
        Ok(rows) => Some(rows),
        Err(msg) => {
            throw_sdk_exception(env, 1, &msg);
            None
        }
    }
}

/// The registration-only invariants layered on top of the shared structural
/// parse (kept pure + separate from the JNI wrapper so they are unit-testable
/// without a JVM):
/// - at least one key row,
/// - no duplicate key IDs — the FFI `decode_identity_pubkeys` inserts into a
///   `BTreeMap` where a later duplicate would otherwise silently overwrite an
///   earlier row, so reject them here (and there) rather than last-wins,
/// - key ID 0 present and MASTER + AUTHENTICATION — the wallet derives and
///   loads the identity's signing key at `MASTER_KEY_INDEX == 0`, and the
///   shared funded path already asserts this, but the address-funded and
///   shielded paths build the identity straight from the decoded map without
///   it. Enforcing it at this shared registration seam closes that gap for
///   every create path uniformly.
///
/// The order-independence matters: key ID 0 is found by id, not row position,
/// so a caller may submit rows in any order.
pub(crate) fn check_registration_invariants(rows: &[DecodedPubkeyRow]) -> Result<(), String> {
    if rows.is_empty() {
        return Err("pubkeysBlob contained no keys".to_string());
    }

    let mut seen = std::collections::BTreeSet::new();
    for row in rows {
        if !seen.insert(row.key_id) {
            return Err(format!("pubkeysBlob has a duplicate keyId {}", row.key_id));
        }
    }

    let key0 = rows
        .iter()
        .find(|r| r.key_id == 0)
        .ok_or("pubkeysBlob is missing key ID 0 (the MASTER key)")?;
    if key0.security_level != SECURITY_LEVEL_MASTER || key0.purpose != PURPOSE_AUTHENTICATION {
        return Err("pubkeysBlob key ID 0 must be MASTER + AUTHENTICATION".to_string());
    }

    Ok(())
}

/// Decode an identity-CREATE registration pubkeys BLOB (all four registration
/// JNI exports): the shared structural parse plus
/// [`check_registration_invariants`]. Throws + returns `None` on any violation.
pub(crate) fn decode_registration_pubkeys_blob(
    env: &mut JNIEnv,
    arr: &JByteArray,
) -> Option<Vec<DecodedPubkeyRow>> {
    let Some(bytes) = convert_or_null(env, arr) else {
        throw_sdk_exception(env, 1, "pubkeysBlob was null/invalid");
        return None;
    };
    let rows = match parse_pubkey_rows(&bytes) {
        Ok(rows) => rows,
        Err(msg) => {
            throw_sdk_exception(env, 1, &msg);
            return None;
        }
    };
    if let Err(msg) = check_registration_invariants(&rows) {
        throw_sdk_exception(env, 1, &msg);
        return None;
    }
    Some(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DPP discriminants used across the fixtures (see the Kotlin
    /// `KeyType` / `KeyPurpose` / `SecurityLevel` enums).
    const KEY_TYPE_ECDSA: u8 = 0;
    const PURPOSE_AUTH: u8 = 0;
    const PURPOSE_ENCRYPTION: u8 = 1;
    const PURPOSE_DECRYPTION: u8 = 2;
    const PURPOSE_TRANSFER: u8 = 3;
    const SEC_MASTER: u8 = 0;
    const SEC_CRITICAL: u8 = 1;
    const SEC_HIGH: u8 = 2;
    const SEC_MEDIUM: u8 = 3;

    /// Test-only encoder — the byte-for-byte mirror of Kotlin's
    /// `IdentityPubkeyCodec.encode`, so a round trip through
    /// [`parse_pubkey_rows`] proves both directions agree.
    struct Row {
        key_id: u32,
        key_type: u8,
        purpose: u8,
        security_level: u8,
        read_only: u8,
        pubkey: Vec<u8>,
        bounds: Option<(u8, [u8; 32], Option<String>)>,
    }

    fn encode(rows: &[Row]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(rows.len() as u32).to_be_bytes());
        for r in rows {
            out.extend_from_slice(&r.key_id.to_be_bytes());
            out.push(r.key_type);
            out.push(r.purpose);
            out.push(r.security_level);
            out.push(r.read_only);
            let kind = r.bounds.as_ref().map_or(0, |(k, _, _)| *k);
            out.push(kind);
            out.extend_from_slice(&(r.pubkey.len() as u16).to_be_bytes());
            out.extend_from_slice(&r.pubkey);
            if let Some((_, id, doc)) = &r.bounds {
                out.extend_from_slice(id);
                if let Some(doc) = doc {
                    let dt = doc.as_bytes();
                    out.extend_from_slice(&(dt.len() as u16).to_be_bytes());
                    out.extend_from_slice(dt);
                }
            }
        }
        out
    }

    fn base_master() -> Row {
        Row {
            key_id: 0,
            key_type: KEY_TYPE_ECDSA,
            purpose: PURPOSE_AUTH,
            security_level: SEC_MASTER,
            read_only: 0,
            pubkey: vec![2u8; 33],
            bounds: None,
        }
    }

    /// The full 6-key registration policy: base 4 (auth MASTER/CRITICAL/HIGH,
    /// TRANSFER/CRITICAL) + the DashPay ENCRYPTION/DECRYPTION pair bound to the
    /// `contactRequest` document type.
    fn six_key_policy() -> Vec<Row> {
        let dashpay_id = [7u8; 32];
        vec![
            base_master(),
            Row {
                key_id: 1,
                key_type: KEY_TYPE_ECDSA,
                purpose: PURPOSE_AUTH,
                security_level: SEC_CRITICAL,
                read_only: 0,
                pubkey: vec![3u8; 33],
                bounds: None,
            },
            Row {
                key_id: 2,
                key_type: KEY_TYPE_ECDSA,
                purpose: PURPOSE_AUTH,
                security_level: SEC_HIGH,
                read_only: 0,
                pubkey: vec![4u8; 33],
                bounds: None,
            },
            Row {
                key_id: 3,
                key_type: KEY_TYPE_ECDSA,
                purpose: PURPOSE_TRANSFER,
                security_level: SEC_CRITICAL,
                read_only: 0,
                pubkey: vec![5u8; 33],
                bounds: None,
            },
            Row {
                key_id: 4,
                key_type: KEY_TYPE_ECDSA,
                purpose: PURPOSE_ENCRYPTION,
                security_level: SEC_MEDIUM,
                read_only: 0,
                pubkey: vec![6u8; 33],
                bounds: Some((2, dashpay_id, Some("contactRequest".to_string()))),
            },
            Row {
                key_id: 5,
                key_type: KEY_TYPE_ECDSA,
                purpose: PURPOSE_DECRYPTION,
                security_level: SEC_MEDIUM,
                read_only: 0,
                pubkey: vec![7u8; 33],
                bounds: Some((2, dashpay_id, Some("contactRequest".to_string()))),
            },
        ]
    }

    #[test]
    fn round_trips_the_full_six_key_policy() {
        let dashpay_id = [7u8; 32];
        let decoded = parse_pubkey_rows(&encode(&six_key_policy())).expect("parse");
        assert_eq!(decoded.len(), 6);
        // Base 4 roles round-trip.
        assert_eq!(
            (
                decoded[0].purpose,
                decoded[0].security_level,
                decoded[0].contract_bounds_kind
            ),
            (PURPOSE_AUTH, SEC_MASTER, 0)
        );
        assert_eq!(
            (decoded[3].purpose, decoded[3].security_level),
            (PURPOSE_TRANSFER, SEC_CRITICAL)
        );
        // DashPay ENCRYPTION / DECRYPTION carry the contract-document bounds.
        for row in &decoded[4..] {
            assert_eq!(row.security_level, SEC_MEDIUM);
            assert_eq!(row.contract_bounds_kind, 2);
            assert_eq!(row.contract_bounds_id, Some(dashpay_id));
            assert_eq!(
                row.contract_bounds_document_type
                    .as_ref()
                    .map(|c| c.to_str().unwrap().to_string()),
                Some("contactRequest".to_string())
            );
        }
        assert_eq!(decoded[4].purpose, PURPOSE_ENCRYPTION);
        assert_eq!(decoded[5].purpose, PURPOSE_DECRYPTION);
        // Every pubkey survives byte-for-byte.
        assert_eq!(decoded[5].pubkey_bytes, vec![7u8; 33]);
    }

    #[test]
    fn round_trips_single_contract_bounds_kind() {
        let id = [9u8; 32];
        let rows = vec![Row {
            key_id: 0,
            key_type: KEY_TYPE_ECDSA,
            purpose: PURPOSE_ENCRYPTION,
            security_level: SEC_MEDIUM,
            read_only: 1,
            pubkey: vec![2u8; 33],
            bounds: Some((1, id, None)),
        }];
        let decoded = parse_pubkey_rows(&encode(&rows)).expect("parse");
        assert_eq!(decoded[0].contract_bounds_kind, 1);
        assert_eq!(decoded[0].contract_bounds_id, Some(id));
        assert!(decoded[0].contract_bounds_document_type.is_none());
        assert!(decoded[0].read_only);
    }

    #[test]
    fn rejects_trailing_bytes() {
        let mut blob = encode(&[base_master()]);
        blob.push(0xAB);
        let err = parse_pubkey_rows(&blob).unwrap_err();
        assert!(err.contains("trailing"), "{err}");
    }

    #[test]
    fn rejects_truncation_at_each_variable_field() {
        let full = encode(&six_key_policy());
        // Truncating anywhere past the row count must fail (never silently
        // return a short list).
        for cut in 4..full.len() {
            assert!(
                parse_pubkey_rows(&full[..cut]).is_err(),
                "expected truncation error at cut {cut}"
            );
        }
    }

    #[test]
    fn rejects_invalid_bounds_kind() {
        let mut rows = vec![base_master()];
        rows[0].bounds = Some((3, [1u8; 32], None));
        // encode() writes kind byte from bounds.0 = 3, then a 32-byte id.
        let err = parse_pubkey_rows(&encode(&rows)).unwrap_err();
        assert!(err.contains("contractBoundsKind"), "{err}");
    }

    #[test]
    fn rejects_invalid_readonly_byte() {
        let mut blob = encode(&[base_master()]);
        // read_only byte sits at offset 4(count)+4(keyId)+3(role) = 11.
        blob[11] = 2;
        let err = parse_pubkey_rows(&blob).unwrap_err();
        assert!(err.contains("readOnly"), "{err}");
    }

    #[test]
    fn rejects_negative_key_id() {
        let mut row = base_master();
        row.key_id = 0x8000_0000; // sign bit set once written as be bytes
        let err = parse_pubkey_rows(&encode(&[row])).unwrap_err();
        assert!(err.contains("non-negative"), "{err}");
    }

    #[test]
    fn rejects_interior_nul_document_type() {
        let rows = vec![Row {
            key_id: 0,
            key_type: KEY_TYPE_ECDSA,
            purpose: PURPOSE_ENCRYPTION,
            security_level: SEC_MEDIUM,
            read_only: 0,
            pubkey: vec![2u8; 33],
            bounds: Some((2, [1u8; 32], Some("con\0tact".to_string()))),
        }];
        let err = parse_pubkey_rows(&encode(&rows)).unwrap_err();
        assert!(err.contains("interior NUL"), "{err}");
    }

    #[test]
    fn rejects_legacy_bare_registration_format() {
        // The retired registration blob was: u32 rowCount, then per row
        // u32 keyId, u16 pubkeyLen, pubkey — no role/bounds bytes. Feeding
        // that shape to the rich parser must fail, not silently misparse.
        let mut legacy = Vec::new();
        legacy.extend_from_slice(&1u32.to_be_bytes());
        legacy.extend_from_slice(&0u32.to_be_bytes()); // keyId 0
        legacy.extend_from_slice(&33u16.to_be_bytes()); // pubkey len
        legacy.extend_from_slice(&[2u8; 33]);
        assert!(parse_pubkey_rows(&legacy).is_err());
    }

    #[test]
    fn empty_row_list_round_trips() {
        let decoded = parse_pubkey_rows(&0u32.to_be_bytes()).expect("parse");
        assert!(decoded.is_empty());
    }

    // ── Registration-only invariants ──────────────────────────────────

    #[test]
    fn registration_accepts_the_full_policy() {
        let rows = parse_pubkey_rows(&encode(&six_key_policy())).unwrap();
        assert!(check_registration_invariants(&rows).is_ok());
    }

    #[test]
    fn registration_master_check_is_order_independent() {
        // Same rows, reversed: key ID 0 is no longer first, but the check
        // finds it by id and still passes.
        let mut policy = six_key_policy();
        policy.reverse();
        let rows = parse_pubkey_rows(&encode(&policy)).unwrap();
        assert!(check_registration_invariants(&rows).is_ok());
    }

    #[test]
    fn registration_rejects_missing_key_zero() {
        // A base set that skips key ID 0 (starts at 1).
        let rows_no_zero: Vec<Row> = six_key_policy()
            .into_iter()
            .filter(|r| r.key_id != 0)
            .collect();
        let rows = parse_pubkey_rows(&encode(&rows_no_zero)).unwrap();
        let err = check_registration_invariants(&rows).unwrap_err();
        assert!(err.contains("missing key ID 0"), "{err}");
    }

    #[test]
    fn registration_rejects_non_master_key_zero() {
        let mut policy = six_key_policy();
        policy[0].security_level = SEC_HIGH; // key 0 no longer MASTER
        let rows = parse_pubkey_rows(&encode(&policy)).unwrap();
        let err = check_registration_invariants(&rows).unwrap_err();
        assert!(err.contains("MASTER + AUTHENTICATION"), "{err}");
    }

    #[test]
    fn registration_rejects_duplicate_key_ids() {
        let mut policy = six_key_policy();
        policy[2].key_id = 1; // collide with key 1
        let rows = parse_pubkey_rows(&encode(&policy)).unwrap();
        let err = check_registration_invariants(&rows).unwrap_err();
        assert!(err.contains("duplicate keyId 1"), "{err}");
    }

    #[test]
    fn registration_rejects_empty_row_list() {
        let err = check_registration_invariants(&[]).unwrap_err();
        assert!(err.contains("no keys"), "{err}");
    }

    // ── Cross-language golden fixture ─────────────────────────────────

    /// The checked-in golden blob, shared byte-for-byte with the Kotlin
    /// encoder test (`RegistrationKeysTest.encoder output matches the
    /// cross-language golden fixture`). Referenced from the single canonical
    /// copy in the Kotlin SDK's test resources so the two can never drift.
    const GOLDEN: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../kotlin-sdk/sdk/src/test/resources/golden/registration_pubkeys_v1.bin"
    ));

    /// Decode the golden blob and assert the full 6-key DashPay policy — role
    /// bytes, ordering, and (critically) that the ENCRYPTION / DECRYPTION rows
    /// carry the REAL `dashpay_contract::ID_BYTES`, not a copied literal. Pins
    /// the Kotlin-side mirrored contract id to the Rust source of truth.
    #[test]
    fn golden_fixture_decodes_to_the_dashpay_policy() {
        let rows = parse_pubkey_rows(GOLDEN).expect("golden fixture must parse");
        assert_eq!(rows.len(), 6);
        // The registration invariants hold on the canonical set.
        check_registration_invariants(&rows).expect("golden fixture is a valid registration set");

        let roles: Vec<(u32, u8, u8, u8, u8)> = rows
            .iter()
            .map(|r| {
                (
                    r.key_id,
                    r.key_type,
                    r.purpose,
                    r.security_level,
                    r.contract_bounds_kind,
                )
            })
            .collect();
        assert_eq!(
            roles,
            vec![
                (0, KEY_TYPE_ECDSA, PURPOSE_AUTH, SEC_MASTER, 0),
                (1, KEY_TYPE_ECDSA, PURPOSE_AUTH, SEC_CRITICAL, 0),
                (2, KEY_TYPE_ECDSA, PURPOSE_AUTH, SEC_HIGH, 0),
                (3, KEY_TYPE_ECDSA, PURPOSE_TRANSFER, SEC_CRITICAL, 0),
                (4, KEY_TYPE_ECDSA, PURPOSE_ENCRYPTION, SEC_MEDIUM, 2),
                (5, KEY_TYPE_ECDSA, PURPOSE_DECRYPTION, SEC_MEDIUM, 2),
            ]
        );

        for row in &rows[4..] {
            assert_eq!(
                row.contract_bounds_id,
                Some(dashpay_contract::ID_BYTES),
                "DashPay enc/dec key must be bound to the canonical contract id"
            );
            assert_eq!(
                row.contract_bounds_document_type
                    .as_ref()
                    .map(|c| c.to_str().unwrap().to_string()),
                Some("contactRequest".to_string())
            );
        }
    }

    #[test]
    fn update_add_key_list_without_key_zero_is_structurally_valid() {
        // Proves the key-ID-0 invariant did NOT leak into the shared
        // structural parser: an ordinary add-key update (keys 4 and 5, no
        // key 0) parses fine — only the registration seam layers the
        // key-0 invariant on top, and it is not applied here.
        let update_rows = vec![
            Row {
                key_id: 4,
                key_type: KEY_TYPE_ECDSA,
                purpose: PURPOSE_AUTH,
                security_level: SEC_HIGH,
                read_only: 0,
                pubkey: vec![2u8; 33],
                bounds: None,
            },
            Row {
                key_id: 5,
                key_type: KEY_TYPE_ECDSA,
                purpose: PURPOSE_AUTH,
                security_level: SEC_HIGH,
                read_only: 0,
                pubkey: vec![3u8; 33],
                bounds: None,
            },
        ];
        let rows = parse_pubkey_rows(&encode(&update_rows)).expect("parse");
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.key_id != 0));
    }
}
