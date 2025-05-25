use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::v1::DocumentTypeV1;
use crate::data_contract::document_type::{
    property_names, DocumentProperty, DocumentPropertyType, DocumentType,
};
use crate::data_contract::errors::DataContractError;
use crate::data_contract::{TokenConfiguration, TokenContractPosition};
use crate::util::json_schema::resolve_uri;
use crate::validation::operations::ProtocolValidationOperation;
use crate::errors::ProtocolError;
use indexmap::IndexMap;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, BTreeSet};

mod v0;
mod v1;

const NOT_ALLOWED_SYSTEM_PROPERTIES: [&str; 1] = ["$id"];
const SYSTEM_PROPERTIES: [&str; 11] = [
    "$id",
    "$ownerId",
    "$createdAt",
    "$updatedAt",
    "$transferredAt",
    "$createdAtBlockHeight",
    "$updatedAtBlockHeight",
    "$transferredAtBlockHeight",
    "$createdAtCoreBlockHeight",
    "$updatedAtCoreBlockHeight",
    "$transferredAtCoreBlockHeight",
];
const MAX_INDEXED_STRING_PROPERTY_LENGTH: u16 = 63;
const MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH: u16 = 255;
const MAX_INDEXED_ARRAY_ITEMS: usize = 1024;

impl DocumentType {
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_schema(
        data_contract_id: Identifier,
        name: &str,
        schema: Value,
        schema_defs: Option<&BTreeMap<String, Value>>,
        token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        data_contact_config: &DataContractConfig,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
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
                document_properties.insert(
                    prefixed_property_key,
                    DocumentProperty {
                        property_type,
                        required: is_required,
                        transient: is_transient,
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

    document_properties.insert(
        property_key,
        DocumentProperty {
            property_type,
            required: is_required,
            transient: is_transient,
        },
    );

    Ok(())
}
