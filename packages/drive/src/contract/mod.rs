use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

use ciborium::value::{Value as CborValue, Value};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use document::Document;

use crate::common::{
    bool_for_system_value_from_tree_map, btree_map_inner_bool_value, btree_map_inner_btree_map,
    btree_map_inner_map_value, btree_map_inner_size_value, btree_map_inner_text_value,
    bytes_for_system_value, bytes_for_system_value_from_tree_map, cbor_inner_array_of_strings,
    cbor_inner_array_value, cbor_inner_bool_value_with_default, cbor_inner_btree_map,
    cbor_inner_text_value, cbor_map_to_btree_map,
};
use crate::contract::types::{DocumentField, DocumentFieldType};
use crate::drive::config::DriveEncoding;
use crate::drive::defaults::{DEFAULT_HASH_SIZE, MAX_INDEX_SIZE};
use crate::drive::{Drive, RootTree};
use crate::error::contract::ContractError;
use crate::error::drive::DriveError;
use crate::error::structure::StructureError;
use crate::error::Error;

mod defaults;
pub mod document;
pub mod types;

// contract
// - id
// - documents
//      - document_type
//          - indices
//               - properties
//                  - name
//                  - ascending
//               - unique

// Struct Definitions
#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Contract {
    pub id: [u8; 32],
    pub document_types: BTreeMap<String, DocumentType>,
    pub keeps_history: bool,
    pub readonly: bool,
    pub documents_keep_history_contract_default: bool,
    pub documents_mutable_contract_default: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct DocumentType {
    pub name: String,
    pub indices: Vec<Index>,
    pub properties: BTreeMap<String, DocumentField>,
    pub required_fields: BTreeSet<String>,
    pub documents_keep_history: bool,
    pub documents_mutable: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Index {
    pub properties: Vec<IndexProperty>,
    pub unique: bool,
}

impl Index {
    // The matches function will take a slice of an array of strings and an optional sort on value.
    // An index matches if all the index_names in the slice are consecutively the index's properties
    // with leftovers permitted.
    // If a sort_on value is provided it must match the last index property.
    // The number returned is the number of unused index properties

    // A case for example if we have an index on person's name and age
    // where we say name == 'Sam' sort by age
    // there is no field operator on age
    // The return value for name == 'Sam' sort by age would be 0
    // The return value for name == 'Sam and age > 5 sort by age would be 0
    // the return value for sort by age would be 1
    pub fn matches(
        &self,
        index_names: &[&str],
        in_field_name: Option<&str>,
        order_by: &[&str],
    ) -> Option<u16> {
        // Here we are trying to figure out if the Index matches the order by
        // To do so we take the index and go backwards as we need the order by clauses to be
        // continuous, but they do not need to be at the end.
        let mut reduced_properties = self.properties.as_slice();
        // let mut should_ignore: Vec<String> = order_by.iter().map(|&str| str.to_string()).collect();
        if !order_by.is_empty() {
            for _ in 0..self.properties.len() {
                if reduced_properties.len() < order_by.len() {
                    return None;
                }
                let matched_ordering = reduced_properties
                    .iter()
                    .rev()
                    .zip(order_by.iter().rev())
                    .all(|(property, &sort)| property.name.as_str() == sort);
                if matched_ordering {
                    break;
                }
                if let Some((_last, elements)) = reduced_properties.split_last() {
                    // should_ignore.push(last.name.clone());
                    reduced_properties = elements;
                } else {
                    return None;
                }
            }
        }

        let last_property = self.properties.last()?;

        // the in field can only be on the last or before last property
        if let Some(in_field_name) = in_field_name {
            if last_property.name.as_str() != in_field_name {
                // it can also be on the before last
                if self.properties.len() == 1 {
                    return None;
                }
                let before_last_property = self.properties.get(self.properties.len() - 2)?;
                if before_last_property.name.as_str() != in_field_name {
                    return None;
                }
            }
        }

        let mut d = self.properties.len();

        for search_name in index_names.iter() {
            if !reduced_properties
                .iter()
                .any(|property| property.name.as_str() == *search_name)
            {
                return None;
            }
            d -= 1;
        }

        Some(d as u16)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct IndexProperty {
    pub name: String,
    pub ascending: bool,
}

// Struct Implementations
impl Contract {
    pub fn deserialize(
        serialized_contract: &[u8],
        contract_id: Option<[u8; 32]>,
        encoding: DriveEncoding,
    ) -> Result<Self, Error> {
        match encoding {
            DriveEncoding::DriveCbor => Contract::from_cbor(serialized_contract, contract_id),
            DriveEncoding::DriveProtobuf => {
                todo!()
            }
        }
    }

    pub fn from_cbor(contract_cbor: &[u8], contract_id: Option<[u8; 32]>) -> Result<Self, Error> {
        let (version, read_contract_cbor) = contract_cbor.split_at(4);
        if !Drive::check_protocol_version_bytes(version) {
            return Err(Error::Structure(StructureError::InvalidProtocolVersion(
                "invalid protocol version",
            )));
        }
        // Deserialize the contract
        let contract: BTreeMap<String, CborValue> = ciborium::de::from_reader(read_contract_cbor)
            .map_err(|_| {
            Error::Structure(StructureError::InvalidCBOR("unable to decode contract"))
        })?;

        // Get the contract id
        let contract_id: [u8; 32] = if let Some(contract_id) = contract_id {
            contract_id
        } else {
            bytes_for_system_value_from_tree_map(&contract, "$id")?
                .ok_or({
                    Error::Contract(ContractError::MissingRequiredKey(
                        "unable to get contract id",
                    ))
                })?
                .try_into()
                .map_err(|_| {
                    Error::Contract(ContractError::FieldRequirementUnmet(
                        "contract_id must be 32 bytes",
                    ))
                })?
        };

        // Does the contract keep history when the contract itself changes?
        let keeps_history: bool = bool_for_system_value_from_tree_map(
            &contract,
            "keepsHistory",
            crate::contract::defaults::DEFAULT_CONTRACT_KEEPS_HISTORY,
        )?;

        // Is the contract mutable?
        let readonly: bool = bool_for_system_value_from_tree_map(
            &contract,
            "readOnly",
            !crate::contract::defaults::DEFAULT_CONTRACT_MUTABILITY,
        )?;

        // Do documents in the contract keep history?
        let documents_keep_history_contract_default: bool = bool_for_system_value_from_tree_map(
            &contract,
            "documentsKeepHistoryContractDefault",
            crate::contract::defaults::DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY,
        )?;

        // Are documents in the contract mutable?
        let documents_mutable_contract_default: bool = bool_for_system_value_from_tree_map(
            &contract,
            "documentsMutableContractDefault",
            crate::contract::defaults::DEFAULT_CONTRACT_DOCUMENT_MUTABILITY,
        )?;

        let definition_references = match contract.get("$defs") {
            None => BTreeMap::new(),
            Some(definition_value) => {
                let definition_map = definition_value.as_map();
                match definition_map {
                    None => BTreeMap::new(),
                    Some(key_value) => cbor_map_to_btree_map(key_value),
                }
            }
        };

        let documents_cbor_value = contract.get("documents").ok_or({
            Error::Contract(ContractError::MissingRequiredKey("unable to get documents"))
        })?;
        let contract_document_types_raw = documents_cbor_value.as_map().ok_or({
            Error::Contract(ContractError::InvalidContractStructure(
                "documents must be a map",
            ))
        })?;

        let mut contract_document_types: BTreeMap<String, DocumentType> = BTreeMap::new();

        // Build the document type hashmap
        for (type_key_value, document_type_value) in contract_document_types_raw {
            if !type_key_value.is_text() {
                return Err(Error::Contract(ContractError::InvalidContractStructure(
                    "document type name is not a string as expected",
                )));
            }

            // Make sure the document_type_value is a map
            if !document_type_value.is_map() {
                return Err(Error::Contract(ContractError::InvalidContractStructure(
                    "document type data is not a map as expected",
                )));
            }

            let document_type = DocumentType::from_cbor_value(
                type_key_value.as_text().expect("confirmed as text"),
                document_type_value.as_map().expect("confirmed as map"),
                &definition_references,
                documents_keep_history_contract_default,
                documents_mutable_contract_default,
            )?;
            contract_document_types.insert(
                String::from(type_key_value.as_text().expect("confirmed as text")),
                document_type,
            );
        }

        Ok(Contract {
            id: contract_id,
            document_types: contract_document_types,
            keeps_history,
            readonly,
            documents_keep_history_contract_default,
            documents_mutable_contract_default,
        })
    }

    pub fn root_path(&self) -> [&[u8]; 2] {
        [
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            &self.id,
        ]
    }

    pub fn documents_path(&self) -> [&[u8]; 3] {
        [
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            &self.id,
            &[1],
        ]
    }

    pub fn document_type_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 4] {
        [
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            &self.id,
            &[1],
            document_type_name.as_bytes(),
        ]
    }

    pub fn documents_primary_key_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 5] {
        [
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            &self.id,
            &[1],
            document_type_name.as_bytes(),
            &[0],
        ]
    }

    pub fn documents_with_history_primary_key_path<'a>(
        &'a self,
        document_type_name: &'a str,
        id: &'a [u8],
    ) -> [&'a [u8]; 6] {
        [
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            &self.id,
            &[1],
            document_type_name.as_bytes(),
            &[0],
            id,
        ]
    }

    pub fn document_type_for_name(&self, document_type_name: &str) -> Result<&DocumentType, Error> {
        self.document_types.get(document_type_name).ok_or({
            Error::Contract(ContractError::DocumentTypeNotFound(
                "can not get document type from contract",
            ))
        })
    }
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
                                types::DocumentFieldType::ByteArray(
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
                    field_type = types::DocumentFieldType::String(
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

    pub fn max_size(&self) -> usize {
        self.properties
            .iter()
            .filter_map(|(_, document_field_type)| {
                document_field_type.document_type.max_byte_size()
            })
            .sum()
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

fn reduced_value_string_representation(value: &Value) -> String {
    match value {
        Value::Integer(integer) => {
            let i: i128 = (*integer).try_into().unwrap();
            format!("{}", i)
        }
        Value::Bytes(bytes) => hex::encode(bytes),
        Value::Float(float) => {
            format!("{}", float)
        }
        Value::Text(text) => {
            let len = text.len();
            if len > 20 {
                let first_text = text.split_at(20).0.to_string();
                format!("{}[...({})]", first_text, len)
            } else {
                text.clone()
            }
        }
        Value::Bool(b) => {
            format!("{}", b)
        }
        Value::Null => "None".to_string(),
        Value::Tag(_, _) => "Tag".to_string(),
        Value::Array(_) => "Array".to_string(),
        Value::Map(_) => "Map".to_string(),
        _ => "".to_string(),
    }
}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "id:{} ", bs58::encode(self.id).into_string())?;
        write!(f, "owner_id:{} ", bs58::encode(self.owner_id).into_string())?;
        if self.properties.is_empty() {
            write!(f, "no properties")?;
        } else {
            for (key, value) in self.properties.iter() {
                write!(f, "{}:{} ", key, reduced_value_string_representation(value))?
            }
        }
        Ok(())
    }
}

impl Index {
    pub fn from_cbor_value(index_type_value_map: &[(Value, Value)]) -> Result<Self, Error> {
        // Decouple the map
        // It contains properties and a unique key
        // If the unique key is absent, then unique is false
        // If present, then use that value
        // For properties, we iterate each and move it to IndexProperty

        let mut unique = false;
        let mut index_properties: Vec<IndexProperty> = Vec::new();

        for (key_value, value_value) in index_type_value_map {
            let key = key_value.as_text().ok_or({
                Error::Contract(ContractError::KeyWrongType("key should be of type text"))
            })?;

            if key == "unique" {
                if value_value.is_bool() {
                    unique = value_value.as_bool().expect("confirmed as bool");
                }
            } else if key == "properties" {
                let properties = value_value.as_array().ok_or({
                    Error::Contract(ContractError::InvalidContractStructure(
                        "property value should be an array",
                    ))
                })?;

                // Iterate over this and get the index properties
                for property in properties {
                    if !property.is_map() {
                        return Err(Error::Contract(ContractError::InvalidContractStructure(
                            "table document is not a map as expected",
                        )));
                    }

                    let index_property = IndexProperty::from_cbor_value(
                        property.as_map().expect("confirmed as map"),
                    )?;
                    index_properties.push(index_property);
                }
            }
        }

        Ok(Index {
            properties: index_properties,
            unique,
        })
    }
}

impl IndexProperty {
    pub fn from_cbor_value(index_property_map: &[(Value, Value)]) -> Result<Self, Error> {
        let property = &index_property_map[0];

        let key = property
            .0 // key
            .as_text()
            .ok_or({
                Error::Contract(ContractError::KeyWrongType("key should be of type string"))
            })?;
        let value = property
            .1 // value
            .as_text()
            .ok_or({
                Error::Contract(ContractError::ValueWrongType(
                    "value should be of type string",
                ))
            })?;

        let ascending = value == "asc";

        Ok(IndexProperty {
            name: key.to_string(),
            ascending,
        })
    }
}

// Helper functions
fn contract_document_types(contract: &HashMap<String, CborValue>) -> Option<&Vec<(Value, Value)>> {
    contract.get("documents").and_then(|id_cbor| {
        if let CborValue::Map(documents) = id_cbor {
            Some(documents)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::common::json_document_to_cbor;
    use crate::contract::{Contract, Document};
    use crate::drive::Drive;

    #[test]
    fn test_cbor_deserialization() {
        let document_cbor = json_document_to_cbor("simple.json", Some(1));
        let (version, read_document_cbor) = document_cbor.split_at(4);
        assert!(Drive::check_protocol_version_bytes(version));
        let document: HashMap<String, ciborium::value::Value> =
            ciborium::de::from_reader(read_document_cbor).expect("cannot deserialize cbor");
        assert!(document.get("a").is_some());
    }

    #[test]
    fn test_document_cbor_serialization() {
        let dashpay_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            Some(1),
        );
        let contract = Contract::from_cbor(&dashpay_cbor, None).unwrap();

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get profile document type");
        let document = document_type.random_document(Some(3333));

        let document_cbor = document.to_cbor();

        let recovered_document = Document::from_cbor(document_cbor.as_slice(), None, None)
            .expect("expected to get document");

        assert_eq!(recovered_document, document);
    }

    #[test]
    fn test_document_display() {
        let dashpay_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            Some(1),
        );
        let contract = Contract::from_cbor(&dashpay_cbor, None).unwrap();

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get profile document type");
        let document = document_type.random_document(Some(3333));

        let document_string = format!("{}", document);
        assert_eq!(document_string.as_str(), "id:2vq574DjKi7ZD8kJ6dMHxT5wu6ZKD2bW5xKAyKAGW7qZ owner_id:ChTEGXJcpyknkADUC5s6tAzvPqVG7x6Lo1Nr5mFtj2mk $createdAt:1627081806.116 $updatedAt:1575820087.909 avatarUrl:W18RuyblDX7hxB38OJYt[...(894)] displayName:wvAD8Grs2h publicMessage:LdWpGtOzOkYXStdxU3G0[...(105)] ")
    }

    #[test]
    fn test_import_contract() {
        let dashpay_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            Some(1),
        );
        let contract = Contract::from_cbor(&dashpay_cbor, None).unwrap();

        assert!(contract.documents_mutable_contract_default);
        assert!(!contract.keeps_history);
        assert!(!contract.readonly); // the contract shouldn't be readonly
        assert!(!contract.documents_keep_history_contract_default);
        assert_eq!(contract.document_types.len(), 3);
        assert!(contract.document_types.get("profile").is_some());
        assert!(
            contract
                .document_types
                .get("profile")
                .unwrap()
                .documents_mutable
        );
        assert!(contract.document_types.get("contactInfo").is_some());
        assert!(
            contract
                .document_types
                .get("contactInfo")
                .unwrap()
                .documents_mutable
        );
        assert!(contract.document_types.get("contactRequest").is_some());
        assert!(
            !contract
                .document_types
                .get("contactRequest")
                .unwrap()
                .documents_mutable
        );
        assert!(contract.document_types.get("non_existent_key").is_none());

        let contact_info_indices = &contract.document_types.get("contactInfo").unwrap().indices;
        assert_eq!(contact_info_indices.len(), 2);
        assert!(contact_info_indices[0].unique);
        assert!(!contact_info_indices[1].unique);
        assert_eq!(contact_info_indices[0].properties.len(), 3);

        assert_eq!(contact_info_indices[0].properties[0].name, "$ownerId");
        assert_eq!(
            contact_info_indices[0].properties[1].name,
            "rootEncryptionKeyIndex"
        );
        assert_eq!(
            contact_info_indices[0].properties[2].name,
            "derivationEncryptionKeyIndex"
        );

        assert!(contact_info_indices[0].properties[0].ascending);
    }
}
