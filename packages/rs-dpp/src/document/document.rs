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
use std::collections::{BTreeMap, HashSet};
use std::convert::TryInto;
use std::fmt;

use ciborium::Value as CborValue;
use serde_json::{json, Value as JsonValue};

use crate::data_contract::{DataContract, DriveContractExt};
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::Value;
use serde::{Deserialize, Serialize};

use crate::data_contract::document_type::{encode_date_timestamp, DocumentType};
use crate::data_contract::errors::DataContractError;

use crate::document::errors::DocumentError;

use crate::identity::TimestampMillis;
use crate::prelude::Identifier;
use crate::prelude::Revision;

use crate::util::hash::hash;
use crate::util::json_value::JsonValueExt;
use crate::ProtocolError;

/// The property names of a document
pub mod property_names {
    pub const ID: &str = "$id";
    pub const REVISION: &str = "$revision";
    pub const OWNER_ID: &str = "$ownerId";
    pub const CREATED_AT: &str = "$createdAt";
    pub const UPDATED_AT: &str = "$updatedAt";
}

pub const IDENTIFIER_FIELDS: [&str; 2] = [property_names::ID, property_names::OWNER_ID];

/// Documents contain the data that goes into data contracts.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Document {
    //todo: add an optional version
    /// The unique document ID.
    #[serde(rename = "$id")]
    pub id: Identifier,
    /// The ID of the document's owner.
    #[serde(rename = "$ownerId")]
    pub owner_id: Identifier,
    /// The document's properties (data).
    #[serde(flatten)]
    pub properties: BTreeMap<String, Value>,
    /// The document revision.
    #[serde(rename = "$revision")]
    pub revision: Option<Revision>,
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
        // todo: maybe merge with document_type.serialize_value_for_key() because we use different
        //   code paths for query and index creation
        // returns the owner id if the key path is $ownerId and an owner id is given
        if key_path == "$ownerId" && owner_id.is_some() {
            Ok(Some(Vec::from(owner_id.unwrap())))
        } else {
            match key_path {
                // returns self.id or self.owner_id if key path is $id or $ownerId
                "$id" => return Ok(Some(self.id.to_vec())),
                "$ownerId" => return Ok(Some(self.owner_id.to_vec())),
                "$createdAt" => {
                    return Ok(self
                        .created_at
                        .map(|time| encode_date_timestamp(time).unwrap()))
                }
                "$updatedAt" => {
                    return Ok(self
                        .updated_at
                        .map(|time| encode_date_timestamp(time).unwrap()))
                }
                _ => {}
            }
            self.properties
                .get_optional_at_path(key_path)?
                .map(|value| document_type.serialize_value_for_key(key_path, value))
                .transpose()
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

    /// Set the value under given path.
    /// The path supports syntax from `lodash` JS lib. Example: "root.people[0].name".
    /// If parents are not present they will be automatically created
    pub fn set(&mut self, path: &str, value: Value) {
        self.properties.insert(path.to_string(), value);
    }

    /// Retrieves field specified by path
    pub fn get(&self, path: &str) -> Option<&Value> {
        // this can only error if path is empty, to which we just return None
        self.properties.get_optional_at_path(path).ok().flatten()
    }

    pub fn set_u8(&mut self, property_name: &str, value: u8) {
        self.properties
            .insert(property_name.to_string(), Value::U8(value));
    }

    pub fn set_i8(&mut self, property_name: &str, value: i8) {
        self.properties
            .insert(property_name.to_string(), Value::I8(value));
    }

    pub fn set_u16(&mut self, property_name: &str, value: u16) {
        self.properties
            .insert(property_name.to_string(), Value::U16(value));
    }

    pub fn set_i16(&mut self, property_name: &str, value: i16) {
        self.properties
            .insert(property_name.to_string(), Value::I16(value));
    }

    pub fn set_u32(&mut self, property_name: &str, value: u32) {
        self.properties
            .insert(property_name.to_string(), Value::U32(value));
    }

    pub fn set_i32(&mut self, property_name: &str, value: i32) {
        self.properties
            .insert(property_name.to_string(), Value::I32(value));
    }

    pub fn set_u64(&mut self, property_name: &str, value: u64) {
        self.properties
            .insert(property_name.to_string(), Value::U64(value));
    }

    pub fn set_i64(&mut self, property_name: &str, value: i64) {
        self.properties
            .insert(property_name.to_string(), Value::I64(value));
    }

    pub fn set_bytes(&mut self, property_name: &str, value: Vec<u8>) {
        self.properties
            .insert(property_name.to_string(), Value::Bytes(value));
    }

    /// The document is only unique within the contract and document type
    /// Hence we must include contract and document type information to get uniqueness
    pub fn hash(
        &self,
        contract: &DataContract,
        document_type: &DocumentType,
    ) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = contract.id.to_vec();
        buf.extend(document_type.name.as_bytes());
        buf.extend(self.serialize(document_type)?);
        Ok(hash(buf))
    }

    pub fn increment_revision(&mut self) -> Result<(), ProtocolError> {
        let Some(revision) = self.revision else {
            return Err(ProtocolError::Document(Box::new(DocumentError::DocumentNoRevisionError {
                document: Box::new(self.clone()),
            })))
        };

        let new_revision = revision
            .checked_add(1)
            .ok_or(ProtocolError::Overflow("overflow when adding 1"))?;

        self.revision = Some(new_revision);

        Ok(())
    }

    pub fn get_identifiers_and_binary_paths<'a>(
        data_contract: &'a DataContract,
        document_type_name: &'a str,
    ) -> Result<(HashSet<&'a str>, HashSet<&'a str>), ProtocolError> {
        let (mut identifiers_paths, binary_paths) =
            data_contract.get_identifiers_and_binary_paths(document_type_name)?;

        identifiers_paths.extend(IDENTIFIER_FIELDS);
        Ok((identifiers_paths, binary_paths))
    }

    pub fn to_json_with_identifiers_using_bytes(&self) -> Result<JsonValue, ProtocolError> {
        let mut value = json!({
            property_names::ID: self.id,
            property_names::OWNER_ID: self.owner_id,
        });
        let value_mut = value.as_object_mut().unwrap();
        if let Some(created_at) = self.created_at {
            value_mut.insert(
                property_names::CREATED_AT.to_string(),
                JsonValue::Number(created_at.into()),
            );
        }
        if let Some(updated_at) = self.updated_at {
            value_mut.insert(
                property_names::UPDATED_AT.to_string(),
                JsonValue::Number(updated_at.into()),
            );
        }
        if let Some(revision) = self.revision {
            value_mut.insert(
                property_names::REVISION.to_string(),
                JsonValue::Number(revision.into()),
            );
        }

        self.properties
            .iter()
            .try_for_each(|(key, property_value)| {
                let serde_value: JsonValue = property_value.try_to_validating_json()?;
                value_mut.insert(key.to_string(), serde_value);
                Ok::<(), ProtocolError>(())
            })?;

        Ok(value)
    }

    pub fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        map.insert(property_names::ID.to_string(), self.id.into());
        map.insert(property_names::OWNER_ID.to_string(), self.owner_id.into());

        if let Some(created_at) = self.created_at {
            map.insert(
                property_names::CREATED_AT.to_string(),
                Value::U64(created_at),
            );
        }
        if let Some(updated_at) = self.updated_at {
            map.insert(
                property_names::UPDATED_AT.to_string(),
                Value::U64(updated_at),
            );
        }
        if let Some(revision) = self.revision {
            map.insert(property_names::REVISION.to_string(), Value::U64(revision));
        }

        map.extend(self.properties.clone());

        Ok(map)
    }

    pub fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        map.insert(property_names::ID.to_string(), self.id.into());
        map.insert(property_names::OWNER_ID.to_string(), self.owner_id.into());

        if let Some(created_at) = self.created_at {
            map.insert(
                property_names::CREATED_AT.to_string(),
                Value::U64(created_at),
            );
        }
        if let Some(updated_at) = self.updated_at {
            map.insert(
                property_names::UPDATED_AT.to_string(),
                Value::U64(updated_at),
            );
        }
        if let Some(revision) = self.revision {
            map.insert(property_names::REVISION.to_string(), Value::U64(revision));
        }

        map.extend(self.properties);

        Ok(map)
    }

    pub fn into_value(self) -> Result<Value, ProtocolError> {
        Ok(self.into_map_value()?.into())
    }

    pub fn to_object(&self) -> Result<Value, ProtocolError> {
        Ok(self.to_map_value()?.into())
    }

    pub fn to_cbor_value(&self) -> Result<CborValue, ProtocolError> {
        self.to_object()
            .map(|v| v.try_into().map_err(ProtocolError::ValueError))?
    }

    pub fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        self.to_object()
            .map(|v| v.try_into().map_err(ProtocolError::ValueError))?
    }

    pub fn from_json_value<S>(mut document_value: JsonValue) -> Result<Self, ProtocolError>
    where
        for<'de> S: Deserialize<'de> + TryInto<Identifier, Error = ProtocolError>,
    {
        let mut document = Self {
            ..Default::default()
        };

        if let Ok(value) = document_value.remove(property_names::ID) {
            let data: S = serde_json::from_value(value)?;
            document.id = data.try_into()?;
        }
        if let Ok(value) = document_value.remove(property_names::OWNER_ID) {
            let data: S = serde_json::from_value(value)?;
            document.owner_id = data.try_into()?;
        }
        if let Ok(value) = document_value.remove(property_names::REVISION) {
            document.revision = serde_json::from_value(value)?
        }
        if let Ok(value) = document_value.remove(property_names::CREATED_AT) {
            document.created_at = serde_json::from_value(value)?
        }
        if let Ok(value) = document_value.remove(property_names::UPDATED_AT) {
            document.updated_at = serde_json::from_value(value)?
        }

        let platform_value: Value = document_value.into();

        document.properties = platform_value
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        Ok(document)
    }

    pub fn from_platform_value(document_value: Value) -> Result<Self, ProtocolError> {
        let mut properties = document_value
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        let mut document = Self {
            ..Default::default()
        };

        document.id = properties.remove_identifier(property_names::ID)?;
        document.owner_id = properties.remove_identifier(property_names::OWNER_ID)?;
        document.revision = properties.remove_optional_integer(property_names::REVISION)?;
        document.created_at = properties.remove_optional_integer(property_names::CREATED_AT)?;
        document.updated_at = properties.remove_optional_integer(property_names::UPDATED_AT)?;

        document.properties = properties;
        Ok(document)
    }
}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "id:{} ", self.id)?;
        write!(f, "owner_id:{} ", self.owner_id)?;
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
                write!(f, "{}:{} ", key, value)?
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

        let document_cbor = document.to_cbor().expect("expected to encode to cbor");

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

        let document_cbor = document.to_cbor().expect("expected to encode to cbor");

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
        assert_eq!(document_string.as_str(), "id:2vq574DjKi7ZD8kJ6dMHxT5wu6ZKD2bW5xKAyKAGW7qZ owner_id:ChTEGXJcpyknkADUC5s6tAzvPqVG7x6Lo1Nr5mFtj2mk created_at:2027-09-24 14:16:54 updated_at:2030-06-20 21:52:44 avatarUrl:string RD1DbW18RuyblDX7hxB3[...(1936)] displayName:string jALmlamgYbnlKUkT1 publicMessage:string oyGtAOjibsOvx9OUjxVO[...(110)] ")
    }
}
