//! The `encryptedFor` declaration of a byte array property: how its ciphertext
//! was produced, so that a wallet or SDK reads the recipe from the contract
//! instead of a side channel.
//!
//! Consensus can only check the shape of the bytes (see
//! [`EncryptionScheme::is_valid_ciphertext_length`]); who can decrypt them,
//! and whether they decrypt at all, is not verifiable on chain.

use crate::document::property_names::OWNER_ID;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The `encryptedFor` declaration of a byte array property.
///
/// Declared as
/// `"encryptedFor": { "recipient": "...", "recipientKey": "...", "senderKey": "...", "scheme": "..." }`
/// on a `byteArray` property that is not an identifier. The three paths name
/// properties of the same document type: the recipient an identifier property
/// (or the writer's own `$ownerId`), the two keys integer properties bounded
/// to `u32` that carry identity key ids. The parser checks all three exist
/// with those types when the contract is registered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncryptedFor {
    /// Whose keys decrypt the bytes.
    pub recipient: EncryptedForRecipient,
    /// Path of the integer property carrying the id of the recipient's key.
    pub recipient_key: String,
    /// Path of the integer property carrying the id of the sender's key.
    pub sender_key: String,
    /// How the bytes were produced.
    pub scheme: EncryptionScheme,
}

/// The recipient of an [`EncryptedFor`] declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncryptedForRecipient {
    /// The document's owner: a message the writer encrypts to themself.
    Owner,
    /// The identifier property, of the same document type, whose value is the
    /// recipient identity's id. A dotted path when the property is nested.
    Property(String),
}

impl EncryptedForRecipient {
    /// The recipient a schema path names: `$ownerId` is the owner, anything
    /// else a property path.
    pub fn from_path(path: &str) -> Self {
        if path == OWNER_ID {
            EncryptedForRecipient::Owner
        } else {
            EncryptedForRecipient::Property(path.to_string())
        }
    }

    /// The path as the schema spells it: `$ownerId` for the owner.
    pub fn as_path(&self) -> &str {
        match self {
            EncryptedForRecipient::Owner => OWNER_ID,
            EncryptedForRecipient::Property(path) => path.as_str(),
        }
    }

    /// The property path when the recipient is a property, `None` for the owner.
    pub fn property_path(&self) -> Option<&str> {
        match self {
            EncryptedForRecipient::Owner => None,
            EncryptedForRecipient::Property(path) => Some(path.as_str()),
        }
    }
}

impl Serialize for EncryptedForRecipient {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_path())
    }
}

impl<'de> Deserialize<'de> for EncryptedForRecipient {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let path = String::deserialize(deserializer)?;
        Ok(EncryptedForRecipient::from_path(&path))
    }
}

impl fmt::Display for EncryptedForRecipient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_path())
    }
}

/// The scheme under which an [`EncryptedFor`] property's bytes were produced.
///
/// Consensus knows the shape each scheme produces and nothing more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionScheme {
    /// The scheme the dashpay contact request uses for `encryptedPublicKey`
    /// (DIP-15): the shared key is the libsecp256k1 ECDH of the sender's
    /// private key and the recipient's public key, `SHA256((y & 1 | 2) || x)`
    /// of the product point; the bytes are a random 16-byte IV followed by
    /// the plaintext under AES-256-CBC with PKCS7 padding and that IV. So a
    /// ciphertext is at least 32 bytes and always a multiple of 16.
    #[serde(rename = "ecdh-secp256k1-aes256-cbc")]
    EcdhSecp256k1Aes256Cbc,
}

impl EncryptionScheme {
    /// Every scheme, in wire-name order.
    pub const ALL: [EncryptionScheme; 1] = [EncryptionScheme::EcdhSecp256k1Aes256Cbc];

    /// The wire name, the value of `encryptedFor.scheme`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            EncryptionScheme::EcdhSecp256k1Aes256Cbc => "ecdh-secp256k1-aes256-cbc",
        }
    }

    /// The scheme a wire name names, `None` for any other name.
    pub fn from_wire_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|scheme| scheme.as_str() == name)
    }

    /// The cipher's block length in bytes; a ciphertext is a multiple of it.
    pub const fn block_length(&self) -> usize {
        match self {
            EncryptionScheme::EcdhSecp256k1Aes256Cbc => 16,
        }
    }

    /// The shortest ciphertext the scheme can produce: the IV prefix plus one
    /// padded block.
    pub const fn minimum_ciphertext_length(&self) -> usize {
        match self {
            EncryptionScheme::EcdhSecp256k1Aes256Cbc => 16 + 16,
        }
    }

    /// Whether `length` bytes have the shape the scheme produces. This is all
    /// consensus can tell about a ciphertext.
    pub fn is_valid_ciphertext_length(&self, length: usize) -> bool {
        length >= self.minimum_ciphertext_length() && length.is_multiple_of(self.block_length())
    }
}

impl fmt::Display for EncryptionScheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn should_round_trip_a_declaration_through_json_with_the_schema_spelling() {
        let declaration = EncryptedFor {
            recipient: EncryptedForRecipient::Property("recipientId".to_string()),
            recipient_key: "recipientKeyId".to_string(),
            sender_key: "senderKeyId".to_string(),
            scheme: EncryptionScheme::EcdhSecp256k1Aes256Cbc,
        };
        let json = serde_json::to_value(&declaration).expect("should serialize");
        assert_eq!(
            json,
            json!({
                "recipient": "recipientId",
                "recipientKey": "recipientKeyId",
                "senderKey": "senderKeyId",
                "scheme": "ecdh-secp256k1-aes256-cbc"
            })
        );
        let back: EncryptedFor = serde_json::from_value(json).expect("should deserialize");
        assert_eq!(back, declaration);
    }

    #[test]
    fn should_spell_the_owner_recipient_as_owner_id() {
        let owner = EncryptedForRecipient::from_path("$ownerId");
        assert_eq!(owner, EncryptedForRecipient::Owner);
        assert_eq!(owner.as_path(), "$ownerId");
        assert_eq!(owner.property_path(), None);
        assert_eq!(
            serde_json::to_value(&owner).expect("should serialize"),
            json!("$ownerId")
        );
        let property = EncryptedForRecipient::from_path("recipientId");
        assert_eq!(property.property_path(), Some("recipientId"));
    }

    #[test]
    fn should_refuse_an_unknown_scheme_and_name_every_known_one() {
        assert_eq!(EncryptionScheme::from_wire_name("rsa"), None);
        for scheme in EncryptionScheme::ALL {
            assert_eq!(
                EncryptionScheme::from_wire_name(scheme.as_str()),
                Some(scheme)
            );
            assert_eq!(
                serde_json::to_value(scheme).expect("should serialize"),
                json!(scheme.as_str())
            );
        }
    }

    #[test]
    fn should_accept_only_an_iv_plus_whole_blocks_for_aes_cbc() {
        let scheme = EncryptionScheme::EcdhSecp256k1Aes256Cbc;
        assert_eq!(scheme.minimum_ciphertext_length(), 32);
        assert_eq!(scheme.block_length(), 16);
        for (length, valid) in [
            (0, false),
            (16, false),
            (31, false),
            (32, true),
            (47, false),
            (48, true),
            (96, true),
            (1040, true),
            (1041, false),
        ] {
            assert_eq!(
                scheme.is_valid_ciphertext_length(length),
                valid,
                "{length} bytes"
            );
        }
    }
}
