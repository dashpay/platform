use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::errors::{DataContractError, StructureError};

use crate::document::property_names;
use crate::document::property_names::{CREATED_AT, UPDATED_AT};

use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use crate::util::deserializer;
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::ProtocolError;

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::v0::v0_methods::DocumentTypeV0Methods;
use crate::document::serialization_traits::{
    DocumentPlatformConversionMethodsV0, DocumentPlatformDeserializationMethodsV0,
    DocumentPlatformSerializationMethodsV0,
};
use crate::document::v0::DocumentV0;
use crate::version::PlatformVersion;
use byteorder::{BigEndian, ReadBytesExt};
use integer_encoding::{VarInt, VarIntReader, VarIntWriter};
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::io::{BufReader, Read};

impl DocumentPlatformSerializationMethodsV0 for DocumentV0 {
    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    fn serialize_v0(&self, document_type: DocumentTypeRef) -> Result<Vec<u8>, ProtocolError> {
        let mut buffer: Vec<u8> = 0.encode_var_vec(); //version 0
        buffer.extend(self.id.as_slice());
        buffer.extend(self.owner_id.as_slice());
        if let Some(revision) = self.revision {
            buffer.extend(revision.encode_var_vec())
        } else if document_type.requires_revision() {
            buffer.extend((1 as Revision).encode_var_vec())
        }
        document_type
            .properties()
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
                                DataContractError::MissingRequiredKey(
                                    "a required field is not present".to_string(),
                                ),
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
                            .r#type
                            .encode_value_ref_with_size(value, field.required)?;
                        // dbg!("we pushed {} with {}", field_name, hex::encode(&value));
                        buffer.extend(value.as_slice());
                        Ok(())
                    }
                } else if field.required {
                    Err(ProtocolError::DataContractError(
                        DataContractError::MissingRequiredKey(format!(
                            "a required field {field_name} is not present"
                        )),
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
    fn serialize_consume_v0(
        mut self,
        document_type: DocumentTypeRef,
    ) -> Result<Vec<u8>, ProtocolError> {
        let mut buffer: Vec<u8> = 0.encode_var_vec(); //version 0
        buffer.extend(self.id.into_buffer());
        buffer.extend(self.owner_id.into_buffer());

        if let Some(revision) = self.revision {
            buffer.extend(revision.to_be_bytes())
        }
        document_type
            .flattened_properties()
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
                    let value = field.r#type.encode_value_with_size(value, field.required)?;
                    buffer.extend(value.as_slice());
                    Ok(())
                } else if field.required {
                    Err(ProtocolError::DataContractError(
                        DataContractError::MissingRequiredKey(
                            "a required field is not present".to_string(),
                        ),
                    ))
                } else {
                    // We don't have something that wasn't required
                    buffer.push(0);
                    Ok(())
                }
            })?;

        Ok(buffer)
    }
}

impl DocumentPlatformDeserializationMethodsV0 for DocumentV0 {
    /// Reads a serialized document and creates a Document from it.
    fn from_bytes_v0(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
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
            .properties()
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
                    let read_value = field.r#type.read_from(&mut buf, field.required);
                    match read_value {
                        Ok(read_value) => read_value.map(|read_value| Ok((key.clone(), read_value))),
                        Err(e) => Some(Err(e)),
                    }
                }
            })
            .collect::<Result<BTreeMap<String, Value>, ProtocolError>>()?;
        Ok(DocumentV0 {
            id: Identifier::new(id),
            properties,
            owner_id: Identifier::new(owner_id),
            revision,
            created_at,
            updated_at,
        }
        .into())
    }
}

impl DocumentPlatformConversionMethodsV0 for DocumentV0 {
    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    fn serialize(
        &self,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_serialization_version
            .default_current_version
        {
            0 => self.serialize_v0(document_type),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentV0::serialize".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Serializes and consumes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    fn serialize_consume(
        self,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_serialization_version
            .default_current_version
        {
            0 => self.serialize_consume_v0(document_type),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentV0::serialize_consume".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Reads a serialized document and creates a DocumentV0 from it.
    fn from_bytes(
        mut serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_structure_version
        {
            0 => {
                let serialized_version = serialized_document.read_varint().map_err(|_| {
                    ProtocolError::DecodingError(
                        "error reading revision from serialized document for revision".to_string(),
                    )
                })?;
                match serialized_version {
                    0 => DocumentV0::from_bytes_v0(serialized_document, document_type),
                    version => Err(ProtocolError::UnknownVersionMismatch {
                        method: "Document::from_bytes (deserialization)".to_string(),
                        known_versions: vec![0],
                        received: version,
                    }),
                }
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_bytes (structure)".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
