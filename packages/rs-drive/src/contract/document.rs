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
use std::fmt;
use std::io::{BufReader, Read};

use byteorder::{BigEndian, WriteBytesExt};
use ciborium::value::Value;
use dpp::data_contract::extra::DriveContractExt;
use serde::{Deserialize, Serialize};

use crate::common::{bytes_for_system_value_from_tree_map, get_key_from_cbor_map};
use crate::contract::{reduced_value_string_representation, Contract};
use crate::drive::defaults::PROTOCOL_VERSION;
use crate::drive::Drive;
use dpp::data_contract::extra::{ContractError, DocumentType};

use crate::error::drive::DriveError;
use crate::error::structure::StructureError;
use crate::error::Error;

/// Documents contain the data that goes into data contracts.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Document {
    /// The unique document ID.
    #[serde(rename = "$id")]
    pub id: [u8; 32],

    /// The document's properties (data).
    #[serde(flatten)]
    pub properties: BTreeMap<String, Value>,

    /// The ID of the document's owner.
    #[serde(rename = "$ownerId")]
    pub owner_id: [u8; 32],
}

impl Document {
    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    pub fn serialize(&self, document_type: &DocumentType) -> Result<Vec<u8>, Error> {
        let mut buffer: Vec<u8> = self.id.as_slice().to_vec();
        buffer.extend(self.owner_id.as_slice());
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
                    Err(Error::Contract(ContractError::MissingRequiredKey(
                        "a required field is not present",
                    )))
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
    pub fn serialize_consume(mut self, document_type: &DocumentType) -> Result<Vec<u8>, Error> {
        let mut buffer: Vec<u8> = Vec::try_from(self.id).unwrap();
        let mut owner_id = Vec::try_from(self.owner_id).unwrap();
        buffer.append(&mut owner_id);
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
                    Err(Error::Contract(ContractError::MissingRequiredKey(
                        "a required field is not present",
                    )))
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
    ) -> Result<Self, Error> {
        let mut buf = BufReader::new(serialized_document);
        if serialized_document.len() < 64 {
            return Err(Error::Drive(DriveError::CorruptedSerialization(
                "serialized document is too small, must have id and owner id",
            )));
        }
        let mut id = [0; 32];
        buf.read_exact(&mut id).map_err(|_| {
            Error::Drive(DriveError::CorruptedSerialization(
                "error reading from serialized document",
            ))
        })?;

        let mut owner_id = [0; 32];
        buf.read_exact(&mut owner_id).map_err(|_| {
            Error::Drive(DriveError::CorruptedSerialization(
                "error reading from serialized document",
            ))
        })?;

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
            .collect::<Result<BTreeMap<String, Value>, ContractError>>()?;
        Ok(Document {
            id,
            properties,
            owner_id,
        })
    }

    /// Reads a CBOR-serialized document and creates a Document from it.
    /// If Document and Owner IDs are provided, they are used, otherwise they are created.
    pub fn from_cbor(
        document_cbor: &[u8],
        document_id: Option<[u8; 32]>,
        owner_id: Option<[u8; 32]>,
    ) -> Result<Self, Error> {
        let (version, read_document_cbor) = document_cbor.split_at(4);
        if !Drive::check_protocol_version_bytes(version) {
            return Err(Error::Structure(StructureError::InvalidProtocolVersion(
                "invalid protocol version",
            )));
        }
        // first we need to deserialize the document and contract indices
        // we would need dedicated deserialization functions based on the document type
        let mut document: BTreeMap<String, Value> = ciborium::de::from_reader(read_document_cbor)
            .map_err(|_| {
            Error::Structure(StructureError::InvalidCBOR("unable to decode contract"))
        })?;

        let owner_id: [u8; 32] = match owner_id {
            None => {
                let owner_id: Vec<u8> =
                    bytes_for_system_value_from_tree_map(&document, "$ownerId")?.ok_or({
                        Error::Contract(ContractError::DocumentOwnerIdMissing(
                            "unable to get document $ownerId",
                        ))
                    })?;
                document.remove("$ownerId");
                if owner_id.len() != 32 {
                    return Err(Error::Contract(ContractError::FieldRequirementUnmet(
                        "invalid owner id",
                    )));
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
                        Error::Contract(ContractError::DocumentIdMissing(
                            "unable to get document $id",
                        ))
                    })?;
                document.remove("$id");
                if document_id.len() != 32 {
                    return Err(Error::Contract(ContractError::FieldRequirementUnmet(
                        "invalid document id",
                    )));
                }
                document_id.as_slice().try_into()
            }
            Some(document_id) => {
                // we need to start by verifying that the document_id is a 256 bit number (32 bytes)
                Ok(document_id)
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

    /// Reads a CBOR-serialized document and creates a Document from it with the provided IDs.
    pub fn from_cbor_with_id(
        document_cbor: &[u8],
        document_id: &[u8],
        owner_id: &[u8],
    ) -> Result<Self, Error> {
        // we need to start by verifying that the owner_id is a 256 bit number (32 bytes)
        if owner_id.len() != 32 {
            return Err(Error::Contract(ContractError::FieldRequirementUnmet(
                "invalid owner id",
            )));
        }

        if document_id.len() != 32 {
            return Err(Error::Contract(ContractError::FieldRequirementUnmet(
                "invalid document id",
            )));
        }

        let (version, read_document_cbor) = document_cbor.split_at(4);
        if !Drive::check_protocol_version_bytes(version) {
            return Err(Error::Structure(StructureError::InvalidProtocolVersion(
                "invalid protocol version",
            )));
        }

        // first we need to deserialize the document and contract indices
        // we would need dedicated deserialization functions based on the document type
        let properties: BTreeMap<String, Value> = ciborium::de::from_reader(read_document_cbor)
            .map_err(|_| {
                Error::Structure(StructureError::InvalidCBOR("unable to decode contract"))
            })?;

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

    /// Serializes the Document to CBOR.
    pub fn to_cbor(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        buffer
            .write_u32::<BigEndian>(PROTOCOL_VERSION)
            .expect("writing protocol version caused error");
        ciborium::ser::into_writer(&self, &mut buffer).expect("unable to serialize into cbor");
        buffer
    }

    /// Return a value given the path to its key for a document type.
    pub fn get_raw_for_document_type<'a>(
        &'a self,
        key_path: &str,
        document_type: &DocumentType,
        owner_id: Option<[u8; 32]>,
    ) -> Result<Option<Vec<u8>>, Error> {
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
                Error::Contract(ContractError::MissingRequiredKey(
                    "key must not be null when getting from document",
                ))
            })?;

            /// Gets the value at the given path. Returns `value` if `key_paths` is empty.
            fn get_value_at_path<'a>(
                value: &'a Value,
                key_paths: &'a [&str],
            ) -> Result<Option<&'a Value>, Error> {
                // return value if key_paths is empty
                if key_paths.is_empty() {
                    Ok(Some(value))
                } else {
                    // split first again
                    let (key, rest_key_paths) = key_paths.split_first().ok_or({
                        Error::Contract(ContractError::MissingRequiredKey(
                            "key must not be null when getting from document",
                        ))
                    })?;
                    let map_values = value.as_map().ok_or({
                        Error::Contract(ContractError::ValueWrongType(
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
    pub fn get_raw_for_contract<'a>(
        &'a self,
        key: &str,
        document_type_name: &str,
        contract: &Contract,
        owner_id: Option<[u8; 32]>,
    ) -> Result<Option<Vec<u8>>, Error> {
        let document_type = contract.document_types().get(document_type_name).ok_or({
            Error::Contract(ContractError::DocumentTypeNotFound(
                "document type should exist for name",
            ))
        })?;
        self.get_raw_for_document_type(key, document_type, owner_id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::json_document_to_cbor;
    use crate::contract::CreateRandomDocument;
    use dpp::data_contract::extra::DriveContractExt;

    #[test]
    fn test_drive_serialization() {
        let dashpay_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            Some(1),
        );
        let contract = <Contract as DriveContractExt>::from_cbor(&dashpay_cbor, None).unwrap();

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
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            Some(1),
        );
        let contract = <Contract as DriveContractExt>::from_cbor(&dashpay_cbor, None).unwrap();

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
        let contract = <Contract as DriveContractExt>::from_cbor(&dashpay_cbor, None).unwrap();

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get profile document type");
        let document = document_type.random_document(Some(3333));

        let document_string = format!("{}", document);
        assert_eq!(document_string.as_str(), "id:2vq574DjKi7ZD8kJ6dMHxT5wu6ZKD2bW5xKAyKAGW7qZ owner_id:ChTEGXJcpyknkADUC5s6tAzvPqVG7x6Lo1Nr5mFtj2mk $createdAt:1627081806.116 $updatedAt:1575820087.909 avatarUrl:1DbW18RuyblDX7hxB38O[...(106)] displayName:rzhRkzY2L213txD6gR2S[...(21)] publicMessage:ixPGeedfb4oeyipRFe8y[...(57)] ")
    }
}
