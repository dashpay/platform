use crate::data_contract::document_type::array_field::ArrayFieldType;
use crate::data_contract::document_type::document_field::{DocumentField, DocumentFieldType};
use crate::data_contract::document_type::{property_names, DocumentType};
use crate::data_contract::errors::{DataContractError, StructureError};
use crate::ProtocolError;
use platform_value::btreemap_extensions::{BTreeValueMapHelper, BTreeValueRemoveFromMapHelper};
use platform_value::Value;
use std::collections::{BTreeMap, BTreeSet};

impl DocumentType {
    pub(super) fn insert_values_v0(
        document_properties: &mut BTreeMap<String, DocumentField>,
        known_required: &BTreeSet<String>,
        prefix: Option<String>,
        property_key: String,
        property_value: &Value,
        schema_defs: &Option<BTreeMap<String, Value>>,
    ) -> Result<(), ProtocolError> {
        let mut to_visit: Vec<(Option<String>, String, &Value)> =
            vec![(prefix, property_key, property_value)];

        while let Some((prefix, property_key, property_value)) = to_visit.pop() {
            let prefixed_property_key = match prefix {
                None => property_key,
                Some(prefix) => [prefix, property_key].join(".").to_owned(),
            };

            let mut inner_properties = property_value.to_btree_ref_string_map()?;

            let type_value = inner_properties
                .remove_optional_string(property_names::TYPE)
                .map_err(ProtocolError::ValueError)?;

            let type_value = match type_value {
                None => {
                    let ref_value = inner_properties
                        .get_str(property_names::REF)
                        .map_err(ProtocolError::ValueError)?;

                    // TODO We can reference to another part of the document
                    //  it must be handled
                    let Some(ref_value) = ref_value.strip_prefix("#/$defs/") else {
                        return Err(ProtocolError::DataContractError(
                            DataContractError::InvalidContractStructure("malformed reference".to_string()),
                        ));
                    };

                    let Some(defs) = schema_defs else {
                        return Err(ProtocolError::DataContractError(
                            DataContractError::InvalidContractStructure(format!("expected schema definitions with path {ref_value}")),
                        ));
                    };

                    inner_properties = defs
                        .get_inner_borrowed_str_value_map(ref_value)
                        .map_err(ProtocolError::ValueError)?;

                    inner_properties.get_string(property_names::TYPE)?
                }
                Some(type_value) => type_value,
            };
            let is_required = known_required.contains(&prefixed_property_key);
            let field_type: DocumentFieldType;

            match type_value.as_str() {
                "array" => {
                    // Only handling bytearrays for v1
                    // Return an error if it is not a byte array
                    field_type = match inner_properties
                        .get_optional_bool(property_names::BYTE_ARRAY)?
                    {
                        Some(inner_bool) => {
                            if inner_bool {
                                match inner_properties
                                    .get_optional_str(property_names::CONTENT_MEDIA_TYPE)?
                                {
                                    Some(content_media_type)
                                        if content_media_type
                                            == "application/x.dash.dpp.identifier" =>
                                    {
                                        DocumentFieldType::Identifier
                                    }
                                    Some(_) | None => DocumentFieldType::ByteArray(
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
                        None => DocumentFieldType::Array(ArrayFieldType::Boolean),
                    };

                    document_properties.insert(
                        prefixed_property_key,
                        DocumentField {
                            document_type: field_type,
                            required: is_required,
                        },
                    );
                }
                "object" => {
                    if let Some(properties_as_value) =
                        inner_properties.get(property_names::PROPERTIES)
                    {
                        let properties =
                            properties_as_value
                                .as_map()
                                .ok_or(ProtocolError::StructureError(
                                    StructureError::ValueWrongType("properties must be a map"),
                                ))?;

                        for (object_property_key, object_property_value) in properties.iter() {
                            let object_property_string = object_property_key
                                .as_text()
                                .ok_or(ProtocolError::StructureError(
                                    StructureError::KeyWrongType("property key must be a string"),
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

                "string" => {
                    field_type = DocumentFieldType::String(
                        inner_properties.get_optional_integer(property_names::MIN_LENGTH)?,
                        inner_properties.get_optional_integer(property_names::MAX_LENGTH)?,
                    );
                    document_properties.insert(
                        prefixed_property_key,
                        DocumentField {
                            document_type: field_type,
                            required: is_required,
                        },
                    );
                }

                _ => {
                    field_type = Self::string_to_field_type(type_value.as_str())
                        .ok_or(DataContractError::ValueWrongType("invalid type"))?;
                    document_properties.insert(
                        prefixed_property_key,
                        DocumentField {
                            document_type: field_type,
                            required: is_required,
                        },
                    );
                }
            }
        }

        Ok(())
    }
}
