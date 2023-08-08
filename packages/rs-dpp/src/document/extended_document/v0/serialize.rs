use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::errors::{DataContractError, StructureError};

use crate::document::property_names::{CREATED_AT, UPDATED_AT};
use crate::document::{property_names, Document};

use crate::identity::TimestampMillis;
use crate::prelude::{DataContract, Revision};
use crate::util::deserializer;
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::ProtocolError;

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::v0::v0_methods::DocumentTypeV0Methods;
use crate::document::extended_document::v0::ExtendedDocumentV0;
use crate::document::serialization_traits::{
    DocumentPlatformConversionMethodsV0, DocumentPlatformDeserializationMethodsV0,
    DocumentPlatformSerializationMethodsV0,
};
use crate::document::v0::DocumentV0;
use crate::serialization::{
    PlatformDeserializableFromVersionedStructure,
    PlatformDeserializableWithBytesLenFromVersionedStructure,
};
use crate::version::PlatformVersion;
use byteorder::{BigEndian, ReadBytesExt};
use dashcore::consensus::ReadExt;
use integer_encoding::{VarInt, VarIntReader, VarIntWriter};
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{Identifier, Value};
use platform_version::version::FeatureVersion;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::io::{BufReader, Read};

impl DocumentPlatformSerializationMethodsV0 for ExtendedDocumentV0 {
    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    fn serialize_v0(&self, document_type: DocumentTypeRef) -> Result<Vec<u8>, ProtocolError> {
        let mut buffer: Vec<u8> = 0.encode_var_vec(); //version 0
        buffer.extend(self.id().as_slice());
        buffer.extend(self.owner_id().as_slice());
        if let Some(revision) = self.revision() {
            buffer.extend(revision.encode_var_vec())
        } else if document_type.requires_revision() {
            buffer.extend((1 as Revision).encode_var_vec())
        }
        document_type
            .properties()
            .iter()
            .try_for_each(|(field_name, field)| {
                if field_name == CREATED_AT {
                    if let Some(created_at) = self.created_at() {
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
                    if let Some(updated_at) = self.updated_at() {
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
                } else if let Some(value) = self.properties().get(field_name) {
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
                            .document_type
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
        buffer.extend(self.id().into_buffer());
        buffer.extend(self.owner_id().into_buffer());

        if let Some(revision) = self.revision() {
            buffer.extend(revision.to_be_bytes())
        }
        document_type
            .flattened_properties()
            .iter()
            .try_for_each(|(field_name, field)| {
                if field_name == CREATED_AT {
                    if let Some(created_at) = self.created_at() {
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
                    if let Some(updated_at) = self.updated_at() {
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
                } else if let Some(value) = self.properties().remove(field_name) {
                    let value = field
                        .document_type
                        .encode_value_with_size(value, field.required)?;
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

impl DocumentPlatformDeserializationMethodsV0 for ExtendedDocumentV0 {
    /// Reads a serialized document and creates a Document from it.
    fn from_bytes_v0(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        // first we deserialize the contract
        let (data_contract, offset) = DataContract::versioned_deserialize_with_bytes_len(
            serialized_document,
            platform_version,
        )?;
        let serialized_document = serialized_document.split_at(offset).1;
        let (document_type_name_len, rest) =
            serialized_document
                .split_first()
                .ok_or(ProtocolError::DecodingError(
                    "error reading document type name len from serialized extended document"
                        .to_string(),
                ))?;
        if serialized_document.len() < *document_type_name_len as usize {
            return Err(ProtocolError::DecodingError(
                "serialized extended document isn't big enough for the document type len"
                    .to_string(),
            ));
        }
        let (document_type_name_bytes, rest) = rest.split_at(*document_type_name_len as usize);

        let document = Document::from_bytes(rest, document_type, platform_version)?;
        let document_type_name = String::from_utf8(document_type_name_bytes.into())
            .map_err(|e| ProtocolError::DecodingError(e.to_string()))?;
        Ok(ExtendedDocumentV0 {
            document_type_name,
            data_contract_id: data_contract.id(),
            document,
            data_contract,
            metadata: None,

            entropy: Default::default(),
        }
        .into())
    }
}

impl DocumentPlatformConversionMethodsV0 for ExtendedDocumentV0 {
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
                method: "ExtendedDocumentV0::serialize".to_string(),
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
                method: "ExtendedDocumentV0::serialize".to_string(),
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

    /// Reads a serialized document and creates an ExtendedDocumentV0 from it.
    fn from_bytes(
        mut serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let serialized_version = serialized_document.read_varint().map_err(|_| {
            ProtocolError::DecodingError(
                "error reading revision from serialized document for revision".to_string(),
            )
        })?;
        match serialized_version {
            0 => ExtendedDocumentV0::from_bytes_v0(
                serialized_document,
                document_type,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ExtendedDocument::from_bytes (deserialization)".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
