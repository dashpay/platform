use crate::data_contract::document_type::document_type::PROTOCOL_VERSION;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::errors::{DataContractError, StructureError};

use crate::document::document::property_names;
use crate::document::document::property_names::{CREATED_AT, UPDATED_AT};

use crate::document::Document;

use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use crate::util::deserializer;
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::ProtocolError;

use byteorder::{BigEndian, ReadBytesExt};
#[cfg(feature = "cbor")]
use ciborium::Value as CborValue;
use integer_encoding::{VarInt, VarIntReader, VarIntWriter};
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::io::{BufReader, Read};

#[cfg(feature = "cbor")]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DocumentForCbor {
    /// The unique document ID.
    #[serde(rename = "$id")]
    pub id: [u8; 32],

    /// The document's properties (data).
    #[serde(flatten)]
    pub properties: BTreeMap<String, CborValue>,

    /// The ID of the document's owner.
    #[serde(rename = "$ownerId")]
    pub owner_id: [u8; 32],

    /// The document revision.
    #[serde(rename = "$revision")]
    pub revision: Option<Revision>,

    #[serde(rename = "$createdAt")]
    pub created_at: Option<TimestampMillis>,
    #[serde(rename = "$updatedAt")]
    pub updated_at: Option<TimestampMillis>,
}

#[cfg(feature = "cbor")]
impl TryFrom<Document> for DocumentForCbor {
    type Error = ProtocolError;

    fn try_from(value: Document) -> Result<Self, Self::Error> {
        let Document {
            id,
            properties,
            owner_id,
            revision,
            created_at,
            updated_at,
        } = value;
        Ok(DocumentForCbor {
            id: id.to_buffer(),
            properties: Value::convert_to_cbor_map(properties)
                .map_err(ProtocolError::ValueError)?,
            owner_id: owner_id.to_buffer(),
            revision,
            created_at,
            updated_at,
        })
    }
}

impl Document {
    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    pub fn serialize(&self, document_type: &DocumentType) -> Result<Vec<u8>, ProtocolError> {
        let mut buffer: Vec<u8> = self.id.as_slice().to_vec();
        buffer.extend(self.owner_id.as_slice());
        if let Some(revision) = self.revision {
            buffer.extend(revision.encode_var_vec())
        } else if document_type.requires_revision() {
            buffer.extend((1 as Revision).encode_var_vec())
        }
        document_type
            .properties
            .iter()
            .try_for_each(|(field_name, field)| {
                if field_name == CREATED_AT {
                    if let Some(created_at) = self.created_at {
                        if !field.required {
                            buffer.push(1);
                        }
                        // dbg!("we pushed created at {}", hex::encode(created_at.to_be_bytes()));
                        buffer.extend(created_at.to_be_bytes());
                        Ok(())
                    } else if field.required {
                        Err(ProtocolError::DataContractError(
                            DataContractError::MissingRequiredKey(
                                "created at field is not present".to_string(),
                            ),
                        ))
                    } else {
                        // dbg!("we pushed created at with 0");
                        // We don't have the created_at that wasn't required
                        buffer.push(0);
                        Ok(())
                    }
                } else if field_name == UPDATED_AT {
                    if let Some(updated_at) = self.updated_at {
                        if !field.required {
                            // dbg!("we added 1", field_name);
                            buffer.push(1);
                        }
                        // dbg!("we pushed updated at {}", hex::encode(updated_at.to_be_bytes()));
                        buffer.extend(updated_at.to_be_bytes());
                        Ok(())
                    } else if field.required {
                        Err(ProtocolError::DataContractError(
                            DataContractError::MissingRequiredKey(
                                "updated at field is not present".to_string(),
                            ),
                        ))
                    } else {
                        // dbg!("we pushed updated at with 0");
                        // We don't have the updated_at that wasn't required
                        buffer.push(0);
                        Ok(())
                    }
                } else if let Some(value) = self.properties.get(field_name) {
                    if value.is_null() {
                        if field.required {
                            Err(ProtocolError::DataContractError(
                                DataContractError::MissingRequiredKey("a required field is not present".to_string()),
                            ))
                        } else {
                            // dbg!("we pushed {} with 0", field_name);
                            // We don't have something that wasn't required
                            buffer.push(0);
                            Ok(())
                        }
                    } else {
                        if !field.required {
                            // dbg!("we added 1", field_name);
                            buffer.push(1);
                        }
                        let value = field
                            .document_type
                            .encode_value_ref_with_size(value, field.required)?;
                        // dbg!("we pushed {} with {}", field_name, hex::encode(&value));
                        buffer.extend(value.as_slice());
                        Ok(())
                    }
                } else if field.required {
                    Err(ProtocolError::DataContractError(
                        DataContractError::MissingRequiredKey(format!("a required field {field_name} is not present")),
                    ))
                } else {
                    // dbg!("we pushed {} with 0", field_name);
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
        let mut buffer: Vec<u8> = self.id.to_vec();
        let mut owner_id = self.owner_id.to_vec();
        buffer.append(&mut owner_id);

        if let Some(revision) = self.revision {
            buffer.extend(revision.to_be_bytes())
        }
        document_type
            .flattened_properties
            .iter()
            .try_for_each(|(field_name, field)| {
                if field_name == CREATED_AT {
                    if let Some(created_at) = self.created_at {
                        buffer.extend(created_at.to_be_bytes());
                        Ok(())
                    } else if field.required {
                        Err(ProtocolError::DataContractError(
                            DataContractError::MissingRequiredKey(
                                "created at field is not present".to_string(),
                            ),
                        ))
                    } else {
                        // We don't have the created_at that wasn't required
                        buffer.push(0);
                        Ok(())
                    }
                } else if field_name == UPDATED_AT {
                    if let Some(updated_at) = self.updated_at {
                        buffer.extend(updated_at.to_be_bytes());
                        Ok(())
                    } else if field.required {
                        Err(ProtocolError::DataContractError(
                            DataContractError::MissingRequiredKey(
                                "created at field is not present".to_string(),
                            ),
                        ))
                    } else {
                        // We don't have the updated_at that wasn't required
                        buffer.push(0);
                        Ok(())
                    }
                } else if let Some(value) = self.properties.remove(field_name) {
                    let value = field
                        .document_type
                        .encode_value_with_size(value, field.required)?;
                    buffer.extend(value.as_slice());
                    Ok(())
                } else if field.required {
                    Err(ProtocolError::DataContractError(
                        DataContractError::MissingRequiredKey("a required field is not present".to_string()),
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
            ProtocolError::DecodingError(
                "error reading from serialized document for id".to_string(),
            )
        })?;

        let mut owner_id = [0; 32];
        buf.read_exact(&mut owner_id).map_err(|_| {
            ProtocolError::DecodingError(
                "error reading from serialized document for owner id".to_string(),
            )
        })?;

        // if the document type is mutable then we should deserialize the revision
        let revision: Option<Revision> = if document_type.requires_revision() {
            let revision = buf.read_varint().map_err(|_| {
                ProtocolError::DecodingError(
                    "error reading revision from serialized document for revision".to_string(),
                )
            })?;
            Some(revision)
        } else {
            None
        };
        let mut created_at = None;
        let mut updated_at = None;
        let properties = document_type
            .properties
            .iter()
            .filter_map(|(key, field)| {
                if key == CREATED_AT {
                    if !field.required {
                        let marker_result = buf.read_u8().map_err(|_| {
                            ProtocolError::DataContractError(DataContractError::CorruptedSerialization(
                                "error reading created at optional byte from serialized document",
                            ))
                        });
                        match marker_result {
                            Ok(marker) => {
                                if marker == 0 {
                                    return Some(Ok((key.clone(), Value::Null)));
                                }
                            }
                            Err(e) => return Some(Err(e)),
                        }
                    }
                    let integer_result = buf.read_u64::<BigEndian>().map_err(|_| {
                        ProtocolError::DataContractError(DataContractError::CorruptedSerialization(
                            "error reading created at from serialized document",
                        ))
                    });
                    match integer_result {
                        Ok(integer) => {
                            created_at = Some(integer);
                            None
                        }
                        Err(e) => Some(Err(e)),
                    }
                } else if key == UPDATED_AT {
                    if !field.required {
                        let marker_result = buf.read_u8().map_err(|_| {
                            ProtocolError::DataContractError(DataContractError::CorruptedSerialization(
                                "error reading updated at optional byte from serialized document",
                            ))
                        });
                        match marker_result {
                            Ok(marker) => {
                                if marker == 0 {
                                    return Some(Ok((key.clone(), Value::Null)));
                                }
                            }
                            Err(e) => return Some(Err(e)),
                        }
                    }
                    let integer_result = buf.read_u64::<BigEndian>().map_err(|_| {
                        ProtocolError::DataContractError(DataContractError::CorruptedSerialization(
                            "error reading updated at from serialized document",
                        ))
                    });
                    match integer_result {
                        Ok(integer) => {
                            updated_at = Some(integer);
                            None
                        }
                        Err(e) => Some(Err(e)),
                    }
                } else {
                    let read_value = field.document_type.read_from(&mut buf, field.required);
                    match read_value {
                        Ok(read_value) => read_value.map(|read_value| Ok((key.clone(), read_value))),
                        Err(e) => Some(Err(e)),
                    }
                }
            })
            .collect::<Result<BTreeMap<String, Value>, ProtocolError>>()?;
        Ok(Document {
            id: Identifier::new(id),
            properties,
            owner_id: Identifier::new(owner_id),
            revision,
            created_at,
            updated_at,
        })
    }

    /// Reads a CBOR-serialized document and creates a Document from it.
    /// If Document and Owner IDs are provided, they are used, otherwise they are created.
    #[cfg(feature = "cbor")]
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
        let document_cbor_map: BTreeMap<String, CborValue> =
            ciborium::de::from_reader(read_document_cbor).map_err(|_| {
                ProtocolError::StructureError(StructureError::InvalidCBOR(
                    "unable to decode document for document call",
                ))
            })?;
        let document_map: BTreeMap<String, Value> =
            Value::convert_from_cbor_map(document_cbor_map).map_err(ProtocolError::ValueError)?;

        Self::from_map(document_map, document_id, owner_id)
    }

    /// Reads a CBOR-serialized document and creates a Document from it.
    /// If Document and Owner IDs are provided, they are used, otherwise they are created.
    pub fn from_map(
        mut document_map: BTreeMap<String, Value>,
        document_id: Option<[u8; 32]>,
        owner_id: Option<[u8; 32]>,
    ) -> Result<Self, ProtocolError> {
        let owner_id = match owner_id {
            None => document_map
                .remove_hash256_bytes(property_names::OWNER_ID)
                .map_err(ProtocolError::ValueError)?,
            Some(owner_id) => owner_id,
        };

        let id = match document_id {
            None => document_map
                .remove_hash256_bytes(property_names::ID)
                .map_err(ProtocolError::ValueError)?,
            Some(document_id) => document_id,
        };

        let revision = document_map.remove_optional_integer(property_names::REVISION)?;

        let created_at = document_map.remove_optional_integer(property_names::CREATED_AT)?;
        let updated_at = document_map.remove_optional_integer(property_names::UPDATED_AT)?;

        // dev-note: properties is everything other than the id and owner id
        Ok(Document {
            properties: document_map,
            owner_id: Identifier::new(owner_id),
            id: Identifier::new(id),
            revision,
            created_at,
            updated_at,
        })
    }

    /// Serializes the Document to CBOR.
    #[cfg(feature = "cbor")]
    pub fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut buffer: Vec<u8> = Vec::new();
        buffer.write_varint(PROTOCOL_VERSION).map_err(|_| {
            ProtocolError::EncodingError("error writing protocol version".to_string())
        })?;
        let cbor_document = DocumentForCbor::try_from(self.clone())?;
        ciborium::ser::into_writer(&cbor_document, &mut buffer).map_err(|_| {
            ProtocolError::EncodingError("unable to serialize into cbor".to_string())
        })?;
        Ok(buffer)
    }
}
