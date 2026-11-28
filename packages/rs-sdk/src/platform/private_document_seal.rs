//! Sealing an entry of any private document type (protocol version 15).
//!
//! A private type's store holds fixed-size entries in the standard private document envelope,
//! each sealed to one key of one identity. [`seal_private_document`] checks what a sender asks
//! for against the type and the identities' keys, and seals one entry, of the type's entry
//! size, that a wallet then appends, paying from its notes, with
//! [`PrivateDocumentCreateFromShieldedPool`](crate::platform::transition::private_document_create_from_shielded_pool::PrivateDocumentCreateFromShieldedPool)
//! or the dpp builder.
//!
//! The keys follow the rule a wallet reads a private type's store with
//! ([`is_private_document_key`]): an entry is sealed to an `ECDSA_SECP256K1` `DECRYPTION` or
//! `ENCRYPTION` key of the recipient, and sealed with the outgoing viewing key of an
//! `ECDSA_SECP256K1` `ENCRYPTION` key of the sender, so the sender's wallet recovers it from the
//! store alone. A sender names itself or not ([`PrivateDocumentSenderAuth`]); a signed entry,
//! which DashPay's private contact requests are ([`crate::platform::dashpay`]), needs a rule for
//! which keys may sign for the type, and is not built here.

use std::collections::BTreeMap;
use std::fmt;

use crate::platform::private_document_scan::validate_private_document_properties;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::encode_private_document_payload;
use dpp::data_contract::DataContract;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose};
use dpp::platform_value::{BinaryData, Identifier, Value};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use platform_encryption::{
    private_document_outgoing_viewing_key, seal_private_document_entry,
    PrivateDocumentEnvelopeError, PrivateDocumentRecipient, PrivateDocumentSender,
    PrivateDocumentTarget, PRIVATE_DOCUMENT_ENVELOPE_OVERHEAD_LEN,
};
use secp256k1::{PublicKey, SecretKey};
use zeroize::{Zeroize, Zeroizing};

/// Whose key a [`PrivateDocumentSealError`] refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrivateDocumentKeyRole {
    /// The sender's key, which the entry is sealed with.
    Sender,
    /// The recipient's key, which the entry is sealed to.
    Recipient,
}

impl fmt::Display for PrivateDocumentKeyRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Sender => "sender",
            Self::Recipient => "recipient",
        })
    }
}

/// The keys a key of `role` must be, as a refusal names them.
fn expected_key(role: PrivateDocumentKeyRole) -> &'static str {
    match role {
        PrivateDocumentKeyRole::Sender => "an ECDSA_SECP256K1 ENCRYPTION key",
        PrivateDocumentKeyRole::Recipient => "an ECDSA_SECP256K1 DECRYPTION or ENCRYPTION key",
    }
}

/// Why an entry was not sealed. Every variant but [`Self::SecretKeyMismatch`] and
/// [`Self::Sealing`] refuses what the sender asked for
/// ([`Self::is_refused_request`]): the type, a key, or the properties.
#[derive(Debug, thiserror::Error)]
pub enum PrivateDocumentSealError {
    /// The contract declares no such document type.
    #[error("the contract {contract_id} has no document type \"{document_type_name}\": {reason}")]
    UnknownDocumentType {
        /// The contract asked for.
        contract_id: Identifier,
        /// The type asked for.
        document_type_name: String,
        /// The contract's reason.
        reason: String,
    },
    /// The document type is not private.
    #[error("the document type \"{document_type_name}\" of contract {contract_id} is not private")]
    NotPrivate {
        /// The contract asked for.
        contract_id: Identifier,
        /// The type asked for.
        document_type_name: String,
    },
    /// The identity has no key of that id.
    #[error("the {role} identity {identity_id} has no key {key_id}")]
    MissingKey {
        /// Whose key.
        role: PrivateDocumentKeyRole,
        /// The identity asked for.
        identity_id: Identifier,
        /// The key asked for.
        key_id: KeyID,
    },
    /// The key is not of a type and purpose an entry is sealed with ([`expected_key`]).
    #[error("the {role} key {key_id} is a {key_type} {purpose} key, not {}", expected_key(*.role))]
    UnusableKey {
        /// Whose key.
        role: PrivateDocumentKeyRole,
        /// The key asked for.
        key_id: KeyID,
        /// Its type.
        key_type: KeyType,
        /// Its purpose.
        purpose: Purpose,
    },
    /// The key is disabled.
    #[error("the {role} key {key_id} is disabled")]
    DisabledKey {
        /// Whose key.
        role: PrivateDocumentKeyRole,
        /// The key asked for.
        key_id: KeyID,
    },
    /// The key's data is not a secp256k1 point.
    #[error("the {role} key {key_id} is not a secp256k1 point: {reason}")]
    InvalidKeyData {
        /// Whose key.
        role: PrivateDocumentKeyRole,
        /// The key asked for.
        key_id: KeyID,
        /// Why its data does not parse.
        reason: String,
    },
    /// The properties do not satisfy the type's schema.
    #[error("the properties do not satisfy \"{document_type_name}\": {reason}")]
    InvalidProperties {
        /// The type asked for.
        document_type_name: String,
        /// The schema's reason.
        reason: String,
    },
    /// The properties do not encode as the type's payload.
    #[error("the properties of \"{document_type_name}\" do not encode: {source}")]
    PayloadEncoding {
        /// The type asked for.
        document_type_name: String,
        /// The encoder's reason, boxed as it is far larger than the other variants.
        source: Box<ProtocolError>,
    },
    /// The properties encode to more than an entry of the type holds.
    #[error(
        "the properties encode to {payload_len} bytes; an entry of \"{document_type_name}\" \
         ({entry_size} bytes) holds at most {capacity}"
    )]
    PayloadTooLarge {
        /// The type asked for.
        document_type_name: String,
        /// The encoded payload's length.
        payload_len: usize,
        /// The type's entry size.
        entry_size: usize,
        /// The payload an entry holds.
        capacity: usize,
    },
    /// The secret key given is not the one of the sender key: the signer holds another key
    /// at the path it was asked for.
    #[error("the secret key given is not the one of the sender key {key_id}")]
    SecretKeyMismatch {
        /// The sender key asked for.
        key_id: KeyID,
    },
    /// The envelope did not seal a checked request.
    #[error("sealing a private document entry: {0}")]
    Sealing(#[source] PrivateDocumentEnvelopeError),
}

impl PrivateDocumentSealError {
    /// Whether this refuses what the sender asked for (the type, a key, or the properties),
    /// rather than reporting a signer or envelope failure.
    pub fn is_refused_request(&self) -> bool {
        match self {
            Self::UnknownDocumentType { .. }
            | Self::NotPrivate { .. }
            | Self::MissingKey { .. }
            | Self::UnusableKey { .. }
            | Self::DisabledKey { .. }
            | Self::InvalidKeyData { .. }
            | Self::InvalidProperties { .. }
            | Self::PayloadEncoding { .. }
            | Self::PayloadTooLarge { .. } => true,
            Self::SecretKeyMismatch { .. } | Self::Sealing(_) => false,
        }
    }
}

/// Whether `key` is one a private document entry can be sealed to, and so one a wallet scans a
/// private type's store with: an `ECDSA_SECP256K1` `DECRYPTION` or `ENCRYPTION` key, disabled
/// ones included, as an entry sealed before a key was disabled still opens with it. The rule
/// for every private type; DashPay's private contact requests also bound the keys' contract
/// bounds, which is DashPay's own rule.
pub fn is_private_document_key(key: &IdentityPublicKey) -> bool {
    key.key_type() == KeyType::ECDSA_SECP256K1
        && matches!(key.purpose(), Purpose::DECRYPTION | Purpose::ENCRYPTION)
}

/// The public key of `recipient`'s key `key_id` that an entry may be sealed to now: one a
/// wallet scans with ([`is_private_document_key`]), not disabled, holding a secp256k1 point.
/// Why not, when it may not.
pub fn private_document_recipient_key(
    recipient: &Identity,
    key_id: KeyID,
) -> Result<PublicKey, PrivateDocumentSealError> {
    let role = PrivateDocumentKeyRole::Recipient;
    let key = identity_key(recipient, key_id, role)?;
    if !is_private_document_key(key) {
        return Err(unusable(key, role));
    }
    enabled_point(key, role)
}

/// The keys of `identity` an entry may be sealed to now
/// ([`private_document_recipient_key`]), by key id.
pub fn private_document_recipient_keys(identity: &Identity) -> Vec<&IdentityPublicKey> {
    identity
        .public_keys()
        .values()
        .filter(|key| private_document_recipient_key(identity, key.id()).is_ok())
        .collect()
}

/// The public key of `sender`'s key `key_id` that it may seal an entry with now: an
/// `ECDSA_SECP256K1` `ENCRYPTION` key, which a wallet derives an outgoing viewing key from
/// ([`is_private_document_key`]), not disabled, holding a secp256k1 point. Why not, when it may
/// not.
pub fn private_document_sender_key(
    sender: &Identity,
    key_id: KeyID,
) -> Result<PublicKey, PrivateDocumentSealError> {
    let role = PrivateDocumentKeyRole::Sender;
    let key = identity_key(sender, key_id, role)?;
    if key.key_type() != KeyType::ECDSA_SECP256K1 || key.purpose() != Purpose::ENCRYPTION {
        return Err(unusable(key, role));
    }
    enabled_point(key, role)
}

/// `identity`'s key `key_id`, or the refusal that it has none.
fn identity_key(
    identity: &Identity,
    key_id: KeyID,
    role: PrivateDocumentKeyRole,
) -> Result<&IdentityPublicKey, PrivateDocumentSealError> {
    identity
        .get_public_key_by_id(key_id)
        .ok_or(PrivateDocumentSealError::MissingKey {
            role,
            identity_id: identity.id(),
            key_id,
        })
}

/// The refusal of `key` as not of the type and purpose `role` takes.
fn unusable(key: &IdentityPublicKey, role: PrivateDocumentKeyRole) -> PrivateDocumentSealError {
    PrivateDocumentSealError::UnusableKey {
        role,
        key_id: key.id(),
        key_type: key.key_type(),
        purpose: key.purpose(),
    }
}

/// The secp256k1 point of `key`, when it is not disabled.
fn enabled_point(
    key: &IdentityPublicKey,
    role: PrivateDocumentKeyRole,
) -> Result<PublicKey, PrivateDocumentSealError> {
    if key.disabled_at().is_some() {
        return Err(PrivateDocumentSealError::DisabledKey {
            role,
            key_id: key.id(),
        });
    }
    PublicKey::from_slice(key.data().as_slice()).map_err(|error| {
        PrivateDocumentSealError::InvalidKeyData {
            role,
            key_id: key.id(),
            reason: error.to_string(),
        }
    })
}

/// How an entry tells its recipient who wrote it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrivateDocumentSenderAuth {
    /// It names nobody.
    Anonymous,
    /// It names the sender's identity and encryption key, with a MAC under the static ECDH of
    /// that key and the recipient's: the recipient learns who wrote it, but could have made the
    /// MAC itself, so it proves nothing to anyone else.
    DeniableMac,
}

/// One entry a sender asks to seal: what it holds, for which type, from whom, to whom.
#[derive(Clone, Copy, Debug)]
pub struct PrivateDocumentEntryRequest<'a> {
    /// The contract declaring the private type.
    pub contract: &'a DataContract,
    /// The private type's name.
    pub document_type_name: &'a str,
    /// The sending identity.
    pub sender: &'a Identity,
    /// The sender's `ECDSA_SECP256K1` `ENCRYPTION` key ([`private_document_sender_key`]): the
    /// entry is sealed with its outgoing viewing key, and a deniable MAC is keyed with it.
    pub sender_encryption_key_id: KeyID,
    /// Whether the entry names the sender.
    pub auth: PrivateDocumentSenderAuth,
    /// The identity the entry is for.
    pub recipient: &'a Identity,
    /// The recipient's key the entry is sealed to ([`private_document_recipient_key`]).
    pub recipient_key_id: KeyID,
    /// The entry's properties, checked against the type's schema.
    pub properties: &'a BTreeMap<String, Value>,
}

/// What sealing a request takes, once checked.
struct CheckedRequest {
    entry_size: usize,
    payload: Vec<u8>,
    sender_public_key: PublicKey,
    recipient_public_key: PublicKey,
}

impl PrivateDocumentEntryRequest<'_> {
    /// Checks the request as [`seal_private_document`] does, without the sender's secret key:
    /// the type is private, the keys may be used ([`private_document_sender_key`],
    /// [`private_document_recipient_key`]), the properties satisfy the type's schema and their
    /// payload fits an entry. Why not, when it does not.
    pub fn check(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<(), PrivateDocumentSealError> {
        self.checked(platform_version).map(|_| ())
    }

    fn checked(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<CheckedRequest, PrivateDocumentSealError> {
        let contract_id = self.contract.id();
        let document_type_name = || self.document_type_name.to_string();
        let document_type = self
            .contract
            .document_type_for_name(self.document_type_name)
            .map_err(|error| PrivateDocumentSealError::UnknownDocumentType {
                contract_id,
                document_type_name: document_type_name(),
                reason: error.to_string(),
            })?;
        let entry_size = document_type
            .private_document()
            .map(|config| config.entry_size() as usize)
            .ok_or_else(|| PrivateDocumentSealError::NotPrivate {
                contract_id,
                document_type_name: document_type_name(),
            })?;
        let sender_public_key =
            private_document_sender_key(self.sender, self.sender_encryption_key_id)?;
        let recipient_public_key =
            private_document_recipient_key(self.recipient, self.recipient_key_id)?;
        validate_private_document_properties(
            self.contract,
            self.document_type_name,
            self.properties,
            platform_version,
        )
        .map_err(|reason| PrivateDocumentSealError::InvalidProperties {
            document_type_name: document_type_name(),
            reason,
        })?;
        let payload = encode_private_document_payload(document_type, self.properties.clone())
            .map_err(|source| PrivateDocumentSealError::PayloadEncoding {
                document_type_name: document_type_name(),
                source: Box::new(source),
            })?;
        let capacity = entry_size.saturating_sub(PRIVATE_DOCUMENT_ENVELOPE_OVERHEAD_LEN);
        if payload.len() > capacity {
            return Err(PrivateDocumentSealError::PayloadTooLarge {
                document_type_name: document_type_name(),
                payload_len: payload.len(),
                entry_size,
                capacity,
            });
        }
        Ok(CheckedRequest {
            entry_size,
            payload,
            sender_public_key,
            recipient_public_key,
        })
    }
}

/// Seals `request` into one entry of its type's store, of the type's entry size.
/// `encryption_secret_key` is the secret key of the sender's encryption key: the entry is sealed
/// with its outgoing viewing key, so the sender's wallet recovers the entry from the store
/// alone, and a deniable MAC is keyed with it. The request is checked first
/// ([`PrivateDocumentEntryRequest::check`]), and the secret key must be the encryption key's.
///
/// `esk_seed` must be 32 fresh random bytes for every entry, never reused, not even on a retry.
pub fn seal_private_document(
    request: &PrivateDocumentEntryRequest,
    encryption_secret_key: &SecretKey,
    esk_seed: &[u8; 32],
    platform_version: &PlatformVersion,
) -> Result<BinaryData, PrivateDocumentSealError> {
    let checked = request.checked(platform_version)?;
    if checked.sender_public_key != PublicKey::from_secret_key(encryption_secret_key) {
        return Err(PrivateDocumentSealError::SecretKeyMismatch {
            key_id: request.sender_encryption_key_id,
        });
    }
    let sender_identity_id = request.sender.id().to_buffer();
    let sender = match request.auth {
        PrivateDocumentSenderAuth::Anonymous => PrivateDocumentSender::Anonymous,
        PrivateDocumentSenderAuth::DeniableMac => PrivateDocumentSender::DeniableMac {
            identity_id: sender_identity_id,
            key_id: request.sender_encryption_key_id,
            secret_key: encryption_secret_key,
        },
    };
    // Both wiped on every return.
    let mut secret = encryption_secret_key.to_secret_bytes();
    let outgoing_viewing_key = Zeroizing::new(private_document_outgoing_viewing_key(&secret));
    secret.zeroize();
    let entry = seal_private_document_entry(
        &PrivateDocumentTarget {
            contract_id: request.contract.id().to_buffer(),
            document_type_name: request.document_type_name,
        },
        checked.entry_size,
        &PrivateDocumentRecipient {
            identity_id: request.recipient.id().to_buffer(),
            public_key: &checked.recipient_public_key,
        },
        &sender,
        &checked.payload,
        &outgoing_viewing_key,
        esk_seed,
    )
    .map_err(PrivateDocumentSealError::Sealing)?;
    Ok(BinaryData::new(entry))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::private_document_entries::{PrivateDocumentEntries, PrivateDocumentEntry};
    use crate::platform::private_document_scan::{
        read_private_document_payload, PrivateDocumentScanKeys,
    };
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{IdentityV0, SecurityLevel};
    use dpp::platform_value::platform_value;
    use dpp::platform_value::string_encoding::Encoding;
    use platform_encryption::PrivateDocumentAuthMode;

    const CONTRACT_ID: [u8; 32] = [7; 32];
    const TYPE_NAME: &str = "note";
    const ENTRY_SIZE: u32 = 640;
    const SENDER_ID: [u8; 32] = [0xA1; 32];
    const RECIPIENT_ID: [u8; 32] = [0xB2; 32];
    const SIGNING_KEY_ID: KeyID = 0;
    const ENCRYPTION_KEY_ID: KeyID = 1;
    const DECRYPTION_KEY_ID: KeyID = 2;
    const DISABLED_KEY_ID: KeyID = 3;

    /// A contract with the private type `note`, a required `text` of at most 80 characters, and
    /// the public type `public`.
    fn contract() -> DataContract {
        contract_of(ENTRY_SIZE, true)
    }

    /// [`contract`] with entries of `entry_size` bytes, registered with full validation or
    /// not: one parsed without it may declare entries too small for its largest payload.
    fn contract_of(entry_size: u32, full_validation: bool) -> DataContract {
        DataContract::from_value(
            platform_value!({
                "$formatVersion": "1",
                "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
                "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
                "version": 1,
                "documentSchemas": {
                    "note": {
                        "type": "object",
                        "private": { "entrySize": entry_size },
                        "documentsMutable": false,
                        "canBeDeleted": false,
                        "properties": {
                            "text": {
                                "type": "string", "minLength": 1, "maxLength": 80,
                                "position": 0
                            }
                        },
                        "required": ["text"],
                        "additionalProperties": false
                    },
                    "public": {
                        "type": "object",
                        "properties": {
                            "text": { "type": "string", "maxLength": 63, "position": 0 }
                        },
                        "additionalProperties": false
                    }
                }
            }),
            full_validation,
            PlatformVersion::latest(),
        )
        .expect("the contract registers")
    }

    fn secret(byte: u8) -> SecretKey {
        SecretKey::from_secret_bytes([byte; 32]).expect("a valid secret key")
    }

    fn key(
        id: KeyID,
        purpose: Purpose,
        key_type: KeyType,
        secret_key: &SecretKey,
        disabled: bool,
    ) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose,
            security_level: if purpose == Purpose::AUTHENTICATION {
                SecurityLevel::HIGH
            } else {
                SecurityLevel::MEDIUM
            },
            contract_bounds: None,
            key_type,
            read_only: false,
            data: BinaryData::new(PublicKey::from_secret_key(secret_key).serialize().to_vec()),
            disabled_at: disabled.then_some(1),
        })
    }

    /// An identity with a signing key 0, an encryption key 1, a decryption key 2 and a disabled
    /// decryption key 3, their secret keys `base`, `base + 1`, `base + 2` and `base + 3`.
    fn identity(id: [u8; 32], base: u8) -> Identity {
        let keys = [
            key(
                SIGNING_KEY_ID,
                Purpose::AUTHENTICATION,
                KeyType::ECDSA_SECP256K1,
                &secret(base),
                false,
            ),
            key(
                ENCRYPTION_KEY_ID,
                Purpose::ENCRYPTION,
                KeyType::ECDSA_SECP256K1,
                &secret(base + 1),
                false,
            ),
            key(
                DECRYPTION_KEY_ID,
                Purpose::DECRYPTION,
                KeyType::ECDSA_SECP256K1,
                &secret(base + 2),
                false,
            ),
            key(
                DISABLED_KEY_ID,
                Purpose::DECRYPTION,
                KeyType::ECDSA_SECP256K1,
                &secret(base + 3),
                true,
            ),
        ];
        Identity::V0(IdentityV0 {
            id: Identifier::from(id),
            public_keys: keys.into_iter().map(|key| (key.id(), key)).collect(),
            balance: 0,
            revision: 0,
        })
    }

    fn sender() -> Identity {
        identity(SENDER_ID, 0x10)
    }

    fn recipient() -> Identity {
        identity(RECIPIENT_ID, 0x20)
    }

    fn properties(text: &str) -> BTreeMap<String, Value> {
        BTreeMap::from([("text".to_string(), Value::Text(text.to_string()))])
    }

    fn seal(
        contract: &DataContract,
        auth: PrivateDocumentSenderAuth,
        recipient_key_id: KeyID,
        properties: &BTreeMap<String, Value>,
    ) -> Result<BinaryData, PrivateDocumentSealError> {
        let sender = sender();
        let recipient = recipient();
        seal_private_document(
            &PrivateDocumentEntryRequest {
                contract,
                document_type_name: TYPE_NAME,
                sender: &sender,
                sender_encryption_key_id: ENCRYPTION_KEY_ID,
                auth,
                recipient: &recipient,
                recipient_key_id,
                properties,
            },
            &secret(0x11),
            &[9; 32],
            PlatformVersion::latest(),
        )
    }

    /// The refusal of `result`, which must be one of the request
    /// ([`PrivateDocumentSealError::is_refused_request`]).
    fn refusal(result: Result<BinaryData, PrivateDocumentSealError>) -> PrivateDocumentSealError {
        let refused = result.expect_err("refused");
        assert!(refused.is_refused_request(), "{refused}");
        refused
    }

    /// A page of the test store holding `entry` at position 0.
    fn page(entry: &BinaryData) -> PrivateDocumentEntries {
        PrivateDocumentEntries {
            chunk_power: Some(1),
            entries: vec![PrivateDocumentEntry {
                position: 0,
                block_height: 10,
                block_time_ms: 1_700_000_000_000,
                data: entry.to_vec(),
            }],
            total_count: 1,
        }
    }

    fn target() -> PrivateDocumentTarget<'static> {
        PrivateDocumentTarget {
            contract_id: CONTRACT_ID,
            document_type_name: TYPE_NAME,
        }
    }

    /// An entry sealed with a deniable MAC opens with the recipient's key, names the sender's
    /// encryption key, verifies against it, and reads back as the properties; the sender's
    /// outgoing viewing key recovers it.
    #[test]
    fn should_seal_an_entry_the_recipient_opens_and_the_sender_recovers() {
        let contract = contract();
        let entry = seal(
            &contract,
            PrivateDocumentSenderAuth::DeniableMac,
            DECRYPTION_KEY_ID,
            &properties("hello"),
        )
        .expect("the entry seals");
        assert_eq!(entry.len(), ENTRY_SIZE as usize);

        let recipient_keys = PrivateDocumentScanKeys::new().with_recipient(
            Identifier::from(RECIPIENT_ID),
            &[(DECRYPTION_KEY_ID, secret(0x22))],
        );
        let scan = recipient_keys.scan_page(&target(), 0, &page(&entry));
        assert_eq!(scan.received.len(), 1, "{:?}", scan.rejected);
        let received = &scan.received[0];
        assert_eq!(received.recipient_key_id, DECRYPTION_KEY_ID);
        assert_eq!(
            received.entry.auth_mode(),
            PrivateDocumentAuthMode::DeniableMac
        );
        assert_eq!(received.entry.sender_identity_id(), &SENDER_ID);
        assert_eq!(received.entry.sender_key_id(), ENCRYPTION_KEY_ID);
        recipient_keys
            .verify_sender(received, &PublicKey::from_secret_key(&secret(0x11)))
            .expect("the MAC is the sender key's");
        assert_eq!(
            read_private_document_payload(
                &contract,
                TYPE_NAME,
                received.entry.payload(),
                PlatformVersion::latest()
            ),
            Ok(properties("hello"))
        );

        let sender_keys = PrivateDocumentScanKeys::new().with_sender(
            Identifier::from(SENDER_ID),
            ENCRYPTION_KEY_ID,
            private_document_outgoing_viewing_key(&secret(0x11).to_secret_bytes()),
        );
        let scan = sender_keys.scan_page(&target(), 0, &page(&entry));
        assert_eq!(scan.sent.len(), 1, "{:?}", scan.rejected);
        assert_eq!(scan.sent[0].outgoing_key_id, ENCRYPTION_KEY_ID);
        assert_eq!(scan.sent[0].entry.recipient_identity_id(), &RECIPIENT_ID);
    }

    /// An anonymous entry names nobody, sealed to the recipient's encryption key.
    #[test]
    fn should_seal_an_anonymous_entry_to_an_encryption_key() {
        let entry = seal(
            &contract(),
            PrivateDocumentSenderAuth::Anonymous,
            ENCRYPTION_KEY_ID,
            &properties("anonymous"),
        )
        .expect("the entry seals");
        let keys = PrivateDocumentScanKeys::new().with_recipient(
            Identifier::from(RECIPIENT_ID),
            &[(ENCRYPTION_KEY_ID, secret(0x21))],
        );
        let scan = keys.scan_page(&target(), 0, &page(&entry));
        assert_eq!(scan.received.len(), 1, "{:?}", scan.rejected);
        assert_eq!(
            scan.received[0].entry.auth_mode(),
            PrivateDocumentAuthMode::Anonymous
        );
    }

    #[test]
    fn should_refuse_a_recipient_key_an_entry_cannot_be_sealed_to() {
        let contract = contract();
        let text = properties("hello");
        let auth = PrivateDocumentSenderAuth::Anonymous;
        let recipient = PrivateDocumentKeyRole::Recipient;
        let signing = refusal(seal(&contract, auth, SIGNING_KEY_ID, &text));
        assert!(
            matches!(
                signing,
                PrivateDocumentSealError::UnusableKey {
                    role,
                    key_id: SIGNING_KEY_ID,
                    key_type: KeyType::ECDSA_SECP256K1,
                    purpose: Purpose::AUTHENTICATION,
                } if role == recipient
            ),
            "{signing:?}"
        );
        // The message names what a recipient key must be.
        assert!(signing
            .to_string()
            .contains("not an ECDSA_SECP256K1 DECRYPTION or ENCRYPTION key"));
        let disabled = refusal(seal(&contract, auth, DISABLED_KEY_ID, &text));
        assert!(
            matches!(
                disabled,
                PrivateDocumentSealError::DisabledKey { role, key_id: DISABLED_KEY_ID }
                    if role == recipient
            ),
            "{disabled:?}"
        );
        let missing = refusal(seal(&contract, auth, 9, &text));
        assert!(
            matches!(
                missing,
                PrivateDocumentSealError::MissingKey { role, identity_id, key_id: 9 }
                    if role == recipient && identity_id == Identifier::from(RECIPIENT_ID)
            ),
            "{missing:?}"
        );
    }

    #[test]
    fn should_list_only_the_keys_an_entry_can_be_sealed_to_now() {
        let recipient = recipient();
        let mut ids: Vec<KeyID> = private_document_recipient_keys(&recipient)
            .into_iter()
            .map(|key| key.id())
            .collect();
        ids.sort();
        assert_eq!(ids, vec![ENCRYPTION_KEY_ID, DECRYPTION_KEY_ID]);
        // The scan rule keeps the disabled key: an entry sealed before still opens with it.
        assert!(is_private_document_key(
            recipient
                .get_public_key_by_id(DISABLED_KEY_ID)
                .expect("the key")
        ));
    }

    #[test]
    fn should_refuse_a_sender_key_that_is_not_an_encryption_key_or_another_secret() {
        let contract = contract();
        let sender = sender();
        let recipient = recipient();
        let text = properties("hello");
        let request = |sender_encryption_key_id| PrivateDocumentEntryRequest {
            contract: &contract,
            document_type_name: TYPE_NAME,
            sender: &sender,
            sender_encryption_key_id,
            auth: PrivateDocumentSenderAuth::Anonymous,
            recipient: &recipient,
            recipient_key_id: DECRYPTION_KEY_ID,
            properties: &text,
        };
        let version = PlatformVersion::latest();
        let decryption_key = refusal(seal_private_document(
            &request(DECRYPTION_KEY_ID),
            &secret(0x12),
            &[9; 32],
            version,
        ));
        assert!(
            matches!(
                decryption_key,
                PrivateDocumentSealError::UnusableKey {
                    role: PrivateDocumentKeyRole::Sender,
                    key_id: DECRYPTION_KEY_ID,
                    purpose: Purpose::DECRYPTION,
                    ..
                }
            ),
            "{decryption_key:?}"
        );
        assert!(decryption_key
            .to_string()
            .contains("not an ECDSA_SECP256K1 ENCRYPTION key"));
        // Another secret than the sender key's is not the sender's request refused, but the
        // signer holding another key.
        let another_secret = seal_private_document(
            &request(ENCRYPTION_KEY_ID),
            &secret(0x44),
            &[9; 32],
            version,
        )
        .expect_err("refused");
        assert!(
            matches!(
                another_secret,
                PrivateDocumentSealError::SecretKeyMismatch {
                    key_id: ENCRYPTION_KEY_ID
                }
            ),
            "{another_secret:?}"
        );
        assert!(!another_secret.is_refused_request());
    }

    #[test]
    fn should_refuse_properties_the_schema_refuses_or_an_entry_cannot_hold() {
        let contract = contract();
        let auth = PrivateDocumentSenderAuth::Anonymous;
        let mut unknown = properties("hello");
        unknown.insert("other".to_string(), Value::Bool(true));
        for refused_properties in [BTreeMap::new(), unknown] {
            let refused = refusal(seal(
                &contract,
                auth,
                DECRYPTION_KEY_ID,
                &refused_properties,
            ));
            assert!(
                matches!(
                    &refused,
                    PrivateDocumentSealError::InvalidProperties { document_type_name, .. }
                        if document_type_name == TYPE_NAME
                ),
                "{refused:?}"
            );
        }
        // Registration refuses a type whose largest payload does not fit its entries; one
        // parsed without that check may hold properties no entry holds.
        let small = contract_of(320, false);
        let long = "é".repeat(80);
        let refused = refusal(seal(&small, auth, DECRYPTION_KEY_ID, &properties(&long)));
        assert!(
            matches!(
                refused,
                PrivateDocumentSealError::PayloadTooLarge {
                    entry_size: 320,
                    capacity: 21,
                    ..
                }
            ),
            "{refused:?}"
        );
    }

    #[test]
    fn should_refuse_a_type_that_is_not_private_or_does_not_exist() {
        let contract = contract();
        let sender = sender();
        let recipient = recipient();
        let text = properties("hello");
        let check = |name| {
            PrivateDocumentEntryRequest {
                contract: &contract,
                document_type_name: name,
                sender: &sender,
                sender_encryption_key_id: ENCRYPTION_KEY_ID,
                auth: PrivateDocumentSenderAuth::Anonymous,
                recipient: &recipient,
                recipient_key_id: DECRYPTION_KEY_ID,
                properties: &text,
            }
            .check(PlatformVersion::latest())
            .expect_err("refused")
        };
        let public = check("public");
        assert!(
            matches!(
                &public,
                PrivateDocumentSealError::NotPrivate { document_type_name, contract_id }
                    if document_type_name == "public"
                        && *contract_id == Identifier::from(CONTRACT_ID)
            ),
            "{public:?}"
        );
        let missing = check("missing");
        assert!(
            matches!(
                &missing,
                PrivateDocumentSealError::UnknownDocumentType { document_type_name, .. }
                    if document_type_name == "missing"
            ),
            "{missing:?}"
        );
        assert!(public.is_refused_request() && missing.is_refused_request());
    }
}
