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

pub mod serialize;

use chrono::{DateTime, NaiveDateTime, Utc};
use std::collections::{BTreeMap, HashSet};
use std::convert::TryInto;
use std::fmt;

#[cfg(feature = "cbor")]
use ciborium::Value as CborValue;
use serde_json::{json, Value as JsonValue};

use crate::data_contract::DataContract;
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::Value;
use serde::{Deserialize, Serialize};

use crate::data_contract::document_type::document_field::v0::DocumentFieldTypeV0;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::errors::DataContractError;

use crate::document::errors::DocumentError;

use crate::identity::TimestampMillis;
use crate::prelude::Identifier;
use crate::prelude::Revision;

use crate::util::hash::hash_to_vec;
use crate::util::json_value::JsonValueExt;
use crate::ProtocolError;

/// Documents contain the data that goes into data contracts.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentV0 {
    /// The unique document ID.
    pub id: Identifier,
    /// The ID of the document's owner.
    pub owner_id: Identifier,
    /// The document's properties (data).
    pub properties: BTreeMap<String, Value>,
    /// The document revision.
    pub revision: Option<Revision>,
    /// The time in milliseconds that the document was created
    pub created_at: Option<TimestampMillis>,
    /// The time in milliseconds that the document was last updated
    pub updated_at: Option<TimestampMillis>,
}

pub trait DocumentV0Methods {
    /// Return a value given the path to its key for a document type.
    fn get_raw_for_document_type<'a>(
        &'a self,
        key_path: &str,
        document_type: &DocumentType,
        owner_id: Option<[u8; 32]>,
    ) -> Result<Option<Vec<u8>>, ProtocolError>;
    /// Return a value given the path to its key and the document type for a contract.
    fn get_raw_for_contract<'a>(
        &'a self,
        key: &str,
        document_type_name: &str,
        contract: &DataContract,
        owner_id: Option<[u8; 32]>,
    ) -> Result<Option<Vec<u8>>, ProtocolError>;
    /// The document is only unique within the contract and document type
    /// Hence we must include contract and document type information to get uniqueness
    fn hash(
        &self,
        contract: &DataContract,
        document_type: &DocumentType,
    ) -> Result<Vec<u8>, ProtocolError>;
    fn increment_revision(&mut self) -> Result<(), ProtocolError>;
    fn get_identifiers_and_binary_paths<'a>(
        data_contract: &'a DataContract,
        document_type_name: &'a str,
    ) -> Result<(HashSet<&'a str>, HashSet<&'a str>), ProtocolError>;
    fn to_json_with_identifiers_using_bytes(&self) -> Result<JsonValue, ProtocolError>;
    fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError>;
    fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError>;
    fn into_value(self) -> Result<Value, ProtocolError>;
    fn to_object(&self) -> Result<Value, ProtocolError>;
    #[cfg(feature = "cbor")]
    fn to_cbor_value(&self) -> Result<CborValue, ProtocolError>;
    fn to_json(&self) -> Result<JsonValue, ProtocolError>;
    fn from_json_value<S>(document_value: JsonValue) -> Result<Self, ProtocolError>
    where
        for<'de> S: Deserialize<'de> + TryInto<Identifier, Error = ProtocolError>;
    fn from_platform_value(document_value: Value) -> Result<Self, ProtocolError>;
}

impl DocumentV0Methods for DocumentV0 {
    /// Return a value given the path to its key for a document type.
    fn get_raw_for_document_type<'a>(
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
                        .map(|time| DocumentFieldTypeV0::encode_date_timestamp(time).unwrap()))
                }
                "$updatedAt" => {
                    return Ok(self
                        .updated_at
                        .map(|time| DocumentFieldTypeV0::encode_date_timestamp(time).unwrap()))
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
    fn get_raw_for_contract<'a>(
        &'a self,
        key: &str,
        document_type_name: &str,
        contract: &DataContract,
        owner_id: Option<[u8; 32]>,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        let document_type = contract.document_types.get(document_type_name).ok_or({
            ProtocolError::DataContractError(DataContractError::DocumentTypeNotFound(
                "document type should exist for name",
            ))
        })?;
        self.get_raw_for_document_type(key, document_type, owner_id)
    }

    /// The document is only unique within the contract and document type
    /// Hence we must include contract and document type information to get uniqueness
    fn hash(
        &self,
        contract: &DataContract,
        document_type: &DocumentType,
    ) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = contract.id.to_vec();
        buf.extend(document_type.name.as_bytes());
        buf.extend(self.serialize(document_type)?);
        Ok(hash_to_vec(buf))
    }

    fn increment_revision(&mut self) -> Result<(), ProtocolError> {
        let Some(revision) = self.revision else {
            return Err(ProtocolError::Document(Box::new(DocumentError::DocumentNoRevisionError {
                document: Box::new(self.clone().into()),
            })))
        };

        let new_revision = revision
            .checked_add(1)
            .ok_or(ProtocolError::Overflow("overflow when adding 1"))?;

        self.revision = Some(new_revision);

        Ok(())
    }

    fn get_identifiers_and_binary_paths<'a>(
        data_contract: &'a DataContract,
        document_type_name: &'a str,
    ) -> Result<(HashSet<&'a str>, HashSet<&'a str>), ProtocolError> {
        let (mut identifiers_paths, binary_paths) =
            data_contract.get_identifiers_and_binary_paths(document_type_name)?;

        identifiers_paths.extend(super::IDENTIFIER_FIELDS);
        Ok((identifiers_paths, binary_paths))
    }

    fn to_json_with_identifiers_using_bytes(&self) -> Result<JsonValue, ProtocolError> {
        let mut value = json!({
            super::property_names::ID: self.id,
            super::property_names::OWNER_ID: self.owner_id,
        });
        let value_mut = value.as_object_mut().unwrap();
        if let Some(created_at) = self.created_at {
            value_mut.insert(
                super::property_names::CREATED_AT.to_string(),
                JsonValue::Number(created_at.into()),
            );
        }
        if let Some(updated_at) = self.updated_at {
            value_mut.insert(
                super::property_names::UPDATED_AT.to_string(),
                JsonValue::Number(updated_at.into()),
            );
        }
        if let Some(revision) = self.revision {
            value_mut.insert(
                super::property_names::REVISION.to_string(),
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

    fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        map.insert(super::property_names::ID.to_string(), self.id.into());
        map.insert(
            super::property_names::OWNER_ID.to_string(),
            self.owner_id.into(),
        );

        if let Some(created_at) = self.created_at {
            map.insert(
                super::property_names::CREATED_AT.to_string(),
                Value::U64(created_at),
            );
        }
        if let Some(updated_at) = self.updated_at {
            map.insert(
                super::property_names::UPDATED_AT.to_string(),
                Value::U64(updated_at),
            );
        }
        if let Some(revision) = self.revision {
            map.insert(
                super::property_names::REVISION.to_string(),
                Value::U64(revision),
            );
        }

        map.extend(self.properties.clone());

        Ok(map)
    }

    fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        map.insert(super::property_names::ID.to_string(), self.id.into());
        map.insert(
            super::property_names::OWNER_ID.to_string(),
            self.owner_id.into(),
        );

        if let Some(created_at) = self.created_at {
            map.insert(
                super::property_names::CREATED_AT.to_string(),
                Value::U64(created_at),
            );
        }
        if let Some(updated_at) = self.updated_at {
            map.insert(
                super::property_names::UPDATED_AT.to_string(),
                Value::U64(updated_at),
            );
        }
        if let Some(revision) = self.revision {
            map.insert(
                super::property_names::REVISION.to_string(),
                Value::U64(revision),
            );
        }

        map.extend(self.properties);

        Ok(map)
    }

    fn into_value(self) -> Result<Value, ProtocolError> {
        Ok(self.into_map_value()?.into())
    }

    fn to_object(&self) -> Result<Value, ProtocolError> {
        Ok(self.to_map_value()?.into())
    }

    #[cfg(feature = "cbor")]
    fn to_cbor_value(&self) -> Result<CborValue, ProtocolError> {
        self.to_object()
            .map(|v| v.try_into().map_err(ProtocolError::ValueError))?
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        self.to_object()
            .map(|v| v.try_into().map_err(ProtocolError::ValueError))?
    }

    fn from_json_value<S>(mut document_value: JsonValue) -> Result<Self, ProtocolError>
    where
        for<'de> S: Deserialize<'de> + TryInto<Identifier, Error = ProtocolError>,
    {
        let mut document = Self {
            ..Default::default()
        };

        if let Ok(value) = document_value.remove(super::property_names::ID) {
            let data: S = serde_json::from_value(value)?;
            document.id = data.try_into()?;
        }
        if let Ok(value) = document_value.remove(super::property_names::OWNER_ID) {
            let data: S = serde_json::from_value(value)?;
            document.owner_id = data.try_into()?;
        }
        if let Ok(value) = document_value.remove(super::property_names::REVISION) {
            document.revision = serde_json::from_value(value)?
        }
        if let Ok(value) = document_value.remove(super::property_names::CREATED_AT) {
            document.created_at = serde_json::from_value(value)?
        }
        if let Ok(value) = document_value.remove(super::property_names::UPDATED_AT) {
            document.updated_at = serde_json::from_value(value)?
        }

        let platform_value: Value = document_value.into();

        document.properties = platform_value
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        Ok(document)
    }

    fn from_platform_value(document_value: Value) -> Result<Self, ProtocolError> {
        let mut properties = document_value
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        let mut document = Self {
            ..Default::default()
        };

        document.id = properties.remove_identifier(super::property_names::ID)?;
        document.owner_id = properties.remove_identifier(super::property_names::OWNER_ID)?;
        document.revision = properties.remove_optional_integer(super::property_names::REVISION)?;
        document.created_at =
            properties.remove_optional_integer(super::property_names::CREATED_AT)?;
        document.updated_at =
            properties.remove_optional_integer(super::property_names::UPDATED_AT)?;

        document.properties = properties;
        Ok(document)
    }
}

impl fmt::Display for DocumentV0 {
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
