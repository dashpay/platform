//! Encrypting and decrypting the byte properties a document type declares `encryptedFor`.
//!
//! A property declared
//! `"encryptedFor": { "recipient": ..., "recipientKey": ..., "senderKey": ..., "scheme": ... }`
//! (protocol version 14) says how its bytes were produced: for which identity, under which two
//! identity keys, named by two key id properties of the same document, and under which
//! [`EncryptionScheme`]. Every helper here reads that declaration from the document type it is
//! given, so they work for any contract that declares one:
//!
//! * [`encrypt_property`] encrypts a plaintext into the property with the keys it is given and
//!   writes the two key id properties;
//! * [`decrypt_property`] reads the property back;
//! * [`select_encryption_keys`] picks the two keys the document type's `identityPublicKey`
//!   references demand through their `keyRequirements`, so a write does not name a key
//!   consensus refuses;
//! * [`encrypt_property_for`] does both and also writes the recipient property;
//! * [`EncryptedPropertyEnvelope::read`] says, for a stored document, whose keys a reader needs.
//!
//! The one scheme, `ecdh-secp256k1-aes256-cbc`, is the one dashpay contact requests use
//! (DIP-15): the shared key is the libsecp256k1 ECDH of one side's private key and the other
//! side's public key, `SHA256((y & 1 | 2) || x)` of the product point, and the bytes are a random
//! 16-byte IV followed by the plaintext under AES-256-CBC with PKCS7 padding. ECDH is symmetric,
//! so the sender reads its own message back with its private key and the recipient's public key.
//!
//! The scheme carries no authentication tag. A wrong key is caught only by the padding check,
//! which a wrong key passes about once in 256 attempts and then returns garbage; a caller that
//! must tell the two apart has to recognise its plaintext.

use crate::Error;
use dpp::dashcore::secp256k1::rand::rngs::StdRng;
use dpp::dashcore::secp256k1::rand::{RngCore, SeedableRng};
use dpp::dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentTypeRef, EncryptedFor,
    EncryptedForRecipient, EncryptionScheme, IdentityKeyReferenceRequirements,
    KeyReferenceIdentityProperty,
};
use dpp::document::{Document, DocumentV0Getters};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose};
use dpp::platform_value::btreemap_extensions::{
    BTreeValueMapInsertionPathHelper, BTreeValueMapPathHelper,
};
use dpp::platform_value::{Identifier, Value};
use platform_encryption::{decrypt_aes_256_cbc, derive_shared_key_ecdh, encrypt_aes_256_cbc};
use std::collections::BTreeMap;
use std::fmt;

/// The length of the IV that prefixes an `ecdh-secp256k1-aes256-cbc` ciphertext.
const AES_CBC_IV_LENGTH: usize = 16;

/// Why an `encryptedFor` helper failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncryptedForError {
    /// The document type has no such property, or it declares no `encryptedFor`.
    #[error("property {property} of document type {document_type} declares no encryptedFor")]
    NotDeclared {
        /// The document type.
        document_type: String,
        /// The property path asked for.
        property: String,
    },
    /// The document does not carry a property the declaration names.
    #[error("the document has no {path} property")]
    MissingProperty {
        /// The property path.
        path: String,
    },
    /// A property the declaration names holds a value of the wrong kind.
    #[error("property {path} is malformed: {reason}")]
    InvalidProperty {
        /// The property path.
        path: String,
        /// What is wrong with it.
        reason: String,
    },
    /// The bytes do not have the shape the scheme produces, the one consensus checks.
    #[error(
        "property {path} holds {length} bytes, not a {scheme} ciphertext (at least {} bytes, a \
         multiple of {})",
        scheme.minimum_ciphertext_length(),
        scheme.block_length()
    )]
    InvalidCiphertextLength {
        /// The property path.
        path: String,
        /// The declared scheme.
        scheme: EncryptionScheme,
        /// The length found.
        length: usize,
    },
    /// The padding did not check out: the keys are not the ones the bytes were encrypted
    /// with, or the bytes are corrupt.
    #[error("decryption failed: the keys are not the ones the property was encrypted with")]
    DecryptionFailed,
    /// A key is not a valid secp256k1 key.
    #[error("invalid key: {0}")]
    InvalidKey(String),
    /// No key of an identity can play a role the declaration needs.
    #[error("no key of identity {identity_id} can be the {role} key: {reason}")]
    NoSuitableKey {
        /// The identity.
        identity_id: Identifier,
        /// The role the key was wanted for.
        role: EncryptionKeyRole,
        /// Why none fits.
        reason: String,
    },
    /// The declaration's recipient is the document owner, the writer, but another identity was
    /// named as the recipient.
    #[error(
        "property {path} is encrypted for the document owner, so the recipient must be the \
         sender {sender_id}, not {recipient_id}"
    )]
    RecipientMustBeOwner {
        /// The property path.
        path: String,
        /// The sender, the writer and owner.
        sender_id: Identifier,
        /// The recipient named.
        recipient_id: Identifier,
    },
}

impl From<EncryptedForError> for Error {
    fn from(error: EncryptedForError) -> Self {
        Error::Generic(error.to_string())
    }
}

/// Which side of an encrypted property a key belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionKeyRole {
    /// The writer's key, named by the declaration's `senderKey` property.
    Sender,
    /// The reader's key, named by the declaration's `recipientKey` property.
    Recipient,
}

impl fmt::Display for EncryptionKeyRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            EncryptionKeyRole::Sender => "sender",
            EncryptionKeyRole::Recipient => "recipient",
        })
    }
}

/// The two keys a message is encrypted under: the ids written into the document's key id
/// properties, the sender's private key and the recipient's public key.
#[derive(Clone, Copy)]
pub struct EncryptionKeys {
    /// The id of the sender's key, written into the declaration's `senderKey` property.
    pub sender_key_id: KeyID,
    /// The private half of that key.
    pub sender_private_key: SecretKey,
    /// The id of the recipient's key, written into the declaration's `recipientKey` property.
    pub recipient_key_id: KeyID,
    /// The public half of that key.
    pub recipient_public_key: PublicKey,
}

impl fmt::Debug for EncryptionKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncryptionKeys")
            .field("sender_key_id", &self.sender_key_id)
            .field("recipient_key_id", &self.recipient_key_id)
            .field("recipient_public_key", &self.recipient_public_key)
            .finish_non_exhaustive()
    }
}

/// The `encryptedFor` declaration of `property_path` on `document_type`.
pub fn encrypted_for_declaration(
    document_type: DocumentTypeRef<'_>,
    property_path: &str,
) -> Result<EncryptedFor, EncryptedForError> {
    document_type
        .flattened_properties()
        .get(property_path)
        .and_then(|property| property.encrypted_for.clone())
        .ok_or_else(|| EncryptedForError::NotDeclared {
            document_type: document_type.name().clone(),
            property: property_path.to_string(),
        })
}

/// Encrypts `plaintext` into the `property_path` property of `properties` under `keys`, the
/// way the property's `encryptedFor` declaration says, and writes the declaration's
/// `recipientKey` and `senderKey` properties with the two key ids. The recipient property is the
/// caller's to write; [`encrypt_property_for`] writes it too.
///
/// The IV is fresh randomness on every call. The plaintext's length is not checked here: the
/// property's own byte bounds are, when the document is validated.
pub fn encrypt_property(
    document_type: DocumentTypeRef<'_>,
    property_path: &str,
    plaintext: &[u8],
    keys: &EncryptionKeys,
    properties: &mut BTreeMap<String, Value>,
) -> Result<(), EncryptedForError> {
    let mut iv = [0u8; AES_CBC_IV_LENGTH];
    StdRng::from_entropy().fill_bytes(&mut iv);
    encrypt_property_with_iv(
        document_type,
        property_path,
        plaintext,
        keys,
        &iv,
        properties,
    )
}

/// [`encrypt_property`] with the IV given. Private: an IV must never be reused under one
/// shared key, so only tests pin it.
fn encrypt_property_with_iv(
    document_type: DocumentTypeRef<'_>,
    property_path: &str,
    plaintext: &[u8],
    keys: &EncryptionKeys,
    iv: &[u8; AES_CBC_IV_LENGTH],
    properties: &mut BTreeMap<String, Value>,
) -> Result<(), EncryptedForError> {
    let declaration = encrypted_for_declaration(document_type, property_path)?;
    let ciphertext = match declaration.scheme {
        EncryptionScheme::EcdhSecp256k1Aes256Cbc => {
            let shared_key =
                derive_shared_key_ecdh(&keys.sender_private_key, &keys.recipient_public_key);
            let mut bytes = iv.to_vec();
            bytes.extend(encrypt_aes_256_cbc(&shared_key, iv, plaintext));
            bytes
        }
    };
    insert(properties, property_path, Value::Bytes(ciphertext))?;
    insert(
        properties,
        &declaration.recipient_key,
        Value::U32(keys.recipient_key_id),
    )?;
    insert(
        properties,
        &declaration.sender_key,
        Value::U32(keys.sender_key_id),
    )
}

/// Decrypts the `property_path` property of `properties`, the way its `encryptedFor`
/// declaration says, with the recipient's private key and the sender's public key: the keys
/// the document's `recipientKey` and `senderKey` properties name (see
/// [`EncryptedPropertyEnvelope::read`]). ECDH is symmetric, so the sender may pass its own
/// private key and the recipient's public key instead.
///
/// A ciphertext of the wrong shape is refused before any decryption. See the module docs for
/// what a wrong key does.
pub fn decrypt_property(
    document_type: DocumentTypeRef<'_>,
    property_path: &str,
    properties: &BTreeMap<String, Value>,
    recipient_private_key: &SecretKey,
    sender_public_key: &PublicKey,
) -> Result<Vec<u8>, EncryptedForError> {
    let declaration = encrypted_for_declaration(document_type, property_path)?;
    let bytes = ciphertext_at_path(properties, property_path)?;
    let scheme = declaration.scheme;
    if !scheme.is_valid_ciphertext_length(bytes.len()) {
        return Err(EncryptedForError::InvalidCiphertextLength {
            path: property_path.to_string(),
            scheme,
            length: bytes.len(),
        });
    }
    match scheme {
        EncryptionScheme::EcdhSecp256k1Aes256Cbc => {
            let Some((iv, blocks)) = bytes.split_first_chunk::<AES_CBC_IV_LENGTH>() else {
                return Err(EncryptedForError::InvalidCiphertextLength {
                    path: property_path.to_string(),
                    scheme,
                    length: bytes.len(),
                });
            };
            let shared_key = derive_shared_key_ecdh(recipient_private_key, sender_public_key);
            decrypt_aes_256_cbc(&shared_key, iv, blocks)
                .map_err(|_| EncryptedForError::DecryptionFailed)
        }
    }
}

/// Whose keys an encrypted property of a stored document is under: what a reader fetches to
/// decrypt it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncryptedPropertyEnvelope {
    /// The identity the bytes are encrypted for: the declaration's recipient property, or the
    /// document owner when the declaration names `$ownerId`.
    pub recipient_id: Identifier,
    /// The id of the recipient's key, from the declaration's `recipientKey` property.
    pub recipient_key_id: KeyID,
    /// The identity whose key the `senderKey` property names: the identity the schema's
    /// `identityPublicKey` reference on that key id says, and the document owner, the writer,
    /// when the schema declares none.
    pub sender_id: Identifier,
    /// The id of the sender's key, from the declaration's `senderKey` property.
    pub sender_key_id: KeyID,
}

impl EncryptedPropertyEnvelope {
    /// Reads the envelope of the `property_path` property of `document`, a document of
    /// `document_type`.
    pub fn read(
        document_type: DocumentTypeRef<'_>,
        property_path: &str,
        document: &Document,
    ) -> Result<Self, EncryptedForError> {
        let declaration = encrypted_for_declaration(document_type, property_path)?;
        let properties = document.properties();
        let recipient_id = match &declaration.recipient {
            EncryptedForRecipient::Owner => document.owner_id(),
            EncryptedForRecipient::Property(path) => identifier_at_path(properties, path)?,
        };
        let sender_id = match key_identity_source(document_type, &declaration.sender_key) {
            Some(KeyIdentitySource::Property(path)) => identifier_at_path(properties, &path)?,
            Some(KeyIdentitySource::Creator) => {
                document.creator_id().unwrap_or_else(|| document.owner_id())
            }
            Some(KeyIdentitySource::Owner) | None => document.owner_id(),
        };
        Ok(Self {
            recipient_id,
            recipient_key_id: key_id_at_path(properties, &declaration.recipient_key)?,
            sender_id,
            sender_key_id: key_id_at_path(properties, &declaration.sender_key)?,
        })
    }
}

/// Picks the keys to encrypt the `property_path` property of a `document_type` document under,
/// from `sender`, the writer, to `recipient`.
///
/// The sender's key is the key of `sender` whose public key `sender_private_key` derives; the
/// recipient's is the enabled `ECDSA_SECP256K1` key of `recipient` with the highest id among
/// those that fit, a `DECRYPTION` key before an `ENCRYPTION` one. A key fits when it meets every
/// `keyRequirements` the document type's `identityPublicKey` references declare on the key id
/// property that names it (the checks consensus runs when the document is written) and is not
/// disabled. When the schema requires no purpose for a side, the dashpay convention stands in:
/// an `ENCRYPTION` key for the sender, a `DECRYPTION` or `ENCRYPTION` key for the recipient, so
/// an authentication key is never used for ECDH.
pub fn select_encryption_keys(
    document_type: DocumentTypeRef<'_>,
    property_path: &str,
    sender: &Identity,
    sender_private_key: &SecretKey,
    recipient: &Identity,
) -> Result<EncryptionKeys, EncryptedForError> {
    let declaration = encrypted_for_declaration(document_type, property_path)?;
    let contract_id = document_type.data_contract_id();

    let sender_requirements = key_requirements_naming(document_type, &declaration.sender_key);
    let sender_public_key =
        PublicKey::from_secret_key(&Secp256k1::signing_only(), sender_private_key);
    let sender_public_key_bytes = sender_public_key.serialize();
    let no_sender_key = |reason: String| EncryptedForError::NoSuitableKey {
        identity_id: sender.id(),
        role: EncryptionKeyRole::Sender,
        reason,
    };
    let sender_key = sender
        .public_keys()
        .values()
        .find(|key| {
            key.key_type() == KeyType::ECDSA_SECP256K1
                && key.data().as_slice() == sender_public_key_bytes.as_slice()
        })
        .ok_or_else(|| {
            no_sender_key("the private key given is not one of its ECDSA_SECP256K1 keys".into())
        })?;
    if let Some(reason) = why_unfit(
        sender_key,
        &sender_requirements,
        &[Purpose::ENCRYPTION],
        contract_id,
    ) {
        return Err(no_sender_key(format!(
            "the key of the private key given, {}, {reason}",
            sender_key.id()
        )));
    }

    let recipient_requirements = key_requirements_naming(document_type, &declaration.recipient_key);
    let recipient_key = recipient
        .public_keys()
        .values()
        .filter(|key| key.key_type() == KeyType::ECDSA_SECP256K1)
        .filter(|key| {
            why_unfit(
                key,
                &recipient_requirements,
                &[Purpose::DECRYPTION, Purpose::ENCRYPTION],
                contract_id,
            )
            .is_none()
        })
        .max_by_key(|key| (key.purpose() == Purpose::DECRYPTION, key.id()))
        .ok_or_else(|| EncryptedForError::NoSuitableKey {
            identity_id: recipient.id(),
            role: EncryptionKeyRole::Recipient,
            reason: format!(
                "none of its enabled ECDSA_SECP256K1 keys {}",
                describe_requirements(&recipient_requirements, "decryption or encryption")
            ),
        })?;
    let recipient_public_key = PublicKey::from_slice(recipient_key.data().as_slice())
        .map_err(|e| EncryptedForError::InvalidKey(format!("recipient key: {e}")))?;

    Ok(EncryptionKeys {
        sender_key_id: sender_key.id(),
        sender_private_key: *sender_private_key,
        recipient_key_id: recipient_key.id(),
        recipient_public_key,
    })
}

/// Encrypts `plaintext` from `sender`, the writer, to `recipient` into the `property_path`
/// property of `properties`: picks the keys with [`select_encryption_keys`], writes the
/// recipient property with the recipient's id when the declaration names one, and encrypts
/// with [`encrypt_property`]. Returns the keys used.
///
/// A declaration whose recipient is `$ownerId` encrypts to the writer, so `recipient` must then
/// be `sender`.
pub fn encrypt_property_for(
    document_type: DocumentTypeRef<'_>,
    property_path: &str,
    plaintext: &[u8],
    sender: &Identity,
    sender_private_key: &SecretKey,
    recipient: &Identity,
    properties: &mut BTreeMap<String, Value>,
) -> Result<EncryptionKeys, EncryptedForError> {
    let declaration = encrypted_for_declaration(document_type, property_path)?;
    let keys = select_encryption_keys(
        document_type,
        property_path,
        sender,
        sender_private_key,
        recipient,
    )?;
    match &declaration.recipient {
        EncryptedForRecipient::Owner if recipient.id() != sender.id() => {
            return Err(EncryptedForError::RecipientMustBeOwner {
                path: property_path.to_string(),
                sender_id: sender.id(),
                recipient_id: recipient.id(),
            });
        }
        EncryptedForRecipient::Owner => {}
        EncryptedForRecipient::Property(path) => insert(
            properties,
            path,
            Value::Identifier(recipient.id().to_buffer()),
        )?,
    }
    encrypt_property(document_type, property_path, plaintext, &keys, properties)?;
    Ok(keys)
}

/// Whose key a key id property names, as the schema's `identityPublicKey` references say.
enum KeyIdentitySource {
    Owner,
    Creator,
    Property(String),
}

/// Where the identity of the key `key_path` names comes from: a `refersTo: identityPublicKey`
/// on the key id itself (`identityProperty`), or on an identifier property whose
/// `keyIdProperty` is `key_path`. `None` when the schema declares neither.
fn key_identity_source(
    document_type: DocumentTypeRef<'_>,
    key_path: &str,
) -> Option<KeyIdentitySource> {
    document_type
        .flattened_properties()
        .iter()
        .find_map(|(path, property)| match &property.property_type {
            DocumentPropertyType::KeyIdWithReference(reference) if path == key_path => {
                Some(match &reference.identity_property {
                    KeyReferenceIdentityProperty::OwnerId => KeyIdentitySource::Owner,
                    KeyReferenceIdentityProperty::CreatorId => KeyIdentitySource::Creator,
                    KeyReferenceIdentityProperty::Property(identity_path) => {
                        KeyIdentitySource::Property(identity_path.clone())
                    }
                })
            }
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::IdentityPublicKey {
                    key_id_property, ..
                },
            ) if key_id_property == key_path => Some(KeyIdentitySource::Property(path.clone())),
            _ => None,
        })
}

/// Every `keyRequirements` the schema declares on a reference to the key `key_path` names: on
/// the key id property itself, and on each identifier property whose `keyIdProperty` it is.
fn key_requirements_naming(
    document_type: DocumentTypeRef<'_>,
    key_path: &str,
) -> Vec<IdentityKeyReferenceRequirements> {
    document_type
        .flattened_properties()
        .iter()
        .filter_map(|(path, property)| match &property.property_type {
            DocumentPropertyType::KeyIdWithReference(reference) if path == key_path => {
                Some(reference.key_requirements.clone())
            }
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::IdentityPublicKey {
                    key_id_property,
                    key_requirements,
                },
            ) if key_id_property == key_path => Some(key_requirements.clone()),
            _ => None,
        })
        .collect()
}

/// Why `key` cannot be named where `requirements` apply, `None` when it can. With no purpose
/// among the requirements, the key's purpose must be one of `default_purposes`.
fn why_unfit(
    key: &IdentityPublicKey,
    requirements: &[IdentityKeyReferenceRequirements],
    default_purposes: &[Purpose],
    declaring_contract_id: Identifier,
) -> Option<String> {
    if key.disabled_at().is_some() {
        return Some("is disabled".to_string());
    }
    for declaration in requirements {
        if let Some(unmet) = declaration.first_unmet_by(key, declaring_contract_id) {
            return Some(format!(
                "does not meet keyRequirements {} {}: it has {}",
                unmet.field(),
                unmet.required(),
                unmet.actual_of(key)
            ));
        }
    }
    let purpose_declared = requirements
        .iter()
        .any(|declaration| declaration.purpose.is_some());
    if !purpose_declared && !default_purposes.contains(&key.purpose()) {
        return Some(format!(
            "has purpose {}, not an encryption purpose",
            key.purpose().wire_name()
        ));
    }
    None
}

/// The requirements a key must meet, in words, for an error that found none meeting them.
fn describe_requirements(
    requirements: &[IdentityKeyReferenceRequirements],
    default_purposes: &str,
) -> String {
    let mut clauses: Vec<String> = requirements
        .iter()
        .flat_map(|declaration| declaration.requirements())
        .map(|requirement| format!("{} {}", requirement.field(), requirement.required()))
        .collect();
    if !requirements
        .iter()
        .any(|declaration| declaration.purpose.is_some())
    {
        clauses.push(format!("purpose {default_purposes}"));
    }
    format!("meets {}", clauses.join(", "))
}

fn insert(
    properties: &mut BTreeMap<String, Value>,
    path: &str,
    value: Value,
) -> Result<(), EncryptedForError> {
    properties
        .insert_at_path(path, value)
        .map_err(|e| EncryptedForError::InvalidProperty {
            path: path.to_string(),
            reason: e.to_string(),
        })
}

fn required_at_path<'a>(
    properties: &'a BTreeMap<String, Value>,
    path: &str,
) -> Result<&'a Value, EncryptedForError> {
    properties
        .get_optional_at_path(path)
        .map_err(|e| EncryptedForError::InvalidProperty {
            path: path.to_string(),
            reason: e.to_string(),
        })?
        .ok_or_else(|| EncryptedForError::MissingProperty {
            path: path.to_string(),
        })
}

/// The ciphertext at `path`: byte values, or a list of integers each a byte, which is how a
/// document built without its contract (from JavaScript, say) holds a byte array.
fn ciphertext_at_path(
    properties: &BTreeMap<String, Value>,
    path: &str,
) -> Result<Vec<u8>, EncryptedForError> {
    let invalid = |reason: String| EncryptedForError::InvalidProperty {
        path: path.to_string(),
        reason,
    };
    match required_at_path(properties, path)? {
        Value::Array(items) => items
            .iter()
            .map(|item| item.to_integer::<u8>().map_err(|e| invalid(e.to_string())))
            .collect(),
        value => value.to_binary_bytes().map_err(|e| invalid(e.to_string())),
    }
}

fn identifier_at_path(
    properties: &BTreeMap<String, Value>,
    path: &str,
) -> Result<Identifier, EncryptedForError> {
    required_at_path(properties, path)?
        .to_identifier()
        .map_err(|e| EncryptedForError::InvalidProperty {
            path: path.to_string(),
            reason: e.to_string(),
        })
}

fn key_id_at_path(
    properties: &BTreeMap<String, Value>,
    path: &str,
) -> Result<KeyID, EncryptedForError> {
    required_at_path(properties, path)?
        .to_integer::<KeyID>()
        .map_err(|e| EncryptedForError::InvalidProperty {
            path: path.to_string(),
            reason: e.to_string(),
        })
}

#[cfg(test)]
mod tests;
