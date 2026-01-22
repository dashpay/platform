use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::v1::DocumentTypeV1;
use crate::data_contract::document_type::{
    property_names, DocumentProperty, DocumentPropertyReference, DocumentPropertyReferenceTarget,
    DocumentPropertyType, DocumentType,
};
use crate::data_contract::errors::DataContractError;
use crate::data_contract::{TokenConfiguration, TokenContractPosition};
use crate::util::json_schema::resolve_uri;
use crate::validation::operations::ProtocolValidationOperation;
use crate::ProtocolError;
use indexmap::IndexMap;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, BTreeSet};

mod v0;
mod v1;

const NOT_ALLOWED_SYSTEM_PROPERTIES: [&str; 1] = ["$id"];

const MAX_INDEXED_STRING_PROPERTY_LENGTH: u16 = 63;
const MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH: u16 = 255;
const MAX_INDEXED_ARRAY_ITEMS: usize = 1024;

impl DocumentType {
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_schema(
        data_contract_id: Identifier,
        data_contract_system_version: u16,
        contract_config_version: u16,
        name: &str,
        schema: Value,
        schema_defs: Option<&BTreeMap<String, Value>>,
        token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        data_contact_config: &DataContractConfig,
        full_validation: bool,
        validation_operations: &mut impl Extend<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .class_method_versions
            .try_from_schema
        {
            0 => DocumentTypeV0::try_from_schema(
                data_contract_id,
                data_contract_system_version,
                contract_config_version,
                name,
                schema,
                schema_defs,
                data_contact_config,
                full_validation,
                validation_operations,
                platform_version,
            )
            .map(|document_type| document_type.into()),
            1 => DocumentTypeV1::try_from_schema(
                data_contract_id,
                data_contract_system_version,
                contract_config_version,
                name,
                schema,
                schema_defs,
                token_configurations,
                data_contact_config,
                full_validation,
                validation_operations,
                platform_version,
            )
            .map(|document_type| document_type.into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "try_from_schema".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn insert_values(
    document_properties: &mut IndexMap<String, DocumentProperty>,
    known_required: &BTreeSet<String>,
    known_transient: &BTreeSet<String>,
    prefix: Option<String>,
    property_key: String,
    property_value: &Value,
    root_schema: &Value,
    config: &DataContractConfig,
) -> Result<(), DataContractError> {
    let mut to_visit: Vec<(Option<String>, String, &Value)> =
        vec![(prefix, property_key, property_value)];

    while let Some((prefix, property_key, property_value)) = to_visit.pop() {
        let prefixed_property_key = match prefix {
            None => property_key,
            Some(prefix) => [prefix, property_key].join(".").to_owned(),
        };

        let mut inner_properties = property_value.to_btree_ref_string_map()?;

        if let Some(schema_ref) = inner_properties.get_optional_str(property_names::REF)? {
            let referenced_sub_schema = resolve_uri(root_schema, schema_ref)?;

            inner_properties = referenced_sub_schema.to_btree_ref_string_map()?
        }

        let is_required = known_required.contains(&prefixed_property_key);
        let is_transient = known_transient.contains(&prefixed_property_key);

        let property_type =
            DocumentPropertyType::try_from_value_map(&inner_properties, &config.into())?;

        match property_type {
            DocumentPropertyType::Object(_) => {
                if let Some(properties_as_value) = inner_properties.get(property_names::PROPERTIES)
                {
                    let properties =
                        properties_as_value
                            .as_map()
                            .ok_or(DataContractError::ValueWrongType(
                                "properties must be a map".to_string(),
                            ))?;

                    for (object_property_key, object_property_value) in properties.iter() {
                        let object_property_string = object_property_key
                            .as_text()
                            .ok_or(DataContractError::KeyWrongType(
                                "property key must be a string".to_string(),
                            ))?
                            .to_string();
                        to_visit.push((
                            Some(prefixed_property_key.clone()),
                            object_property_string,
                            object_property_value,
                        ));
                    }
                }
            }
            property_type => {
                let reference = parse_property_reference(&inner_properties, &property_type)?;
                document_properties.insert(
                    prefixed_property_key,
                    DocumentProperty {
                        property_type,
                        required: is_required,
                        transient: is_transient,
                        reference,
                    },
                );
            }
        };
    }

    Ok(())
}

// TODO: This is quite big
fn insert_values_nested(
    document_properties: &mut IndexMap<String, DocumentProperty>,
    known_required: &BTreeSet<String>,
    known_transient: &BTreeSet<String>,
    property_key: String,
    property_value: &Value,
    root_schema: &Value,
    config: &DataContractConfig,
) -> Result<(), DataContractError> {
    let mut inner_properties = property_value.to_btree_ref_string_map()?;

    if let Some(schema_ref) = inner_properties.get_optional_str(property_names::REF)? {
        let referenced_sub_schema = resolve_uri(root_schema, schema_ref)?;

        inner_properties = referenced_sub_schema.to_btree_ref_string_map()?;
    }

    let is_required = known_required.contains(&property_key);

    let is_transient = known_transient.contains(&property_key);

    let property_type =
        match DocumentPropertyType::try_from_value_map(&inner_properties, &config.into())? {
            DocumentPropertyType::Object(_) => {
                let mut nested_properties = IndexMap::new();
                if let Some(properties_as_value) = inner_properties.get(property_names::PROPERTIES)
                {
                    let properties =
                        properties_as_value
                            .as_map()
                            .ok_or(DataContractError::ValueWrongType(
                                "properties must be a map".to_string(),
                            ))?;

                    let mut sorted_properties: Vec<_> = properties.iter().collect();

                    sorted_properties.sort_by(|(_, value_1), (_, value_2)| {
                        let pos_1: u64 = value_1
                            .get_integer(property_names::POSITION)
                            .expect("expected a position");
                        let pos_2: u64 = value_2
                            .get_integer(property_names::POSITION)
                            .expect("expected a position");
                        pos_1.cmp(&pos_2)
                    });

                    // Create a new set with the prefix removed from the keys
                    let stripped_required: BTreeSet<String> = known_required
                        .iter()
                        .filter_map(|key| {
                            if key.starts_with(&property_key) && key.len() > property_key.len() {
                                Some(key[property_key.len() + 1..].to_string())
                            } else {
                                None
                            }
                        })
                        .collect();

                    let stripped_transient: BTreeSet<String> = known_transient
                        .iter()
                        .filter_map(|key| {
                            if key.starts_with(&property_key) && key.len() > property_key.len() {
                                Some(key[property_key.len() + 1..].to_string())
                            } else {
                                None
                            }
                        })
                        .collect();

                    for (object_property_key, object_property_value) in properties.iter() {
                        let object_property_string = object_property_key
                            .as_text()
                            .ok_or(DataContractError::KeyWrongType(
                                "property key must be a string".to_string(),
                            ))?
                            .to_string();

                        insert_values_nested(
                            &mut nested_properties,
                            &stripped_required,
                            &stripped_transient,
                            object_property_string,
                            object_property_value,
                            root_schema,
                            config,
                        )?;
                    }
                }

                DocumentPropertyType::Object(nested_properties)
            }
            property_type => property_type,
        };

    let reference = parse_property_reference(&inner_properties, &property_type)?;

    document_properties.insert(
        property_key,
        DocumentProperty {
            property_type,
            required: is_required,
            transient: is_transient,
            reference,
        },
    );

    Ok(())
}

fn parse_property_reference(
    inner_properties: &BTreeMap<String, &Value>,
    property_type: &DocumentPropertyType,
) -> Result<Option<DocumentPropertyReference>, DataContractError> {
    let Some(refers_to_value) = inner_properties.get(property_names::REFERS_TO) else {
        return Ok(None);
    };

    if !matches!(property_type, DocumentPropertyType::Identifier) {
        return Err(DataContractError::InvalidContractStructure(
            "refersTo is only allowed on identifier properties".to_string(),
        ));
    }

    let refers_to_map = refers_to_value.to_btree_ref_string_map()?;

    let target = match refers_to_map
        .get_str(property_names::TYPE)
        .map_err(|e| DataContractError::ValueWrongType(e.to_string()))?
    {
        "identity" => DocumentPropertyReferenceTarget::Identity,
        "contract" => DocumentPropertyReferenceTarget::Contract,
        other => {
            return Err(DataContractError::InvalidContractStructure(format!(
                "invalid refersTo type {other}"
            )))
        }
    };

    let must_exist = refers_to_map
        .get_optional_bool("mustExist")
        .map_err(|e| DataContractError::ValueWrongType(e.to_string()))?
        .unwrap_or(true);

    Ok(Some(DocumentPropertyReference { target, must_exist }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use platform_value::Identifier;
    use platform_version::version::PlatformVersion;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn should_parse_refers_to_on_identifier_property() {
        let platform_version = PlatformVersion::latest();
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");

        let schema = json!({
            "type": "object",
            "properties": {
                "toUserId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "identity"
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        });

        let value = platform_value::to_value(schema).expect("schema should convert");

        let document_type = DocumentType::try_from_schema(
            Identifier::random(),
            0,
            config.version(),
            "msg",
            value,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut vec![],
            platform_version,
        )
        .expect("should parse");

        let reference = document_type
            .as_ref()
            .flattened_properties()
            .get("toUserId")
            .and_then(|p| p.reference.clone())
            .expect("reference should be present");

        assert!(matches!(
            reference.target,
            DocumentPropertyReferenceTarget::Identity
        ));
        assert!(reference.must_exist);
    }

    #[test]
    fn should_reject_refers_to_on_non_identifier_property() {
        let platform_version = PlatformVersion::latest();
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");

        let schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "position": 0,
                    "refersTo": { "type": "identity" }
                }
            },
            "required": [],
            "additionalProperties": false
        });

        let value = platform_value::to_value(schema).expect("schema should convert");

        let err = DocumentType::try_from_schema(
            Identifier::random(),
            0,
            config.version(),
            "msg",
            value,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut vec![],
            platform_version,
        )
        .expect_err("should fail");

        let message = err.to_string();
        assert!(
            message.contains("refersTo is only allowed on identifier properties"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn should_parse_refers_to_with_must_exist_false() {
        let platform_version = PlatformVersion::latest();
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");

        let schema = json!({
            "type": "object",
            "properties": {
                "toUserId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "identity",
                        "mustExist": false
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        });

        let value = platform_value::to_value(schema).expect("schema should convert");

        let document_type = DocumentType::try_from_schema(
            Identifier::random(),
            0,
            config.version(),
            "msg",
            value,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut vec![],
            platform_version,
        )
        .expect("should parse");

        let reference = document_type
            .as_ref()
            .flattened_properties()
            .get("toUserId")
            .and_then(|p| p.reference.clone())
            .expect("reference should be present");

        assert!(matches!(
            reference.target,
            DocumentPropertyReferenceTarget::Identity
        ));
        assert!(!reference.must_exist);
    }

    #[test]
    fn should_parse_refers_to_with_contract_target() {
        let platform_version = PlatformVersion::latest();
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");

        let schema = json!({
            "type": "object",
            "properties": {
                "toContractId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "contract"
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        });

        let value = platform_value::to_value(schema).expect("schema should convert");

        let document_type = DocumentType::try_from_schema(
            Identifier::random(),
            0,
            config.version(),
            "msg",
            value,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut vec![],
            platform_version,
        )
        .expect("should parse");

        let reference = document_type
            .as_ref()
            .flattened_properties()
            .get("toContractId")
            .and_then(|p| p.reference.clone())
            .expect("reference should be present");

        assert!(matches!(
            reference.target,
            DocumentPropertyReferenceTarget::Contract
        ));
        assert!(reference.must_exist);
    }
}
