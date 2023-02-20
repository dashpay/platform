// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Documents.
//!
//! This module defines the `Document` struct and implements its functions.
//!

use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::io::{BufReader, Read};

use ciborium::value::{Integer, Value};
use integer_encoding::{VarInt, VarIntReader, VarIntWriter};

use crate::data_contract::{DataContract, DriveContractExt};
use serde::{Deserialize, Serialize};

use crate::data_contract::document_type::document_type::PROTOCOL_VERSION;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::errors::{DataContractError, StructureError};
use crate::data_contract::extra::common::{
    bytes_for_system_value_from_tree_map, get_key_from_cbor_map,
    reduced_value_string_representation,
};
use crate::util::deserializer;
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::ProtocolError;

use crate::document::document_transition::INITIAL_REVISION;
use crate::prelude::*;
use crate::util::cbor_value::CborBTreeMapHelper;
use anyhow::{anyhow, bail};

//todo: rename
/// Documents contain the data that goes into data contracts.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DocumentStub {
    /// The unique document ID.
    #[serde(rename = "$id")]
    pub id: [u8; 32],

    /// The document's properties (data).
    #[serde(flatten)]
    pub properties: BTreeMap<String, Value>,

    /// The ID of the document's owner.
    #[serde(rename = "$ownerId")]
    pub owner_id: [u8; 32],

    /// The document revision.
    #[serde(rename = "$revision")]
    pub revision: Revision,
}

impl DocumentStub {
    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    pub fn serialize(&self, document_type: &DocumentType) -> Result<Vec<u8>, ProtocolError> {
        let mut buffer: Vec<u8> = self.id.as_slice().to_vec();
        buffer.extend(self.owner_id.as_slice());
        if document_type.documents_mutable {
            buffer.append(&mut self.revision.encode_var_vec());
        }
        document_type
            .properties
            .iter()
            .try_for_each(|(field_name, field)| {
                if let Some(value) = self.properties.get(field_name) {
                    let value = field
                        .document_type
                        .encode_value_ref_with_size(value, field.required)?;
                    buffer.extend(value.as_slice());
                    Ok(())
                } else if field.required {
                    Err(ProtocolError::DataContractError(
                        DataContractError::MissingRequiredKey("a required field is not present"),
                    ))
                } else {
                    // We don't have something that wasn't required
                    buffer.push(0);
                    Ok(())
                }
            })?;
        Ok(buffer)
    }

    /// Serializes and consumes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    pub fn serialize_consume(
        mut self,
        document_type: &DocumentType,
    ) -> Result<Vec<u8>, ProtocolError> {
        let mut buffer: Vec<u8> = Vec::try_from(self.id).unwrap();
        let mut owner_id = Vec::try_from(self.owner_id).unwrap();
        buffer.append(&mut owner_id);
        if document_type.documents_mutable {
            buffer.append(&mut self.revision.encode_var_vec());
        }
        document_type
            .properties
            .iter()
            .try_for_each(|(field_name, field)| {
                if let Some(value) = self.properties.remove(field_name) {
                    let value = field
                        .document_type
                        .encode_value_with_size(value, field.required)?;
                    buffer.extend(value.as_slice());
                    Ok(())
                } else if field.required {
                    Err(ProtocolError::DataContractError(
                        DataContractError::MissingRequiredKey("a required field is not present"),
                    ))
                } else {
                    // We don't have something that wasn't required
                    buffer.push(0);
                    Ok(())
                }
            })?;
        Ok(buffer)
    }

    /// Reads a serialized document and creates a Document from it.
    pub fn from_bytes(
        serialized_document: &[u8],
        document_type: &DocumentType,
    ) -> Result<Self, ProtocolError> {
        let mut buf = BufReader::new(serialized_document);
        if serialized_document.len() < 64 {
            return Err(ProtocolError::DecodingError(
                "serialized document is too small, must have id and owner id".to_string(),
            ));
        }
        let mut id = [0; 32];
        buf.read_exact(&mut id).map_err(|_| {
            ProtocolError::DecodingError("error reading id from serialized document".to_string())
        })?;

        let mut owner_id = [0; 32];
        buf.read_exact(&mut owner_id).map_err(|_| {
            ProtocolError::DecodingError(
                "error reading owner id from serialized document".to_string(),
            )
        })?;

        let revision = if document_type.documents_mutable {
            let revision: Revision = buf.read_varint().map_err(|_| {
                ProtocolError::DataContractError(DataContractError::CorruptedSerialization(
                    "error reading varint revision from serialized document",
                ))
            })?;
            revision
        } else {
            INITIAL_REVISION as Revision
        };

        let properties = document_type
            .properties
            .iter()
            .filter_map(|(key, field)| {
                let read_value = field.document_type.read_from(&mut buf, field.required);
                match read_value {
                    Ok(read_value) => read_value.map(|read_value| Ok((key.clone(), read_value))),
                    Err(e) => Some(Err(e)),
                }
            })
            .collect::<Result<BTreeMap<String, Value>, ProtocolError>>()?;
        Ok(DocumentStub {
            id,
            properties,
            owner_id,
            revision,
        })
    }

    /// Reads a CBOR-serialized document and creates a Document from it.
    /// If Document and Owner IDs are provided, they are used, otherwise they are created.
    pub fn from_cbor(
        document_cbor: &[u8],
        document_id: Option<[u8; 32]>,
        owner_id: Option<[u8; 32]>,
    ) -> Result<Self, ProtocolError> {
        let SplitProtocolVersionOutcome {
            main_message_bytes: read_document_cbor,
            ..
        } = deserializer::split_protocol_version(document_cbor)?;

        // first we need to deserialize the document and contract indices
        // we would need dedicated deserialization functions based on the document type
        let mut document: BTreeMap<String, Value> = ciborium::de::from_reader(read_document_cbor)
            .map_err(|_| {
            ProtocolError::StructureError(StructureError::InvalidCBOR(
                "unable to decode document for document call",
            ))
        })?;

        let owner_id: [u8; 32] = match owner_id {
            None => {
                let owner_id: Vec<u8> =
                    bytes_for_system_value_from_tree_map(&document, "$ownerId")?.ok_or({
                        ProtocolError::DataContractError(DataContractError::DocumentOwnerIdMissing(
                            "unable to get document $ownerId",
                        ))
                    })?;
                document.remove("$ownerId");
                if owner_id.len() != 32 {
                    return Err(ProtocolError::DataContractError(
                        DataContractError::FieldRequirementUnmet("invalid owner id"),
                    ));
                }
                owner_id.as_slice().try_into()
            }
            Some(owner_id) => Ok(owner_id),
        }
        .expect("conversion to 32bytes shouldn't fail");

        let id: [u8; 32] = match document_id {
            None => {
                let document_id: Vec<u8> = bytes_for_system_value_from_tree_map(&document, "$id")?
                    .ok_or({
                        ProtocolError::DataContractError(DataContractError::DocumentIdMissing(
                            "unable to get document $id",
                        ))
                    })?;
                document.remove("$id");
                if document_id.len() != 32 {
                    return Err(ProtocolError::DataContractError(
                        DataContractError::FieldRequirementUnmet("invalid document id"),
                    ));
                }
                document_id.as_slice().try_into()
            }
            Some(document_id) => {
                // we need to start by verifying that the document_id is a 256 bit number (32 bytes)
                Ok(document_id)
            }
        }
        .expect("document_id must be 32 bytes");

        let revision: Revision = document
            .remove_optional_integer("$revision")?
            .unwrap_or(INITIAL_REVISION as Revision);

        // dev-note: properties is everything other than the id and owner id
        Ok(DocumentStub {
            properties: document,
            owner_id,
            id,
            revision,
        })
    }

    /// Reads a CBOR-serialized document and creates a Document from it with the provided IDs.
    pub fn from_cbor_with_id(
        document_cbor: &[u8],
        document_id: &[u8],
        owner_id: &[u8],
    ) -> Result<Self, ProtocolError> {
        // we need to start by verifying that the owner_id is a 256 bit number (32 bytes)
        if owner_id.len() != 32 {
            return Err(ProtocolError::DataContractError(
                DataContractError::FieldRequirementUnmet("invalid owner id"),
            ));
        }

        if document_id.len() != 32 {
            return Err(ProtocolError::DataContractError(
                DataContractError::FieldRequirementUnmet("invalid document id"),
            ));
        }

        let SplitProtocolVersionOutcome {
            main_message_bytes: read_document_cbor,
            ..
        } = deserializer::split_protocol_version(document_cbor)?;

        // first we need to deserialize the document and contract indices
        // we would need dedicated deserialization functions based on the document type
        let properties: BTreeMap<String, Value> = ciborium::de::from_reader(read_document_cbor)
            .map_err(|_| {
                ProtocolError::StructureError(StructureError::InvalidCBOR(
                    "unable to decode contract for document call with id",
                ))
            })?;

        let revision: Revision = properties.get_integer("$revision")?;

        // dev-note: properties is everything other than the id and owner id
        Ok(DocumentStub {
            properties,
            owner_id: owner_id
                .try_into()
                .expect("try_into shouldn't fail, document_id must be 32 bytes"),
            id: document_id
                .try_into()
                .expect("try_into shouldn't fail, document_id must be 32 bytes"),
            revision,
        })
    }

    /// Serializes the Document to CBOR.
    pub fn to_cbor(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        buffer
            .write_varint(PROTOCOL_VERSION)
            .expect("writing protocol version caused error");
        ciborium::ser::into_writer(&self, &mut buffer).expect("unable to serialize into cbor");
        buffer
    }

    /// Return a value given the path to its key for a document type.
    pub fn get_raw_for_document_type(
        &self,
        key_path: &str,
        document_type: &DocumentType,
        owner_id: Option<[u8; 32]>,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        // returns the owner id if the key path is $ownerId and an owner id is given
        if key_path == "$ownerId" && owner_id.is_some() {
            Ok(Some(Vec::from(owner_id.unwrap())))
        } else {
            match key_path {
                // returns self.id or self.owner_id if key path is $id or $ownerId
                "$id" => return Ok(Some(Vec::from(self.id))),
                "$ownerId" => return Ok(Some(Vec::from(self.owner_id))),
                _ => {}
            }
            // split the key path
            let key_paths: Vec<&str> = key_path.split('.').collect::<Vec<&str>>();
            // key is the first key of the key path and rest_key_paths are the rest
            let (key, rest_key_paths) = key_paths.split_first().ok_or({
                ProtocolError::DataContractError(DataContractError::MissingRequiredKey(
                    "key must not be null when getting from document",
                ))
            })?;

            /// Gets the value at the given path. Returns `value` if `key_paths` is empty.
            fn get_value_at_path<'a>(
                value: &'a Value,
                key_paths: &'a [&str],
            ) -> Result<Option<&'a Value>, ProtocolError> {
                // return value if key_paths is empty
                if key_paths.is_empty() {
                    Ok(Some(value))
                } else {
                    // split first again
                    let (key, rest_key_paths) = key_paths.split_first().ok_or({
                        ProtocolError::DataContractError(DataContractError::MissingRequiredKey(
                            "key must not be null when getting from document",
                        ))
                    })?;
                    let map_values = value.as_map().ok_or({
                        ProtocolError::DataContractError(DataContractError::ValueWrongType(
                            "inner key must refer to a value map",
                        ))
                    })?;
                    // given a map of values and a key, get the corresponding value
                    match get_key_from_cbor_map(map_values, key) {
                        None => Ok(None),
                        Some(value) => get_value_at_path(value, rest_key_paths),
                    }
                }
            }

            // match the value at the given key
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

    /// Return a value given the path to its key and the document type for a contract.
    pub fn get_raw_for_contract(
        &self,
        key: &str,
        document_type_name: &str,
        contract: &DataContract,
        owner_id: Option<[u8; 32]>,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        let document_type = contract.document_types().get(document_type_name).ok_or({
            ProtocolError::DataContractError(DataContractError::DocumentTypeNotFound(
                "document type should exist for name",
            ))
        })?;
        self.get_raw_for_document_type(key, document_type, owner_id)
    }

    /// Temporary helper method to get property in u64 format
    /// Imitating JsonValueExt trait
    pub fn get_u64(&self, property_name: &str) -> Result<u64, anyhow::Error> {
        let property_value = self.properties.get(property_name).ok_or_else(|| {
            anyhow!(
                "the property '{}' doesn't exist in '{:?}'",
                property_name,
                self
            )
        })?;

        if let Value::Integer(s) = property_value {
            return (*s)
                .try_into()
                .map_err(|_| anyhow!("unable convert {} to u64", property_name));
        }
        bail!(
            "getting property '{}' failed: {:?} isn't a number",
            property_name,
            property_value
        );
    }

    /// Temporary helper method to get property in u32 format
    /// Imitating JsonValueExt trait
    pub fn get_u32(&self, property_name: &str) -> Result<u32, ProtocolError> {
        let property_value =
            self.properties
                .get(property_name)
                .ok_or(ProtocolError::DocumentKeyMissing(format!(
                    "the property '{}' doesn't exist in '{:?}'",
                    property_name, self
                )))?;

        if let Value::Integer(s) = property_value {
            (*s).try_into()
                .map_err(|_| ProtocolError::DecodingError("expected a u32 integer".to_string()))
        } else {
            Err(ProtocolError::DecodingError(
                "expected an integer".to_string(),
            ))
        }
    }

    /// Temporary helper method to get property in bytes format
    /// Imitating JsonValueExt trait
    pub fn get_bytes(&self, property_name: &str) -> Result<Vec<u8>, anyhow::Error> {
        let property_value = self.properties.get(property_name).ok_or_else(|| {
            anyhow!(
                "the property '{}' doesn't exist in '{:?}'",
                property_name,
                self
            )
        })?;

        if let Value::Bytes(s) = property_value {
            return Ok(s.clone());
        }
        bail!(
            "getting property '{}' failed: {:?} isn't an array of bytes",
            property_name,
            property_value
        );
    }

    pub fn set_u8(&mut self, property_name: &str, value: u8) {
        self.properties.insert(
            property_name.to_string(),
            Value::Integer(Integer::from(value)),
        );
    }

    pub fn set_i64(&mut self, property_name: &str, value: i64) {
        self.properties.insert(
            property_name.to_string(),
            Value::Integer(Integer::from(value)),
        );
    }

    pub fn set_bytes(&mut self, property_name: &str, value: Vec<u8>) {
        self.properties
            .insert(property_name.to_string(), Value::Bytes(value));
    }

    pub fn increment_revision(&mut self) -> Result<(), ProtocolError> {
        let revision = self.revision;

        let new_revision = revision
            .checked_add(1)
            .ok_or(ProtocolError::Overflow("overflow when adding 1"))?;

        self.revision = new_revision;

        Ok(())
    }
}

impl fmt::Display for DocumentStub {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::data_contract::extra::common::json_document_to_cbor;

    #[test]
    fn test_serialization() {
        let dashpay_cbor = json_document_to_cbor(
            "../rs-dpp/src/tests/payloads/contract/dashpay-contract.json",
            Some(1),
        )
        .expect("expected to get cbor contract");
        let contract = <DataContract as DriveContractExt>::from_cbor(&dashpay_cbor, None).unwrap();

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get profile document type");
        let document = document_type.random_document(Some(3333));

        let document_cbor = document.to_cbor();

        let serialized_document = document
            .serialize(document_type)
            .expect("expected to serialize");

        let deserialized_document = document_type
            .document_from_bytes(serialized_document.as_slice())
            .expect("expected to deserialize a document");
        assert_eq!(document, deserialized_document);
        assert!(serialized_document.len() < document_cbor.len());
        for _i in 0..10000 {
            let document = document_type.random_document(Some(3333));
            let _serialized_document = document
                .serialize_consume(document_type)
                .expect("expected to serialize");
        }
    }

    #[test]
    fn test_document_cbor_serialization() {
        let dashpay_cbor = json_document_to_cbor(
            "../rs-dpp/src/tests/payloads/contract/dashpay-contract.json",
            Some(1),
        )
        .expect("expected to get cbor contract");
        let contract = <DataContract as DriveContractExt>::from_cbor(&dashpay_cbor, None).unwrap();

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get profile document type");
        let document = document_type.random_document(Some(3333));

        let document_cbor = document.to_cbor();

        let recovered_document = DocumentStub::from_cbor(document_cbor.as_slice(), None, None)
            .expect("expected to get document");

        assert_eq!(recovered_document, document);
    }

    #[test]
    fn test_document_display() {
        let dashpay_cbor = json_document_to_cbor(
            "../rs-dpp/src/tests/payloads/contract/dashpay-contract.json",
            Some(1),
        )
        .expect("expected to get cbor contract");
        let contract = <DataContract as DriveContractExt>::from_cbor(&dashpay_cbor, None).unwrap();

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get profile document type");
        let document = document_type.random_document(Some(3333));

        let document_string = format!("{}", document);
        assert_eq!(document_string.as_str(), "id:2vq574DjKi7ZD8kJ6dMHxT5wu6ZKD2bW5xKAyKAGW7qZ owner_id:ChTEGXJcpyknkADUC5s6tAzvPqVG7x6Lo1Nr5mFtj2mk $createdAt:1627081806.116 $updatedAt:1575820087.909 avatarUrl:1DbW18RuyblDX7hxB38O[...(106)] displayName:rzhRkzY2L213txD6gR2S[...(21)] publicMessage:ixPGeedfb4oeyipRFe8y[...(57)] ")
    }
}
