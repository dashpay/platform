use crate::data_contract::errors::DataContractError;

use crate::document::property_names::{CREATED_AT, UPDATED_AT};
use crate::document::Document;

use crate::prelude::{DataContract, Revision};

use crate::ProtocolError;

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::document::extended_document::v0::ExtendedDocumentV0;
use crate::document::serialization_traits::deserialize::v0::ExtendedDocumentPlatformDeserializationMethodsV0;
use crate::document::serialization_traits::serialize::v0::ExtendedDocumentPlatformSerializationMethodsV0;
use crate::document::serialization_traits::{
    DocumentPlatformConversionMethodsV0, ExtendedDocumentPlatformConversionMethodsV0,
};

use crate::serialization::PlatformDeserializableWithBytesLenFromVersionedStructure;
use crate::version::PlatformVersion;

use integer_encoding::{VarInt, VarIntReader};

use platform_version::version::FeatureVersion;

impl ExtendedDocumentPlatformSerializationMethodsV0 for ExtendedDocumentV0 {
    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    fn serialize_v0(&self) -> Result<Vec<u8>, ProtocolError> {
        let document_type = self.document_type()?;
        let mut buffer: Vec<u8> = 0.encode_var_vec(); //version 0

        // $id
        buffer.extend(self.id().as_slice());

        // $ownerId
        buffer.extend(self.owner_id().as_slice());

        // $revision
        if let Some(revision) = self.revision() {
            buffer.extend(revision.encode_var_vec())
        } else if document_type.requires_revision() {
            buffer.extend((1 as Revision).encode_var_vec())
        }

        // $createdAt
        if let Some(created_at) = self.created_at() {
            if !document_type.required_fields().contains(CREATED_AT) {
                buffer.push(1);
            }
            // dbg!("we pushed created at {}", hex::encode(created_at.to_be_bytes()));
            buffer.extend(created_at.to_be_bytes());
        } else if document_type.required_fields().contains(CREATED_AT) {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "created at field is not present".to_string(),
                ),
            ));
        } else {
            // dbg!("we pushed created at with 0");
            // We don't have the created_at that wasn't required
            buffer.push(0);
        }

        // $updatedAt
        if let Some(updated_at) = self.updated_at() {
            if !document_type.required_fields().contains(UPDATED_AT) {
                // dbg!("we added 1", field_name);
                buffer.push(1);
            }
            // dbg!("we pushed updated at {}", hex::encode(updated_at.to_be_bytes()));
            buffer.extend(updated_at.to_be_bytes());
        } else if document_type.required_fields().contains(UPDATED_AT) {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "updated at field is not present".to_string(),
                ),
            ));
        } else {
            // dbg!("we pushed updated at with 0");
            // We don't have the updated_at that wasn't required
            buffer.push(0);
        }

        document_type
            .properties()
            .iter()
            .try_for_each(|(field_name, field)| {
                if let Some(value) = self.properties().get(field_name) {
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
                            .property_type
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
    fn serialize_consume_v0(mut self) -> Result<Vec<u8>, ProtocolError> {
        let document_type = self.document_type()?.to_owned_document_type();
        let mut buffer: Vec<u8> = 0.encode_var_vec(); //version 0

        // $id
        buffer.extend(self.id().into_buffer());

        // $ownerId
        buffer.extend(self.owner_id().into_buffer());

        // $revision
        if let Some(revision) = self.revision() {
            buffer.extend(revision.to_be_bytes())
        }

        // $createdAt
        if let Some(created_at) = self.created_at() {
            if !document_type.required_fields().contains(CREATED_AT) {
                buffer.push(1);
            }

            buffer.extend(created_at.to_be_bytes());
        } else if document_type.required_fields().contains(CREATED_AT) {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "created at field is not present".to_string(),
                ),
            ));
        } else {
            // We don't have the created_at that wasn't required
            buffer.push(0);
        }

        // $updatedAt
        if let Some(updated_at) = self.updated_at() {
            if !document_type.required_fields().contains(UPDATED_AT) {
                buffer.push(1);
            }

            buffer.extend(updated_at.to_be_bytes());
        } else if document_type.required_fields().contains(UPDATED_AT) {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "updated at field is not present".to_string(),
                ),
            ));
        } else {
            // We don't have the updated_at that wasn't required
            buffer.push(0);
        }

        document_type
            .properties()
            .iter()
            .try_for_each(|(field_name, field)| {
                if let Some(value) = self.properties_as_mut().remove(field_name) {
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
                            .property_type
                            .encode_value_with_size(value, field.required)?;
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
}

impl ExtendedDocumentPlatformDeserializationMethodsV0 for ExtendedDocumentV0 {
    /// Reads a serialized document and creates a Document from it.
    fn from_bytes_v0(
        serialized_document: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        // first we deserialize the contract
        let (data_contract, offset) = DataContract::versioned_deserialize_with_bytes_len(
            serialized_document,
            true, //since this would only happen on the client, we should validate
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
        let document_type_name = String::from_utf8(document_type_name_bytes.into())
            .map_err(|e| ProtocolError::DecodingError(e.to_string()))?;

        let document_type = data_contract.document_type_for_name(document_type_name.as_str())?;

        let document = Document::from_bytes(rest, document_type, platform_version)?;

        Ok(ExtendedDocumentV0 {
            document_type_name,
            data_contract_id: data_contract.id(),
            document,
            data_contract,
            metadata: None,

            entropy: Default::default(),
        })
    }
}

impl ExtendedDocumentPlatformConversionMethodsV0 for ExtendedDocumentV0 {
    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    fn serialize(&self, platform_version: &PlatformVersion) -> Result<Vec<u8>, ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .document_serialization_version
            .default_current_version
        {
            0 => self.serialize_v0(),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ExtendedDocumentV0::serialize".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn serialize_specific_version(
        &self,
        feature_version: FeatureVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match feature_version {
            0 => self.serialize_v0(),
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
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_serialization_version
            .default_current_version
        {
            0 => self.serialize_consume_v0(),
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
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let serialized_version = serialized_document.read_varint().map_err(|_| {
            ProtocolError::DecodingError(
                "error reading revision from serialized document for revision".to_string(),
            )
        })?;
        match serialized_version {
            0 => ExtendedDocumentV0::from_bytes_v0(serialized_document, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ExtendedDocument::from_bytes (deserialization)".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
