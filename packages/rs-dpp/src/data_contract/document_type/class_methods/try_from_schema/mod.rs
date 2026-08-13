use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::v1::DocumentTypeV1;
use crate::data_contract::document_type::{
    property_names, DocumentProperty, DocumentPropertyReferenceTarget, DocumentPropertyType,
    DocumentType,
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

mod common;
mod v0;
mod v1;
mod v2;
mod v3;

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
            2 => DocumentType::try_from_schema_v2(
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
            ),
            3 => DocumentType::try_from_schema_v3(
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
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "try_from_schema".to_string(),
                known_versions: vec![0, 1, 2, 3],
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
    platform_version: &PlatformVersion,
) -> Result<(), DataContractError> {
    let mut to_visit: Vec<(Option<String>, String, &Value)> =
        vec![(prefix, property_key, property_value)];

    while let Some((prefix, property_key, property_value)) = to_visit.pop() {
        let is_top_level = prefix.is_none();
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
        let required_since = apply_required_since(
            &inner_properties,
            is_required,
            is_top_level,
            platform_version,
        )?;

        match DocumentPropertyType::try_from_value_map(&inner_properties, &config.into())? {
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
                let property_type =
                    apply_property_reference(&inner_properties, property_type, platform_version)?;
                document_properties.insert(
                    prefixed_property_key,
                    DocumentProperty {
                        property_type,
                        required: is_required,
                        transient: is_transient,
                        required_since,
                    },
                );
            }
        };
    }

    Ok(())
}

// TODO: This is quite big
#[allow(clippy::too_many_arguments)]
fn insert_values_nested(
    document_properties: &mut IndexMap<String, DocumentProperty>,
    known_required: &BTreeSet<String>,
    known_transient: &BTreeSet<String>,
    is_top_level: bool,
    property_key: String,
    property_value: &Value,
    root_schema: &Value,
    config: &DataContractConfig,
    platform_version: &PlatformVersion,
) -> Result<(), DataContractError> {
    let mut inner_properties = property_value.to_btree_ref_string_map()?;

    if let Some(schema_ref) = inner_properties.get_optional_str(property_names::REF)? {
        let referenced_sub_schema = resolve_uri(root_schema, schema_ref)?;

        inner_properties = referenced_sub_schema.to_btree_ref_string_map()?;
    }

    let is_required = known_required.contains(&property_key);

    let is_transient = known_transient.contains(&property_key);

    let required_since = apply_required_since(
        &inner_properties,
        is_required,
        is_top_level,
        platform_version,
    )?;

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

                    // Nested properties are emitted below in source-map order (the
                    // `properties.iter()` loop), and that `IndexMap` insertion order is
                    // consensus-observable: historical contracts were committed in source-map
                    // order, so re-sorting nested properties by `position` would soft-fork any
                    // contract whose source order differs from its position order. A previous
                    // `position`-based `sort_by` here was dead code (its sorted result was never
                    // read) and read `position` with `.expect()`, which could panic on adversarial
                    // schema input during block execution. Removed (ordering unchanged). Do NOT
                    // reintroduce a nested-property sort — even a correct one — nor a panicking
                    // `position` read here.

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
                            false,
                            object_property_string,
                            object_property_value,
                            root_schema,
                            config,
                            platform_version,
                        )?;
                    }
                }

                DocumentPropertyType::Object(nested_properties)
            }
            property_type => property_type,
        };

    let property_type =
        apply_property_reference(&inner_properties, property_type, platform_version)?;

    document_properties.insert(
        property_key,
        DocumentProperty {
            property_type,
            required: is_required,
            transient: is_transient,
            required_since,
        },
    );

    Ok(())
}

/// Parses the `requiredSince` keyword: the contract version from which the
/// property is required. Only meaningful on top-level required properties —
/// the document wire format encodes a required property without a presence
/// flag, so requiredness that varies by contract version must be resolvable
/// per property from the current schema alone (see the per-document contract
/// version stamp in document serialization format 3).
///
/// Versioned on `apply_required_since` in the platform version's document
/// type schema versions. `None` selects the behavior of the versions that
/// predate the keyword: it is ignored entirely, so their parses stay
/// byte-for-byte identical to what they always produced.
fn apply_required_since(
    inner_properties: &BTreeMap<String, &Value>,
    is_required: bool,
    is_top_level: bool,
    platform_version: &PlatformVersion,
) -> Result<Option<u32>, DataContractError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .apply_required_since
    {
        None => Ok(None),
        Some(0) => apply_required_since_v0(inner_properties, is_required, is_top_level),
        Some(version) => Err(DataContractError::Unsupported(format!(
            "apply_required_since version {version} is not supported"
        ))),
    }
}

fn apply_required_since_v0(
    inner_properties: &BTreeMap<String, &Value>,
    is_required: bool,
    is_top_level: bool,
) -> Result<Option<u32>, DataContractError> {
    let Some(required_since_value) = inner_properties.get(property_names::REQUIRED_SINCE) else {
        return Ok(None);
    };

    if !is_top_level {
        return Err(DataContractError::InvalidContractStructure(
            "requiredSince is only allowed on top-level properties".to_string(),
        ));
    }

    if !is_required {
        return Err(DataContractError::InvalidContractStructure(
            "requiredSince is only allowed on properties listed in required".to_string(),
        ));
    }

    let required_since: u32 = required_since_value
        .to_integer()
        .map_err(|e| DataContractError::ValueWrongType(e.to_string()))?;

    if required_since == 0 {
        return Err(DataContractError::InvalidContractStructure(
            "requiredSince must be a contract version of at least 1".to_string(),
        ));
    }

    Ok(Some(required_since))
}

/// Folds a `refersTo` declaration into the property type: an identifier property
/// with `refersTo` becomes `IdentifierWithReference(target)`. Non-identifier
/// properties cannot carry `refersTo`.
///
/// Versioned on `apply_property_reference` in the platform version's document
/// type schema versions. `None` selects the behavior of the versions that
/// predate the keyword: it is ignored entirely, so their parses stay
/// byte-for-byte identical to what they always produced.
fn apply_property_reference(
    inner_properties: &BTreeMap<String, &Value>,
    property_type: DocumentPropertyType,
    platform_version: &PlatformVersion,
) -> Result<DocumentPropertyType, DataContractError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .apply_property_reference
    {
        None => Ok(property_type),
        Some(0) => apply_property_reference_v0(inner_properties, property_type),
        Some(version) => Err(DataContractError::Unsupported(format!(
            "apply_property_reference version {version} is not supported"
        ))),
    }
}

fn apply_property_reference_v0(
    inner_properties: &BTreeMap<String, &Value>,
    property_type: DocumentPropertyType,
) -> Result<DocumentPropertyType, DataContractError> {
    let Some(refers_to_value) = inner_properties.get(property_names::REFERS_TO) else {
        return Ok(property_type);
    };

    if !matches!(
        property_type,
        DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_)
    ) {
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
        "token" => DocumentPropertyReferenceTarget::Token,
        "permanentDocument" => {
            // An absent contractId means the reference targets a document
            // type of the declaring contract itself
            let contract_id = refers_to_map
                .get(property_names::CONTRACT_ID)
                .map(|value| {
                    value
                        .to_identifier()
                        .map_err(|e| DataContractError::ValueWrongType(e.to_string()))
                })
                .transpose()?;

            let document_type_name = refers_to_map
                .get_str(property_names::DOCUMENT_TYPE)
                .map_err(|e| DataContractError::ValueWrongType(e.to_string()))?;

            if document_type_name.is_empty() || document_type_name.len() > 64 {
                return Err(DataContractError::InvalidContractStructure(
                    "permanentDocument refersTo documentType must be between 1 and 64 characters"
                        .to_string(),
                ));
            }

            DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id,
                document_type_name: document_type_name.to_string(),
            }
        }
        "identityPublicKey" => {
            let key_id_property = refers_to_map
                .get_str(property_names::KEY_ID_PROPERTY)
                .map_err(|e| DataContractError::ValueWrongType(e.to_string()))?;

            if key_id_property.is_empty() || key_id_property.len() > 256 {
                return Err(DataContractError::InvalidContractStructure(
                    "identityPublicKey refersTo keyIdProperty must be between 1 and 256 characters"
                        .to_string(),
                ));
            }

            DocumentPropertyReferenceTarget::IdentityPublicKey {
                key_id_property: key_id_property.to_string(),
            }
        }
        other => {
            return Err(DataContractError::InvalidContractStructure(format!(
                "invalid refersTo type {other}"
            )))
        }
    };

    Ok(DocumentPropertyType::IdentifierWithReference(target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use platform_value::string_encoding::Encoding;
    use serde_json::json;

    fn try_document_type_from_schema(
        schema: serde_json::Value,
    ) -> Result<DocumentType, ProtocolError> {
        try_document_type_from_schema_on_version(schema, PlatformVersion::latest())
    }

    fn try_document_type_from_schema_on_version(
        schema: serde_json::Value,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentType, ProtocolError> {
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");

        let value = platform_value::to_value(schema).expect("schema should convert");

        DocumentType::try_from_schema(
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
    }

    #[test]
    fn should_parse_refers_to_on_identifier_property() {
        let document_type = try_document_type_from_schema(json!({
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
        }))
        .expect("should parse");

        let property_type = document_type
            .as_ref()
            .flattened_properties()
            .get("toUserId")
            .map(|p| p.property_type.clone())
            .expect("property should be present");

        assert!(matches!(
            property_type,
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::Identity
            )
        ));
    }

    #[test]
    fn should_reject_refers_to_on_non_identifier_property() {
        let err = try_document_type_from_schema(json!({
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
        }))
        .expect_err("should fail");

        let message = err.to_string();
        assert!(
            message.contains("refersTo is only allowed on identifier properties"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn should_parse_permanent_document_refers_to() {
        let contract_id = Identifier::from([7u8; 32]);

        let document_type = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "parentNoteId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "permanentDocument",
                        "contractId": contract_id.to_string(Encoding::Base58),
                        "documentType": "note"
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect("should parse");

        let property_type = document_type
            .as_ref()
            .flattened_properties()
            .get("parentNoteId")
            .map(|p| p.property_type.clone())
            .expect("property should be present");

        assert_eq!(
            property_type,
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::PermanentDocument {
                    contract_id: Some(contract_id),
                    document_type_name: "note".to_string(),
                }
            )
        );
    }

    #[test]
    fn should_parse_permanent_document_refers_to_without_contract_id_as_own_contract() {
        let document_type = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "parentNoteId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "permanentDocument",
                        "documentType": "note"
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect("should parse");

        let property_type = document_type
            .as_ref()
            .flattened_properties()
            .get("parentNoteId")
            .map(|p| p.property_type.clone())
            .expect("property should be present");

        assert_eq!(
            property_type,
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::PermanentDocument {
                    contract_id: None,
                    document_type_name: "note".to_string(),
                }
            )
        );
    }

    #[test]
    fn should_reject_permanent_document_refers_to_with_invalid_contract_id() {
        let err = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "parentNoteId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "permanentDocument",
                        "contractId": "not-a-valid-identifier",
                        "documentType": "note"
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");

        let message = err.to_string();
        assert!(message.contains("base 58"), "unexpected error: {message}");
    }

    #[test]
    fn should_reject_permanent_document_refers_to_with_oversized_document_type_name() {
        let err = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "parentNoteId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "permanentDocument",
                        "contractId": Identifier::from([7u8; 32]).to_string(Encoding::Base58),
                        "documentType": "a".repeat(65)
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");

        let message = err.to_string();
        assert!(
            message.contains("between 1 and 64 characters"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn should_reject_permanent_document_refers_to_without_document_type() {
        try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "parentNoteId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "permanentDocument",
                        "contractId": Identifier::from([7u8; 32]).to_string(Encoding::Base58)
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");
    }

    #[test]
    fn should_parse_identity_public_key_refers_to() {
        let document_type = try_document_type_from_schema(json!({
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
                        "type": "identityPublicKey",
                        "keyIdProperty": "toKeyIndex"
                    }
                },
                "toKeyIndex": {
                    "type": "integer",
                    "position": 1
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect("should parse");

        let property_type = document_type
            .as_ref()
            .flattened_properties()
            .get("toUserId")
            .map(|p| p.property_type.clone())
            .expect("property should be present");

        assert_eq!(
            property_type,
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::IdentityPublicKey {
                    key_id_property: "toKeyIndex".to_string(),
                }
            )
        );
    }

    #[test]
    fn should_reject_identity_public_key_refers_to_without_key_id_property() {
        try_document_type_from_schema(json!({
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
                        "type": "identityPublicKey"
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");
    }

    #[test]
    fn should_ignore_refers_to_on_platform_versions_predating_it() {
        // Platform versions whose tables carry `apply_property_reference: None`
        // predate the `refersTo` keyword: even if it appears in a schema they
        // parse (only possible without full validation — their meta-schemas
        // reject it), they must ignore it and keep producing the plain
        // identifier type they always produced.
        let platform_version = PlatformVersion::get(13).expect("platform version 13 should exist");

        let document_type = try_document_type_from_schema_on_version(
            json!({
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
            }),
            platform_version,
        )
        .expect("should parse");

        let property_type = document_type
            .as_ref()
            .flattened_properties()
            .get("toUserId")
            .map(|p| p.property_type.clone())
            .expect("property should be present");

        assert!(matches!(property_type, DocumentPropertyType::Identifier));
    }

    #[test]
    fn should_not_reject_refers_to_on_non_identifier_property_on_platform_versions_predating_it() {
        let platform_version = PlatformVersion::get(13).expect("platform version 13 should exist");

        try_document_type_from_schema_on_version(
            json!({
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
            }),
            platform_version,
        )
        .expect("a parse predating refersTo should ignore the keyword entirely");
    }

    // ================================================================
    //  requiredSince
    // ================================================================

    #[test]
    fn should_parse_required_since_on_top_level_required_property() {
        let document_type = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60},
                "b": {"type": "string", "position": 1, "maxLength": 60, "requiredSince": 3},
            },
            "required": ["a", "b"],
            "additionalProperties": false
        }))
        .expect("should parse");

        let properties = document_type.as_ref().flattened_properties().clone();
        assert_eq!(properties.get("a").unwrap().required_since, None);
        assert_eq!(properties.get("b").unwrap().required_since, Some(3));
        assert!(properties.get("b").unwrap().required);
    }

    #[test]
    fn should_reject_required_since_on_optional_property() {
        let result = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60, "requiredSince": 2},
            },
            "required": [],
            "additionalProperties": false
        }));

        assert!(
            result.is_err(),
            "requiredSince on a property not listed in required must be rejected"
        );
    }

    #[test]
    fn should_reject_required_since_on_nested_property() {
        let result = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "outer": {
                    "type": "object",
                    "position": 0,
                    "properties": {
                        "inner": {"type": "string", "position": 0, "maxLength": 60, "requiredSince": 2},
                    },
                    "required": ["inner"],
                    "additionalProperties": false
                },
            },
            "required": [],
            "additionalProperties": false
        }));

        assert!(
            result.is_err(),
            "requiredSince on a nested property must be rejected"
        );
    }

    #[test]
    fn should_reject_required_since_of_zero() {
        let result = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60, "requiredSince": 0},
            },
            "required": ["a"],
            "additionalProperties": false
        }));

        assert!(
            result.is_err(),
            "requiredSince of 0 must be rejected (contract versions start at 1)"
        );
    }

    #[test]
    fn should_ignore_required_since_on_platform_versions_predating_it() {
        // Platform versions whose tables carry `apply_required_since: None`
        // predate the keyword: even if it appears in a schema they parse
        // (only possible without full validation — their meta-schemas reject
        // it), they must ignore it and keep producing the plain required
        // property they always produced.
        let platform_version = PlatformVersion::get(13).expect("platform version 13 should exist");

        let document_type = try_document_type_from_schema_on_version(
            json!({
                "type": "object",
                "properties": {
                    "a": {"type": "string", "position": 0, "maxLength": 60, "requiredSince": 3},
                },
                "required": ["a"],
                "additionalProperties": false
            }),
            platform_version,
        )
        .expect("a parse predating requiredSince should ignore the keyword entirely");

        let properties = document_type.as_ref().flattened_properties().clone();
        assert_eq!(properties.get("a").unwrap().required_since, None);
        assert!(properties.get("a").unwrap().required);
    }
}
