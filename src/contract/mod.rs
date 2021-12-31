use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use ciborium::value::{Value as CborValue, Value};
use grovedb::Error;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct IndexProperty {
    name: String,
    ascending: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Index {
    properties: Vec<IndexProperty>,
    unique: bool,
}
    
impl Index {
    pub fn from_cbor_value(index_type_value_map: &Vec<(Value, Value)) -> Result<Self, Error> {
        
    }
}

#[derive(Serialize, Deserialize)]
pub struct DocumentType {
    indices: Vec<Index>,
}

impl DocumentType {
    pub fn from_cbor_value(document_type_value_map: &Vec<(Value, Value)) -> Result<Self, Error> {
        let index_values= cbor_inner_array_value(&document_type_value_map, "indices").ok_or(Error::CorruptedData(String::from(
            "unable to get indices from the contract",
        )))?;

        for index_value in index_values {
            let CborValue::Map(index_map) = index_value;
            if !index_map {
                Err(Error::CorruptedData(String::from(
                    "table document is not a map as expected",
                )))
            }
        }

        // for each type we should insert the indices that are top level
        let top_level_indices = top_level_indices(index_value);
        contract_document_types.push(DocumentType{
            indices: vec![]
        })
    }

    pub fn top_level_indices(
        &mut self,
    ) -> Result<(Vec<Index>), Error> {
        self.indices.iter().map(|index | {
            index.properties.get(0)
        }).collect()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Contract {
    document_types: HashMap<String, DocumentType>,
    id: vec<u8>,
}

fn contract_document_types(contract: &HashMap<String, CborValue>) -> Option<&Vec<(Value, Value)>> {
    contract
        .get("documents")
        .map(|id_cbor| {
            if let CborValue::Map(documents) = id_cbor {
                Some(documents)
            } else {
                None
            }
        }).flatten()
}

fn cbor_inner_map_value(document_type: &Vec<(Value, Value)>, key: &str) -> Option<Vec<(Value, Value)>> {
    for (key_value, value_value) in document_type.iter() {
        let CborValue::Text(tuple_key) = key_value;
        if !key {
            None
        }
        if tuple_key == key {
            let CborValue::Map(value_map) = value_value;
            if !value_map {
                None
            }
            Some(value_map)
        }
    }
    None
}

fn cbor_inner_array_value(document_type: &Vec<(Value, Value)>, key: &str) -> Option<Vec<(Value)>> {
    for (key_value, value_value) in document_type.iter() {
        let CborValue::Text(tuple_key) = key_value;
        if !key {
            None
        }
        if tuple_key == key {
            let CborValue::Array(value_map) = value_value;
            if !value_map {
                None
            }
            Some(value_map)
        }
    }
    None
}

fn indices_from_values(values: &Vec<(Value)>) -> Option<Vec<Index>> {
    values.iter().map(|id_cbor| {
        if let CborValue::Map(documents) = id_cbor {
            Some(documents)
        } else {
            None
        }
    }).flatten()
}

pub fn json_document_to_cbor(path: impl AsRef<Path>) -> Vec<u8> {
    let file = File::open(path).expect("file not found");
    let reader = BufReader::new(file);
    let json: serde_json::Value =
        serde_json::from_reader(reader).expect("expected a valid json");
    let mut buffer: Vec<u8> = Vec::new();
    ciborium::ser::into_writer(&json, &mut buffer).expect("unable to serialize into cbor");
    buffer
}

impl Contract {
    pub fn from_cbor(contract_cbor: contract_cbor) -> Result<Self, Error> {
        // first we need to deserialize the document and contract indices
        let contract: HashMap<String, CborValue> = ciborium::de::from_reader(contract_cbor)
            .map_err(|_| Error::CorruptedData(String::from("unable to decode contract")))?;
        
        let contract_document_types_raw = contract_document_types(&contract).ok_or(Error::CorruptedData(String::from(
            "unable to get documents from contract",
        )))?;
        
        let mut contract_document_types : HashMap<String, DocumentType> = HashMap::new();
        
        for (type_key_value, document_type_value) in contract_document_types_raw {
            

            let CborValue::Text(type_key) = type_key_value;
            if !type_key {
                Err(Error::CorruptedData(String::from(
                    "table type is not a string as expected",
                )))
            }
            let CborValue::Map(document_type) = document_type_value;
            if !document_type {
                Err(Error::CorruptedData(String::from(
                    "table document is not a map as expected",
                )))
            }

            let document_type = DocumentType::from_cbor_value(document_type)?;
            contract_document_types.insert(type_key, document_type);
        }
        let mut contract = Contract{ document_types: contract_document_types, id: () };
    }


}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use serde::{Deserialize, Serialize};
    use std::{collections::HashMap, fs::File, io::BufReader, path::Path};
    use tempdir::TempDir;
    use crate::contract::{Contract, json_document_to_cbor};

    #[test]
    fn test_cbor_deserialization() {
        let document_cbor = json_document_to_cbor("simple.json");
        let document: HashMap<String, ciborium::value::Value> =
            ciborium::de::from_reader(document_cbor.as_slice()).expect("cannot deserialize cbor");
        assert!(document.get("a").is_some());
    }

    #[test]
    fn test_import_contract() {
        let dashpay_cbor = json_document_to_cbor("dashpay-contract.json");
        let contract = Contract::from_cbor(dashpay_cbor)?;
    }
}