use std::collections::{BTreeMap, BTreeSet};
use ciborium::value::Value;
use crate::common::{btree_map_inner_bool_value, btree_map_inner_btree_map, btree_map_inner_map_value, btree_map_inner_size_value, btree_map_inner_text_value, bytes_for_system_value, cbor_inner_array_of_strings, cbor_inner_array_value, cbor_inner_bool_value_with_default, cbor_inner_btree_map, cbor_inner_text_value, cbor_map_to_btree_map};
use crate::contract::types::{DocumentField, DocumentFieldType};
use crate::drive::defaults::{DEFAULT_HASH_SIZE, MAX_INDEX_SIZE};
use crate::error::contract::ContractError;
use crate::error::Error;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use crate::contract::document::Document;
use crate::contract::types;
use crate::error::drive::DriveError;

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct DocumentType {
    pub name: String,
    pub indices: Vec<Index>,
    pub properties: BTreeMap<String, DocumentField>,
    pub required_fields: BTreeSet<String>,
    pub documents_keep_history: bool,
    pub documents_mutable: bool,
}


impl DocumentType {
    // index_names can be in any order
    // in field name must be in the last two indexes.
    pub fn index_for_types(
        &self,
        index_names: &[&str],
        in_field_name: Option<&str>,
        order_by: &[&str],
    ) -> Option<(&Index, u16)> {
        let mut best_index: Option<(&Index, u16)> = None;
        let mut best_difference = u16::MAX;
        for index in self.indices.iter() {
            let difference_option = index.matches(index_names, in_field_name, order_by);
            if let Some(difference) = difference_option {
                if difference == 0 {
                    return Some((index, 0));
                } else if difference < best_difference {
                    best_difference = difference;
                    best_index = Some((index, best_difference));
                }
            }
        }
        best_index
    }

    pub fn serialize_value_for_key<'a>(
        &'a self,
        key: &str,
        value: &Value,
    ) -> Result<Vec<u8>, Error> {
        match key {
            "$ownerId" | "$id" => {
                let bytes = bytes_for_system_value(value)?.ok_or({
                    Error::Contract(ContractError::FieldRequirementUnmet(
                        "expected system value to be deserialized",
                    ))
                })?;
                if bytes.len() != DEFAULT_HASH_SIZE {
                    Err(Error::Contract(ContractError::FieldRequirementUnmet(
                        "expected system value to be 32 bytes long",
                    )))
                } else {
                    Ok(bytes)
                }
            }
            _ => {
                let field_type = self.properties.get(key).ok_or({
                    Error::Contract(ContractError::DocumentTypeFieldNotFound(
                        "expected contract to have field",
                    ))
                })?;
                let bytes = field_type.document_type.encode_value_for_tree_keys(value)?;
                if bytes.len() > MAX_INDEX_SIZE {
                    Err(Error::Contract(ContractError::FieldRequirementUnmet(
                        "value must be less than 256 bytes long",
                    )))
                } else {
                    Ok(bytes)
                }
            }
        }
    }

    pub fn from_cbor_value(
        name: &str,
        document_type_value_map: &[(Value, Value)],
        definition_references: &BTreeMap<String, &Value>,
        default_keeps_history: bool,
        default_mutability: bool,
    ) -> Result<Self, Error> {
        let mut document_properties: BTreeMap<String, DocumentField> = BTreeMap::new();

        // Do documents of this type keep history? (Overrides contract value)
        let documents_keep_history: bool = cbor_inner_bool_value_with_default(
            document_type_value_map,
            "documentsKeepHistory",
            default_keeps_history,
        );

        // Are documents of this type mutable? (Overrides contract value)
        let documents_mutable: bool = cbor_inner_bool_value_with_default(
            document_type_value_map,
            "documentsMutable",
            default_mutability,
        );

        let index_values = cbor_inner_array_value(document_type_value_map, "indices");
        let indices: Vec<Index> = match index_values {
            None => {
                vec![]
            }
            Some(index_values) => {
                let mut m_indexes = Vec::with_capacity(index_values.len());
                for index_value in index_values {
                    if !index_value.is_map() {
                        return Err(Error::Contract(ContractError::InvalidContractStructure(
                            "table document is not a map as expected",
                        )));
                    }
                    let index =
                        Index::from_cbor_value(index_value.as_map().expect("confirmed as map"))?;
                    m_indexes.push(index);
                }
                m_indexes
            }
        };

        // Extract the properties
        let property_values =
            cbor_inner_btree_map(document_type_value_map, "properties").ok_or({
                Error::Contract(ContractError::InvalidContractStructure(
                    "unable to get document properties from the contract",
                ))
            })?;

        let mut required_fields =
            cbor_inner_array_of_strings(document_type_value_map, "required").unwrap_or_default();

        fn insert_values(
            document_properties: &mut BTreeMap<String, DocumentField>,
            known_required: &mut BTreeSet<String>,
            prefix: Option<&str>,
            property_key: String,
            property_value: &Value,
            definition_references: &BTreeMap<String, &Value>,
        ) -> Result<(), Error> {
            let prefixed_property_key = match prefix {
                None => property_key,
                Some(prefix) => [prefix, property_key.as_str()].join("."),
            };

            if !property_value.is_map() {
                return Err(Error::Contract(ContractError::InvalidContractStructure(
                    "document property is not a map as expected",
                )));
            }

            let inner_property_values = property_value.as_map().expect("confirmed as map");
            let base_inner_properties = cbor_map_to_btree_map(inner_property_values);

            let type_value = cbor_inner_text_value(inner_property_values, "type");
            let result: Result<(&str, BTreeMap<String, &Value>), Error> = match type_value {
                None => {
                    let ref_value = btree_map_inner_text_value(&base_inner_properties, "$ref")
                        .ok_or({
                            Error::Contract(ContractError::InvalidContractStructure(
                                "cannot find type property",
                            ))
                        })?;
                    if !ref_value.starts_with("#/$defs/") {
                        return Err(Error::Contract(ContractError::InvalidContractStructure(
                            "malformed reference",
                        )));
                    }
                    let ref_value = ref_value.split_at(8).1;
                    let inner_properties_map =
                        btree_map_inner_map_value(definition_references, ref_value).ok_or({
                            Error::Contract(ContractError::ReferenceDefinitionNotFound(
                                "document reference not found",
                            ))
                        })?;
                    let type_value =
                        cbor_inner_text_value(inner_properties_map, "type").ok_or({
                            Error::Contract(ContractError::InvalidContractStructure(
                                "cannot find type property on reference",
                            ))
                        })?;
                    let inner_properties = cbor_map_to_btree_map(inner_properties_map);
                    Ok((type_value, inner_properties))
                }
                Some(type_value) => Ok((type_value, base_inner_properties)),
            };

            let (type_value, inner_properties) = result?;

            let required = known_required.contains(&type_value.to_string());

            let field_type: DocumentFieldType;

            match type_value {
                "array" => {
                    // Only handling bytearrays for v1
                    // Return an error if it is not a byte array
                    field_type = match btree_map_inner_bool_value(&inner_properties, "byteArray") {
                        Some(inner_bool) => {
                            if inner_bool {
                                DocumentFieldType::ByteArray(
                                    btree_map_inner_size_value(&inner_properties, "minItems"),
                                    btree_map_inner_size_value(&inner_properties, "maxItems"),
                                )
                            } else {
                                return Err(Error::Contract(
                                    ContractError::InvalidContractStructure(
                                        "byteArray should always be true if defined",
                                    ),
                                ));
                            }
                        }
                        None => {
                            return Err(Error::Drive(DriveError::Unsupported(
                                "arrays not yet supported",
                            )));
                            //DocumentFieldType::Array()
                        }
                    };

                    document_properties.insert(
                        prefixed_property_key,
                        DocumentField {
                            document_type: field_type,
                            required,
                        },
                    );
                }
                "object" => {
                    let properties = btree_map_inner_btree_map(&inner_properties, "properties")
                        .ok_or({
                            Error::Contract(ContractError::InvalidContractStructure(
                                "object must have properties",
                            ))
                        })?;
                    for (object_property_key, object_property_value) in properties.into_iter() {
                        insert_values(
                            document_properties,
                            known_required,
                            Some(&prefixed_property_key),
                            object_property_key,
                            object_property_value,
                            definition_references,
                        )?
                    }
                }
                "string" => {
                    field_type = DocumentFieldType::String(
                        btree_map_inner_size_value(&inner_properties, "minLength"),
                        btree_map_inner_size_value(&inner_properties, "maxLength"),
                    );
                    document_properties.insert(
                        prefixed_property_key,
                        DocumentField {
                            document_type: field_type,
                            required,
                        },
                    );
                }
                _ => {
                    field_type = types::string_to_field_type(type_value).ok_or({
                        Error::Contract(ContractError::ValueWrongType("invalid type"))
                    })?;
                    document_properties.insert(
                        prefixed_property_key,
                        DocumentField {
                            document_type: field_type,
                            required,
                        },
                    );
                }
            }
            Ok(())
        }

        // Based on the property name, determine the type
        for (property_key, property_value) in property_values {
            insert_values(
                &mut document_properties,
                &mut required_fields,
                None,
                property_key,
                property_value,
                definition_references,
            )?;
        }

        // Add system properties
        if required_fields.contains("$createdAt") {
            document_properties.insert(
                String::from("$createdAt"),
                DocumentField {
                    document_type: DocumentFieldType::Date,
                    required: true,
                },
            );
        }

        if required_fields.contains("$updatedAt") {
            document_properties.insert(
                String::from("$updatedAt"),
                DocumentField {
                    document_type: DocumentFieldType::Date,
                    required: true,
                },
            );
        }

        Ok(DocumentType {
            name: String::from(name),
            indices,
            properties: document_properties,
            required_fields,
            documents_keep_history,
            documents_mutable,
        })
    }

    pub fn max_size(&self) -> u16 {
        let properties_max_size : usize = self.properties
            .iter()
            .filter_map(|(_, document_field_type)| {
                document_field_type.document_type.max_byte_size()
            })
            .sum();
        if properties_max_size > u16::MAX as usize {
            u16::MAX
        } else {
            properties_max_size as u16
        }
    }

    pub fn top_level_indices(&self) -> Result<Vec<&IndexProperty>, Error> {
        let mut index_properties: Vec<&IndexProperty> = Vec::with_capacity(self.indices.len());
        for index in &self.indices {
            if let Some(property) = index.properties.get(0) {
                index_properties.push(property);
            }
        }
        Ok(index_properties)
    }

    pub fn document_field_for_property(&self, property: &str) -> Option<DocumentField> {
        self.properties.get(property).cloned()
    }

    pub fn document_field_type_for_property(&self, property: &str) -> Option<DocumentFieldType> {
        match property {
            "$id" => Some(DocumentFieldType::ByteArray(
                Some(DEFAULT_HASH_SIZE),
                Some(DEFAULT_HASH_SIZE),
            )),
            "$ownerId" => Some(DocumentFieldType::ByteArray(
                Some(DEFAULT_HASH_SIZE),
                Some(DEFAULT_HASH_SIZE),
            )),
            "$createdAt" => Some(DocumentFieldType::Date),
            "$updatedAt" => Some(DocumentFieldType::Date),
            &_ => self
                .document_field_for_property(property)
                .map(|document_field| document_field.document_type),
        }
    }

    pub fn random_documents(&self, count: u32, seed: Option<u64>) -> Vec<Document> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        let mut vec: Vec<Document> = vec![];
        for _i in 0..count {
            vec.push(self.random_document_with_rng(&mut rng));
        }
        vec
    }

    pub fn document_from_bytes(&self, bytes: &[u8]) -> Result<Document, Error> {
        Document::from_bytes(bytes, self)
    }

    pub fn random_document(&self, seed: Option<u64>) -> Document {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        self.random_document_with_rng(&mut rng)
    }

    pub fn random_document_with_rng(&self, rng: &mut StdRng) -> Document {
        let id = rng.gen::<[u8; 32]>();
        let owner_id = rng.gen::<[u8; 32]>();
        let properties = self
            .properties
            .iter()
            .map(|(key, document_field)| {
                (key.clone(), document_field.document_type.random_value(rng))
            })
            .collect();

        Document {
            id,
            properties,
            owner_id,
        }
    }

    pub fn random_filled_documents(&self, count: u32, seed: Option<u64>) -> Vec<Document> {
        let mut rng = match seed {
            None => rand::rngs::StdRng::from_entropy(),
            Some(seed_value) => rand::rngs::StdRng::seed_from_u64(seed_value),
        };
        let mut vec: Vec<Document> = vec![];
        for _i in 0..count {
            vec.push(self.random_filled_document_with_rng(&mut rng));
        }
        vec
    }

    pub fn random_filled_document(&self, seed: Option<u64>) -> Document {
        let mut rng = match seed {
            None => rand::rngs::StdRng::from_entropy(),
            Some(seed_value) => rand::rngs::StdRng::seed_from_u64(seed_value),
        };
        self.random_filled_document_with_rng(&mut rng)
    }

    pub fn random_filled_document_with_rng(&self, rng: &mut StdRng) -> Document {
        let id = rng.gen::<[u8; 32]>();
        let owner_id = rng.gen::<[u8; 32]>();
        let properties = self
            .properties
            .iter()
            .map(|(key, document_field)| {
                (
                    key.clone(),
                    document_field.document_type.random_filled_value(rng),
                )
            })
            .collect();

        Document {
            id,
            properties,
            owner_id,
        }
    }
}