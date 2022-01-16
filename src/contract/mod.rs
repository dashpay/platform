mod types;

use crate::drive::RootTree;
use ciborium::value::{Value as CborValue, Value};
use grovedb::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
#[derive(Serialize, Deserialize)]
pub struct Contract {
    pub(crate) document_types: HashMap<String, DocumentType>,
    pub(crate) id: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct DocumentType {
    pub(crate) name: String,
    pub(crate) indices: Vec<Index>,
    pub(crate) properties: HashMap<String, types::DocumentFieldType>,
}

#[derive(Serialize, Deserialize)]
pub struct Document {
    pub(crate) id: Vec<u8>,
    pub(crate) properties: HashMap<String, CborValue>,
    pub(crate) owner_id: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Index {
    pub(crate) properties: Vec<IndexProperty>,
    pub(crate) unique: bool,
}

impl Index {
    // The matches function will take a slice of an array of strings and an optional sort on value.
    // An index matches if all the index_names in the slice are consecutively the index's properties
    // with leftovers permitted.
    // If a sort_on value is provided it must match the last index property.
    // The number returned is the number of unused index properties
    pub fn matches(&self, index_names: &[&str], sort_on: Option<&str>) -> Option<u16> {
        let mut d = self.properties.len();
        if sort_on.is_some() {
            let last_property = self.properties.last();
            if last_property.is_none() {
                return None;
            } else if last_property.unwrap().name.as_str() != sort_on.unwrap() {
                return None;
            } else {
                let last_search_name = index_names.last();
                if last_search_name.is_some() {
                    if *last_search_name.unwrap() != sort_on.unwrap() {
                        // we can remove the -1 here
                        // this is a case for example if we have an index on person's name and age
                        // where we say name == 'Sam' sort by age
                        // there is no field operator on age
                        // The return value for name == 'Sam' sort by age would be 0
                        // The return value for name == 'Sam and age > 5 sort by age would be 0
                        // the return value for sort by age would be 1
                        d -= 1;
                    }
                }
            }
        }
        for (property_name, search_name) in self.properties.iter().zip(index_names.iter()) {
            if property_name.name.as_str() != *search_name {
                return None;
            }
            d -= 1;
        }

        Some(d as u16)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct IndexProperty {
    pub(crate) name: String,
    pub(crate) ascending: bool,
}

// Struct Implementations
impl Contract {
    pub fn from_cbor(contract_cbor: &[u8]) -> Result<Self, Error> {
        // Deserialize the contract
        let contract: HashMap<String, CborValue> = ciborium::de::from_reader(contract_cbor)
            .map_err(|_| Error::CorruptedData(String::from("unable to decode contract")))?;

        // Get the contract id
        let contract_id = base58_value_as_bytes_from_hash_map(&contract, "$id").ok_or(
            Error::CorruptedData(String::from("unable to get contract id")),
        )?;

        let documents_cbor_value =
            contract
                .get("documents")
                .ok_or(Error::CorruptedData(String::from(
                    "unable to get documents",
                )))?;
        let mut contract_document_types_raw =
            documents_cbor_value
                .as_map()
                .ok_or(Error::CorruptedData(String::from(
                    "unable to get documents",
                )))?;

        let mut contract_document_types: HashMap<String, DocumentType> = HashMap::new();

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

    pub fn root_path(&self) -> Vec<&[u8]> {
        vec![RootTree::ContractDocuments.into(), &self.id]
    }

    pub fn documents_path(&self) -> Vec<&[u8]> {
        vec![RootTree::ContractDocuments.into(), &self.id, b"1"]
    }

    pub fn document_type_path<'a>(&'a self, document_type_name: &'a str) -> Vec<&'a [u8]> {
        vec![
            RootTree::ContractDocuments.into(),
            &self.id,
            b"1",
            document_type_name.as_bytes(),
        ]
    }

    pub fn documents_primary_key_path<'a>(&'a self, document_type_name: &'a str) -> Vec<&'a [u8]> {
        vec![
            RootTree::ContractDocuments.into(),
            &self.id,
            b"1",
            document_type_name.as_bytes(),
            b"0",
        ]
    }
}

impl DocumentType {
    pub fn index_for_types(
        &self,
        index_names: &[&str],
        sort_on: Option<&str>,
    ) -> Option<(&Index, u16)> {
        let mut best_index: Option<(&Index, u16)> = None;
        let mut best_difference = u16::MAX;
        for index in self.indices.iter() {
            let difference_option = index.matches(index_names, sort_on);
            if difference_option.is_some() {
                let difference = difference_option.unwrap();
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
        let field_type = self
            .properties
            .get(key)
            .ok_or(Error::CorruptedData(String::from(
                "expected document to have field",
            )))?;
        Ok(types::encode_document_field_type(field_type, value)?)
    }

    pub fn from_cbor_value(
        name: &str,
        document_type_value_map: &Vec<(Value, Value)>,
    ) -> Result<Self, Error> {
        let mut indices: Vec<Index> = Vec::new();
        let mut document_properties: HashMap<String, types::DocumentFieldType> = HashMap::new();

        let index_values = cbor_inner_array_value(&document_type_value_map, "indices").ok_or(
            Error::CorruptedData(String::from("unable to get indices from the contract")),
        )?;

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
        let property_values = cbor_inner_map_value(&document_type_value_map, "properties").ok_or(
            Error::CorruptedData(String::from(
                "unable to get document properties from the contract",
            )),
        )?;

        // Based on the property name, determine the type
        for (property_key, property_value) in property_values {
            if !property_key.is_text() {
                return Err(Error::CorruptedData(String::from(
                    "property key should be text",
                )));
            }

            if !property_value.is_map() {
                return Err(Error::CorruptedData(String::from(
                    "document property is not a map as expected",
                )));
            }

            let property_values = property_value.as_map().expect("confirmed as map");
            let type_value = cbor_inner_text_value(property_values, "type").ok_or(
                Error::CorruptedData(String::from("cannot find type property")),
            )?;

            let mut field_type: types::DocumentFieldType;

            if type_value == "array" {
                // Only handling bytearrays for v1
                // Return an error if it is not a byte array
                let is_byte_array_value = cbor_inner_bool_value(property_values, "byteArray")
                    .ok_or(Error::CorruptedData(String::from(
                        "cannot find byteArray property for array type",
                    )))?;
                if is_byte_array_value {
                    field_type = types::DocumentFieldType::ByteArray;
                } else {
                    return Err(Error::CorruptedData(String::from("invalid type")));
                }
            } else {
                field_type = types::string_to_field_type(type_value)
                    .ok_or(Error::CorruptedData(String::from("invalid type")))?;
            }

            document_properties.insert(
                property_key
                    .as_text()
                    .expect("confirmed as text")
                    .to_string(),
                field_type,
            );
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

    pub fn top_level_indices(&self) -> Result<Vec<IndexProperty>, Error> {
        let mut index_properties: Vec<IndexProperty> = Vec::new();
        for index in &self.indices {
            let property = index.properties.get(0);
            if property.is_some() {
                index_properties.push(property.expect("confirmed is some").clone());
            }
        }
        Ok(index_properties)
    }
}

impl Document {
    pub fn from_cbor(document_cbor: &[u8], owner_id: &[u8]) -> Result<Self, Error> {
        // we need to start by verifying that the owner_id is a 256 bit number (32 bytes)
        if owner_id.len() != 32 {
            Err(Error::CorruptedData(String::from("invalid owner id")))?
        }
        // first we need to deserialize the document and contract indices
        // we would need dedicated deserialization functions based on the document type
        let mut document: HashMap<String, CborValue> = ciborium::de::from_reader(document_cbor)
            .map_err(|_| Error::CorruptedData(String::from("unable to decode contract")))?;
        let document_id: Vec<u8> = base58_value_as_bytes_from_hash_map(&document, "$id").ok_or(
            Error::CorruptedData(String::from("unable to get document id")),
        )?;
        document.remove("$id");

        // dev-note: properties is everything other than the id
        let document = Document {
            properties: document,
            owner_id: Vec::from(owner_id),
            id: document_id,
        };
        Ok(document)
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.properties.get(key)
    }

    pub fn get_raw_for_contract<'a>(
        &'a self,
        key: &str,
        document_type_name: &str,
        contract: &Contract,
    ) -> Result<Option<Vec<u8>>, Error> {
        match self.properties.get(key) {
            None => Ok(None),
            Some(value) => {
                let document_type =
                    contract
                        .document_types
                        .get(document_type_name)
                        .ok_or(Error::CorruptedData(String::from(
                            "document type should exist for name",
                        )))?;
                Ok(Some(document_type.serialize_value_for_key(key, value)?))
            }
        }
    }
}

impl Index {
    pub fn from_cbor_value(index_type_value_map: &Vec<(Value, Value)>) -> Result<Self, Error> {
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
                .ok_or(Error::CorruptedData(String::from(
                    "key should be of type text",
                )))?;

            if key == "unique" {
                if value_value.is_bool() {
                    unique = value_value.as_bool().expect("confirmed as bool");
                }
            } else if key == "properties" {
                let properties =
                    value_value
                        .as_array()
                        .ok_or(Error::CorruptedData(String::from(
                            "property value should be an array",
                        )))?;

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
    pub fn from_cbor_value(index_property_map: &Vec<(Value, Value)>) -> Result<Self, Error> {
        let property = index_property_map[0].clone();

        let key = property
            .0 // key
            .as_text()
            .ok_or(Error::CorruptedData(String::from(
                "key should be of type string",
            )))?;
        let value = property
            .1 // value
            .as_text()
            .ok_or(Error::CorruptedData(String::from(
                "value should be of type string",
            )))?;

        let ascending = if value == "asc" { true } else { false };

        Ok(IndexProperty {
            name: key.to_string(),
            ascending,
        })
    }
}

// Helper functions
fn contract_document_types(contract: &HashMap<String, CborValue>) -> Option<&Vec<(Value, Value)>> {
    contract
        .get("documents")
        .map(|id_cbor| {
            if let CborValue::Map(documents) = id_cbor {
                Some(documents)
            } else {
                None
            }
        })
        .flatten()
}

fn get_key_from_cbor_map(cbor_map: &Vec<(Value, Value)>, key: &str) -> Option<Value> {
    for (cbor_key, cbor_value) in cbor_map.iter() {
        if !cbor_key.is_text() {
            continue;
        }

        if cbor_key.as_text().expect("confirmed as text") == key {
            return Some(cbor_value.clone());
        }
    }
    return None;
}

fn cbor_inner_array_value(document_type: &Vec<(Value, Value)>, key: &str) -> Option<Vec<Value>> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if key_value.is_array() {
        let array_value = key_value.as_array().expect("confirmed as array");
        return Some(array_value.clone());
    }
    return None;
}

fn cbor_inner_map_value(
    document_type: &Vec<(Value, Value)>,
    key: &str,
) -> Option<Vec<(Value, Value)>> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if key_value.is_map() {
        let map_value = key_value.as_map().expect("confirmed as map");
        return Some(map_value.clone());
    }
    return None;
}

fn cbor_inner_text_value(document_type: &Vec<(Value, Value)>, key: &str) -> Option<String> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if key_value.is_text() {
        let string_value = key_value.as_text().expect("confirmed as text");
        return Some(string_value.to_string());
    }
    return None;
}

fn cbor_inner_bool_value(document_type: &Vec<(Value, Value)>, key: &str) -> Option<bool> {
    let key_value = get_key_from_cbor_map(document_type, key)?;
    if key_value.is_bool() {
        let bool_value = key_value.as_bool().expect("confirmed as text");
        return Some(bool_value);
    }
    return None;
}

fn base58_value_as_bytes_from_hash_map(
    document: &HashMap<String, CborValue>,
    key: &str,
) -> Option<Vec<u8>> {
    document
        .get(key)
        .map(|id_cbor| {
            if let CborValue::Text(b) = id_cbor {
                match bs58::decode(b).into_vec() {
                    Ok(data) => Some(data),
                    Err(_) => None,
                }
            } else {
                None
            }
        })
        .flatten()
}

#[cfg(test)]
mod tests {
    use crate::contract::Contract;
    use crate::drive::Drive;
    use serde::{Deserialize, Serialize};
    use std::{collections::HashMap, fs::File, io::BufReader, path::Path};
    use tempdir::TempDir;

    fn json_document_to_cbor(path: impl AsRef<Path>) -> Vec<u8> {
        let file = File::open(path).expect("file not found");
        let reader = BufReader::new(file);
        let json: serde_json::Value =
            serde_json::from_reader(reader).expect("expected a valid json");
        let mut buffer: Vec<u8> = Vec::new();
        ciborium::ser::into_writer(&json, &mut buffer).expect("unable to serialize into cbor");
        buffer
    }

    #[test]
    fn test_cbor_deserialization() {
        let document_cbor = json_document_to_cbor("simple.json");
        let document: HashMap<String, ciborium::value::Value> =
            ciborium::de::from_reader(document_cbor.as_slice()).expect("cannot deserialize cbor");
        assert!(document.get("a").is_some());
    }

    #[test]
    fn test_import_contract() {
        let dashpay_cbor = json_document_to_cbor("test/contract/dashpay/dashpay-contract.json");
        let contract = Contract::from_cbor(&dashpay_cbor).unwrap();

        assert_eq!(contract.document_types.len(), 3);
        assert_eq!(contract.document_types.get("profile").is_some(), true);
        assert_eq!(contract.document_types.get("contactInfo").is_some(), true);
        assert_eq!(
            contract.document_types.get("contactRequest").is_some(),
            true
        );
        assert_eq!(
            contract.document_types.get("non_existent_key").is_some(),
            false
        );

        let contact_info_indices = &contract.document_types.get("contactInfo").unwrap().indices;
        assert_eq!(contact_info_indices.len(), 2);
        assert_eq!(contact_info_indices[0].unique, true);
        assert_eq!(contact_info_indices[1].unique, false);
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

        assert_eq!(contact_info_indices[0].properties[0].ascending, true);
    }
}
