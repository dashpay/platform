use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::errors::DataContractError;

use crate::document::property_names::{
    CREATED_AT, CREATED_AT_BLOCK_HEIGHT, CREATED_AT_CORE_BLOCK_HEIGHT, UPDATED_AT,
    UPDATED_AT_BLOCK_HEIGHT, UPDATED_AT_CORE_BLOCK_HEIGHT,
};

#[cfg(feature = "validation")]
use crate::prelude::ConsensusValidationResult;

use crate::prelude::Revision;

use crate::ProtocolError;

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::document::serialization_traits::deserialize::v0::DocumentPlatformDeserializationMethodsV0;
use crate::document::serialization_traits::serialize::v0::DocumentPlatformSerializationMethodsV0;
use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use crate::document::v0::DocumentV0;
use crate::version::PlatformVersion;
use byteorder::{BigEndian, ReadBytesExt};
use integer_encoding::{VarInt, VarIntReader};

use platform_value::{Identifier, Value};
use platform_version::version::FeatureVersion;

use std::collections::BTreeMap;

use crate::consensus::basic::decode::DecodingError;
#[cfg(feature = "validation")]
use crate::consensus::basic::BasicError;
#[cfg(feature = "validation")]
use crate::consensus::ConsensusError;
use std::io::{BufReader, Read};

impl DocumentPlatformSerializationMethodsV0 for DocumentV0 {
    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    fn serialize_v0(&self, document_type: DocumentTypeRef) -> Result<Vec<u8>, ProtocolError> {
        let mut buffer: Vec<u8> = 0.encode_var_vec(); //version 0

        // $id
        buffer.extend(self.id.as_slice());

        // $ownerId
        buffer.extend(self.owner_id.as_slice());

        // $revision
        if let Some(revision) = self.revision {
            buffer.extend(revision.encode_var_vec())
        } else if document_type.requires_revision() {
            buffer.extend((1 as Revision).encode_var_vec())
        }

        let mut bitwise_exists_flag: u8 = 0;

        let mut time_fields_data_buffer = vec![];

        // $createdAt
        if let Some(created_at) = &self.created_at {
            bitwise_exists_flag |= 1;
            // dbg!("we pushed created at {}", hex::encode(created_at.to_be_bytes()));
            time_fields_data_buffer.extend(created_at.to_be_bytes());
        } else if document_type.required_fields().contains(CREATED_AT) {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "created at field is not present".to_string(),
                ),
            ));
        }

        // $updatedAt
        if let Some(updated_at) = &self.updated_at {
            bitwise_exists_flag |= 2;
            // dbg!("we pushed updated at {}", hex::encode(updated_at.to_be_bytes()));
            time_fields_data_buffer.extend(updated_at.to_be_bytes());
        } else if document_type.required_fields().contains(UPDATED_AT) {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "updated at field is not present".to_string(),
                ),
            ));
        }

        // $createdAtBlockHeight
        if let Some(created_at_block_height) = &self.created_at_block_height {
            bitwise_exists_flag |= 4;
            time_fields_data_buffer.extend(created_at_block_height.to_be_bytes());
        } else if document_type
            .required_fields()
            .contains(CREATED_AT_BLOCK_HEIGHT)
        {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "created_at_block_height field is not present".to_string(),
                ),
            ));
        }

        // $updatedAtBlockHeight
        if let Some(updated_at_block_height) = &self.updated_at_block_height {
            bitwise_exists_flag |= 8;
            time_fields_data_buffer.extend(updated_at_block_height.to_be_bytes());
        } else if document_type
            .required_fields()
            .contains(UPDATED_AT_BLOCK_HEIGHT)
        {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "updated_at_block_height field is not present".to_string(),
                ),
            ));
        }

        // $createdAtCoreBlockHeight
        if let Some(created_at_core_block_height) = &self.created_at_core_block_height {
            bitwise_exists_flag |= 16;
            time_fields_data_buffer.extend(created_at_core_block_height.to_be_bytes());
        } else if document_type
            .required_fields()
            .contains(CREATED_AT_CORE_BLOCK_HEIGHT)
        {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "created_at_core_block_height field is not present".to_string(),
                ),
            ));
        }

        // $updatedAtCoreBlockHeight
        if let Some(updated_at_core_block_height) = &self.updated_at_core_block_height {
            bitwise_exists_flag |= 32;
            time_fields_data_buffer.extend(updated_at_core_block_height.to_be_bytes());
        } else if document_type
            .required_fields()
            .contains(UPDATED_AT_CORE_BLOCK_HEIGHT)
        {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "updated_at_core_block_height field is not present".to_string(),
                ),
            ));
        }

        buffer.push(bitwise_exists_flag);
        buffer.append(&mut time_fields_data_buffer);

        // User defined properties
        document_type
            .properties()
            .iter()
            .try_for_each(|(field_name, property)| {
                if let Some(value) = self.properties.get(field_name) {
                    if value.is_null() {
                        if property.required {
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
                        if !property.required {
                            // dbg!("we added 1", field_name);
                            buffer.push(1);
                        }
                        let value = property
                            .property_type
                            .encode_value_ref_with_size(value, property.required)?;
                        // dbg!("we pushed {} with {}", field_name, hex::encode(&value));
                        buffer.extend(value.as_slice());
                        Ok(())
                    }
                } else if property.required {
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

        // $id
        buffer.extend(self.id.into_buffer());

        // $ownerId
        buffer.extend(self.owner_id.into_buffer());

        // $revision
        if let Some(revision) = self.revision {
            buffer.extend(revision.to_be_bytes())
        }
        let mut bitwise_exists_flag: u8 = 0;

        let mut time_fields_data_buffer = vec![];

        // $createdAt
        if let Some(created_at) = &self.created_at {
            bitwise_exists_flag |= 1;
            // dbg!("we pushed created at {}", hex::encode(created_at.to_be_bytes()));
            time_fields_data_buffer.extend(created_at.to_be_bytes());
        } else if document_type.required_fields().contains(CREATED_AT) {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "created at field is not present".to_string(),
                ),
            ));
        }

        // $updatedAt
        if let Some(updated_at) = &self.updated_at {
            bitwise_exists_flag |= 2;
            // dbg!("we pushed updated at {}", hex::encode(updated_at.to_be_bytes()));
            time_fields_data_buffer.extend(updated_at.to_be_bytes());
        } else if document_type.required_fields().contains(UPDATED_AT) {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "updated at field is not present".to_string(),
                ),
            ));
        }

        // $createdAtBlockHeight
        if let Some(created_at_block_height) = &self.created_at_block_height {
            bitwise_exists_flag |= 4;
            time_fields_data_buffer.extend(created_at_block_height.to_be_bytes());
        } else if document_type
            .required_fields()
            .contains(CREATED_AT_BLOCK_HEIGHT)
        {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "created_at_block_height field is not present".to_string(),
                ),
            ));
        }

        // $updatedAtBlockHeight
        if let Some(updated_at_block_height) = &self.updated_at_block_height {
            bitwise_exists_flag |= 8;
            time_fields_data_buffer.extend(updated_at_block_height.to_be_bytes());
        } else if document_type
            .required_fields()
            .contains(UPDATED_AT_BLOCK_HEIGHT)
        {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "updated_at_block_height field is not present".to_string(),
                ),
            ));
        }

        // $createdAtCoreBlockHeight
        if let Some(created_at_core_block_height) = &self.created_at_core_block_height {
            bitwise_exists_flag |= 16;
            time_fields_data_buffer.extend(created_at_core_block_height.to_be_bytes());
        } else if document_type
            .required_fields()
            .contains(CREATED_AT_CORE_BLOCK_HEIGHT)
        {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "created_at_core_block_height field is not present".to_string(),
                ),
            ));
        }

        // $updatedAtCoreBlockHeight
        if let Some(updated_at_core_block_height) = &self.updated_at_core_block_height {
            bitwise_exists_flag |= 32;
            time_fields_data_buffer.extend(updated_at_core_block_height.to_be_bytes());
        } else if document_type
            .required_fields()
            .contains(UPDATED_AT_CORE_BLOCK_HEIGHT)
        {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "updated_at_core_block_height field is not present".to_string(),
                ),
            ));
        }

        buffer.push(bitwise_exists_flag);
        buffer.append(&mut time_fields_data_buffer);

        // User defined properties
        document_type
            .properties()
            .iter()
            .try_for_each(|(field_name, property)| {
                if let Some(value) = self.properties.remove(field_name) {
                    if value.is_null() {
                        if property.required {
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
                        if !property.required {
                            // dbg!("we added 1", field_name);
                            buffer.push(1);
                        }
                        let value = property
                            .property_type
                            .encode_value_with_size(value, property.required)?;
                        // dbg!("we pushed {} with {}", field_name, hex::encode(&value));
                        buffer.extend(value.as_slice());
                        Ok(())
                    }
                } else if property.required {
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
}

impl DocumentPlatformDeserializationMethodsV0 for DocumentV0 {
    /// Reads a serialized document and creates a Document from it.
    fn from_bytes_v0(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, DataContractError> {
        let mut buf = BufReader::new(serialized_document);
        if serialized_document.len() < 64 {
            return Err(DataContractError::DecodingDocumentError(
                DecodingError::new(
                    "serialized document is too small, must have id and owner id".to_string(),
                ),
            ));
        }

        // $id
        let mut id = [0; 32];
        buf.read_exact(&mut id).map_err(|_| {
            DataContractError::DecodingDocumentError(DecodingError::new(
                "error reading from serialized document for id".to_string(),
            ))
        })?;

        // $ownerId
        let mut owner_id = [0; 32];
        buf.read_exact(&mut owner_id).map_err(|_| {
            DataContractError::DecodingDocumentError(DecodingError::new(
                "error reading from serialized document for owner id".to_string(),
            ))
        })?;

        // $revision
        // if the document type is mutable then we should deserialize the revision
        let revision: Option<Revision> = if document_type.requires_revision() {
            let revision = buf.read_varint().map_err(|_| {
                DataContractError::DecodingDocumentError(DecodingError::new(
                    "error reading revision from serialized document for revision".to_string(),
                ))
            })?;
            Some(revision)
        } else {
            None
        };

        let timestamp_flags = buf.read_u8().map_err(|_| {
            DataContractError::CorruptedSerialization(
                "error reading timestamp flags from serialized document".to_string(),
            )
        })?;

        let created_at = if timestamp_flags & 1 > 0 {
            Some(buf.read_u64::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading created_at timestamp from serialized document".to_string(),
                )
            })?)
        } else {
            None
        };

        let updated_at = if timestamp_flags & 2 > 0 {
            Some(buf.read_u64::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading updated_at timestamp from serialized document".to_string(),
                )
            })?)
        } else {
            None
        };

        let created_at_block_height = if timestamp_flags & 4 > 0 {
            Some(buf.read_u64::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading created_at_block_height from serialized document".to_string(),
                )
            })?)
        } else {
            None
        };

        let updated_at_block_height = if timestamp_flags & 8 > 0 {
            Some(buf.read_u64::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading updated_at_block_height from serialized document".to_string(),
                )
            })?)
        } else {
            None
        };

        let created_at_core_block_height = if timestamp_flags & 16 > 0 {
            Some(buf.read_u32::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading created_at_core_block_height from serialized document"
                        .to_string(),
                )
            })?)
        } else {
            None
        };

        let updated_at_core_block_height = if timestamp_flags & 32 > 0 {
            Some(buf.read_u32::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading updated_at_core_block_height from serialized document"
                        .to_string(),
                )
            })?)
        } else {
            None
        };

        let mut finished_buffer = false;

        let properties = document_type
            .properties()
            .iter()
            .filter_map(|(key, property)| {
                if finished_buffer {
                    return if property.required {
                        Some(Err(DataContractError::CorruptedSerialization(
                            "required field after finished buffer".to_string(),
                        )))
                    } else {
                        None
                    };
                }
                let read_value = property
                    .property_type
                    .read_optionally_from(&mut buf, property.required);

                match read_value {
                    Ok(read_value) => {
                        finished_buffer |= read_value.1;
                        read_value.0.map(|read_value| Ok((key.clone(), read_value)))
                    }
                    Err(e) => Some(Err(e)),
                }
            })
            .collect::<Result<BTreeMap<String, Value>, DataContractError>>()?;

        Ok(DocumentV0 {
            id: Identifier::new(id),
            properties,
            owner_id: Identifier::new(owner_id),
            revision,
            created_at,
            updated_at,
            created_at_block_height,
            updated_at_block_height,
            created_at_core_block_height,
            updated_at_core_block_height,
        })
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
            .document_versions
            .document_serialization_version
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

    fn serialize_specific_version(
        &self,
        document_type: DocumentTypeRef,
        feature_version: FeatureVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match feature_version {
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
            .document_versions
            .document_serialization_version
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
        let serialized_version = serialized_document.read_varint().map_err(|_| {
            DataContractError::DecodingDocumentError(DecodingError::new(
                "error reading revision from serialized document for revision".to_string(),
            ))
        })?;
        match serialized_version {
            0 => DocumentV0::from_bytes_v0(serialized_document, document_type, platform_version)
                .map_err(ProtocolError::DataContractError),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_bytes (deserialization)".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Reads a serialized document and creates a DocumentV0 from it.
    #[cfg(feature = "validation")]
    fn from_bytes_in_consensus(
        mut serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Self>, ProtocolError> {
        let serialized_version = serialized_document.read_varint().map_err(|_| {
            DataContractError::DecodingDocumentError(DecodingError::new(
                "error reading revision from serialized document for revision".to_string(),
            ))
        })?;
        match serialized_version {
            0 => {
                match DocumentV0::from_bytes_v0(
                    serialized_document,
                    document_type,
                    platform_version,
                ) {
                    Ok(document) => Ok(ConsensusValidationResult::new_with_data(document)),
                    Err(err) => Ok(ConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(BasicError::ContractError(err)),
                    )),
                }
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_bytes (deserialization)".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
