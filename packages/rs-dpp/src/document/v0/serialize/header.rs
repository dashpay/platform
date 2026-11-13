use crate::consensus::basic::decode::DecodingError;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::errors::DataContractError;
use crate::document::v0::DocumentV0;
use crate::nft::TradeMode;
use crate::prelude::Revision;
use crate::ProtocolError;
use integer_encoding::VarIntReader;
use platform_value::Identifier;
use platform_version::version::FeatureVersion;
use std::io::Read;

impl DocumentV0 {
    /// Reads the id and the revision of a serialized document from the fields
    /// every serialization format (0 to 3) writes ahead of the document's
    /// properties, without decoding the properties.
    ///
    /// The fields are the format, the contract version stamp (format 3), the
    /// id, the owner id, the creator id (formats 2 and 3, for a type whose
    /// documents can change hands) and the revision (for a type whose documents
    /// carry one). Which of them are present depends on the type's
    /// configuration, which a contract update cannot change, and never on its
    /// properties. Bytes stored before an update changed how the properties
    /// are encoded (the member order of a nested object, an optional member
    /// added ahead of others) give their id and revision here, although
    /// [`DocumentV0::from_bytes`] with the updated type no longer reads them.
    /// For bytes [`DocumentV0::from_bytes`] reads, this returns the id and the
    /// revision it decodes.
    ///
    /// The history migration on the first block of protocol version 15 keys
    /// every retained revision by what this returns, so what it accepts and
    /// returns for bytes written in formats 0 to 3 must never change.
    pub fn id_and_revision_from_bytes(
        mut serialized_document: &[u8],
        document_type: DocumentTypeRef,
    ) -> Result<(Identifier, Option<Revision>), ProtocolError> {
        let format: FeatureVersion = serialized_document
            .read_varint()
            .map_err(|error| header_error("serialization format", error))?;
        match format {
            0..=2 => {}
            3 => {
                let stamp: u64 = serialized_document
                    .read_varint()
                    .map_err(|error| header_error("contract version stamp", error))?;
                if stamp > u32::MAX as u64 {
                    return Err(ProtocolError::DataContractError(
                        DataContractError::CorruptedSerialization(
                            "contract version stamp does not fit in a u32".to_string(),
                        ),
                    ));
                }
            }
            version => {
                return Err(ProtocolError::UnknownVersionMismatch {
                    method: "DocumentV0::id_and_revision_from_bytes".to_string(),
                    known_versions: vec![0, 1, 2, 3],
                    received: version,
                })
            }
        }
        let mut id = [0; 32];
        serialized_document
            .read_exact(&mut id)
            .map_err(|error| header_error("id", error))?;
        let mut identifier = [0; 32];
        serialized_document
            .read_exact(&mut identifier)
            .map_err(|error| header_error("owner id", error))?;
        if format >= 2
            && (document_type.trade_mode() != TradeMode::None
                || document_type.documents_transferable().is_transferable())
        {
            let mut has_creator_id = [0];
            serialized_document
                .read_exact(&mut has_creator_id)
                .map_err(|error| header_error("creator id flag", error))?;
            if has_creator_id[0] > 0 {
                serialized_document
                    .read_exact(&mut identifier)
                    .map_err(|error| header_error("creator id", error))?;
            }
        }
        let revision = if document_type.requires_revision() {
            Some(
                serialized_document
                    .read_varint()
                    .map_err(|error| header_error("revision", error))?,
            )
        } else {
            None
        };
        Ok((Identifier::new(id), revision))
    }
}

fn header_error(field: &str, error: std::io::Error) -> ProtocolError {
    ProtocolError::DataContractError(DataContractError::DecodingDocumentError(
        DecodingError::new(format!(
            "error reading the {field} from a serialized document's header: {error}"
        )),
    ))
}

#[cfg(test)]
mod tests {
    use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
    use crate::data_contract::document_type::DocumentType;
    use crate::data_contract::errors::DataContractError;
    use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use crate::document::v0::DocumentV0;
    use crate::ProtocolError;
    use integer_encoding::VarInt;
    use platform_value::{platform_value, Identifier, Value};
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    /// A type with an object property `o` holding `members` in this order and
    /// an optional `status`, configured by `settings` (merged into the type's
    /// schema), in a contract with moderators when `settings` gives them
    /// abilities.
    fn document_type(settings: Value, members: Vec<(&str, Value)>) -> DocumentType {
        let platform_version = PlatformVersion::latest();
        let mut schema = platform_value!({
            "type": "object",
            "properties": {
                "o": {
                    "type": "object", "position": 0,
                    "properties": Value::Map(
                        members
                            .into_iter()
                            .map(|(name, schema)| (Value::Text(name.to_string()), schema))
                            .collect(),
                    ),
                    "required": ["b", "c"], "additionalProperties": false,
                },
                "status": {"type": "integer", "minimum": 0, "maximum": 3, "position": 1},
            },
            "required": ["o"], "additionalProperties": false,
        });
        let Value::Map(settings) = settings else {
            panic!("type settings are a map");
        };
        let mut moderated = false;
        for (key, value) in settings {
            let key = key.as_text().expect("a setting name");
            moderated |= key == "moderatorAbilities";
            schema.set_value(key, value).expect("the schema is a map");
        }
        let config =
            DataContractConfig::default_for_version(platform_version).expect("a default config");
        let config = if moderated {
            config.with_moderation(Some(ContractModerationConfig {
                banlist: false,
                suspensions: false,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
            }))
        } else {
            config
        };
        DocumentType::try_from_schema(
            Identifier::new([2; 32]),
            1,
            config.version(),
            "note",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut Vec::new(),
            platform_version,
        )
        .expect("a valid document type")
    }

    fn members(order: &[&'static str]) -> Vec<(&'static str, Value)> {
        order
            .iter()
            .map(|name| {
                let schema = match *name {
                    "a" => platform_value!({"type": "string", "maxLength": 20, "position": 2}),
                    "b" => platform_value!({"type": "string", "maxLength": 20, "position": 0}),
                    "c" => {
                        platform_value!({"type": "integer", "minimum": 0, "maximum": 10, "position": 1})
                    }
                    other => panic!("no member {other}"),
                };
                (*name, schema)
            })
            .collect()
    }

    fn document(revision: Option<u64>, creator_id: Option<[u8; 32]>) -> DocumentV0 {
        let mut properties = BTreeMap::new();
        properties.insert("o".to_string(), platform_value!({"b": "hello", "c": 5u8}));
        DocumentV0 {
            id: Identifier::new([9; 32]),
            owner_id: Identifier::new([7; 32]),
            creator_id: creator_id.map(Identifier::new),
            properties,
            revision,
            created_at: Some(1_000),
            ..Default::default()
        }
    }

    /// Every type configuration that changes which header fields are
    /// written: a revision or none, and a creator block or none.
    fn configurations() -> Vec<(&'static str, Value)> {
        vec![
            ("mutable", platform_value!({"documentsMutable": true})),
            ("immutable", platform_value!({"documentsMutable": false})),
            (
                "changed by its moderators",
                platform_value!({
                    "documentsMutable": false,
                    "moderatorAbilities": {"changeFields": ["status"]},
                }),
            ),
            (
                "transferable",
                platform_value!({"documentsMutable": false, "transferable": 1}),
            ),
            (
                "sold by its owner",
                platform_value!({"documentsMutable": false, "transferable": 1, "tradeMode": 1}),
            ),
        ]
    }

    fn serialize(document: &DocumentV0, format: u16, document_type: &DocumentType) -> Vec<u8> {
        let document_type = document_type.as_ref();
        match format {
            0 => document.serialize_v0(document_type),
            1 => document.serialize_v1(document_type),
            2 => document.serialize_v2(document_type),
            3 => document.serialize_v3(document_type),
            _ => unreachable!("formats 0 to 3"),
        }
        .expect("a serializable document")
    }

    #[test]
    fn should_read_the_id_and_revision_from_bytes_decodes_in_every_format() {
        let platform_version = PlatformVersion::latest();
        for (name, settings) in configurations() {
            let document_type = document_type(settings, members(&["b", "c"]));
            let moderated = document_type.as_ref().has_moderator_changeable_fields();
            if moderated {
                assert!(
                    !document_type.as_ref().documents_mutable()
                        && !document_type
                            .as_ref()
                            .documents_transferable()
                            .is_transferable()
                        && document_type.as_ref().requires_revision(),
                    "the moderators' fields alone give the {name} type a revision"
                );
            }
            let revision = document_type.as_ref().requires_revision().then_some(4);
            for creator_id in [None, Some([3; 32])] {
                for format in 0..=3 {
                    let mut document = document(revision, creator_id);
                    if moderated && format == 3 {
                        // The moderation stamp sits in the time fields, after the
                        // revision; only format 3 has a place for it.
                        document.moderated_at = Some(2_000);
                        document.moderated_by = Some(Identifier::new([5; 32]));
                    }
                    let bytes = serialize(&document, format, &document_type);
                    let decoded =
                        DocumentV0::from_bytes(&bytes, document_type.as_ref(), platform_version)
                            .expect("bytes the type reads");
                    assert_eq!(
                        DocumentV0::id_and_revision_from_bytes(&bytes, document_type.as_ref())
                            .expect("a readable header"),
                        (decoded.id, decoded.revision),
                        "{name} type, format {format}, creator {creator_id:?}"
                    );
                    assert_eq!(decoded.revision, revision);
                }
            }
        }
    }

    #[test]
    fn should_read_the_header_of_bytes_written_before_a_nested_member_change() {
        let platform_version = PlatformVersion::latest();
        for (name, settings) in configurations() {
            let written = document_type(settings.clone(), members(&["b", "c"]));
            let revision = written.as_ref().requires_revision().then_some(4);
            for (change, order) in [
                ("reordered", &["c", "b"][..]),
                ("prepended optional member", &["a", "b", "c"][..]),
            ] {
                let updated = document_type(settings.clone(), members(order));
                for format in 0..=3 {
                    let bytes = serialize(&document(revision, Some([3; 32])), format, &written);
                    assert!(
                        DocumentV0::from_bytes(&bytes, updated.as_ref(), platform_version).is_err(),
                        "the {change} type no longer reads the {name} document in format {format}"
                    );
                    assert_eq!(
                        DocumentV0::id_and_revision_from_bytes(&bytes, updated.as_ref())
                            .expect("a readable header"),
                        (Identifier::new([9; 32]), revision),
                        "{name} type, {change}, format {format}"
                    );
                }
            }
        }
    }

    #[test]
    fn should_refuse_a_header_it_cannot_read() {
        let document_type = document_type(
            platform_value!({"documentsMutable": true}),
            members(&["b", "c"]),
        );
        let bytes = serialize(&document(Some(4), None), 3, &document_type);
        for truncated in [&bytes[..0], &bytes[..1], &bytes[..40], &bytes[..66]] {
            assert!(
                DocumentV0::id_and_revision_from_bytes(truncated, document_type.as_ref()).is_err(),
                "{} bytes hold no complete header",
                truncated.len()
            );
        }
        assert!(matches!(
            DocumentV0::id_and_revision_from_bytes(&[4, 0], document_type.as_ref()),
            Err(ProtocolError::UnknownVersionMismatch { received: 4, .. })
        ));
    }

    #[test]
    fn should_refuse_a_header_cut_inside_its_creator_id_block() {
        let document_type = document_type(
            platform_value!({"documentsMutable": false, "transferable": 1}),
            members(&["b", "c"]),
        );
        for format in [2, 3] {
            let bytes = serialize(&document(Some(4), Some([3; 32])), format, &document_type);
            // The format, an unset contract version stamp in format 3, the id
            // and the owner id come before the creator id flag.
            let flag = if format == 3 { 66 } else { 65 };
            assert_eq!(bytes[flag], 1, "format {format} marks a creator id");
            for (end, cut) in [
                (flag, "before the creator id flag"),
                (flag + 1, "right after the creator id flag"),
                (flag + 17, "inside the creator id"),
                (flag + 33, "before the revision"),
            ] {
                assert!(
                    DocumentV0::id_and_revision_from_bytes(&bytes[..end], document_type.as_ref())
                        .is_err(),
                    "format {format} cut {cut} holds no complete header"
                );
            }
            assert_eq!(
                DocumentV0::id_and_revision_from_bytes(&bytes[..flag + 34], document_type.as_ref())
                    .expect("a complete header"),
                (Identifier::new([9; 32]), Some(4)),
                "format {format}"
            );
        }
    }

    #[test]
    fn should_refuse_a_contract_version_stamp_the_full_reader_refuses() {
        let platform_version = PlatformVersion::latest();
        let document_type = document_type(
            platform_value!({"documentsMutable": true}),
            members(&["b", "c"]),
        );
        let bytes = serialize(&document(Some(4), None), 3, &document_type);
        assert_eq!(bytes[1], 0, "an unset stamp is one byte");
        let with_stamp = |stamp: u64| [&bytes[..1], &stamp.encode_var_vec(), &bytes[2..]].concat();
        let largest = with_stamp(u32::MAX as u64);
        assert!(DocumentV0::from_bytes(&largest, document_type.as_ref(), platform_version).is_ok());
        assert_eq!(
            DocumentV0::id_and_revision_from_bytes(&largest, document_type.as_ref())
                .expect("a stamp that fits in a u32"),
            (Identifier::new([9; 32]), Some(4))
        );
        let too_large = with_stamp(u32::MAX as u64 + 1);
        assert!(
            DocumentV0::from_bytes(&too_large, document_type.as_ref(), platform_version).is_err()
        );
        assert!(matches!(
            DocumentV0::id_and_revision_from_bytes(&too_large, document_type.as_ref()),
            Err(ProtocolError::DataContractError(
                DataContractError::CorruptedSerialization(_)
            ))
        ));
    }
}
