use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::class_methods::apply_required_since::apply_required_since;
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

            let property_agreement = match refers_to_map.get(property_names::PROPERTY_AGREEMENT) {
                None => BTreeMap::new(),
                Some(agreement_value) => {
                    let agreement_map = agreement_value.to_btree_ref_string_map()?;
                    if agreement_map.is_empty() || agreement_map.len() > 10 {
                        return Err(DataContractError::InvalidContractStructure(
                            "permanentDocument refersTo propertyAgreement must declare \
                             between 1 and 10 property pairs"
                                .to_string(),
                        ));
                    }
                    agreement_map
                        .iter()
                        .map(|(referring_property, referenced_value)| {
                            let referenced_property =
                                referenced_value.as_text().ok_or_else(|| {
                                    DataContractError::InvalidContractStructure(
                                        "propertyAgreement values must be referenced \
                                         property paths (strings)"
                                            .to_string(),
                                    )
                                })?;
                            for path in [referring_property.as_str(), referenced_property] {
                                if path.is_empty() || path.len() > 256 {
                                    return Err(DataContractError::InvalidContractStructure(
                                        "propertyAgreement property paths must be between 1 \
                                         and 256 characters"
                                            .to_string(),
                                    ));
                                }
                            }
                            Ok((referring_property.clone(), referenced_property.to_string()))
                        })
                        .collect::<Result<BTreeMap<String, String>, DataContractError>>()?
                }
            };

            DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id,
                document_type_name: document_type_name.to_string(),
                property_agreement,
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

    // `propertyAgreement` compares against a referenced DOCUMENT's values —
    // no other target kind has a document body to agree with.
    if refers_to_map.contains_key(property_names::PROPERTY_AGREEMENT)
        && !matches!(
            target,
            DocumentPropertyReferenceTarget::PermanentDocument { .. }
        )
    {
        return Err(DataContractError::InvalidContractStructure(
            "propertyAgreement is only allowed on permanentDocument references".to_string(),
        ));
    }

    Ok(DocumentPropertyType::IdentifierWithReference(target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use crate::data_contract::document_type::validate_required_since_within_contract_version;
    use platform_value::string_encoding::Encoding;
    use serde_json::json;

    fn try_document_type_from_schema(
        schema: serde_json::Value,
    ) -> Result<DocumentType, ProtocolError> {
        try_document_type_from_schema_on_version(schema, PlatformVersion::latest())
    }

    /// Same as [`try_document_type_from_schema`] but with `full_validation`
    /// on — the index validations (the timeRange source rules among them)
    /// only run on the validating parse.
    fn try_document_type_from_schema_full_validation(
        schema: serde_json::Value,
    ) -> Result<DocumentType, ProtocolError> {
        let platform_version = PlatformVersion::latest();
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
            true,
            &mut vec![],
            platform_version,
        )
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
    fn should_parse_property_agreement_on_permanent_document_refers_to() {
        let document_type = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "hashtag": { "type": "string", "position": 0, "maxLength": 63 },
                "postId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 1,
                    "refersTo": {
                        "type": "permanentDocument",
                        "documentType": "post",
                        "propertyAgreement": { "hashtag": "hashtag" }
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
            .get("postId")
            .map(|p| p.property_type.clone())
            .expect("property should be present");

        let DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::PermanentDocument {
                property_agreement, ..
            },
        ) = property_type
        else {
            panic!("expected a permanentDocument reference");
        };
        assert_eq!(
            property_agreement,
            BTreeMap::from([("hashtag".to_string(), "hashtag".to_string())])
        );
    }

    #[test]
    fn should_reject_property_agreement_on_non_document_reference() {
        let err = try_document_type_from_schema(json!({
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
                        "propertyAgreement": { "hashtag": "hashtag" }
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");

        let message = err.to_string();
        assert!(
            message.contains("propertyAgreement is only allowed on permanentDocument"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn should_reject_empty_property_agreement() {
        let err = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "postId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "permanentDocument",
                        "documentType": "post",
                        "propertyAgreement": {}
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");

        let message = err.to_string();
        assert!(
            message.contains("between 1 and 10 property pairs"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn should_reject_non_string_property_agreement_values() {
        let err = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "postId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "permanentDocument",
                        "documentType": "post",
                        "propertyAgreement": { "hashtag": 7 }
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");

        let message = err.to_string();
        assert!(
            message.contains("must be referenced property paths"),
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
                    property_agreement: Default::default(),
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
                    property_agreement: Default::default(),
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
    fn should_reject_required_since_above_u32_max() {
        // The meta-schema caps the value at u32::MAX too; this pins the
        // parser-side rejection so it does not depend on meta-schema
        // coverage (parses without full validation skip the meta-schema)
        let result = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60, "requiredSince": 4_294_967_296_u64},
            },
            "required": ["a"],
            "additionalProperties": false
        }));

        assert!(
            result.is_err(),
            "requiredSince above u32::MAX must be rejected"
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
    fn should_parse_required_since_reached_through_a_ref() {
        // A `$ref`'d property resolves to its `$defs` entry before keywords
        // are read, so an annotation hidden behind a reference is parsed
        // exactly like a direct one — any validation that only scans raw
        // property JSON would miss it, which is why the
        // `requiredSince <= contract version` invariant is enforced on
        // parsed properties (validate_required_since_within_contract_version)
        let platform_version = PlatformVersion::latest();
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");

        let schema_defs: BTreeMap<String, Value> = [(
            "annotated".to_string(),
            platform_value::to_value(json!({
                "type": "string", "maxLength": 60, "requiredSince": 2
            }))
            .expect("defs should convert"),
        )]
        .into_iter()
        .collect();

        let schema = platform_value::to_value(json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60},
                "b": {"$ref": "#/$defs/annotated", "position": 1},
            },
            "required": ["a", "b"],
            "additionalProperties": false
        }))
        .expect("schema should convert");

        let document_type = DocumentType::try_from_schema(
            Identifier::random(),
            0,
            config.version(),
            "msg",
            schema,
            Some(&schema_defs),
            &BTreeMap::new(),
            &config,
            false,
            &mut vec![],
            platform_version,
        )
        .expect("should parse");

        let properties = document_type.as_ref().flattened_properties().clone();
        assert_eq!(properties.get("b").unwrap().required_since, Some(2));

        // The parsed-property invariant check sees the annotation the raw
        // JSON hides: version 1 (too old for requiredSince 2) rejects,
        // version 2 accepts
        let mut document_types = BTreeMap::new();
        document_types.insert("msg".to_string(), document_type);

        assert!(
            validate_required_since_within_contract_version(&document_types, 1).is_err(),
            "requiredSince 2 must be rejected on a version 1 contract even through $ref"
        );
        assert!(validate_required_since_within_contract_version(&document_types, 2).is_ok());
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
    #[test]
    fn should_reject_time_range_on_user_defined_property() {
        // No user property type parses to a millisecond timestamp — `type:
        // "string"` with `format: "date-time"` stays `String` — so a
        // user-defined time-range source must be rejected rather than
        // accepted as an index that could never bucket anything meaningful.
        let err = try_document_type_from_schema_full_validation(json!({
            "type": "object",
            "properties": {
                "eventAt": {
                    "type": "string",
                    "maxLength": 63,
                    "position": 0
                }
            },
            "indices": [
                {
                    "name": "byEventTime",
                    "properties": [{ "eventAt": "asc" }],
                    "timeRange": { "on": "eventAt", "range": 21_600u64, "step": 7_200u64 }
                }
            ],
            "required": ["eventAt"],
            "additionalProperties": false
        }))
        .expect_err("a user-defined time-range source must be rejected");

        assert!(
            err.to_string().contains("system timestamps"),
            "expected the system-timestamp restriction, got: {err}"
        );
    }

    #[test]
    fn should_parse_time_range_on_required_system_timestamp() {
        try_document_type_from_schema_full_validation(json!({
            "type": "object",
            "properties": {
                "hashtag": {
                    "type": "string",
                    "maxLength": 63,
                    "position": 0
                }
            },
            "indices": [
                {
                    "name": "trending",
                    "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }],
                    "timeRange": { "on": "$createdAt", "range": 21_600u64, "step": 7_200u64 }
                }
            ],
            "required": ["$createdAt", "hashtag"],
            "additionalProperties": false
        }))
        .expect("a required system timestamp is the supported time-range source");
    }

    /// The overlap-factor cap is a versioned system limit
    /// (`SystemLimits::max_time_range_overlap_factor`), enforced at
    /// registration rather than at parse; both sides of the boundary are
    /// pinned here through the versioned dispatch. 24 is a day-long window
    /// sliding hourly — the natural worst case the cap is sized for.
    #[test]
    fn should_enforce_the_versioned_time_range_overlap_factor_cap() {
        let time_range_schema = |range_seconds: u64| {
            json!({
                "type": "object",
                "properties": {
                    "hashtag": {
                        "type": "string",
                        "maxLength": 63,
                        "position": 0
                    }
                },
                "indices": [
                    {
                        "name": "trending",
                        "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }],
                        "timeRange": { "on": "$createdAt", "range": range_seconds, "step": 3_600u64 }
                    }
                ],
                "required": ["$createdAt", "hashtag"],
                "additionalProperties": false
            })
        };

        try_document_type_from_schema_full_validation(time_range_schema(24 * 3_600))
            .expect("an overlap factor at the cap must register");

        let err = try_document_type_from_schema_full_validation(time_range_schema(25 * 3_600))
            .expect_err("an overlap factor over the cap must be rejected at registration");
        assert!(
            err.to_string().contains("overlap factor"),
            "expected the overlap-factor rejection, got: {err}"
        );
    }

    #[test]
    fn should_parse_unique_time_range_index_with_non_overlapping_windows_on_created_at() {
        // "one report per author per day": `range == step` makes the buckets a
        // partition, and `$createdAt` is immutable, which is exactly the pair
        // of conditions a unique bucketed index needs. Asserted through the
        // full-validation parse so the doctype-level checks (unique-index
        // limit, required system timestamp) run too.
        const ONE_DAY_SECONDS: u64 = 24 * 3_600;
        let document_type = try_document_type_from_schema_full_validation(json!({
            "type": "object",
            "properties": {
                "author": {
                    "type": "string",
                    "maxLength": 63,
                    "position": 0
                }
            },
            "indices": [
                {
                    "name": "dailyReport",
                    "properties": [{ "$createdAt": "asc" }, { "author": "asc" }],
                    "unique": true,
                    "timeRange": { "on": "$createdAt", "range": ONE_DAY_SECONDS, "step": ONE_DAY_SECONDS }
                }
            ],
            "required": ["$createdAt", "author"],
            "additionalProperties": false
        }))
        .expect("a non-overlapping $createdAt bucketing may be unique");

        let index = document_type
            .as_ref()
            .indexes()
            .get("dailyReport")
            .expect("the index should be registered")
            .clone();
        assert!(index.unique);
        assert_eq!(
            index
                .time_range
                .expect("the transform should survive the schema parse")
                .overlap_factor(),
            1
        );
    }
}
