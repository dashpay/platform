mod types;

use crate::drive::{Drive, RootTree};
use ciborium::value::{Value as CborValue, Value};
use grovedb::Error;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

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
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Contract {
    pub document_types: BTreeMap<String, DocumentType>,
    pub id: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct DocumentType {
    pub name: String,
    pub indices: Vec<Index>,
    pub properties: HashMap<String, types::DocumentFieldType>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Document {
    pub id: [u8; 32],
    pub properties: HashMap<String, CborValue>,
    pub owner_id: [u8; 32],
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

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct IndexProperty {
    pub(crate) name: String,
    pub(crate) ascending: bool,
}

// Struct Implementations
impl Contract {
    pub fn from_cbor(contract_cbor: &[u8]) -> Result<Self, Error> {
        let (version, read_contract_cbor) = contract_cbor.split_at(4);
        if !Drive::check_protocol_version_bytes(version) {
            return Err(Error::CorruptedData(String::from(
                "invalid protocol version",
            )));
        }
        // Deserialize the contract
        let contract: HashMap<String, CborValue> = ciborium::de::from_reader(read_contract_cbor)
            .map_err(|_| Error::CorruptedData(String::from("unable to decode contract")))?;

        // Get the contract id
        let contract_id: [u8; 32] = bytes_for_system_value_from_hash_map(&contract, "$id")?
            .ok_or_else(|| Error::CorruptedData(String::from("unable to get contract id")))?
            .try_into()
            .map_err(|_| Error::CorruptedData(String::from("contract_id must be 32 bytes")))?;

        let documents_cbor_value = contract
            .get("documents")
            .ok_or_else(|| Error::CorruptedData(String::from("unable to get documents")))?;
        let contract_document_types_raw = documents_cbor_value
            .as_map()
            .ok_or_else(|| Error::CorruptedData(String::from("unable to get documents")))?;

        let mut contract_document_types: BTreeMap<String, DocumentType> = BTreeMap::new();

        // Build the document type hashmap
        for (type_key_value, document_type_value) in contract_document_types_raw {
            if !type_key_value.is_text() {
                return Err(Error::CorruptedData(String::from(
                    "table type is not a string as expected",
                )));
            }

            // Make sure the document_type_value is a map
            if !document_type_value.is_map() {
                return Err(Error::CorruptedData(String::from(
                    "table type is not a map as expected",
                )));
            }

            let document_type = DocumentType::from_cbor_value(
                type_key_value.as_text().expect("confirmed as text"),
                document_type_value.as_map().expect("confirmed as map"),
            )?;
            contract_document_types.insert(
                String::from(type_key_value.as_text().expect("confirmed as text")),
                document_type,
            );
        }

        Ok(Contract {
            id: contract_id,
            document_types: contract_document_types,
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
            "$ownerId" | "$id" => bytes_for_system_value(value)?.ok_or_else(|| {
                Error::CorruptedData(String::from("expected system value to be deserialized"))
            }),
            _ => {
                let field_type = self.properties.get(key).ok_or_else(|| {
                    Error::CorruptedData(String::from("expected document to have field"))
                })?;
                types::encode_document_field_type(field_type, value)
            }
        }
    }

    pub fn from_cbor_value(
        name: &str,
        document_type_value_map: &[(Value, Value)],
    ) -> Result<Self, Error> {
        let mut document_properties: HashMap<String, types::DocumentFieldType> = HashMap::new();

        let index_values = match cbor_inner_array_value(document_type_value_map, "indices") {
            Some(index_values) => index_values,
            None => {
                return Ok(DocumentType {
                    name: String::from(name),
                    indices: vec![],
                    properties: document_properties,
                })
            }
        };

        let mut indices: Vec<Index> = Vec::with_capacity(index_values.len());
        for index_value in index_values {
            if !index_value.is_map() {
                return Err(Error::CorruptedData(String::from(
                    "table document is not a map as expected",
                )));
            }
            let index = Index::from_cbor_value(index_value.as_map().expect("confirmed as map"))?;
            indices.push(index);
        }

        // Extract the properties
        let property_values = cbor_inner_map_value(document_type_value_map, "properties")
            .ok_or_else(|| {
                Error::CorruptedData(String::from(
                    "unable to get document properties from the contract",
                ))
            })?;

        fn insert_values(
            document_properties: &mut HashMap<String, types::DocumentFieldType>,
            prefix: Option<&str>,
            property_key: &Value,
            property_value: &Value,
        ) -> Result<(), Error> {
            if !property_key.is_text() {
                return Err(Error::CorruptedData(String::from(
                    "property key should be text",
                )));
            }

            let property_key_string = property_key
                .as_text()
                .expect("confirmed as text")
                .to_string();

            let prefixed_property_key = match prefix {
                None => property_key_string,
                Some(prefix) => [prefix, property_key_string.as_str()].join("."),
            };

            if !property_value.is_map() {
                return Err(Error::CorruptedData(String::from(
                    "document property is not a map as expected",
                )));
            }

            let inner_property_values = property_value.as_map().expect("confirmed as map");
            let type_value = cbor_inner_text_value(inner_property_values, "type")
                .ok_or_else(|| Error::CorruptedData(String::from("cannot find type property")))?;

            let field_type: types::DocumentFieldType;

            match type_value {
                "array" => {
                    // Only handling bytearrays for v1
                    // Return an error if it is not a byte array
                    field_type = match cbor_inner_bool_value(inner_property_values, "byteArray") {
                        Some(inner_bool) => {
                            if inner_bool {
                                types::DocumentFieldType::ByteArray
                            } else {
                                return Err(Error::CorruptedData(String::from("invalid type")));
                            }
                        }
                        None => types::DocumentFieldType::Array,
                    };

                    document_properties.insert(prefixed_property_key, field_type);
                }
                "object" => {
                    let properties = cbor_inner_map_value(inner_property_values, "properties")
                        .ok_or_else(|| {
                            Error::CorruptedData(String::from(
                                "cannot find byteArray property for array type",
                            ))
                        })?;
                    for (object_property_key, object_property_value) in properties.iter() {
                        insert_values(
                            document_properties,
                            Some(&prefixed_property_key),
                            object_property_key,
                            object_property_value,
                        )?
                    }
                }
                _ => {
                    field_type = types::string_to_field_type(type_value)
                        .ok_or_else(|| Error::CorruptedData(String::from("invalid type")))?;
                    document_properties.insert(prefixed_property_key, field_type);
                }
            }
            Ok(())
        }

        // Based on the property name, determine the type
        for (property_key, property_value) in property_values {
            insert_values(&mut document_properties, None, property_key, property_value)?;
        }

        // Add system properties
        document_properties.insert(String::from("$createdAt"), types::DocumentFieldType::Date);
        document_properties.insert(String::from("$updatedAt"), types::DocumentFieldType::Date);

        Ok(DocumentType {
            name: String::from(name),
            indices,
            properties: document_properties,
        })
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
}

impl Document {
    pub fn from_cbor(
        document_cbor: &[u8],
        document_id: Option<&[u8]>,
        owner_id: Option<&[u8]>,
    ) -> Result<Self, Error> {
        let (version, read_document_cbor) = document_cbor.split_at(4);
        if !Drive::check_protocol_version_bytes(version) {
            return Err(Error::CorruptedData(String::from(
                "invalid protocol version",
            )));
        }
        // first we need to deserialize the document and contract indices
        // we would need dedicated deserialization functions based on the document type
        let mut document: HashMap<String, CborValue> =
            ciborium::de::from_reader(read_document_cbor)
                .map_err(|_| Error::CorruptedData(String::from("unable to decode contract")))?;

        let owner_id: [u8; 32] = match owner_id {
            None => {
                let owner_id: Vec<u8> =
                    bytes_for_system_value_from_hash_map(&document, "$ownerId")?.ok_or_else(
                        || Error::CorruptedData(String::from("unable to get document $ownerId")),
                    )?;
                document.remove("$ownerId");
                if owner_id.len() != 32 {
                    return Err(Error::CorruptedData(String::from("invalid owner id")));
                }
                owner_id.as_slice().try_into()
            }
            Some(owner_id) => {
                // we need to start by verifying that the owner_id is a 256 bit number (32 bytes)
                if owner_id.len() != 32 {
                    return Err(Error::CorruptedData(String::from("invalid owner id")));
                }
                owner_id.try_into()
            }
        }
        .expect("conversion to 32bytes shouldn't fail");

        let id: [u8; 32] = match document_id {
            None => {
                let document_id: Vec<u8> = bytes_for_system_value_from_hash_map(&document, "$id")?
                    .ok_or_else(|| {
                        Error::CorruptedData(String::from("unable to get document $id"))
                    })?;
                document.remove("$id");
                if document_id.len() != 32 {
                    return Err(Error::CorruptedData(String::from("invalid document id")));
                }
                document_id.as_slice().try_into()
            }
            Some(document_id) => {
                // we need to start by verifying that the document_id is a 256 bit number (32 bytes)
                if document_id.len() != 32 {
                    return Err(Error::CorruptedData(String::from("invalid document id")));
                }
                document_id.try_into()
            }
        }
        .expect("document_id must be 32 bytes");

        // dev-note: properties is everything other than the id and owner id
        Ok(Document {
            properties: document,
            owner_id,
            id,
        })
    }

    pub fn from_cbor_with_id(
        document_cbor: &[u8],
        document_id: &[u8],
        owner_id: &[u8],
    ) -> Result<Self, Error> {
        // we need to start by verifying that the owner_id is a 256 bit number (32 bytes)
        if owner_id.len() != 32 {
            return Err(Error::CorruptedData(String::from("invalid owner id")));
        }

        if document_id.len() != 32 {
            return Err(Error::CorruptedData(String::from("invalid document id")));
        }

        let (version, read_document_cbor) = document_cbor.split_at(4);
        if !Drive::check_protocol_version_bytes(version) {
            return Err(Error::CorruptedData(String::from(
                "invalid protocol version",
            )));
        }

        // first we need to deserialize the document and contract indices
        // we would need dedicated deserialization functions based on the document type
        let properties: HashMap<String, CborValue> = ciborium::de::from_reader(read_document_cbor)
            .map_err(|_| Error::CorruptedData(String::from("unable to decode contract")))?;

        // dev-note: properties is everything other than the id and owner id
        Ok(Document {
            properties,
            owner_id: owner_id
                .try_into()
                .expect("try_into shouldn't fail, document_id must be 32 bytes"),
            id: document_id
                .try_into()
                .expect("try_into shouldn't fail, document_id must be 32 bytes"),
        })
    }

    pub fn get_raw_for_document_type<'a>(
        &'a self,
        key_path: &str,
        document_type: &DocumentType,
        owner_id: Option<&[u8]>,
    ) -> Result<Option<Vec<u8>>, Error> {
        if key_path == "$ownerId" && owner_id.is_some() {
            Ok(Some(Vec::from(owner_id.unwrap())))
        } else {
            match key_path {
                "$id" => return Ok(Some(Vec::from(self.id))),
                "$ownerId" => return Ok(Some(Vec::from(self.owner_id))),
                _ => {}
            }
            let key_paths: Vec<&str> = key_path.split('.').collect::<Vec<&str>>();
            let (key, rest_key_paths) = key_paths.split_first().ok_or_else(|| {
                Error::CorruptedData(String::from(
                    "key must not be null when getting from document",
                ))
            })?;

            fn get_value_at_path<'a>(
                value: &'a Value,
                key_paths: &'a [&str],
            ) -> Result<Option<&'a Value>, Error> {
                if key_paths.is_empty() {
                    Ok(Some(value))
                } else {
                    let (key, rest_key_paths) = key_paths.split_first().ok_or_else(|| {
                        Error::CorruptedData(String::from(
                            "key must not be null when getting from document",
                        ))
                    })?;
                    let map_values = value.as_map().ok_or_else(|| {
                        Error::CorruptedData(String::from("inner key must refer to a value map"))
                    })?;
                    match get_key_from_cbor_map(map_values, key) {
                        None => Ok(None),
                        Some(value) => get_value_at_path(value, rest_key_paths),
                    }
                }
            }

            match self.properties.get(*key) {
                None => Ok(None),
                Some(value) => match get_value_at_path(value, rest_key_paths)? {
                    None => Ok(None),
                    Some(path_value) => Ok(Some(
                        document_type.serialize_value_for_key(key_path, path_value)?,
                    )),
                },
            }
        }
    }

    pub fn get_raw_for_contract<'a>(
        &'a self,
        key: &str,
        document_type_name: &str,
        contract: &Contract,
        owner_id: Option<&[u8]>,
    ) -> Result<Option<Vec<u8>>, Error> {
        let document_type = contract
            .document_types
            .get(document_type_name)
            .ok_or_else(|| {
                Error::CorruptedData(String::from("document type should exist for name"))
            })?;
        self.get_raw_for_document_type(key, document_type, owner_id)
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
            let key = key_value
                .as_text()
                .ok_or_else(|| Error::CorruptedData(String::from("key should be of type text")))?;

            if key == "unique" {
                if value_value.is_bool() {
                    unique = value_value.as_bool().expect("confirmed as bool");
                }
            } else if key == "properties" {
                let properties = value_value.as_array().ok_or_else(|| {
                    Error::CorruptedData(String::from("property value should be an array"))
                })?;

                // Iterate over this and get the index properties
                for property in properties {
                    if !property.is_map() {
                        return Err(Error::CorruptedData(String::from(
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
            .ok_or_else(|| Error::CorruptedData(String::from("key should be of type string")))?;
        let value = property
            .1 // value
            .as_text()
            .ok_or_else(|| Error::CorruptedData(String::from("value should be of type string")))?;

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

fn get_key_from_cbor_map<'a>(cbor_map: &'a [(Value, Value)], key: &'a str) -> Option<&'a Value> {
    for (cbor_key, cbor_value) in cbor_map.iter() {
        if !cbor_key.is_text() {
            continue;
        }

        if cbor_key.as_text().expect("confirmed as text") == key {
            return Some(cbor_value);
        }
    }
    None
}

fn cbor_inner_array_value<'a>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Option<&'a Vec<Value>> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Array(key_value) = key_value {
        return Some(key_value);
    }
    None
}

fn cbor_inner_map_value<'a>(
    document_type: &'a [(Value, Value)],
    key: &'a str,
) -> Option<&'a Vec<(Value, Value)>> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Map(map_value) = key_value {
        return Some(map_value);
    }
    None
}

fn cbor_inner_text_value<'a>(document_type: &'a [(Value, Value)], key: &'a str) -> Option<&'a str> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Text(string_value) = key_value {
        return Some(string_value);
    }
    None
}

fn cbor_inner_bool_value(document_type: &[(Value, Value)], key: &str) -> Option<bool> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if let Value::Bool(bool_value) = key_value {
        return Some(*bool_value);
    }
    None
}

pub fn bytes_for_system_value(value: &Value) -> Result<Option<Vec<u8>>, Error> {
    match value {
        Value::Bytes(bytes) => Ok(Some(bytes.clone())),
        Value::Text(text) => match bs58::decode(text).into_vec() {
            Ok(data) => Ok(Some(data)),
            Err(_) => Ok(None),
        },
        Value::Array(array) => array
            .iter()
            .map(|byte| match byte {
                Value::Integer(int) => {
                    let value_as_u8: u8 = (*int)
                        .try_into()
                        .map_err(|_| Error::CorruptedData(String::from("expected u8 value")))?;
                    Ok(Some(value_as_u8))
                }
                _ => Err(Error::CorruptedData(String::from(
                    "not an array of integers",
                ))),
            })
            .collect::<Result<Option<Vec<u8>>, Error>>(),
        _ => Err(Error::CorruptedData(String::from(
            "system value is incorrect type",
        ))),
    }
}

fn bytes_for_system_value_from_hash_map(
    document: &HashMap<String, CborValue>,
    key: &str,
) -> Result<Option<Vec<u8>>, Error> {
    let value = document.get(key);
    if value.is_some() {
        bytes_for_system_value(value.unwrap())
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::json_document_to_cbor;
    use crate::contract::Contract;
    use crate::drive::Drive;
    use std::collections::HashMap;

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
    fn test_import_contract() {
        let dashpay_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            Some(1),
        );
        let contract = Contract::from_cbor(&dashpay_cbor).unwrap();

        assert_eq!(contract.document_types.len(), 3);
        assert!(contract.document_types.get("profile").is_some());
        assert!(contract.document_types.get("contactInfo").is_some());
        assert!(contract.document_types.get("contactRequest").is_some());
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
