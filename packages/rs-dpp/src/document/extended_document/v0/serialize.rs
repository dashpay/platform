use crate::document::Document;

use crate::prelude::DataContract;

use crate::ProtocolError;

use crate::data_contract::accessors::v0::DataContractV0Getters;

use crate::document::extended_document::v0::ExtendedDocumentV0;
use crate::document::serialization_traits::deserialize::v0::ExtendedDocumentPlatformDeserializationMethodsV0;
use crate::document::serialization_traits::serialize::v0::ExtendedDocumentPlatformSerializationMethodsV0;
use crate::document::serialization_traits::{
    DocumentPlatformConversionMethodsV0, ExtendedDocumentPlatformConversionMethodsV0,
};

use crate::serialization::{
    PlatformDeserializableWithBytesLenFromVersionedStructure,
    PlatformSerializableWithPlatformVersion,
};
use crate::version::PlatformVersion;

use integer_encoding::{VarInt, VarIntReader};

use crate::consensus::basic::decode::DecodingError;
use crate::data_contract::errors::DataContractError;
use platform_version::version::FeatureVersion;

impl ExtendedDocumentPlatformSerializationMethodsV0 for ExtendedDocumentV0 {
    /// Serializes the extended document.
    ///
    /// The serialization of an extended document follows the pattern:
    /// data contract | document type name | document
    fn serialize_v0(&self, platform_version: &PlatformVersion) -> Result<Vec<u8>, ProtocolError> {
        let mut buffer: Vec<u8> = 0.encode_var_vec(); //version 0

        buffer.append(
            &mut self
                .data_contract
                .serialize_to_bytes_with_platform_version(platform_version)?,
        );
        buffer.push(self.document_type_name.len() as u8);
        buffer.extend(self.document_type_name.as_bytes());
        buffer.append(
            &mut self
                .document
                .serialize(self.document_type()?, platform_version)?,
        );
        Ok(buffer)
    }

    /// Serializes the extended document.
    ///
    /// The serialization of an extended document follows the pattern:
    /// data contract | document type name | document
    fn serialize_consume_v0(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        let ExtendedDocumentV0 {
            document_type_name,
            document,
            data_contract,
            ..
        } = self;

        let mut serialized_document = document.serialize_consume(
            data_contract.document_type_for_name(document_type_name.as_str())?,
            platform_version,
        )?;

        let mut buffer: Vec<u8> = 0.encode_var_vec(); //version 0

        buffer.append(
            &mut data_contract
                .serialize_consume_to_bytes_with_platform_version(platform_version)?,
        );
        buffer.push(document_type_name.len() as u8);
        buffer.append(&mut document_type_name.into_bytes());
        buffer.append(&mut serialized_document);
        Ok(buffer)
    }
}

impl ExtendedDocumentPlatformDeserializationMethodsV0 for ExtendedDocumentV0 {
    /// Reads a serialized document and creates a Document from it.
    fn from_bytes_v0(
        serialized_extended_document: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        // first we deserialize the contract
        let (data_contract, offset) = DataContract::versioned_deserialize_with_bytes_len(
            serialized_extended_document,
            true, //since this would only happen on the client, we should validate
            platform_version,
        )?;
        let serialized_document = serialized_extended_document.split_at(offset).1;
        let (document_type_name_len, rest) =
            serialized_document
                .split_first()
                .ok_or(DataContractError::DecodingDocumentError(
                    DecodingError::new(
                        "error reading document type name len from serialized extended document"
                            .to_string(),
                    ),
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
    fn serialize_to_bytes(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .document_serialization_version
            .default_current_version
        {
            0 => self.serialize_v0(platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ExtendedDocumentV0::serialize".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn serialize_specific_version_to_bytes(
        &self,
        feature_version: FeatureVersion,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match feature_version {
            0 => self.serialize_v0(platform_version),
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
    fn serialize_consume_to_bytes(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_serialization_version
            .default_current_version
        {
            0 => self.serialize_consume_v0(platform_version),
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
