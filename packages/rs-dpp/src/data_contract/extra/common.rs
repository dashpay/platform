use crate::data_contract::document_type::DocumentType;
use crate::data_contract::errors::StructureError;
use crate::document::Document;
use crate::prelude::DataContract;
use crate::util::cbor_value::cbor_value_into_json_value;
use crate::util::serializer::serializable_value_to_cbor;
use crate::ProtocolError;
use crate::ProtocolError::ValueError;
use ciborium::Value;
use platform_value::Identifier;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fs::File;
use std::io::BufReader;
use std::iter::FromIterator;
use std::path::Path;

pub fn get_key_from_cbor_map<'a>(
    cbor_map: &'a [(Value, Value)],
    key: &'a str,
) -> Option<&'a Value> {
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

/// Reads a JSON file and converts it to serde_value.
pub fn json_document_to_json_value(
    path: impl AsRef<Path>,
) -> Result<serde_json::Value, ProtocolError> {
    let file = File::open(path.as_ref()).map_err(|_| {
        ProtocolError::FileNotFound(format!(
            "file not found at path {}",
            path.as_ref().to_str().unwrap()
        ))
    })?;

    let reader = BufReader::new(file);
    serde_json::from_reader(reader)
        .map_err(|_| ProtocolError::DecodingError("error decoding value from document".to_string()))
}

/// Reads a JSON file and converts it to serde_value.
pub fn json_document_to_platform_value(
    path: impl AsRef<Path>,
) -> Result<platform_value::Value, ProtocolError> {
    let file = File::open(path.as_ref()).map_err(|_| {
        ProtocolError::FileNotFound(format!(
            "file not found at path {}",
            path.as_ref().to_str().unwrap()
        ))
    })?;

    let reader = BufReader::new(file);
    serde_json::from_reader(reader)
        .map_err(|_| ProtocolError::DecodingError("error decoding value from document".to_string()))
}

/// Reads a JSON file and converts it to CBOR.
pub fn json_document_to_cbor(
    path: impl AsRef<Path>,
    protocol_version: Option<u32>,
) -> Result<Vec<u8>, ProtocolError> {
    let json = json_document_to_json_value(path)?;
    serializable_value_to_cbor(&json, protocol_version)
}

/// Reads a JSON file and converts it a contract.
pub fn json_document_to_contract(path: impl AsRef<Path>) -> Result<DataContract, ProtocolError> {
    let file = File::open(path.as_ref()).map_err(|_| {
        ProtocolError::FileNotFound(format!(
            "file not found at path {}",
            path.as_ref().to_str().unwrap()
        ))
    })?;

    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| {
        ProtocolError::DecodingError(format!("error decoding contract from document {e}"))
    })
}

/// Reads a JSON file and converts it a document.
pub fn json_document_to_contract_with_owner_id(
    path: impl AsRef<Path>,
    owner_id: Option<Identifier>,
) -> Result<DataContract, ProtocolError> {
    let mut value = json_document_to_platform_value(path)?;
    if let Some(owner_id) = owner_id {
        value.set_value(
            "$ownerId",
            platform_value::Value::Identifier(owner_id.into_buffer()),
        )?;
    }
    DataContract::from_raw_object(value)
}

/// Reads a JSON file and converts it a document.
pub fn json_document_to_document(
    path: impl AsRef<Path>,
    owner_id: Option<Identifier>,
    document_type: &DocumentType,
) -> Result<Document, ProtocolError> {
    let mut value = json_document_to_platform_value(path)?;
    if let Some(owner_id) = owner_id {
        value.set_value(
            "$ownerId",
            platform_value::Value::Identifier(owner_id.into_buffer()),
        )?;
    }
    document_type.convert_value_to_document(value)
}

/// Make sure the protocol version is correct.
pub const fn check_protocol_version(_version: u32) -> bool {
    // Temporary disabled due protocol version is dynamic and goes from consensus params
    true
}

/// Makes sure the protocol version is correct given the version as a u8.
pub fn check_protocol_version_bytes(version_bytes: &[u8]) -> bool {
    if version_bytes.len() != 4 {
        false
    } else {
        let version_set_bytes: [u8; 4] = version_bytes
            .try_into()
            .expect("slice with incorrect length");
        let version = u32::from_be_bytes(version_set_bytes);
        check_protocol_version(version)
    }
}

pub fn reduced_value_string_representation(value: &Value) -> String {
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
