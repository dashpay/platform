//! DashPay `contactInfo` self-encryption (DIP-15).
//!
//! `contactInfo` documents carry the owner's PRIVATE per-contact metadata
//! (alias, note, hidden flag, accepted accounts) — encrypted so only the
//! owner can read them, unlike `contactRequest` payloads which are shared
//! with the counterparty via ECDH.
//!
//! **No reference client has implemented this document type yet**
//! (DashSync-iOS, dashj and dash-shared-core all lack it), so we
//! follow the DIP-15 spec exactly so a future client interops:
//!
//! - Key derivation (DIP-15): two hardened children of the identity's
//!   registered ENCRYPTION key in the owner's HD tree:
//!   `root / 65536' / index'` for `encToUserId`,
//!   `root / 65537' / index'` for `privateData`, where `root` is the
//!   identity-auth path of the key referenced by `rootEncryptionKeyIndex`
//!   and `index` is `derivationEncryptionKeyIndex`.
//! - `encToUserId`: AES-256-ECB of the 32-byte contact id (two raw blocks,
//!   no IV/padding — see `platform_encryption`'s rationale).
//! - `privateData`: `IV(16) ‖ AES-256-CBC(plaintext)`, where the plaintext is
//!   the DIP-15 "Dash message data" (Bitcoin P2P) serialization:
//!   `version (u32 LE)`, `aliasName (varstr)`, `note (varstr)`,
//!   `displayHidden (u8)`, `acceptedAccounts (varInt count + u32 LE[])`.
//!   `version = major << 16 | minor`: an unknown MAJOR ⇒ discard the whole
//!   document; an unknown MINOR ⇒ parse the known fields and ignore trailing
//!   bytes (the forward-compat seam). The contract validates `privateData` by
//!   LENGTH only (48–2048 bytes; the schema's "array in cbor" description is
//!   advisory, not enforced), so tiny payloads are padded with trailing zero
//!   bytes to the 48-byte ciphertext floor — a reader dispatches on `version`
//!   and ignores them.

use key_wallet::bip32::ChildNumber;
use key_wallet::wallet::Wallet;
use key_wallet::Network;
use zeroize::Zeroizing;

use key_wallet::bip32::KeyDerivationType;

use crate::error::PlatformWalletError;
use crate::wallet::identity::network::identity_auth_derivation_path_for_type;

/// DIP-15 child index for the `encToUserId` encryption key (2^16 —
/// "to discount other potential derivations of this key in other
/// applications").
pub const ENC_TO_USER_ID_CHILD: u32 = 1 << 16;

/// DIP-15 child index for the `privateData` encryption key (2^16 + 1).
pub const PRIVATE_DATA_CHILD: u32 = (1 << 16) + 1;

/// The deployed schema's `privateData` minimum length (bytes, IV included).
const PRIVATE_DATA_MIN_LEN: usize = 48;

/// Plaintext floor so `IV(16) ‖ AES-256-CBC/PKCS7(plaintext)` reaches the
/// 48-byte ciphertext floor: a 17-byte plaintext pads to 32 (CBC) + 16 (IV).
const MIN_PLAINTEXT_LEN: usize = PRIVATE_DATA_MIN_LEN - 16 - 15;

/// The deployed schema's `privateData` maximum length (bytes, IV included):
/// `"privateData": { "maxItems": 2048 }` in `dashpay.schema.json`.
const PRIVATE_DATA_MAX_LEN: usize = 2048;

/// Plaintext ceiling so `IV(16) ‖ AES-256-CBC/PKCS7(plaintext)` fits the
/// schema's 2048-byte cap: PKCS7 always adds 1..=16 padding bytes, so the
/// largest admissible plaintext is `(2048 - 16) - 1 = 2031` bytes (a
/// 2031-byte plaintext pads to a 2032-byte ciphertext; 2032 + 16 = 2048).
pub const MAX_PLAINTEXT_LEN: usize = PRIVATE_DATA_MAX_LEN - 16 - 1;

/// DIP-15 `version` for the v0 field set: `major(0) << 16 | minor(0)`.
const PRIVATE_DATA_VERSION_V0: u32 = 0;

/// The major version this codec understands. A document with a different
/// major version is discarded whole (DIP-15 §"Versioning of Private Data").
const SUPPORTED_MAJOR: u32 = 0;

/// The pair of AES-256 keys for one `contactInfo` document.
pub struct ContactInfoKeys {
    /// Key for `encToUserId` (AES-256-ECB).
    pub enc_to_user_id_key: Zeroizing<[u8; 32]>,
    /// Key for `privateData` (AES-256-CBC).
    pub private_data_key: Zeroizing<[u8; 32]>,
}

/// Derive the two `contactInfo` AES keys from the wallet seed.
///
/// `root_encryption_key_id` is the identity's registered ENCRYPTION
/// key id (the document's `rootEncryptionKeyIndex`);
/// `derivation_index` is the per-document
/// `derivationEncryptionKeyIndex`. Requires a key-resident wallet;
/// external-signable wallets have no in-process HD slot and need a
/// host-side signing hook.
pub fn derive_contact_info_keys(
    wallet: &Wallet,
    network: Network,
    identity_index: u32,
    root_encryption_key_id: u32,
    derivation_index: u32,
) -> Result<ContactInfoKeys, PlatformWalletError> {
    let root_path = identity_auth_derivation_path_for_type(
        network,
        KeyDerivationType::ECDSA,
        identity_index,
        root_encryption_key_id,
    )?;

    let derive_child = |feature: u32| -> Result<Zeroizing<[u8; 32]>, PlatformWalletError> {
        let path = root_path.extend([
            ChildNumber::from_hardened_idx(feature).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Invalid contactInfo feature index: {e}"
                ))
            })?,
            ChildNumber::from_hardened_idx(derivation_index).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Invalid contactInfo derivation index: {e}"
                ))
            })?,
        ]);
        let ext = wallet.derive_extended_private_key(&path).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to derive contactInfo key: {e}"
            ))
        })?;
        Ok(Zeroizing::new(ext.private_key.secret_bytes()))
    };

    Ok(ContactInfoKeys {
        enc_to_user_id_key: derive_child(ENC_TO_USER_ID_CHILD)?,
        private_data_key: derive_child(PRIVATE_DATA_CHILD)?,
    })
}

/// Decrypted `contactInfo.privateData` payload (DIP-15 v0 fields).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContactInfoPrivateData {
    /// User-chosen nickname for the contact.
    pub alias_name: Option<String>,
    /// Free-form note.
    pub note: Option<String>,
    /// Whether the contact is hidden / ignored (DIP-15 `displayHidden` — the
    /// hide flag, also the cross-device ignore signal).
    pub display_hidden: bool,
    /// Accepted rotated account-references of an established contact (DIP-15
    /// `acceptedAccounts`). Empty until multi-account is populated.
    pub accepted_accounts: Vec<u32>,
}

// --- DIP-15 "Dash message data" (Bitcoin P2P) (de)serialization helpers ---

/// Append a Bitcoin CompactSize var-int.
fn write_varint(out: &mut Vec<u8>, n: u64) {
    if n < 0xFD {
        out.push(n as u8);
    } else if n <= 0xFFFF {
        out.push(0xFD);
        out.extend_from_slice(&(n as u16).to_le_bytes());
    } else if n <= 0xFFFF_FFFF {
        out.push(0xFE);
        out.extend_from_slice(&(n as u32).to_le_bytes());
    } else {
        out.push(0xFF);
        out.extend_from_slice(&n.to_le_bytes());
    }
}

/// Append a Bitcoin variable-length string (var-int length + UTF-8 bytes).
fn write_varstr(out: &mut Vec<u8>, s: &str) {
    write_varint(out, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
}

/// Bounds-checked little-endian reader over the decrypted plaintext.
struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], PlatformWalletError> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.buf.len());
        let Some(end) = end else {
            return Err(PlatformWalletError::InvalidIdentityData(
                "contactInfo privateData is truncated".to_string(),
            ));
        };
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, PlatformWalletError> {
        Ok(self.take(1)?[0])
    }

    fn u32_le(&mut self) -> Result<u32, PlatformWalletError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn varint(&mut self) -> Result<u64, PlatformWalletError> {
        match self.u8()? {
            0xFF => {
                let b = self.take(8)?;
                Ok(u64::from_le_bytes(b.try_into().expect("8 bytes")))
            }
            0xFE => Ok(self.u32_le()? as u64),
            0xFD => {
                let b = self.take(2)?;
                Ok(u16::from_le_bytes([b[0], b[1]]) as u64)
            }
            n => Ok(n as u64),
        }
    }

    fn varstr(&mut self) -> Result<String, PlatformWalletError> {
        let len = self.varint()? as usize;
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| {
            PlatformWalletError::InvalidIdentityData(
                "contactInfo privateData string is not valid UTF-8".to_string(),
            )
        })
    }
}

fn empty_to_none(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Encode the `privateData` plaintext in the DIP-15 var-int format.
///
/// Pads with trailing zero bytes up to [`MIN_PLAINTEXT_LEN`] so the
/// AES-256-CBC ciphertext (IV included) reaches the schema's 48-byte floor.
/// A DIP-15 reader dispatches on `version` and ignores bytes past the final
/// v0 field, so the padding round-trips invisibly.
pub fn encode_private_data(data: &ContactInfoPrivateData) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&PRIVATE_DATA_VERSION_V0.to_le_bytes());
    write_varstr(&mut out, data.alias_name.as_deref().unwrap_or(""));
    write_varstr(&mut out, data.note.as_deref().unwrap_or(""));
    out.push(u8::from(data.display_hidden));
    write_varint(&mut out, data.accepted_accounts.len() as u64);
    for account in &data.accepted_accounts {
        out.extend_from_slice(&account.to_le_bytes());
    }
    if out.len() < MIN_PLAINTEXT_LEN {
        out.resize(MIN_PLAINTEXT_LEN, 0);
    }
    out
}

/// [`encode_private_data`] with the schema's size cap enforced.
///
/// The publish path MUST use this (before persisting any local state):
/// an over-cap plaintext produces a `privateData` blob the contract's
/// 2048-byte `maxItems` rejects at broadcast — a PERMANENT failure, not a
/// transient one, so it has to surface as an error to the caller instead
/// of leaving locally-persisted metadata durably divergent from chain.
/// Unlike the account-label codec (which truncates to its 80-byte field),
/// alias/note are user-visible verbatim, so we reject rather than
/// silently truncate.
pub fn encode_private_data_bounded(
    data: &ContactInfoPrivateData,
) -> Result<Vec<u8>, PlatformWalletError> {
    let out = encode_private_data(data);
    if out.len() > MAX_PLAINTEXT_LEN {
        return Err(PlatformWalletError::InvalidIdentityData(format!(
            "contactInfo privateData plaintext is {} bytes; the encrypted document would \
             exceed the contract's {PRIVATE_DATA_MAX_LEN}-byte cap (max plaintext \
             {MAX_PLAINTEXT_LEN} bytes) — shorten the alias/note",
            out.len()
        )));
    }
    Ok(out)
}

/// Decode a `privateData` plaintext (inverse of [`encode_private_data`]).
///
/// Tolerant per DIP-15 versioning: an unknown **major** version discards the
/// whole document (`Err`); trailing bytes past the known v0 fields (padding,
/// or a higher **minor** version's extra fields) are ignored.
pub fn decode_private_data(bytes: &[u8]) -> Result<ContactInfoPrivateData, PlatformWalletError> {
    let mut r = Reader::new(bytes);

    let version = r.u32_le()?;
    let major = version >> 16;
    if major != SUPPORTED_MAJOR {
        return Err(PlatformWalletError::InvalidIdentityData(format!(
            "contactInfo privateData major version {major} is incompatible — discarding"
        )));
    }

    let alias_name = empty_to_none(r.varstr()?);
    let note = empty_to_none(r.varstr()?);
    let display_hidden = r.u8()? != 0;

    let count = r.varint()?;
    // Bounded by the read: a bogus huge count errors out on the first missing
    // u32 (the buffer is ≤ 2048 bytes), so no unbounded allocation.
    let mut accepted_accounts = Vec::new();
    for _ in 0..count {
        accepted_accounts.push(r.u32_le()?);
    }

    // Ignore any trailing bytes (padding / higher-minor fields).
    Ok(ContactInfoPrivateData {
        alias_name,
        note,
        display_hidden,
        accepted_accounts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;

    fn test_wallet() -> Wallet {
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::None)
            .expect("test wallet")
    }

    /// The two feature children and distinct derivation indices must
    /// all yield distinct keys, deterministically.
    #[test]
    fn key_derivation_is_deterministic_and_domain_separated() {
        let wallet = test_wallet();

        let keys_a = derive_contact_info_keys(&wallet, Network::Testnet, 0, 2, 0).expect("derive");
        let keys_a2 = derive_contact_info_keys(&wallet, Network::Testnet, 0, 2, 0).expect("derive");
        assert_eq!(
            *keys_a.enc_to_user_id_key, *keys_a2.enc_to_user_id_key,
            "deterministic"
        );
        assert_ne!(
            *keys_a.enc_to_user_id_key, *keys_a.private_data_key,
            "65536' and 65537' children must differ"
        );

        let keys_b = derive_contact_info_keys(&wallet, Network::Testnet, 0, 2, 1).expect("derive");
        assert_ne!(
            *keys_a.enc_to_user_id_key, *keys_b.enc_to_user_id_key,
            "derivation index must be load-bearing"
        );
    }

    /// DIP-15 round-trip across present/absent strings and empty/non-empty
    /// `acceptedAccounts`.
    #[test]
    fn private_data_dip15_round_trips() {
        for data in [
            ContactInfoPrivateData {
                alias_name: Some("Alice".to_string()),
                note: Some("met at devnet UAT".to_string()),
                display_hidden: true,
                accepted_accounts: vec![1, 0xDEAD_BEEF, 42],
            },
            ContactInfoPrivateData::default(),
            ContactInfoPrivateData {
                alias_name: None,
                note: Some("note only".to_string()),
                display_hidden: false,
                accepted_accounts: vec![],
            },
        ] {
            let decoded = decode_private_data(&encode_private_data(&data)).expect("decode");
            assert_eq!(decoded, data);
        }
    }

    /// The exact DIP-15 wire bytes for a fixed input — pins the cross-client
    /// format so a refactor can't silently change it.
    #[test]
    fn private_data_wire_format_byte_vector() {
        let data = ContactInfoPrivateData {
            alias_name: Some("AB".to_string()),
            note: None,
            display_hidden: true,
            accepted_accounts: vec![1],
        };
        let encoded = encode_private_data(&data);
        assert_eq!(
            encoded,
            vec![
                0x00, 0x00, 0x00, 0x00, // version = 0
                0x02, 0x41, 0x42, // aliasName: len 2, "AB"
                0x00, // note: len 0
                0x01, // displayHidden = 1
                0x01, 0x01, 0x00, 0x00, 0x00, // acceptedAccounts: count 1, [1]
                0x00, 0x00, 0x00, // padding to the 17-byte plaintext floor
            ],
            "DIP-15 privateData wire format changed"
        );
        assert_eq!(decode_private_data(&encoded).expect("decode"), data);
    }

    /// Tiny payloads pad to the plaintext floor so the ciphertext clears 48
    /// bytes; the padding is ignored on decode.
    #[test]
    fn private_data_pads_to_plaintext_floor() {
        let empty = ContactInfoPrivateData::default();
        let encoded = encode_private_data(&empty);
        assert!(
            encoded.len() >= MIN_PLAINTEXT_LEN,
            "tiny payloads must be padded to ≥{MIN_PLAINTEXT_LEN} plaintext bytes (got {})",
            encoded.len()
        );
        assert_eq!(
            decode_private_data(&encoded).expect("decode padded"),
            empty,
            "padding must be ignored"
        );
    }

    /// Over-cap payloads are rejected BEFORE encryption/persist: the
    /// contract's `privateData` cap is 2048 bytes (IV + CBC ciphertext),
    /// so a plaintext past [`MAX_PLAINTEXT_LEN`] would fail the broadcast
    /// permanently — after the publish path already persisted local
    /// metadata. The bounded encoder must error, not truncate. Was red
    /// against the cap-less encoder.
    #[test]
    fn private_data_over_cap_is_rejected_at_cap_is_accepted() {
        // Fixed overhead around the note: version(4) + alias varstr(1, empty)
        // + note varint prefix + displayHidden(1) + accepted count varint(1).
        // A ~2100-byte note is safely over the 2031-byte plaintext cap.
        let over = ContactInfoPrivateData {
            alias_name: None,
            note: Some("x".repeat(2100)),
            display_hidden: false,
            accepted_accounts: Vec::new(),
        };
        assert!(
            encode_private_data_bounded(&over).is_err(),
            "a plaintext past MAX_PLAINTEXT_LEN must be rejected, not encrypted"
        );

        // Boundary: size the note so the encoded plaintext lands EXACTLY on
        // MAX_PLAINTEXT_LEN — must be accepted and round-trip.
        let mut at_cap = ContactInfoPrivateData {
            alias_name: None,
            note: Some(String::new()),
            display_hidden: false,
            accepted_accounts: Vec::new(),
        };
        // Find the note length whose encoding hits the cap exactly: encode
        // once to measure the fixed overhead of a 3-byte varstr prefix
        // (lengths ≥ 253 use 0xFD + u16).
        let overhead = {
            let probe = ContactInfoPrivateData {
                alias_name: None,
                note: Some("y".repeat(300)),
                display_hidden: false,
                accepted_accounts: Vec::new(),
            };
            encode_private_data(&probe).len() - 300
        };
        at_cap.note = Some("y".repeat(MAX_PLAINTEXT_LEN - overhead));
        let encoded = encode_private_data_bounded(&at_cap).expect("at-cap payload is admissible");
        assert_eq!(encoded.len(), MAX_PLAINTEXT_LEN);
        assert_eq!(decode_private_data(&encoded).expect("decode"), at_cap);
    }

    /// Forward-compat: a v0 decoder reading bytes with extra trailing data
    /// (a higher minor version's fields) parses the v0 fields and ignores
    /// the rest — DIP-15's minor-version rule.
    #[test]
    fn decode_ignores_trailing_higher_minor_fields() {
        let data = ContactInfoPrivateData {
            alias_name: Some("X".to_string()),
            note: None,
            display_hidden: false,
            accepted_accounts: vec![7],
        };
        let mut wire = encode_private_data(&data);
        // Append junk standing in for a future minor field after the v0 fields.
        wire.extend_from_slice(&[0xAB, 0xCD, 0xEF, 0x99, 0x01]);
        assert_eq!(
            decode_private_data(&wire).expect("decode"),
            data,
            "trailing higher-minor bytes must be ignored"
        );
    }

    /// An unknown MAJOR version discards the whole document.
    #[test]
    fn decode_rejects_incompatible_major() {
        let mut wire = encode_private_data(&ContactInfoPrivateData::default());
        // Set major = 1 (version = 1 << 16) — incompatible.
        wire[0..4].copy_from_slice(&(1u32 << 16).to_le_bytes());
        assert!(
            decode_private_data(&wire).is_err(),
            "an unknown major version must be rejected, not partially parsed"
        );
    }

    /// A truncated payload errors rather than panicking.
    #[test]
    fn decode_truncated_errors() {
        assert!(decode_private_data(&[0x00, 0x00]).is_err());
        // version ok, but aliasName claims 5 bytes that aren't there.
        assert!(decode_private_data(&[0x00, 0x00, 0x00, 0x00, 0x05, 0x41]).is_err());
    }

    /// End-to-end: derive keys, encrypt both fields, decrypt both — and the
    /// ciphertext blob respects the schema's 48..=2048 bounds.
    #[test]
    fn full_contact_info_encryption_round_trip() {
        let wallet = test_wallet();
        let keys = derive_contact_info_keys(&wallet, Network::Testnet, 0, 2, 0).expect("derive");

        let contact_id = [0x5Au8; 32];
        let enc =
            platform_encryption::encrypt_enc_to_user_id(&keys.enc_to_user_id_key, &contact_id);
        assert_eq!(
            platform_encryption::decrypt_enc_to_user_id(&keys.enc_to_user_id_key, &enc),
            contact_id
        );

        let data = ContactInfoPrivateData {
            alias_name: Some("Bob".to_string()),
            note: None,
            display_hidden: false,
            accepted_accounts: vec![3],
        };
        let iv = [0x77u8; 16];
        let blob = platform_encryption::encrypt_private_data(
            &keys.private_data_key,
            &iv,
            &encode_private_data(&data),
        );
        assert!(
            (PRIVATE_DATA_MIN_LEN..=2048).contains(&blob.len()),
            "blob must satisfy the schema's 48..=2048 bounds, got {}",
            blob.len()
        );
        let plain =
            platform_encryption::decrypt_private_data(&keys.private_data_key, &blob).expect("dec");
        assert_eq!(decode_private_data(&plain).expect("decode"), data);
    }
}
