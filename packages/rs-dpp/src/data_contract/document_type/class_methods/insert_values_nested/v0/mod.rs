use crate::data_contract::document_type::property::array::ArrayItemType;
use crate::data_contract::document_type::property::{DocumentProperty, DocumentPropertyType};
use crate::data_contract::document_type::{property_names, DocumentType};
use crate::data_contract::errors::{DataContractError, StructureError};
use crate::util::json_schema::resolve_uri;
use crate::ProtocolError;
use platform_value::btreemap_extensions::{BTreeValueMapHelper, BTreeValueRemoveFromMapHelper};
use platform_value::{Value, ValueMapHelper};
use std::collections::{BTreeMap, BTreeSet};

impl DocumentType {
    pub(super) fn insert_values_nested_v0(
        document_properties: &mut BTreeMap<String, DocumentProperty>,
        known_required: &BTreeSet<String>,
        property_key: String,
        property_value: &Value,
        root_schema: &Value,
    ) -> Result<(), ProtocolError> {
        let mut inner_properties = property_value.to_btree_ref_string_map()?;

        let type_value = inner_properties
            .remove_optional_string(property_names::TYPE)
            .map_err(ProtocolError::ValueError)?;
        let type_value = match type_value {
            None => {
                let ref_value = inner_properties
                    .get_str(property_names::REF)
                    .map_err(ProtocolError::ValueError)?;

                let referenced_sub_schema = resolve_uri(root_schema, ref_value)?.to_map()?;

                referenced_sub_schema
                    .get_key(property_names::TYPE)?
                    .to_text()?
            }
            Some(type_value) => type_value,
        };
        let is_required = known_required.contains(&property_key);
        let field_type: DocumentPropertyType;

        match type_value.as_str() {
            "integer" => {
                field_type = DocumentPropertyType::Integer;
            }
            "number" => {
                field_type = DocumentPropertyType::Number;
            }
            "string" => {
                field_type = DocumentPropertyType::String(
                    inner_properties.get_optional_integer(property_names::MIN_LENGTH)?,
                    inner_properties.get_optional_integer(property_names::MAX_LENGTH)?,
                );
            }
            "array" => {
                // Only handling bytearrays for v1
                // Return an error if it is not a byte array
                field_type = match inner_properties.get_optional_bool(property_names::BYTE_ARRAY)? {
                    Some(inner_bool) => {
                        if inner_bool {
                            match inner_properties
                                .get_optional_str(property_names::CONTENT_MEDIA_TYPE)?
                            {
                                Some(content_media_type)
                                    if content_media_type
                                        == "application/x.dash.dpp.identifier" =>
                                {
                                    DocumentPropertyType::Identifier
                                }
                                Some(_) | None => DocumentPropertyType::ByteArray(
                                    inner_properties
                                        .get_optional_integer(property_names::MIN_ITEMS)?,
                                    inner_properties
                                        .get_optional_integer(property_names::MAX_ITEMS)?,
                                ),
                            }
                        } else {
                            return Err(ProtocolError::DataContractError(
                                DataContractError::InvalidContractStructure(
                                    "byteArray should always be true if defined".to_string(),
                                ),
                            ));
                        }
                    }
                    // TODO: Contract indices and new encoding format don't support arrays
                    //   but we still can use them as document fields with current cbor encoding
                    //   This is a temporary workaround to bring back v0.22 behavior and should be
                    //   replaced with a proper array support in future versions
                    None => DocumentPropertyType::Array(ArrayItemType::Boolean),
                };
            }
            "object" => {
                let mut nested_properties = BTreeMap::new();
                if let Some(properties_as_value) = inner_properties.get(property_names::PROPERTIES)
                {
                    let properties =
                        properties_as_value
                            .as_map()
                            .ok_or(ProtocolError::StructureError(
                                StructureError::ValueWrongType("properties must be a map"),
                            ))?;

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

                    for (object_property_key, object_property_value) in properties.iter() {
                        let object_property_string = object_property_key
                            .as_text()
                            .ok_or(ProtocolError::StructureError(StructureError::KeyWrongType(
                                "property key must be a string",
                            )))?
                            .to_string();

                        Self::insert_values_nested_v0(
                            &mut nested_properties,
                            &stripped_required,
                            object_property_string,
                            object_property_value,
                            root_schema,
                        )?;
                    }
                }
                field_type = DocumentPropertyType::Object(nested_properties);
                document_properties.insert(
                    property_key,
                    DocumentProperty {
                        r#type: field_type,
                        required: is_required,
                    },
                );
                return Ok(());
            }
            _ => {
                field_type = DocumentPropertyType::try_from_name(type_value.as_str())?;
            }
        }

        document_properties.insert(
            property_key,
            DocumentProperty {
                r#type: field_type,
                required: is_required,
            },
        );

        Ok(())
    }
}
