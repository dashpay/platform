use ciborium::value::{Value as CborValue, Value};
use grovedb::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::ptr::swap;
use std::rc::{Rc, Weak};
use base64::DecodeError;
use byteorder::{BigEndian, WriteBytesExt};

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
    pub(crate) indices: Vec<Index>,
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

#[derive(Clone, Serialize, Deserialize)]
pub struct IndexProperty {
    pub(crate) name: String,
    pub(crate) ascending: bool,
}

// TODO: Make the error messages uniform

impl Document {
    pub fn from_cbor(document_cbor: &[u8], owner_id: &[u8]) -> Result<Self, Error> {
        // we need to start by verifying that the owner_id is a 256 bit number (32 bytes)
        if owner_id.len() != 32 {
            Err(Error::CorruptedData(String::from("invalid owner id")))?
        }
        // first we need to deserialize the document and contract indices
        let mut document: HashMap<String, CborValue> = ciborium::de::from_reader(document_cbor)
            .map_err(|_| Error::CorruptedData(String::from("unable to decode contract")))?;
        let document_id: Vec<u8> = base58_value_as_bytes_from_hash_map(&document, "$id")
            .ok_or(Error::CorruptedData(String::from(
                "unable to get document id",
            )))?;
        document.remove("$id");
        
        let document = Document{
            properties: document,
            owner_id: Vec::from(owner_id),
            id: document_id,
        };
        Ok(document)
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.properties.get(key)
    }

    pub fn get_raw_for_contract<'a>(&'a self, key: &str, contract: &Contract) -> Option<Vec<u8>> {
        let value = self.properties.get(key)?;
        match value {
            Value::Integer(i) => {
                let mut wtr = vec![];
                wtr.write_i128::<BigEndian>(i128::from(*i));
                Some(wtr)
            }
            Value::Bytes(bytes) => {
                Some(bytes.clone())
            }
            Value::Float(f) => {
                let mut wtr = vec![];
                wtr.write_f64::<BigEndian>(*f);
                Some(wtr)
            }
            Value::Text(text) => {
                let len = text.len();
                // todo: get this for the contract and not with this stupid hack
                if len == 44 {
                    match base64::decode(text) {
                        Ok(decoded) => {
                            Some(decoded)
                        }
                        Err(_) => {
                            Some(text.as_bytes().to_vec())
                        }
                    }
                } else {
                    Some(text.as_bytes().to_vec())
                }
            }
            Value::Bool(bool) => {
                if *bool { Some(vec![1]) } else { Some(vec![0]) }
            }
            _ => {
                None
            }
        }
    }
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
}

impl DocumentType {
    pub fn from_cbor_value(document_type_value_map: &Vec<(Value, Value)>) -> Result<Self, Error> {
        let mut indices: Vec<Index> = Vec::new();

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

        Ok(DocumentType { indices })

        // for each type we should insert the indices that are top level
        // let top_level_indices = top_level_indices(index_value);
        // contract_document_types.push(DocumentType { indices: vec![] })
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

fn cbor_inner_array_value(document_type: &Vec<(Value, Value)>, key: &str) -> Option<Vec<Value>> {
    for (key_value, value_value) in document_type.iter() {
        if !key_value.is_text() {
            continue;
        }

        if key_value.as_text().expect("confirmed as text") == key {
            // Get the array value and return that
            // First check if it's actually an array
            if value_value.is_array() {
                let value_array = value_value.as_array().expect("confirmed as array").clone();
                return Some(value_array);
            } else {
                return None;
            }
        }
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
