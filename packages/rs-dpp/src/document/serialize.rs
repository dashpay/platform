use crate::data_contract::document_type::document_type::PROTOCOL_VERSION;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::errors::{DataContractError, StructureError};
use crate::data_contract::extra::common::bytes_for_system_value_from_tree_map;
use crate::document::Document;
use crate::document::DocumentInStateTransition;
use crate::util::deserializer;
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::ProtocolError;
use bincode::Options;
use ciborium::Value;
use integer_encoding::VarIntWriter;
use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};
use std::io::{BufReader, Read};

impl Document {
    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    pub fn serialize(&self, document_type: &DocumentType) -> Result<Vec<u8>, ProtocolError> {
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
                    Err(ProtocolError::DataContractError(
                        DataContractError::MissingRequiredKey("a required field is not present"),
                    ))
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
    pub fn serialize_consume(
        mut self,
        document_type: &DocumentType,
    ) -> Result<Vec<u8>, ProtocolError> {
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
                    Err(ProtocolError::DataContractError(
                        DataContractError::MissingRequiredKey("a required field is not present"),
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
            ProtocolError::DecodingError("error reading from serialized document".to_string())
        })?;

        let mut owner_id = [0; 32];
        buf.read_exact(&mut owner_id).map_err(|_| {
            ProtocolError::DecodingError("error reading from serialized document".to_string())
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
            .collect::<Result<BTreeMap<String, Value>, ProtocolError>>()?;
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
    ) -> Result<Self, ProtocolError> {
        let SplitProtocolVersionOutcome {
            main_message_bytes: read_document_cbor,
            ..
        } = deserializer::split_protocol_version(document_cbor)?;

        // first we need to deserialize the document and contract indices
        // we would need dedicated deserialization functions based on the document type
        let mut document: BTreeMap<String, Value> = ciborium::de::from_reader(read_document_cbor)
            .map_err(|_| {
            ProtocolError::StructureError(StructureError::InvalidCBOR(
                "unable to decode contract for document call",
            ))
        })?;

        let owner_id: [u8; 32] = match owner_id {
            None => {
                let owner_id: Vec<u8> =
                    bytes_for_system_value_from_tree_map(&document, "$ownerId")?.ok_or({
                        ProtocolError::DataContractError(DataContractError::DocumentOwnerIdMissing(
                            "unable to get document $ownerId",
                        ))
                    })?;
                document.remove("$ownerId");
                if owner_id.len() != 32 {
                    return Err(ProtocolError::DataContractError(
                        DataContractError::FieldRequirementUnmet("invalid owner id"),
                    ));
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
                        ProtocolError::DataContractError(DataContractError::DocumentIdMissing(
                            "unable to get document $id",
                        ))
                    })?;
                document.remove("$id");
                if document_id.len() != 32 {
                    return Err(ProtocolError::DataContractError(
                        DataContractError::FieldRequirementUnmet("invalid document id"),
                    ));
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
    ) -> Result<Self, ProtocolError> {
        // we need to start by verifying that the owner_id is a 256 bit number (32 bytes)
        if owner_id.len() != 32 {
            return Err(ProtocolError::DataContractError(
                DataContractError::FieldRequirementUnmet("invalid owner id"),
            ));
        }

        if document_id.len() != 32 {
            return Err(ProtocolError::DataContractError(
                DataContractError::FieldRequirementUnmet("invalid document id"),
            ));
        }
        let SplitProtocolVersionOutcome {
            main_message_bytes: read_document_cbor,
            ..
        } = deserializer::split_protocol_version(document_cbor)?;

        // first we need to deserialize the document and contract indices
        // we would need dedicated deserialization functions based on the document type
        let properties: BTreeMap<String, Value> = ciborium::de::from_reader(read_document_cbor)
            .map_err(|_| {
                ProtocolError::StructureError(StructureError::InvalidCBOR(
                    "unable to decode contract for document call with id",
                ))
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
            .write_varint(PROTOCOL_VERSION)
            .expect("writing protocol version caused error");
        ciborium::ser::into_writer(&self, &mut buffer).expect("unable to serialize into cbor");
        buffer
    }
}
