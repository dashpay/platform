//! DashPay `contactInfo` self-encryption (DIP-15, M3 task 13).
//!
//! `contactInfo` documents carry the owner's PRIVATE per-contact
//! metadata (alias, note, hidden flag) — encrypted so only the owner
//! can read them, unlike `contactRequest` payloads which are shared
//! with the counterparty via ECDH.
//!
//! **No reference client ever implemented this document type**
//! (research/07: DashSync-iOS, dashj and dash-shared-core all lack
//! it), so the conventions here SET the de-facto wire format:
//!
//! - Key derivation (DIP-15): two hardened children of the identity's
//!   registered ENCRYPTION key in the owner's HD tree:
//!   `root / 65536' / index'` for `encToUserId`,
//!   `root / 65537' / index'` for `privateData`, where `root` is the
//!   identity-auth path of the key referenced by
//!   `rootEncryptionKeyIndex` and `index` is
//!   `derivationEncryptionKeyIndex`.
//! - `encToUserId`: AES-256-ECB of the 32-byte contact id (two raw
//!   blocks, no IV/padding — see `platform_encryption`'s rationale).
//! - `privateData`: `IV(16) ‖ AES-256-CBC(CBOR array
//!   [aliasName, note, displayHidden, padding?])`. The deployed
//!   schema's description ("array in cbor") wins over DIP-15 prose
//!   (varint stream with version/acceptedAccounts) — research/07 §C.
//!   A 4th byte-string element pads tiny payloads up to the schema's
//!   48-byte ciphertext floor; decoders read the first three elements
//!   and ignore the rest, which is also the forward-compat seam.

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

/// The deployed schema's `privateData` minimum length (bytes,
/// IV included). Tiny CBOR payloads are padded up to this floor via
/// the 4th array element.
const PRIVATE_DATA_MIN_LEN: usize = 48;

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
/// host-side signing hook (gap G4).
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

/// Decrypted `contactInfo.privateData` payload.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContactInfoPrivateData {
    /// User-chosen nickname for the contact.
    pub alias_name: Option<String>,
    /// Free-form note.
    pub note: Option<String>,
    /// Whether the contact is hidden from the contact list (also the
    /// cross-device reject signal — G5 stage 2).
    pub display_hidden: bool,
}

/// Encode the `privateData` plaintext as the CBOR array
/// `[aliasName, note, displayHidden, padding?]`.
///
/// The optional 4th element is a CBOR byte string sized so the
/// AES-256-CBC ciphertext (IV included) reaches the schema's 48-byte
/// floor; decoders ignore it.
pub fn encode_private_data(data: &ContactInfoPrivateData) -> Vec<u8> {
    use ciborium::Value;

    let text_or_null = |s: &Option<String>| match s {
        Some(v) => Value::Text(v.clone()),
        None => Value::Null,
    };

    let mut elements = vec![
        text_or_null(&data.alias_name),
        text_or_null(&data.note),
        Value::Bool(data.display_hidden),
    ];

    let serialize = |elements: &[Value]| -> Vec<u8> {
        let mut out = Vec::new();
        ciborium::into_writer(&Value::Array(elements.to_vec()), &mut out)
            .expect("CBOR serialization to a Vec cannot fail");
        out
    };

    let bare = serialize(&elements);
    // IV(16) + PKCS7-padded CBC needs ≥ 17 plaintext bytes to produce
    // a ≥ 32-byte ciphertext block region, i.e. a 48-byte blob.
    let min_plaintext = PRIVATE_DATA_MIN_LEN - 16 - 15;
    if bare.len() < min_plaintext {
        elements.push(Value::Bytes(vec![0u8; min_plaintext - bare.len()]));
        return serialize(&elements);
    }
    bare
}

/// Decode a `privateData` plaintext (inverse of
/// [`encode_private_data`]; tolerant of extra trailing elements).
pub fn decode_private_data(bytes: &[u8]) -> Result<ContactInfoPrivateData, PlatformWalletError> {
    use ciborium::Value;

    let value: Value = ciborium::from_reader(bytes).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "contactInfo privateData is not CBOR: {e}"
        ))
    })?;
    let Value::Array(elements) = value else {
        return Err(PlatformWalletError::InvalidIdentityData(
            "contactInfo privateData is not a CBOR array".to_string(),
        ));
    };
    if elements.len() < 3 {
        return Err(PlatformWalletError::InvalidIdentityData(format!(
            "contactInfo privateData array has {} elements (need ≥ 3)",
            elements.len()
        )));
    }

    let text_or_none = |v: &Value| match v {
        Value::Text(s) => Some(s.clone()),
        _ => None,
    };

    Ok(ContactInfoPrivateData {
        alias_name: text_or_none(&elements[0]),
        note: text_or_none(&elements[1]),
        display_hidden: matches!(elements[2], Value::Bool(true))
            || matches!(&elements[2], Value::Integer(i) if *i == 1.into()),
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

    /// CBOR round-trip across present/absent fields, and the padded
    /// minimal payload still decodes (the 4th element is ignored).
    #[test]
    fn private_data_cbor_round_trips_and_pads_to_schema_floor() {
        let full = ContactInfoPrivateData {
            alias_name: Some("Alice".to_string()),
            note: Some("met at devnet UAT".to_string()),
            display_hidden: true,
        };
        let decoded = decode_private_data(&encode_private_data(&full)).expect("decode");
        assert_eq!(decoded, full);

        let empty = ContactInfoPrivateData::default();
        let encoded = encode_private_data(&empty);
        assert!(
            encoded.len() >= 17,
            "tiny payloads must be padded so IV + CBC ciphertext ≥ 48 bytes (got {} plaintext)",
            encoded.len()
        );
        let decoded = decode_private_data(&encoded).expect("decode padded");
        assert_eq!(decoded, empty, "padding element must be ignored");
    }

    /// End-to-end: derive keys, encrypt both fields, decrypt both
    /// fields — and the ciphertext blob respects the schema bounds.
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
