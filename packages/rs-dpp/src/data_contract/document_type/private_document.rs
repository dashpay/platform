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
use crate::data_contract::document_type::property::DocumentProperty;
use crate::data_contract::document_type::{property_names, DocumentPropertyType, DocumentTypeRef};
use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use crate::document::DocumentV0;
use crate::ProtocolError;
use indexmap::IndexMap;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
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

/// The most bytes a payload of `document_type` takes as its property types bound it: its
/// properties in the document property encoding, as a document writes them after its system
/// fields, each one present at its largest and one written with a presence byte (optional or
/// transient) counting that byte. A private type's entries carry their plaintext in this
/// encoding.
///
/// It is an upper bound, read from the property types alone: a schema's `enum`, or a
/// `maxProperties` keeping some optional properties from appearing together, can only make the
/// real largest payload smaller, and is not counted. A type sized between the two is refused by
/// [`validate_private_document_entry_size`] although every payload its schema admits would fit;
/// size it to this bound, or bound the property types themselves.
///
/// Refused with `UnboundedPrivateDocumentProperty` when a property sets no largest value, since
/// then no entry size holds every payload.
pub fn largest_private_document_payload_size(
    document_type: DocumentTypeRef,
) -> Result<u32, ProtocolError> {
    let mut size = 0u32;
    for (property_name, property) in document_type.properties() {
        // A private type refuses `requiredSince`, so a property is required at every contract
        // version an entry is written under.
        let presence_byte = !property.required || property.transient;
        let Some(value_size) = property
            .property_type
            .max_encoded_value_size(property.required)
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

/// The smallest `entrySize` that holds every payload the property types of `document_type`
/// allow in the standard private document envelope (version 1): its largest payload plus
/// [`PRIVATE_DOCUMENT_ENVELOPE_V1_OVERHEAD_BYTES`]. As
/// [`largest_private_document_payload_size`] says, `enum` and `maxProperties` are not counted.
pub fn minimum_private_document_entry_size(
    document_type: DocumentTypeRef,
) -> Result<u32, ProtocolError> {
    Ok(largest_private_document_payload_size(document_type)?
        .saturating_add(PRIVATE_DOCUMENT_ENVELOPE_V1_OVERHEAD_BYTES))
}

/// Refuses a private document type whose `entrySize` cannot hold its largest payload in the
/// standard private document envelope (version 1), which clients check before registering one.
/// The chain does not: it never reads an entry, so it only holds `entrySize` to
/// `max_private_document_entry_size`, and such a type registers but takes no full payload. A type
/// that is not private passes.
pub fn validate_private_document_entry_size(
    document_type: DocumentTypeRef,
) -> Result<(), ProtocolError> {
    let Some(private_document) = document_type.private_document() else {
        return Ok(());
    };
    let minimum_entry_size = minimum_private_document_entry_size(document_type)?;
    if private_document.entry_size() < minimum_entry_size {
        return Err(ProtocolError::PrivateDocumentEntrySizeTooSmall {
            document_type_name: document_type.name().clone(),
            entry_size: private_document.entry_size(),
            minimum_entry_size,
        });
    }
    Ok(())
}

/// The document serialization format whose property encoding a private document payload is:
/// a payload holds a type's properties exactly as a format 3 document writes them after its
/// system fields, whatever format later documents take.
const PRIVATE_DOCUMENT_PAYLOAD_DOCUMENT_FORMAT: u16 = 3;

/// What a format 3 document of a private type writes before its properties, when its `$id` and
/// `$ownerId` are zeros and it carries no contract version stamp: the format version, the stamp
/// (0), the two ids and the system field flags, none set. A private type keeps no revision, no
/// times and no creator, so nothing else comes first.
fn private_document_payload_prefix() -> [u8; 68] {
    let mut prefix = [0u8; 68];
    prefix[0] = PRIVATE_DOCUMENT_PAYLOAD_DOCUMENT_FORMAT as u8;
    prefix
}

fn refuse_a_public_type(document_type: DocumentTypeRef) -> Result<(), ProtocolError> {
    match document_type.private_document() {
        Some(_) => Ok(()),
        None => Err(ProtocolError::NotSupported(format!(
            "document type {} is not private, so it has no private document payload",
            document_type.name()
        ))),
    }
}

/// Refuses a present object holding no member, at any depth: the document property encoding
/// writes it, but a document's reader reads an object with no members back as absent, so the
/// payload would not read back as it was written.
fn refuse_present_empty_objects(
    fields: &IndexMap<String, DocumentProperty>,
    values: &BTreeMap<&str, &Value>,
    path: &str,
) -> Result<(), ProtocolError> {
    for (name, field) in fields {
        let DocumentPropertyType::Object(members) = &field.property_type else {
            continue;
        };
        let Some(Value::Map(entries)) = values.get(name.as_str()).copied() else {
            continue;
        };
        let object_path = if path.is_empty() {
            name.clone()
        } else {
            format!("{path}.{name}")
        };
        let present: BTreeMap<&str, &Value> = entries
            .iter()
            .filter_map(|(key, value)| {
                let key = key.as_text()?;
                (members.contains_key(key) && !value.is_null()).then_some((key, value))
            })
            .collect();
        if present.is_empty() {
            return Err(ProtocolError::NotSupported(format!(
                "object {object_path} holds no member: a document reads an object with no members \
                 back as absent, so a private document payload cannot carry one"
            )));
        }
        refuse_present_empty_objects(members, &present, &object_path)?;
    }
    Ok(())
}

/// The payload a private document entry of `document_type` carries for `properties`: the type's
/// properties in the document property encoding, as a format 3 document writes them after its
/// system fields, whatever the version of its contract's config. A value of the wrong type is
/// refused, and so is a present object holding no member, which would read back as absent; the
/// schema's bounds are not checked here, so a sender validates `properties` against the type
/// first (`DataContract::validate_document_properties`).
pub fn encode_private_document_payload(
    document_type: DocumentTypeRef,
    properties: BTreeMap<String, Value>,
) -> Result<Vec<u8>, ProtocolError> {
    refuse_a_public_type(document_type)?;
    let present: BTreeMap<&str, &Value> = properties
        .iter()
        .filter(|(_, value)| !value.is_null())
        .map(|(name, value)| (name.as_str(), value))
        .collect();
    refuse_present_empty_objects(document_type.properties(), &present, "")?;
    let document = DocumentV0 {
        properties,
        ..Default::default()
    };
    let serialized = document.serialize_in_format_3(document_type)?;
    serialized
        .strip_prefix(private_document_payload_prefix().as_slice())
        .map(<[u8]>::to_vec)
        .ok_or_else(|| {
            ProtocolError::CorruptedCodeExecution(
                "a private type's document serializes its properties behind 68 bytes of \
                 system fields"
                    .to_string(),
            )
        })
}

/// The properties a private document payload of `document_type` holds, read as a document's
/// reader reads them after its system fields: a value cut short, or a byte after the last
/// property, is refused. An entry is immutable while its type can gain optional properties, at
/// the end of the type or of one of its objects, so a payload written before them ends before
/// them, and reads them as absent. The schema's bounds are not checked here, so a recipient
/// validates the properties against the type before trusting them
/// (`DataContract::validate_document_properties`).
pub fn decode_private_document_payload(
    document_type: DocumentTypeRef,
    payload: &[u8],
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<String, Value>, ProtocolError> {
    refuse_a_public_type(document_type)?;
    let mut serialized = private_document_payload_prefix().to_vec();
    serialized.extend_from_slice(payload);
    Ok(DocumentV0::from_bytes(&serialized, document_type, platform_version)?.properties)
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
    use platform_version::version::PlatformVersion;
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

        let largest =
            largest_private_document_payload_size(secret).expect("every property is bounded");
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
        assert_eq!(PRIVATE_DOCUMENT_ENVELOPE_V1_OVERHEAD_BYTES, 299);
        let required = json!(["small", "large", "key"]);
        let minimum = minimum_private_document_entry_size(
            contract(1000, every_bounded_kind(), required.clone())
                .document_type_for_name("secret")
                .expect("the secret type"),
        )
        .expect("every property is bounded");
        assert_eq!(minimum, 583 + 299);

        let fitting = contract(minimum, every_bounded_kind(), required.clone());
        for name in ["secret", "note"] {
            validate_private_document_entry_size(
                fitting.document_type_for_name(name).expect("the type"),
            )
            .unwrap_or_else(|error| panic!("{name} passes: {error}"));
        }

        let short = contract(minimum - 1, every_bounded_kind(), required);
        match validate_private_document_entry_size(
            short.document_type_for_name("secret").expect("the type"),
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

    /// A typed array is its varint count and that many of its largest element, as its encoder
    /// writes it: one of no element takes 1 byte whatever its element type, and one larger than
    /// `u16::MAX` bytes keeps its exact size.
    #[test]
    fn should_size_a_typed_array_as_its_encoding_takes_it() {
        use crate::data_contract::document_type::property::array::TypedArrayProperty;
        use crate::data_contract::document_type::{ByteArrayPropertySizes, StringPropertySizes};

        let typed_array = |item_type: DocumentPropertyType, max_items: u16| {
            DocumentPropertyType::TypedArray(TypedArrayProperty {
                item_type: Box::new(item_type),
                item_constraints: Default::default(),
                min_items: None,
                max_items,
                unique_items: false,
            })
        };
        let unbounded_string = DocumentPropertyType::String(StringPropertySizes {
            min_length: None,
            max_length: None,
            max_bytes: None,
        });
        let hundred_bytes = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
            min_size: Some(100),
            max_size: Some(100),
        });

        for (array, value) in [
            (
                typed_array(unbounded_string.clone(), 0),
                Value::Array(vec![]),
            ),
            (
                typed_array(hundred_bytes, 1000),
                Value::Array(vec![Value::Bytes(vec![1; 100]); 1000]),
            ),
        ] {
            assert_eq!(
                array.max_encoded_value_size(true),
                Some(
                    array
                        .encode_value_ref_with_size(&value, true)
                        .expect("encodes")
                        .len() as u32
                )
            );
        }
        assert_eq!(
            typed_array(unbounded_string, 1).max_encoded_value_size(true),
            None
        );
    }

    /// A payload is the type's properties as a document writes them after its system fields:
    /// every property at its largest encodes to exactly the largest payload size, and decodes
    /// back to properties that encode to the same bytes.
    #[test]
    fn should_encode_and_decode_a_payload_as_a_document_writes_its_properties() {
        let platform_version = PlatformVersion::latest();
        let contract = contract(1000, every_bounded_kind(), json!(["small", "large", "key"]));
        let secret = contract
            .document_type_for_name("secret")
            .expect("the secret type");

        let payload =
            encode_private_document_payload(secret, largest_values()).expect("the values encode");
        assert_eq!(
            payload.len() as u32,
            largest_private_document_payload_size(secret).expect("bounded")
        );
        let decoded = decode_private_document_payload(secret, &payload, platform_version)
            .expect("the payload decodes");
        assert_eq!(decoded.get("label"), Some(&Value::Text(wide(10))));
        assert_eq!(decoded.get("key"), Some(&Value::Bytes(vec![1; 33])));
        assert_eq!(
            encode_private_document_payload(secret, decoded).expect("re-encodes"),
            payload
        );

        // Only the required properties: each absent optional one is its presence byte.
        let smallest = BTreeMap::from([
            ("small".to_string(), Value::U64(1)),
            ("large".to_string(), Value::I64(2)),
            ("key".to_string(), Value::Bytes(vec![1; 33])),
        ]);
        assert_eq!(
            encode_private_document_payload(secret, smallest)
                .expect("encodes")
                .len(),
            1 + 8 + 33 + 8
        );
    }

    /// A payload holding anything but what its properties encode to, a byte after the last
    /// property or one cut off, is refused, and a public type has no payload.
    #[test]
    fn should_refuse_a_payload_its_properties_do_not_encode_to() {
        let platform_version = PlatformVersion::latest();
        let contract = contract(1000, every_bounded_kind(), json!(["small", "large", "key"]));
        let secret = contract
            .document_type_for_name("secret")
            .expect("the secret type");
        let payload = encode_private_document_payload(secret, largest_values()).expect("encodes");

        let mut trailing = payload.clone();
        trailing.push(0);
        assert!(decode_private_document_payload(secret, &trailing, platform_version).is_err());
        assert!(decode_private_document_payload(
            secret,
            &payload[..payload.len() - 1],
            platform_version
        )
        .is_err());

        let note = contract
            .document_type_for_name("note")
            .expect("the note type");
        assert!(matches!(
            encode_private_document_payload(note, BTreeMap::new()),
            Err(ProtocolError::NotSupported(_))
        ));
        assert!(matches!(
            decode_private_document_payload(note, &[0], platform_version),
            Err(ProtocolError::NotSupported(_))
        ));
    }

    /// A payload is written in format 3 whatever the contract's config: a type of a contract
    /// keeping a version 0 config, whose documents are written in format 0, encodes and decodes
    /// its payload as well, its integers at the 8 bytes it counts them.
    #[test]
    fn should_encode_a_payload_whatever_the_version_of_the_contract_config() {
        let platform_version = PlatformVersion::latest();
        for config in [
            json!({ "$formatVersion": "0" }),
            json!({ "$formatVersion": "1", "sizedIntegerTypes": true }),
        ] {
            let contract = DataContract::from_value(
                platform_value::to_value(json!({
                    "$formatVersion": "1",
                    "id": Identifier::from([7; 32]).to_string(Encoding::Base58),
                    "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
                    "version": 1,
                    "config": config,
                    "documentSchemas": {
                        "secret": {
                            "type": "object",
                            "private": { "entrySize": 400 },
                            "documentsMutable": false,
                            "canBeDeleted": false,
                            "properties": {
                                "small": {
                                    "type": "integer", "minimum": 0, "maximum": 255,
                                    "position": 0
                                },
                                "label": { "type": "string", "maxLength": 4, "position": 1 }
                            },
                            "required": ["small"],
                            "additionalProperties": false
                        }
                    }
                }))
                .expect("the contract converts"),
                true,
                platform_version,
            )
            .expect("the contract registers");
            let secret = contract
                .document_type_for_name("secret")
                .expect("the secret type");
            let properties = BTreeMap::from([
                ("small".to_string(), Value::U64(255)),
                ("label".to_string(), Value::Text(wide(4))),
            ]);
            let payload = encode_private_document_payload(secret, properties)
                .unwrap_or_else(|error| panic!("{config}: encodes: {error}"));
            assert_eq!(
                payload.len() as u32,
                largest_private_document_payload_size(secret).expect("bounded"),
                "{config}"
            );
            let decoded = decode_private_document_payload(secret, &payload, platform_version)
                .unwrap_or_else(|error| panic!("{config}: decodes: {error}"));
            assert_eq!(
                decoded.get("label"),
                Some(&Value::Text(wide(4))),
                "{config}"
            );
        }
    }

    /// A contract with the private type `secret` of `properties`, `required` of them required.
    fn private_type_contract(
        properties: serde_json::Value,
        required: serde_json::Value,
    ) -> DataContract {
        contract(1000, properties, required)
    }

    /// A present object holding no member, required or optional, at any depth, is refused: a
    /// document reads it back as absent. One holding a member encodes and reads back.
    #[test]
    fn should_refuse_a_present_object_holding_no_member() {
        let platform_version = PlatformVersion::latest();
        let contract = private_type_contract(
            json!({
                "meta": {
                    "type": "object",
                    "properties": {
                        "flag": { "type": "boolean", "position": 0 },
                        "inner": {
                            "type": "object",
                            "properties": { "x": { "type": "boolean", "position": 0 } },
                            "additionalProperties": false,
                            "position": 1
                        }
                    },
                    "additionalProperties": false,
                    "position": 0
                },
                "more": {
                    "type": "object",
                    "properties": { "y": { "type": "boolean", "position": 0 } },
                    "additionalProperties": false,
                    "position": 1
                }
            }),
            json!(["meta"]),
        );
        let secret = contract
            .document_type_for_name("secret")
            .expect("the secret type");
        let object = |members: Vec<(&str, Value)>| {
            Value::Map(
                members
                    .into_iter()
                    .map(|(key, value)| (Value::Text(key.to_string()), value))
                    .collect(),
            )
        };
        for (case, properties) in [
            ("an empty required object", vec![("meta", object(vec![]))]),
            (
                "an empty optional object",
                vec![
                    ("meta", object(vec![("flag", Value::Bool(true))])),
                    ("more", object(vec![])),
                ],
            ),
            (
                "an empty object inside one",
                vec![("meta", object(vec![("inner", object(vec![]))]))],
            ),
        ] {
            let properties = properties
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect();
            assert!(
                matches!(
                    encode_private_document_payload(secret, properties),
                    Err(ProtocolError::NotSupported(_))
                ),
                "{case}"
            );
        }

        let properties = BTreeMap::from([(
            "meta".to_string(),
            object(vec![("inner", object(vec![("x", Value::Bool(false))]))]),
        )]);
        let payload =
            encode_private_document_payload(secret, properties).expect("a member is present");
        let decoded =
            decode_private_document_payload(secret, &payload, platform_version).expect("decodes");
        assert_eq!(
            encode_private_document_payload(secret, decoded).expect("re-encodes"),
            payload
        );
    }

    /// An entry is immutable while its type can gain optional properties: a payload written
    /// before the type gained one at its end and one at the end of its object reads under the
    /// newer type, the new properties absent, and still refuses a byte after its last property.
    #[test]
    fn should_read_a_payload_written_before_the_type_gained_optional_properties() {
        let platform_version = PlatformVersion::latest();
        let meta = |with_member: bool| {
            let mut properties = json!({
                "a": { "type": "integer", "minimum": 0, "maximum": 255, "position": 0 }
            });
            if with_member {
                properties["b"] = json!({ "type": "boolean", "position": 1 });
            }
            json!({
                "type": "object",
                "properties": properties,
                "required": ["a"],
                "additionalProperties": false,
                "position": 1
            })
        };
        let before = private_type_contract(
            json!({ "flag": { "type": "boolean", "position": 0 }, "meta": meta(false) }),
            json!(["flag", "meta"]),
        );
        let after = private_type_contract(
            json!({
                "flag": { "type": "boolean", "position": 0 },
                "meta": meta(true),
                "extra": { "type": "boolean", "position": 2 }
            }),
            json!(["flag", "meta"]),
        );
        let properties = BTreeMap::from([
            ("flag".to_string(), Value::Bool(true)),
            (
                "meta".to_string(),
                Value::Map(vec![(Value::Text("a".to_string()), Value::U64(7))]),
            ),
        ]);
        let written = encode_private_document_payload(
            before.document_type_for_name("secret").expect("the type"),
            properties,
        )
        .expect("encodes");

        let newer = after.document_type_for_name("secret").expect("the type");
        let read = decode_private_document_payload(newer, &written, platform_version)
            .expect("an older payload reads under the newer type");
        assert_eq!(read.get("flag"), Some(&Value::Bool(true)));
        assert_eq!(read.get("extra"), None);
        assert_eq!(
            read.get("meta")
                .and_then(|meta| meta.get_optional_value("b").ok().flatten()),
            None
        );

        let mut trailing = written.clone();
        trailing.push(0);
        assert!(decode_private_document_payload(
            before.document_type_for_name("secret").expect("the type"),
            &trailing,
            platform_version
        )
        .is_err());
    }

    /// The type's schema validates a payload's properties, before sealing and after opening:
    /// the largest values pass, and a string over its bound, which decodes, is refused.
    #[cfg(feature = "validation")]
    #[test]
    fn should_validate_payload_properties_against_the_type() {
        use crate::data_contract::document_type::property_constraints::DocumentSystemValues;
        use crate::data_contract::methods::validate_document::DataContractDocumentValidationMethodsV0;

        let platform_version = PlatformVersion::latest();
        let contract = contract(1000, every_bounded_kind(), json!(["small", "large", "key"]));
        let validate = |properties: BTreeMap<String, Value>| {
            contract
                .validate_document_properties(
                    "secret",
                    Value::from(properties),
                    &DocumentSystemValues::default(),
                    platform_version,
                )
                .expect("validation runs")
        };
        assert!(validate(largest_values()).is_valid());
        let mut too_long = largest_values();
        too_long.insert("label".to_string(), Value::Text(wide(11)));
        assert!(!validate(too_long).is_valid());
    }

    /// Integers take the width the contract's documents store them in: with sized integer
    /// types off, as a version 0 config always has them and DashPay keeps them, a bounded
    /// integer is an i64 of 8 bytes, as format 0 writes every integer.
    #[test]
    fn should_count_integers_at_the_width_the_contract_stores_them() {
        for config in [
            json!({ "$formatVersion": "0" }),
            json!({ "$formatVersion": "1", "sizedIntegerTypes": false }),
            json!({ "$formatVersion": "1", "sizedIntegerTypes": true }),
        ] {
            let sized = config.get("sizedIntegerTypes") == Some(&json!(true));
            let contract = DataContract::from_value(
                platform_value::to_value(json!({
                    "$formatVersion": "1",
                    "id": Identifier::from([7; 32]).to_string(Encoding::Base58),
                    "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
                    "version": 1,
                    "config": config,
                    "documentSchemas": {
                        "secret": {
                            "type": "object",
                            "private": { "entrySize": 400 },
                            "documentsMutable": false,
                            "canBeDeleted": false,
                            "properties": {
                                "small": {
                                    "type": "integer", "minimum": 0, "maximum": 255,
                                    "position": 0
                                }
                            },
                            "required": ["small"],
                            "additionalProperties": false
                        }
                    }
                }))
                .expect("the contract converts"),
                true,
                PlatformVersion::latest(),
            )
            .expect("the contract registers");
            assert_eq!(
                largest_private_document_payload_size(
                    contract
                        .document_type_for_name("secret")
                        .expect("the secret type")
                )
                .expect("bounded"),
                if sized { 1 } else { 8 },
                "{config}"
            );
        }
    }

    /// A `Date` property carries a marker byte when it is optional; its largest size counts it,
    /// as its encoding writes it.
    #[test]
    fn should_size_a_date_with_its_optional_marker() {
        for required in [true, false] {
            assert_eq!(
                DocumentPropertyType::Date.max_encoded_value_size(required),
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
