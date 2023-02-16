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

use chrono::{DateTime, NaiveDateTime, Utc};
use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::io::{BufReader, Read};

use ciborium::value::Value;
use integer_encoding::VarIntWriter;

use crate::data_contract::{DataContract, DriveContractExt};
use serde::{Deserialize, Serialize};

use crate::data_contract::document_type::{encode_unsigned_integer, DocumentType};
use crate::data_contract::errors::{DataContractError, StructureError};
use crate::data_contract::extra::common::{
    bytes_for_system_value_from_tree_map, get_key_from_cbor_map,
    reduced_value_string_representation,
};
use crate::identity::TimestampMillis;
use crate::util::deserializer;
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::ProtocolError;

/// The property names of a document
pub mod property_names {
    pub const ID: &str = "$id";
    pub const DOCUMENT_TYPE: &str = "$type";
    pub const REVISION: &str = "$revision";
    pub const OWNER_ID: &str = "$ownerId";
    pub const CREATED_AT: &str = "$createdAt";
    pub const UPDATED_AT: &str = "$updatedAt";
}

/// Documents contain the data that goes into data contracts.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Document {
    /// The unique document ID.
    #[serde(rename = "$id")]
    pub id: [u8; 32],
    /// The ID of the document's owner.
    #[serde(rename = "$ownerId")]
    pub owner_id: [u8; 32],
    /// The document's properties (data).
    #[serde(flatten)]
    pub properties: BTreeMap<String, Value>,
    /// The document revision.
    #[serde(rename = "$revision")]
    pub revision: Option<u32>,
    #[serde(rename = "$createdAt")]
    pub created_at: Option<TimestampMillis>,
    #[serde(rename = "$updatedAt")]
    pub updated_at: Option<TimestampMillis>,
}

impl Document {
    /// Return a value given the path to its key for a document type.
    pub fn get_raw_for_document_type<'a>(
        &'a self,
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
                "$createdAt" => {
                    return Ok(self
                        .created_at
                        .map(|time| encode_unsigned_integer(time).unwrap()))
                }
                "$updatedAt" => {
                    return Ok(self
                        .updated_at
                        .map(|time| encode_unsigned_integer(time).unwrap()))
                }
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
    pub fn get_raw_for_contract<'a>(
        &'a self,
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
}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "id:{} ", bs58::encode(self.id).into_string())?;
        write!(f, "owner_id:{} ", bs58::encode(self.owner_id).into_string())?;
        if let Some(created_at) = self.created_at {
            let naive = NaiveDateTime::from_timestamp_millis(created_at as i64).unwrap_or_default();
            let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
            write!(f, "created_at:{} ", datetime.format("%Y-%m-%d %H:%M:%S"))?;
        }
        if let Some(updated_at) = self.updated_at {
            let naive = NaiveDateTime::from_timestamp_millis(updated_at as i64).unwrap_or_default();
            let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
            write!(f, "updated_at:{} ", datetime.format("%Y-%m-%d %H:%M:%S"))?;
        }

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

        let recovered_document = Document::from_cbor(document_cbor.as_slice(), None, None)
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
        assert_eq!(document_string.as_str(), "id:2vq574DjKi7ZD8kJ6dMHxT5wu6ZKD2bW5xKAyKAGW7qZ owner_id:ChTEGXJcpyknkADUC5s6tAzvPqVG7x6Lo1Nr5mFtj2mk created_at:2027-09-24 14:16:54 updated_at:2030-06-20 21:52:44 avatarUrl:RD1DbW18RuyblDX7hxB3[...(1936)] displayName:jALmlamgYbnlKUkT1 publicMessage:oyGtAOjibsOvx9OUjxVO[...(110)] ")
    }
}
