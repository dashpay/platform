use crate::data_contract::config::moderation::ContractModerationReason;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::property_constraints::SystemProperty;
use crate::data_contract::document_type::{
    property_at_path, DocumentPropertyType, DocumentTypeRef,
};
use crate::data_contract::errors::DataContractError;
use crate::document::{Document, DocumentV0Getters};
use crate::identity::TimestampMillis;
#[cfg(feature = "serde-conversion")]
use crate::serialization::serde_bytes_var;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use byteorder::{BigEndian, ReadBytesExt};
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufReader, Read};

/// The record a contract keeps of a document one of its moderators deleted.
///
/// The document itself is gone; this is what is left to say that it was removed, not lost:
/// whose it was, who removed it, why and when, a hash of what it was, and the values of the
/// fields its type keeps public (`moderatorAbilities.deleteKeepsFields`). It is stored under
/// the contract, by document type then document id, paid for by the moderator, and never
/// deleted. A document id commits to the nonce of its create transition and is produced at
/// most once, so the removed id can not be created again; what can bring the document back
/// is a moderator's restore, within `SystemLimits::contract_document_restore_window_ms` of
/// the removal, which marks the record restored and leaves it in place. A later deletion of
/// the restored document replaces the record with a fresh one.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, Encode, Decode, DecodeUntrusted, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct ContractDocumentRemoval {
    /// The identity that owned the document when it was removed.
    pub document_owner_id: Identifier,
    /// The contract owner or moderator that removed it.
    pub moderator_id: Identifier,
    /// Why the moderator removed it. The code is a number the moderator sets and nothing
    /// checks; the text may be empty.
    pub reason: ContractModerationReason,
    /// The time of the block that removed it, in milliseconds.
    pub removed_at: TimestampMillis,
    /// A double SHA-256 of the document as it was serialized under its document type at the
    /// time of the removal (`Document::serialize`): what a restore must bring back, byte for
    /// byte.
    pub document_hash: [u8; 32],
    /// Set once a moderator restored the document: it is live again, at its id, as it was.
    pub restoration: Option<ContractDocumentRestoration>,
    /// The values the record keeps of the document, the fields its type lists under
    /// `moderatorAbilities.deleteKeepsFields`, as the record stores them: encoded as the
    /// document encodes its properties (see [`encode_kept_fields`]), so read, as a document is,
    /// under the document's type ([`ContractDocumentRemoval::kept_values`]). What of the
    /// document stays public once it is gone. Empty on a type that lists none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "serde-conversion", serde(with = "serde_bytes_var"))]
    pub kept_fields: Vec<u8>,
}

/// The mark a restore leaves on a removal record: who brought the document back, and when.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, Encode, Decode, DecodeUntrusted, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct ContractDocumentRestoration {
    /// The contract owner or moderator that restored the document.
    pub moderator_id: Identifier,
    /// The time of the block that restored it, in milliseconds.
    pub restored_at: TimestampMillis,
}

/// No value at the path.
const KEPT_VALUE_ABSENT: u8 = 0;
/// A value follows.
const KEPT_VALUE_PRESENT: u8 = 1;

/// Encodes the values a removal record keeps of `document`, a document of `document_type`
/// (`moderatorAbilities.deleteKeepsFields`), the way the document encodes its properties: for
/// each path the type lists, in the list's order, `0` when the document holds no value there,
/// or `1` followed by the value as an optional property of its type is written
/// (`DocumentPropertyType::encode_value_ref_with_size`), an object length-prefixed with its
/// members in order, a time or a block height as eight big-endian bytes and a core block
/// height as four. The paths themselves are not written: the type lists them and never changes
/// the list, so a reader knows them from the type. Empty for a type that keeps none.
pub fn encode_kept_fields(
    document: &Document,
    document_type: DocumentTypeRef,
) -> Result<Vec<u8>, ProtocolError> {
    let mut encoded = Vec::new();
    for path in document_type.moderator_deletion_kept_fields() {
        let value = match SystemProperty::from_name(path) {
            Some(property) => system_property_bytes(document, property),
            None => {
                let property = kept_property_type(&document_type, path)?;
                match document.get(path).filter(|value| !value.is_null()) {
                    Some(value) => Some(property.encode_value_ref_with_size(value, false)?),
                    None => None,
                }
            }
        };
        match value {
            Some(value) => {
                encoded.push(KEPT_VALUE_PRESENT);
                encoded.extend(value);
            }
            None => encoded.push(KEPT_VALUE_ABSENT),
        }
    }
    Ok(encoded)
}

/// Reads the values [`encode_kept_fields`] wrote, under `document_type`: each kept path to its
/// value, typed as reading the document gives it, a path the document held no value at left
/// out. Refuses bytes that end before the last path or run past it.
pub fn decode_kept_fields(
    encoded: &[u8],
    document_type: DocumentTypeRef,
) -> Result<BTreeMap<String, Value>, ProtocolError> {
    let corrupted = |message: String| {
        ProtocolError::DataContractError(DataContractError::CorruptedSerialization(message))
    };
    let mut buf = BufReader::new(encoded);
    let mut values = BTreeMap::new();
    for path in document_type.moderator_deletion_kept_fields() {
        let value = match SystemProperty::from_name(path) {
            Some(property) => {
                let marker = buf
                    .read_u8()
                    .map_err(|_| corrupted(format!("kept fields end before \"{path}\"")))?;
                match marker {
                    KEPT_VALUE_ABSENT => None,
                    KEPT_VALUE_PRESENT => Some(
                        read_system_property(&mut buf, property)
                            .map_err(|_| corrupted(format!("kept fields end inside \"{path}\"")))?,
                    ),
                    marker => {
                        return Err(corrupted(format!(
                            "kept field \"{path}\" has unknown marker {marker}"
                        )))
                    }
                }
            }
            None => {
                let (value, finished) = kept_property_type(&document_type, path)?
                    .read_optionally_from(&mut buf, false)
                    .map_err(ProtocolError::DataContractError)?;
                if finished {
                    return Err(corrupted(format!("kept fields end before \"{path}\"")));
                }
                value
            }
        };
        if let Some(value) = value {
            values.insert(path.clone(), value);
        }
    }
    let mut trailing = [0u8; 1];
    if buf
        .read(&mut trailing)
        .map_err(|error| corrupted(error.to_string()))?
        > 0
    {
        return Err(corrupted(
            "kept fields run past the last path their type keeps".to_string(),
        ));
    }
    Ok(values)
}

/// The type of the property at `path`, one the parser admitted as kept on `document_type`.
fn kept_property_type<'a>(
    document_type: &'a DocumentTypeRef,
    path: &str,
) -> Result<&'a DocumentPropertyType, ProtocolError> {
    property_at_path(document_type.properties(), path)
        .map(|property| &property.property_type)
        .ok_or_else(|| {
            ProtocolError::CorruptedCodeExecution(format!(
                "document type {} keeps \"{path}\", which it does not declare",
                document_type.name()
            ))
        })
}

/// The bytes `document` holds for `property`, as the document stores its times and heights.
fn system_property_bytes(document: &Document, property: SystemProperty) -> Option<Vec<u8>> {
    let long = |value: Option<u64>| value.map(|value| value.to_be_bytes().to_vec());
    let short = |value: Option<u32>| value.map(|value| value.to_be_bytes().to_vec());
    match property {
        SystemProperty::CreatedAt => long(document.created_at()),
        SystemProperty::UpdatedAt => long(document.updated_at()),
        SystemProperty::TransferredAt => long(document.transferred_at()),
        SystemProperty::CreatedAtBlockHeight => long(document.created_at_block_height()),
        SystemProperty::UpdatedAtBlockHeight => long(document.updated_at_block_height()),
        SystemProperty::TransferredAtBlockHeight => long(document.transferred_at_block_height()),
        SystemProperty::CreatedAtCoreBlockHeight => short(document.created_at_core_block_height()),
        SystemProperty::UpdatedAtCoreBlockHeight => short(document.updated_at_core_block_height()),
        SystemProperty::TransferredAtCoreBlockHeight => {
            short(document.transferred_at_core_block_height())
        }
    }
}

/// Reads what [`system_property_bytes`] wrote for `property`: a `U64` for a time or a block
/// height, a `U32` for a core block height.
fn read_system_property(
    buf: &mut BufReader<&[u8]>,
    property: SystemProperty,
) -> std::io::Result<Value> {
    match property {
        SystemProperty::CreatedAtCoreBlockHeight
        | SystemProperty::UpdatedAtCoreBlockHeight
        | SystemProperty::TransferredAtCoreBlockHeight => {
            buf.read_u32::<BigEndian>().map(Value::U32)
        }
        SystemProperty::CreatedAt
        | SystemProperty::UpdatedAt
        | SystemProperty::TransferredAt
        | SystemProperty::CreatedAtBlockHeight
        | SystemProperty::UpdatedAtBlockHeight
        | SystemProperty::TransferredAtBlockHeight => buf.read_u64::<BigEndian>().map(Value::U64),
    }
}

/// What a removal record says of `property`, from the paths its document type keeps
/// (`kept_paths`, its `moderatorAbilities.deleteKeepsFields`) and the values the record keeps
/// (`kept_values`, [`ContractDocumentRemoval::kept_values`]): `None` when the record does not
/// keep it, neither kept itself nor inside a kept object; otherwise the value, `Some(None)`
/// when the document held none there. Kept paths never nest, so at most one covers it.
pub fn kept_value_at(
    kept_paths: &BTreeSet<String>,
    kept_values: &BTreeMap<String, Value>,
    property: &str,
) -> Option<Option<Value>> {
    if kept_paths.contains(property) {
        return Some(kept_values.get(property).cloned());
    }
    let (kept, inner) = kept_paths.iter().find_map(|kept| {
        let inner = property.strip_prefix(kept.as_str())?.strip_prefix('.')?;
        Some((kept, inner))
    })?;
    // Down through the kept object, a member it lacks or a step through what is no object
    // reading as absent, as a document's own path does
    Some(
        kept_values
            .get(kept)
            .and_then(|object| object.get_optional_value_at_path(inner).ok().flatten())
            .filter(|value| !value.is_null())
            .cloned(),
    )
}

impl ContractDocumentRemoval {
    /// Whether the document was restored: the record then describes a removal that was
    /// undone, and the document is live again.
    pub fn is_restored(&self) -> bool {
        self.restoration.is_some()
    }

    /// The values the record keeps, read under `document_type`, the removed document's type,
    /// as its documents are: each path the type keeps to its value, a path the document held
    /// no value at left out (see [`decode_kept_fields`]). Empty when the record keeps none.
    pub fn kept_values(
        &self,
        document_type: DocumentTypeRef,
    ) -> Result<BTreeMap<String, Value>, ProtocolError> {
        if self.kept_fields.is_empty() {
            return Ok(BTreeMap::new());
        }
        decode_kept_fields(&self.kept_fields, document_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::{is_path_listed, DocumentType};
    use crate::document::DocumentV0;
    use platform_value::platform_value;
    use platform_version::version::PlatformVersion;

    /// A post keeping a text it may lack, an integer, an identifier, a whole object, a member
    /// of another object, and its creation time and core block height
    fn post_type() -> DocumentType {
        let platform_version = PlatformVersion::latest();
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("expected a default config")
            .with_moderation(Some(ContractModerationConfig {
                banlist: false,
                suspensions: false,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
            }));
        DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "post",
            platform_value!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                    "hashtag": { "type": "string", "maxLength": 61, "position": 1 },
                    "score": { "type": "integer", "position": 2 },
                    "author": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 3,
                    },
                    "meta": {
                        "type": "object",
                        "position": 4,
                        "properties": {
                            "tags": {
                                "type": "array",
                                "items": { "type": "string", "maxLength": 20 },
                                "maxItems": 5,
                                "position": 0,
                            },
                            "note": { "type": "string", "maxLength": 20, "position": 1 },
                        },
                        "additionalProperties": false,
                    },
                    "extra": {
                        "type": "object",
                        "position": 5,
                        "properties": {
                            "count": { "type": "integer", "position": 0 },
                            "label": { "type": "string", "maxLength": 20, "position": 1 },
                        },
                        "additionalProperties": false,
                    },
                },
                "required": ["$createdAt", "$createdAtCoreBlockHeight"],
                "additionalProperties": false,
                "moderatorAbilities": {
                    "delete": true,
                    "deleteKeepsFields": [
                        "$createdAt",
                        "$createdAtCoreBlockHeight",
                        "author",
                        "extra.count",
                        "hashtag",
                        "meta",
                        "score",
                        "text",
                    ],
                },
            }),
            None,
            &BTreeMap::new(),
            &config,
            true,
            &mut vec![],
            platform_version,
        )
        .expect("expected the post type")
    }

    fn post(properties: BTreeMap<String, Value>) -> Document {
        Document::V0(DocumentV0 {
            id: Identifier::new([2; 32]),
            owner_id: Identifier::new([3; 32]),
            properties,
            created_at: Some(1_700_000_000_000),
            created_at_core_block_height: Some(55),
            ..Default::default()
        })
    }

    #[test]
    fn should_keep_values_as_the_document_holds_them() {
        let document_type = post_type();
        let meta = Value::Map(vec![
            (
                Value::Text("tags".to_string()),
                Value::Array(vec![
                    Value::Text("privacy".to_string()),
                    Value::Text("payments".to_string()),
                ]),
            ),
            (
                Value::Text("note".to_string()),
                Value::Text("n".to_string()),
            ),
        ]);
        let document = post(BTreeMap::from([
            ("hashtag".to_string(), Value::Text("dash".to_string())),
            ("score".to_string(), Value::I64(-3)),
            ("author".to_string(), Value::Identifier([4; 32])),
            ("meta".to_string(), meta.clone()),
            (
                "extra".to_string(),
                Value::Map(vec![
                    (Value::Text("count".to_string()), Value::I64(7)),
                    (
                        Value::Text("label".to_string()),
                        Value::Text("not kept".to_string()),
                    ),
                ]),
            ),
        ]));

        let encoded =
            encode_kept_fields(&document, document_type.as_ref()).expect("expected to encode");
        // One marker per kept path, in the list's order: `$createdAt` first, then its value
        assert_eq!(encoded[0], KEPT_VALUE_PRESENT);
        assert_eq!(&encoded[1..9], &1_700_000_000_000u64.to_be_bytes());
        // `text`, last in the list, holds nothing
        assert_eq!(encoded.last(), Some(&KEPT_VALUE_ABSENT));

        let removal = ContractDocumentRemoval {
            kept_fields: encoded,
            ..Default::default()
        };
        assert_eq!(
            removal
                .kept_values(document_type.as_ref())
                .expect("expected to decode"),
            BTreeMap::from([
                ("$createdAt".to_string(), Value::U64(1_700_000_000_000)),
                ("$createdAtCoreBlockHeight".to_string(), Value::U32(55)),
                ("author".to_string(), Value::Identifier([4; 32])),
                ("extra.count".to_string(), Value::I64(7)),
                ("hashtag".to_string(), Value::Text("dash".to_string())),
                ("meta".to_string(), meta),
                ("score".to_string(), Value::I64(-3)),
            ])
        );
    }

    #[test]
    fn should_read_a_kept_path_or_a_path_inside_a_kept_object() {
        let document_type = post_type();
        let kept_paths = document_type.moderator_deletion_kept_fields();
        let document = post(BTreeMap::from([
            ("hashtag".to_string(), Value::Text("dash".to_string())),
            (
                "meta".to_string(),
                Value::Map(vec![(
                    Value::Text("tags".to_string()),
                    Value::Array(vec![Value::Text("privacy".to_string())]),
                )]),
            ),
            (
                "extra".to_string(),
                Value::Map(vec![(Value::Text("count".to_string()), Value::I64(7))]),
            ),
        ]));
        let removal = ContractDocumentRemoval {
            kept_fields: encode_kept_fields(&document, document_type.as_ref())
                .expect("expected to encode"),
            ..Default::default()
        };
        let kept_values = removal
            .kept_values(document_type.as_ref())
            .expect("expected to decode");
        // Each as the document holds it, at the path the document holds it
        for path in ["hashtag", "meta.tags", "extra.count"] {
            let value = document.get(path).cloned();
            assert!(value.is_some(), "{path}");
            assert_eq!(
                kept_value_at(kept_paths, &kept_values, path),
                Some(value),
                "{path}"
            );
        }
        // Kept, but nothing where the document held nothing, inside a kept object or not
        for path in ["meta.note", "text"] {
            assert_eq!(
                kept_value_at(kept_paths, &kept_values, path),
                Some(None),
                "{path}"
            );
        }
        // Not kept: a member of an object only partly kept, the object itself, a name
        // sharing a kept path's prefix, and the owner, which every record holds apart
        for path in ["extra.label", "extra", "hashtagged", "$ownerId"] {
            assert!(!is_path_listed(kept_paths, path), "{path}");
            assert_eq!(
                kept_value_at(kept_paths, &kept_values, path),
                None,
                "{path}"
            );
        }
    }

    #[test]
    fn should_keep_nothing_of_a_document_that_holds_none_of_the_paths() {
        let document_type = post_type();
        let document = Document::V0(DocumentV0 {
            id: Identifier::new([2; 32]),
            owner_id: Identifier::new([3; 32]),
            ..Default::default()
        });
        let encoded =
            encode_kept_fields(&document, document_type.as_ref()).expect("expected to encode");
        assert_eq!(encoded, vec![KEPT_VALUE_ABSENT; 8]);
        assert_eq!(
            decode_kept_fields(&encoded, document_type.as_ref()).expect("expected to decode"),
            BTreeMap::new()
        );
    }

    #[test]
    fn should_refuse_kept_fields_cut_short_or_running_past_the_last_path() {
        let document_type = post_type();
        let document = post(BTreeMap::from([(
            "hashtag".to_string(),
            Value::Text("dash".to_string()),
        )]));
        let encoded =
            encode_kept_fields(&document, document_type.as_ref()).expect("expected to encode");
        decode_kept_fields(&encoded[..encoded.len() - 1], document_type.as_ref())
            .expect_err("the last path's marker is missing");
        decode_kept_fields(&encoded[..3], document_type.as_ref())
            .expect_err("cut short inside a time");
        let mut trailing = encoded.clone();
        trailing.push(0);
        decode_kept_fields(&trailing, document_type.as_ref()).expect_err("a byte past the end");
        let mut unknown_marker = encoded;
        unknown_marker[0] = 7;
        decode_kept_fields(&unknown_marker, document_type.as_ref()).expect_err("an unknown marker");
    }
}
