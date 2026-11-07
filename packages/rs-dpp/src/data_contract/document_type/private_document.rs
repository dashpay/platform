//! Private document types: the `private` keyword (protocol version 15).
//!
//! A private document type's documents are fixed-size encrypted entries. An anonymous
//! transition paid from the shielded pool appends them: nobody owns, replaces or deletes one,
//! and the chain never reads inside one, so it can check nothing a document type usually
//! promises about its documents. Recipients find their entries by trying to decrypt each one.
//!
//! The type's `properties` describe the plaintext, so that clients can decode it with types.
//! The chain checks them at registration like any schema, and never against an entry.

use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::{property_names, DocumentTypeRef};
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use std::num::NonZeroU32;

/// What a document type's `private` keyword declares (`{ "entrySize": n }`). Built only by the
/// parse of the keyword and the stored form read back, each of which refuses an entry of no
/// bytes, so an entry is never empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrivateDocumentConfig {
    entry_size: NonZeroU32,
}

impl PrivateDocumentConfig {
    /// A private document type whose entries take `entry_size` bytes each.
    pub(crate) fn new(entry_size: NonZeroU32) -> Self {
        Self { entry_size }
    }

    /// How many bytes every entry a client appends takes: fixed for the type, so that entries
    /// do not differ in size by what they say. Drive stores each with a header of its own,
    /// which this does not count.
    pub fn entry_size(&self) -> u32 {
        self.entry_size.get()
    }
}

/// The doctype-level keywords a private document type may declare. Each other keyword is a
/// promise about the type's documents (an index, a history, a transfer, a cost, a rule, a
/// reference, ...) that the chain could only keep by reading them, and is refused. A keyword
/// added later is refused here until someone decides what it means on a private type.
///
/// `documentsMutable`, `canBeDeleted` and `documentsKeepHistory` are admitted only to be
/// written out as `false`, against a contract default saying otherwise. A document type's own
/// `$defs` is not listed: every parse refuses it on any type, the contract's `$defs` being
/// where definitions go.
pub(crate) const PRIVATE_DOCUMENT_TYPE_KEYWORDS: [&str; 15] = [
    "type",
    "properties",
    "required",
    "additionalProperties",
    "description",
    "$comment",
    property_names::MIN_PROPERTIES,
    property_names::MAX_PROPERTIES,
    property_names::DEPENDENT_REQUIRED,
    property_names::PRIVATE,
    property_names::DOCUMENTS_MUTABLE,
    property_names::CAN_BE_DELETED,
    property_names::DOCUMENTS_KEEP_HISTORY,
    property_names::REQUIRES_IDENTITY_ENCRYPTION_BOUNDED_KEY,
    property_names::REQUIRES_IDENTITY_DECRYPTION_BOUNDED_KEY,
];

/// The bytes the standard private document envelope (version 1) wraps the encrypted body in: the
/// commitment (32), the ephemeral public key (33), the body's AEAD tag (16) and the sender-recovery
/// ciphertext (81).
pub const PRIVATE_DOCUMENT_ENVELOPE_V1_WRAPPER_BYTES: u32 = 32 + 33 + 16 + 81;

/// The bytes the standard private document envelope (version 1) puts in the body before the
/// payload: the envelope version (1), the authentication mode (1), the sender's identity id (32)
/// and key id (4), the authentication (65), the recipient's identity id (32) and the payload's
/// length (2).
pub const PRIVATE_DOCUMENT_ENVELOPE_V1_BODY_HEADER_BYTES: u32 = 1 + 1 + 32 + 4 + 65 + 32 + 2;

/// Every byte of an entry the standard private document envelope (version 1) takes besides the
/// payload: 299.
pub const PRIVATE_DOCUMENT_ENVELOPE_V1_OVERHEAD_BYTES: u32 =
    PRIVATE_DOCUMENT_ENVELOPE_V1_WRAPPER_BYTES + PRIVATE_DOCUMENT_ENVELOPE_V1_BODY_HEADER_BYTES;

/// The most bytes a payload of `document_type` takes: its properties in the document property
/// encoding, as a document writes them after its system fields, each one present at its largest
/// and one written with a presence byte (optional or transient) counting that byte. A private
/// type's entries carry their plaintext in this encoding.
///
/// Refused with `UnboundedPrivateDocumentProperty` when a property sets no largest value, since
/// then no entry size holds every payload.
pub fn largest_private_document_payload_size(
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> Result<u32, ProtocolError> {
    let mut size = 0u32;
    for (property_name, property) in document_type.properties() {
        // A private type refuses `requiredSince`, so a property is required at every contract
        // version an entry is written under.
        let presence_byte = !property.required || property.transient;
        let Some(value_size) = property
            .property_type
            .max_encoded_value_size(property.required, platform_version)?
        else {
            return Err(ProtocolError::UnboundedPrivateDocumentProperty {
                document_type_name: document_type.name().clone(),
                property_name: property_name.clone(),
            });
        };
        size = size
            .saturating_add(value_size)
            .saturating_add(u32::from(presence_byte));
    }
    Ok(size)
}

/// The smallest `entrySize` that holds every payload of `document_type` in the standard private
/// document envelope (version 1): its largest payload plus
/// [`PRIVATE_DOCUMENT_ENVELOPE_V1_OVERHEAD_BYTES`].
pub fn minimum_private_document_entry_size(
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> Result<u32, ProtocolError> {
    Ok(
        largest_private_document_payload_size(document_type, platform_version)?
            .saturating_add(PRIVATE_DOCUMENT_ENVELOPE_V1_OVERHEAD_BYTES),
    )
}

/// Refuses a private document type whose `entrySize` cannot hold its largest payload in the
/// standard private document envelope (version 1), which clients check before registering one.
/// The chain does not: it never reads an entry, so it only holds `entrySize` to
/// `max_private_document_entry_size`, and such a type registers but takes no full payload. A type
/// that is not private passes.
pub fn validate_private_document_entry_size(
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    let Some(private_document) = document_type.private_document() else {
        return Ok(());
    };
    let minimum_entry_size = minimum_private_document_entry_size(document_type, platform_version)?;
    if private_document.entry_size() < minimum_entry_size {
        return Err(ProtocolError::PrivateDocumentEntrySizeTooSmall {
            document_type_name: document_type.name().clone(),
            entry_size: private_document.entry_size(),
            minimum_entry_size,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use crate::data_contract::document_type::DocumentPropertyType;
    use crate::data_contract::DataContract;
    use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use crate::document::DocumentV0;
    use platform_value::string_encoding::Encoding;
    use platform_value::{Identifier, Value};
    use serde_json::json;
    use std::collections::BTreeMap;

    /// A contract holding the private `secret` type of `entry_size`-byte entries with
    /// `properties`, of which `required` are required, and the public `note` type.
    fn contract(
        entry_size: u32,
        properties: serde_json::Value,
        required: serde_json::Value,
    ) -> DataContract {
        DataContract::from_value(
            platform_value::to_value(json!({
                "$formatVersion": "1",
                "id": Identifier::from([7; 32]).to_string(Encoding::Base58),
                "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
                "version": 1,
                "documentSchemas": {
                    "secret": {
                        "type": "object",
                        "private": { "entrySize": entry_size },
                        "documentsMutable": false,
                        "canBeDeleted": false,
                        "properties": properties,
                        "required": required,
                        "additionalProperties": false
                    },
                    "note": {
                        "type": "object",
                        "properties": {
                            "text": { "type": "string", "maxLength": 63, "position": 0 }
                        },
                        "additionalProperties": false
                    }
                }
            }))
            .expect("the contract converts"),
            true,
            PlatformVersion::latest(),
        )
        .expect("the contract registers")
    }

    /// One property of every kind a largest size bounds, optional ones among them.
    fn every_bounded_kind() -> serde_json::Value {
        json!({
            "small": { "type": "integer", "minimum": 0, "maximum": 255, "position": 0 },
            "large": { "type": "integer", "position": 1 },
            "label": { "type": "string", "maxLength": 10, "position": 2 },
            "short": { "type": "string", "maxBytes": 20, "position": 3 },
            "both": { "type": "string", "maxLength": 4, "maxBytes": 10, "position": 4 },
            "key": {
                "type": "array", "byteArray": true, "minItems": 33, "maxItems": 33,
                "position": 5
            },
            "blob": { "type": "array", "byteArray": true, "maxItems": 300, "position": 6 },
            "flag": { "type": "boolean", "position": 7 },
            "who": {
                "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier", "position": 8
            },
            "meta": {
                "type": "object",
                "properties": {
                    "a": { "type": "integer", "minimum": 0, "maximum": 65535, "position": 0 },
                    "b": { "type": "string", "maxLength": 5, "position": 1 }
                },
                "required": ["a"],
                "additionalProperties": false,
                "position": 9
            },
            "tags": {
                "type": "array", "maxItems": 3, "items": { "type": "string", "maxLength": 8 },
                "position": 10
            }
        })
    }

    /// Four bytes a character, the most UTF-8 takes.
    fn wide(characters: usize) -> String {
        "\u{1F600}".repeat(characters)
    }

    /// Every property of [`every_bounded_kind`] at its largest value.
    fn largest_values() -> BTreeMap<String, Value> {
        BTreeMap::from([
            ("small".to_string(), Value::U64(255)),
            ("large".to_string(), Value::I64(i64::MAX)),
            ("label".to_string(), Value::Text(wide(10))),
            ("short".to_string(), Value::Text("a".repeat(20))),
            // Four characters, ten bytes: the byte bound is the lower one.
            ("both".to_string(), Value::Text(format!("{}ab", wide(2)))),
            ("key".to_string(), Value::Bytes(vec![1; 33])),
            ("blob".to_string(), Value::Bytes(vec![2; 300])),
            ("flag".to_string(), Value::Bool(true)),
            ("who".to_string(), Value::Identifier([3; 32])),
            (
                "meta".to_string(),
                Value::Map(vec![
                    (Value::Text("a".to_string()), Value::U64(65535)),
                    (Value::Text("b".to_string()), Value::Text(wide(5))),
                ]),
            ),
            (
                "tags".to_string(),
                Value::Array(vec![Value::Text(wide(8)); 3]),
            ),
        ])
    }

    /// The largest payload is what a document of the type holding every property at its largest
    /// serializes to after its system fields: the format version (1), the contract version stamp
    /// (1), `$id` (32), `$ownerId` (32) and the system field flags (2), 68 bytes for a private
    /// type, which keeps no revision, times or creator.
    #[test]
    fn should_size_the_largest_payload_as_a_document_serializes_it() {
        let platform_version = PlatformVersion::latest();
        let contract = contract(1000, every_bounded_kind(), json!(["small", "large", "key"]));
        let secret = contract
            .document_type_for_name("secret")
            .expect("the secret type");

        let largest = largest_private_document_payload_size(secret, platform_version)
            .expect("every property is bounded");
        // 1 + 8 + (1 + 1 + 40) + (1 + 1 + 20) + (1 + 1 + 10) + 33 + (1 + 2 + 300) + (1 + 1)
        // + (1 + 32) + (1 + 1 + 2 + 1 + 1 + 20) + (1 + 1 + 3 * (1 + 32))
        assert_eq!(largest, 583);

        let document = DocumentV0 {
            id: Identifier::from([4; 32]),
            owner_id: Identifier::from([5; 32]),
            properties: largest_values(),
            ..Default::default()
        };
        let serialized = document
            .serialize(secret, &contract, platform_version)
            .expect("the document serializes");
        assert_eq!(&serialized[..2], &[3, 0]);
        assert_eq!(&serialized[2..34], &[4; 32]);
        assert_eq!(&serialized[34..66], &[5; 32]);
        assert_eq!(&serialized[66..68], &[0, 0]);
        assert_eq!(serialized.len() - 68, largest as usize);
    }

    /// The smallest entry size is the largest payload and the envelope's 299 bytes; a type of that
    /// size passes, one byte less is refused with both numbers, and a public type always passes.
    #[test]
    fn should_refuse_a_private_type_whose_entries_cannot_hold_its_largest_payload() {
        let platform_version = PlatformVersion::latest();
        assert_eq!(PRIVATE_DOCUMENT_ENVELOPE_V1_OVERHEAD_BYTES, 299);
        let required = json!(["small", "large", "key"]);
        let minimum = minimum_private_document_entry_size(
            contract(1000, every_bounded_kind(), required.clone())
                .document_type_for_name("secret")
                .expect("the secret type"),
            platform_version,
        )
        .expect("every property is bounded");
        assert_eq!(minimum, 583 + 299);

        let fitting = contract(minimum, every_bounded_kind(), required.clone());
        for name in ["secret", "note"] {
            validate_private_document_entry_size(
                fitting.document_type_for_name(name).expect("the type"),
                platform_version,
            )
            .unwrap_or_else(|error| panic!("{name} passes: {error}"));
        }

        let short = contract(minimum - 1, every_bounded_kind(), required);
        match validate_private_document_entry_size(
            short.document_type_for_name("secret").expect("the type"),
            platform_version,
        ) {
            Err(ProtocolError::PrivateDocumentEntrySizeTooSmall {
                document_type_name,
                entry_size,
                minimum_entry_size,
            }) => {
                assert_eq!(document_type_name, "secret");
                assert_eq!(entry_size, minimum - 1);
                assert_eq!(minimum_entry_size, minimum);
            }
            other => panic!("expected the entry size refusal, got {other:?}"),
        }
    }

    /// A string without `maxLength` or `maxBytes`, or a byte array without `maxItems`, sets no
    /// largest payload, so no entry size holds every one.
    #[test]
    fn should_refuse_a_private_type_with_an_unbounded_property() {
        for property in [
            json!({ "type": "string", "position": 1 }),
            json!({ "type": "array", "byteArray": true, "position": 1 }),
        ] {
            let contract = contract(
                4096,
                json!({
                    "small": { "type": "integer", "minimum": 0, "maximum": 255, "position": 0 },
                    "open": property
                }),
                json!(["small"]),
            );
            match validate_private_document_entry_size(
                contract.document_type_for_name("secret").expect("the type"),
                PlatformVersion::latest(),
            ) {
                Err(ProtocolError::UnboundedPrivateDocumentProperty {
                    document_type_name,
                    property_name,
                }) => {
                    assert_eq!(document_type_name, "secret");
                    assert_eq!(property_name, "open");
                }
                other => panic!("expected the unbounded refusal, got {other:?}"),
            }
        }
    }

    /// A `Date` property carries a marker byte when it is optional; its largest size counts it,
    /// as its encoding writes it.
    #[test]
    fn should_size_a_date_with_its_optional_marker() {
        let platform_version = PlatformVersion::latest();
        for required in [true, false] {
            assert_eq!(
                DocumentPropertyType::Date
                    .max_encoded_value_size(required, platform_version)
                    .expect("sized"),
                Some(
                    DocumentPropertyType::Date
                        .encode_value_ref_with_size(&Value::Float(1.5), required)
                        .expect("encodes")
                        .len() as u32
                ),
                "required: {required}"
            );
        }
    }
}
