use crate::data_contract::document_type::{DocumentPropertyType, DocumentTypeRef};
use crate::data_contract::errors::DataContractError;

use crate::document::property_names::{
    CREATED_AT, CREATED_AT_BLOCK_HEIGHT, CREATED_AT_CORE_BLOCK_HEIGHT, PRICE, TRANSFERRED_AT,
    TRANSFERRED_AT_BLOCK_HEIGHT, TRANSFERRED_AT_CORE_BLOCK_HEIGHT, UPDATED_AT,
    UPDATED_AT_BLOCK_HEIGHT, UPDATED_AT_CORE_BLOCK_HEIGHT,
};

#[cfg(feature = "validation")]
use crate::prelude::ConsensusValidationResult;

use crate::prelude::{DataContract, Revision};

use crate::ProtocolError;

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
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
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::config::DataContractConfig;
use crate::nft::TradeMode;
use std::io::{BufReader, Read};

impl DocumentPlatformSerializationMethodsV0 for DocumentV0 {
    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    /// In serialize v0 all integers are always encoded as i64s
    fn serialize_v0(&self, document_type: DocumentTypeRef) -> Result<Vec<u8>, ProtocolError> {
        let mut buffer: Vec<u8> = 0u64.encode_var_vec(); //version 0

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

        let mut bitwise_exists_flag: u16 = 0;

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

        // $transferredAt
        if let Some(transferred_at) = &self.transferred_at {
            bitwise_exists_flag |= 4;
            // dbg!("we pushed transferred at {}", hex::encode(transferred_at.to_be_bytes()));
            time_fields_data_buffer.extend(transferred_at.to_be_bytes());
        } else if document_type.required_fields().contains(TRANSFERRED_AT) {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "transferred at field is not present".to_string(),
                ),
            ));
        }

        // $createdAtBlockHeight
        if let Some(created_at_block_height) = &self.created_at_block_height {
            bitwise_exists_flag |= 8;
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
            bitwise_exists_flag |= 16;
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

        // $transferredAtBlockHeight
        if let Some(transferred_at_block_height) = &self.transferred_at_block_height {
            bitwise_exists_flag |= 32;
            time_fields_data_buffer.extend(transferred_at_block_height.to_be_bytes());
        } else if document_type
            .required_fields()
            .contains(TRANSFERRED_AT_BLOCK_HEIGHT)
        {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "transferred_at_block_height field is not present".to_string(),
                ),
            ));
        }

        // $createdAtCoreBlockHeight
        if let Some(created_at_core_block_height) = &self.created_at_core_block_height {
            bitwise_exists_flag |= 64;
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
            bitwise_exists_flag |= 128;
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

        // $transferredAtCoreBlockHeight
        if let Some(transferred_at_core_block_height) = &self.transferred_at_core_block_height {
            bitwise_exists_flag |= 256;
            time_fields_data_buffer.extend(transferred_at_core_block_height.to_be_bytes());
        } else if document_type
            .required_fields()
            .contains(TRANSFERRED_AT_CORE_BLOCK_HEIGHT)
        {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "transferred_at_core_block_height field is not present".to_string(),
                ),
            ));
        }

        buffer.extend(bitwise_exists_flag.to_be_bytes().as_slice());
        buffer.append(&mut time_fields_data_buffer);

        // Now we serialize the price which might not be necessary unless called for by the document type

        if document_type.trade_mode().seller_sets_price() {
            if let Some(price) = self.properties.get(PRICE) {
                buffer.push(1);
                let price_as_u64: u64 = price.to_integer().map_err(ProtocolError::ValueError)?;
                buffer.append(&mut price_as_u64.to_be_bytes().to_vec());
            } else {
                buffer.push(0);
            }
        }

        // User defined properties
        document_type
            .properties()
            .iter()
            .try_for_each(|(field_name, property)| {
                if let Some(value) = self.properties.get(field_name) {
                    if value.is_null() {
                        if property.required && !property.transient {
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
                        if !property.required || property.transient {
                            // dbg!("we added 1", field_name);
                            buffer.push(1);
                        }
                        let value = if property.property_type.is_integer() {
                            DocumentPropertyType::I64
                                .encode_value_ref_with_size(value, property.required)
                        } else {
                            property
                                .property_type
                                .encode_value_ref_with_size(value, property.required)
                        }?;

                        // dbg!("we pushed {} with {}", field_name, hex::encode(&value));
                        buffer.extend(value.as_slice());
                        Ok(())
                    }
                } else if property.required && !property.transient {
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

    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    /// Serialize v1 will encode integers normally with their known size.
    /// Otherwise it is almost identical to V0. V1 represents the original code.
    fn serialize_v1(&self, document_type: DocumentTypeRef) -> Result<Vec<u8>, ProtocolError> {
        let mut buffer: Vec<u8> = 1u64.encode_var_vec(); //version 1

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

        let mut bitwise_exists_flag: u16 = 0;

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

        // $transferredAt
        if let Some(transferred_at) = &self.transferred_at {
            bitwise_exists_flag |= 4;
            // dbg!("we pushed transferred at {}", hex::encode(transferred_at.to_be_bytes()));
            time_fields_data_buffer.extend(transferred_at.to_be_bytes());
        } else if document_type.required_fields().contains(TRANSFERRED_AT) {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "transferred at field is not present".to_string(),
                ),
            ));
        }

        // $createdAtBlockHeight
        if let Some(created_at_block_height) = &self.created_at_block_height {
            bitwise_exists_flag |= 8;
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
            bitwise_exists_flag |= 16;
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

        // $transferredAtBlockHeight
        if let Some(transferred_at_block_height) = &self.transferred_at_block_height {
            bitwise_exists_flag |= 32;
            time_fields_data_buffer.extend(transferred_at_block_height.to_be_bytes());
        } else if document_type
            .required_fields()
            .contains(TRANSFERRED_AT_BLOCK_HEIGHT)
        {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "transferred_at_block_height field is not present".to_string(),
                ),
            ));
        }

        // $createdAtCoreBlockHeight
        if let Some(created_at_core_block_height) = &self.created_at_core_block_height {
            bitwise_exists_flag |= 64;
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
            bitwise_exists_flag |= 128;
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

        // $transferredAtCoreBlockHeight
        if let Some(transferred_at_core_block_height) = &self.transferred_at_core_block_height {
            bitwise_exists_flag |= 256;
            time_fields_data_buffer.extend(transferred_at_core_block_height.to_be_bytes());
        } else if document_type
            .required_fields()
            .contains(TRANSFERRED_AT_CORE_BLOCK_HEIGHT)
        {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "transferred_at_core_block_height field is not present".to_string(),
                ),
            ));
        }

        buffer.extend(bitwise_exists_flag.to_be_bytes().as_slice());
        buffer.append(&mut time_fields_data_buffer);

        // Now we serialize the price which might not be necessary unless called for by the document type

        if document_type.trade_mode().seller_sets_price() {
            if let Some(price) = self.properties.get(PRICE) {
                buffer.push(1);
                let price_as_u64: u64 = price.to_integer().map_err(ProtocolError::ValueError)?;
                buffer.append(&mut price_as_u64.to_be_bytes().to_vec());
            } else {
                buffer.push(0);
            }
        }

        // User defined properties
        document_type
            .properties()
            .iter()
            .try_for_each(|(field_name, property)| {
                if let Some(value) = self.properties.get(field_name) {
                    if value.is_null() {
                        if property.required && !property.transient {
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
                        if !property.required || property.transient {
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
                } else if property.required && !property.transient {
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

    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    /// Serialize v2 will encode the creator id as well.
    fn serialize_v2(&self, document_type: DocumentTypeRef) -> Result<Vec<u8>, ProtocolError> {
        let mut buffer: Vec<u8> = 2u64.encode_var_vec(); //version 2

        // $id
        buffer.extend(self.id.as_slice());

        // $ownerId
        buffer.extend(self.owner_id.as_slice());

        if document_type.trade_mode() != TradeMode::None
            || document_type.documents_transferable().is_transferable()
        {
            if let Some(creator_id) = self.creator_id {
                buffer.push(1);
                buffer.extend(creator_id.as_slice());
            } else {
                buffer.push(0);
            }
        }

        // $revision
        if let Some(revision) = self.revision {
            buffer.extend(revision.encode_var_vec())
        } else if document_type.requires_revision() {
            buffer.extend((1 as Revision).encode_var_vec())
        }

        let mut bitwise_exists_flag: u16 = 0;

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

        // $transferredAt
        if let Some(transferred_at) = &self.transferred_at {
            bitwise_exists_flag |= 4;
            // dbg!("we pushed transferred at {}", hex::encode(transferred_at.to_be_bytes()));
            time_fields_data_buffer.extend(transferred_at.to_be_bytes());
        } else if document_type.required_fields().contains(TRANSFERRED_AT) {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "transferred at field is not present".to_string(),
                ),
            ));
        }

        // $createdAtBlockHeight
        if let Some(created_at_block_height) = &self.created_at_block_height {
            bitwise_exists_flag |= 8;
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
            bitwise_exists_flag |= 16;
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

        // $transferredAtBlockHeight
        if let Some(transferred_at_block_height) = &self.transferred_at_block_height {
            bitwise_exists_flag |= 32;
            time_fields_data_buffer.extend(transferred_at_block_height.to_be_bytes());
        } else if document_type
            .required_fields()
            .contains(TRANSFERRED_AT_BLOCK_HEIGHT)
        {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "transferred_at_block_height field is not present".to_string(),
                ),
            ));
        }

        // $createdAtCoreBlockHeight
        if let Some(created_at_core_block_height) = &self.created_at_core_block_height {
            bitwise_exists_flag |= 64;
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
            bitwise_exists_flag |= 128;
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

        // $transferredAtCoreBlockHeight
        if let Some(transferred_at_core_block_height) = &self.transferred_at_core_block_height {
            bitwise_exists_flag |= 256;
            time_fields_data_buffer.extend(transferred_at_core_block_height.to_be_bytes());
        } else if document_type
            .required_fields()
            .contains(TRANSFERRED_AT_CORE_BLOCK_HEIGHT)
        {
            return Err(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey(
                    "transferred_at_core_block_height field is not present".to_string(),
                ),
            ));
        }

        buffer.extend(bitwise_exists_flag.to_be_bytes().as_slice());
        buffer.append(&mut time_fields_data_buffer);

        // Now we serialize the price which might not be necessary unless called for by the document type

        if document_type.trade_mode().seller_sets_price() {
            if let Some(price) = self.properties.get(PRICE) {
                buffer.push(1);
                let price_as_u64: u64 = price.to_integer().map_err(ProtocolError::ValueError)?;
                buffer.append(&mut price_as_u64.to_be_bytes().to_vec());
            } else {
                buffer.push(0);
            }
        }

        // User defined properties
        document_type
            .properties()
            .iter()
            .try_for_each(|(field_name, property)| {
                if let Some(value) = self.properties.get(field_name) {
                    if value.is_null() {
                        if property.required && !property.transient {
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
                        if !property.required || property.transient {
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
                } else if property.required && !property.transient {
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

        let timestamp_flags = buf.read_u16::<BigEndian>().map_err(|_| {
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

        let transferred_at = if timestamp_flags & 4 > 0 {
            Some(buf.read_u64::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading transferred_at timestamp from serialized document".to_string(),
                )
            })?)
        } else {
            None
        };

        let created_at_block_height = if timestamp_flags & 8 > 0 {
            Some(buf.read_u64::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading created_at_block_height from serialized document".to_string(),
                )
            })?)
        } else {
            None
        };

        let updated_at_block_height = if timestamp_flags & 16 > 0 {
            Some(buf.read_u64::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading updated_at_block_height from serialized document".to_string(),
                )
            })?)
        } else {
            None
        };

        let transferred_at_block_height = if timestamp_flags & 32 > 0 {
            Some(buf.read_u64::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading transferred_at_block_height from serialized document"
                        .to_string(),
                )
            })?)
        } else {
            None
        };

        let created_at_core_block_height = if timestamp_flags & 64 > 0 {
            Some(buf.read_u32::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading created_at_core_block_height from serialized document"
                        .to_string(),
                )
            })?)
        } else {
            None
        };

        let updated_at_core_block_height = if timestamp_flags & 128 > 0 {
            Some(buf.read_u32::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading updated_at_core_block_height from serialized document"
                        .to_string(),
                )
            })?)
        } else {
            None
        };

        let transferred_at_core_block_height = if timestamp_flags & 256 > 0 {
            Some(buf.read_u32::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading updated_at_core_block_height from serialized document"
                        .to_string(),
                )
            })?)
        } else {
            None
        };

        // Now we deserialize the price which might not be necessary unless called for by the document type

        let price = if document_type.trade_mode().seller_sets_price() {
            let has_price = buf.read_u8().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading has price bool from serialized document".to_string(),
                )
            })?;
            if has_price > 0 {
                let price = buf.read_u64::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading price u64 from serialized document".to_string(),
                    )
                })?;
                Some(price)
            } else {
                None
            }
        } else {
            None
        };

        let mut finished_buffer = false;

        let mut properties = document_type
            .properties()
            .iter()
            .filter_map(|(key, property)| {
                if finished_buffer {
                    return if property.required && !property.transient {
                        Some(Err(DataContractError::CorruptedSerialization(
                            "required field after finished buffer".to_string(),
                        )))
                    } else {
                        None
                    };
                }

                // In version 0 all integers are encoded as I64 (in theory)
                let read_value = if property.property_type.is_integer() {
                    DocumentPropertyType::I64
                        .read_optionally_from(&mut buf, property.required & !property.transient)
                } else {
                    property
                        .property_type
                        .read_optionally_from(&mut buf, property.required & !property.transient)
                };

                match read_value {
                    Ok(read_value) => {
                        finished_buffer |= read_value.1;
                        read_value.0.map(|read_value| Ok((key.clone(), read_value)))
                    }
                    Err(e) => Some(Err(e)),
                }
            })
            .collect::<Result<BTreeMap<String, Value>, DataContractError>>()?;

        if let Some(price) = price {
            properties.insert(PRICE.to_string(), price.into());
        }

        Ok(DocumentV0 {
            id: Identifier::new(id),
            properties,
            owner_id: Identifier::new(owner_id),
            revision,
            created_at,
            updated_at,
            transferred_at,
            created_at_block_height,
            updated_at_block_height,
            transferred_at_block_height,
            created_at_core_block_height,
            updated_at_core_block_height,
            transferred_at_core_block_height,
            creator_id: None,
        })
    }

    /// Reads a serialized document and creates a Document from it.
    fn from_bytes_v1(
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

        let timestamp_flags = buf.read_u16::<BigEndian>().map_err(|_| {
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

        let transferred_at = if timestamp_flags & 4 > 0 {
            Some(buf.read_u64::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading transferred_at timestamp from serialized document".to_string(),
                )
            })?)
        } else {
            None
        };

        let created_at_block_height = if timestamp_flags & 8 > 0 {
            Some(buf.read_u64::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading created_at_block_height from serialized document".to_string(),
                )
            })?)
        } else {
            None
        };

        let updated_at_block_height = if timestamp_flags & 16 > 0 {
            Some(buf.read_u64::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading updated_at_block_height from serialized document".to_string(),
                )
            })?)
        } else {
            None
        };

        let transferred_at_block_height = if timestamp_flags & 32 > 0 {
            Some(buf.read_u64::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading transferred_at_block_height from serialized document"
                        .to_string(),
                )
            })?)
        } else {
            None
        };

        let created_at_core_block_height = if timestamp_flags & 64 > 0 {
            Some(buf.read_u32::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading created_at_core_block_height from serialized document"
                        .to_string(),
                )
            })?)
        } else {
            None
        };

        let updated_at_core_block_height = if timestamp_flags & 128 > 0 {
            Some(buf.read_u32::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading updated_at_core_block_height from serialized document"
                        .to_string(),
                )
            })?)
        } else {
            None
        };

        let transferred_at_core_block_height = if timestamp_flags & 256 > 0 {
            Some(buf.read_u32::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading updated_at_core_block_height from serialized document"
                        .to_string(),
                )
            })?)
        } else {
            None
        };

        // Now we deserialize the price which might not be necessary unless called for by the document type

        let price = if document_type.trade_mode().seller_sets_price() {
            let has_price = buf.read_u8().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading has price bool from serialized document".to_string(),
                )
            })?;
            if has_price > 0 {
                let price = buf.read_u64::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading price u64 from serialized document".to_string(),
                    )
                })?;
                Some(price)
            } else {
                None
            }
        } else {
            None
        };

        let mut finished_buffer = false;

        let mut properties = document_type
            .properties()
            .iter()
            .filter_map(|(key, property)| {
                if finished_buffer {
                    return if property.required && !property.transient {
                        Some(Err(DataContractError::CorruptedSerialization(
                            "required field after finished buffer".to_string(),
                        )))
                    } else {
                        None
                    };
                }
                let read_value = property
                    .property_type
                    .read_optionally_from(&mut buf, property.required & !property.transient);

                match read_value {
                    Ok(read_value) => {
                        finished_buffer |= read_value.1;
                        read_value.0.map(|read_value| Ok((key.clone(), read_value)))
                    }
                    Err(e) => Some(Err(e)),
                }
            })
            .collect::<Result<BTreeMap<String, Value>, DataContractError>>()?;

        if let Some(price) = price {
            properties.insert(PRICE.to_string(), price.into());
        }

        Ok(DocumentV0 {
            id: Identifier::new(id),
            properties,
            owner_id: Identifier::new(owner_id),
            revision,
            created_at,
            updated_at,
            transferred_at,
            created_at_block_height,
            updated_at_block_height,
            transferred_at_block_height,
            created_at_core_block_height,
            updated_at_core_block_height,
            transferred_at_core_block_height,
            creator_id: None,
        })
    }

    /// Reads a serialized document and creates a Document from it.
    fn from_bytes_v2(
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

        // $creatorId
        let creator_id: Option<Identifier> = if document_type.trade_mode() != TradeMode::None
            || document_type.documents_transferable().is_transferable()
        {
            let has_creator_id = buf.read_u8().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading has creator id bool from serialized document".to_string(),
                )
            })?;
            if has_creator_id > 0 {
                // $creatorId
                let mut known_owner_id = [0; 32];
                buf.read_exact(&mut known_owner_id).map_err(|_| {
                    DataContractError::DecodingDocumentError(DecodingError::new(
                        "error reading from serialized document for creator id".to_string(),
                    ))
                })?;
                Some(known_owner_id.into())
            } else {
                None
            }
        } else {
            None
        };

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

        let timestamp_flags = buf.read_u16::<BigEndian>().map_err(|_| {
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

        let transferred_at = if timestamp_flags & 4 > 0 {
            Some(buf.read_u64::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading transferred_at timestamp from serialized document".to_string(),
                )
            })?)
        } else {
            None
        };

        let created_at_block_height = if timestamp_flags & 8 > 0 {
            Some(buf.read_u64::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading created_at_block_height from serialized document".to_string(),
                )
            })?)
        } else {
            None
        };

        let updated_at_block_height = if timestamp_flags & 16 > 0 {
            Some(buf.read_u64::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading updated_at_block_height from serialized document".to_string(),
                )
            })?)
        } else {
            None
        };

        let transferred_at_block_height = if timestamp_flags & 32 > 0 {
            Some(buf.read_u64::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading transferred_at_block_height from serialized document"
                        .to_string(),
                )
            })?)
        } else {
            None
        };

        let created_at_core_block_height = if timestamp_flags & 64 > 0 {
            Some(buf.read_u32::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading created_at_core_block_height from serialized document"
                        .to_string(),
                )
            })?)
        } else {
            None
        };

        let updated_at_core_block_height = if timestamp_flags & 128 > 0 {
            Some(buf.read_u32::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading updated_at_core_block_height from serialized document"
                        .to_string(),
                )
            })?)
        } else {
            None
        };

        let transferred_at_core_block_height = if timestamp_flags & 256 > 0 {
            Some(buf.read_u32::<BigEndian>().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading updated_at_core_block_height from serialized document"
                        .to_string(),
                )
            })?)
        } else {
            None
        };

        // Now we deserialize the price which might not be necessary unless called for by the document type

        let price = if document_type.trade_mode().seller_sets_price() {
            let has_price = buf.read_u8().map_err(|_| {
                DataContractError::CorruptedSerialization(
                    "error reading has price bool from serialized document".to_string(),
                )
            })?;
            if has_price > 0 {
                let price = buf.read_u64::<BigEndian>().map_err(|_| {
                    DataContractError::CorruptedSerialization(
                        "error reading price u64 from serialized document".to_string(),
                    )
                })?;
                Some(price)
            } else {
                None
            }
        } else {
            None
        };

        let mut finished_buffer = false;

        let mut properties = document_type
            .properties()
            .iter()
            .filter_map(|(key, property)| {
                if finished_buffer {
                    return if property.required && !property.transient {
                        Some(Err(DataContractError::CorruptedSerialization(
                            "required field after finished buffer".to_string(),
                        )))
                    } else {
                        None
                    };
                }
                let read_value = property
                    .property_type
                    .read_optionally_from(&mut buf, property.required & !property.transient);

                match read_value {
                    Ok(read_value) => {
                        finished_buffer |= read_value.1;
                        read_value.0.map(|read_value| Ok((key.clone(), read_value)))
                    }
                    Err(e) => Some(Err(e)),
                }
            })
            .collect::<Result<BTreeMap<String, Value>, DataContractError>>()?;

        if let Some(price) = price {
            properties.insert(PRICE.to_string(), price.into());
        }

        Ok(DocumentV0 {
            id: Identifier::new(id),
            properties,
            owner_id: Identifier::new(owner_id),
            revision,
            created_at,
            updated_at,
            transferred_at,
            created_at_block_height,
            updated_at_block_height,
            transferred_at_block_height,
            created_at_core_block_height,
            updated_at_core_block_height,
            transferred_at_core_block_height,
            creator_id,
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
        contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        if matches!(contract, DataContract::V0(_))
            || matches!(contract.config(), DataContractConfig::V0(_))
        {
            // Any data contract in version 0 should always serialize documents in version 0
            // This is because integers in such a data contract if made through normal versioning should always
            // be i64
            // While it's possible in theory maybe that they are not i64 using serialize_v0
            // will encode all integers as i64.
            self.serialize_v0(document_type)
        } else {
            match platform_version
                .dpp
                .document_versions
                .document_serialization_version
                .default_current_version
            {
                0 => self.serialize_v0(document_type),
                // Version 1 coincides with protocol version 9, which contains tokens, new document types,
                // and most importantly different integer types.
                // Document types now have properties that are known to be things like u8, i32 etc.
                1 => self.serialize_v1(document_type),
                2 => self.serialize_v2(document_type),
                version => Err(ProtocolError::UnknownVersionMismatch {
                    method: "DocumentV0::serialize".to_string(),
                    known_versions: vec![0, 1, 2],
                    received: version,
                }),
            }
        }
    }

    fn serialize_specific_version(
        &self,
        document_type: DocumentTypeRef,
        contract: &DataContract,
        feature_version: FeatureVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        if (matches!(contract, DataContract::V0(_))
            || matches!(contract.config(), DataContractConfig::V0(_)))
            && feature_version != 0
        {
            // Any data contract in version 0 should always serialize documents in version 0
            // This is because integers in such a data contract if made through normal versioning should always
            // be i64
            // While it's possible in theory maybe that they are not i64 using serialize_v0
            // will encode all integers as i64.
            return Err(ProtocolError::NotSupported("Serializing with data contract version 0 or data contract config version 0 is not supported outside of feature version 0".to_string()));
        };
        match feature_version {
            0 => self.serialize_v0(document_type),
            1 => self.serialize_v1(document_type),
            2 => self.serialize_v2(document_type),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentV0::serialize".to_string(),
                known_versions: vec![0, 1, 2],
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
            0 => {
                match DocumentV0::from_bytes_v0(
                    serialized_document,
                    document_type,
                    platform_version,
                )
                .map_err(ProtocolError::DataContractError)
                {
                    Ok(document) => Ok(document),
                    Err(first_err) => {
                        // let's try decoding in V1 just to be safe
                        // Version 0 will decode all integers as I64
                        // Version 1 will decode all integers properly
                        // When version was 0 used (protocol version 1 to 8) integers other than I64
                        // existed, but were probably never used, which is why we try v1 just to be safe
                        match DocumentV0::from_bytes_v1(
                            serialized_document,
                            document_type,
                            platform_version,
                        ) {
                            Ok(document_from_version_1_deserialization) => {
                                Ok(document_from_version_1_deserialization)
                            }
                            Err(_) => Err(first_err),
                        }
                    }
                }
            }
            1 => {
                match DocumentV0::from_bytes_v1(
                    serialized_document,
                    document_type,
                    platform_version,
                ) {
                    Ok(document) => Ok(document),
                    Err(first_err) => {
                        // Version byte 1 means document was serialized with sized integer types.
                        // If deserialization fails, it might be due to the config downgrade (V1→V0) bug
                        // where all integer properties became I64 instead of their sized types.
                        // Fallback: reconstruct document type with sized types from schema and retry.
                        match document_type.clone_with_sized_integer_types() {
                            Ok(sized_doc_type) => DocumentV0::from_bytes_v1(
                                serialized_document,
                                sized_doc_type.as_ref(),
                                platform_version,
                            )
                            .map_err(|_| ProtocolError::DataContractError(first_err)),
                            Err(_) => Err(ProtocolError::DataContractError(first_err)),
                        }
                    }
                }
            }
            2 => {
                match DocumentV0::from_bytes_v2(
                    serialized_document,
                    document_type,
                    platform_version,
                ) {
                    Ok(document) => Ok(document),
                    Err(first_err) => {
                        // Same fallback logic as version byte 1
                        match document_type.clone_with_sized_integer_types() {
                            Ok(sized_doc_type) => DocumentV0::from_bytes_v2(
                                serialized_document,
                                sized_doc_type.as_ref(),
                                platform_version,
                            )
                            .map_err(|_| ProtocolError::DataContractError(first_err)),
                            Err(_) => Err(ProtocolError::DataContractError(first_err)),
                        }
                    }
                }
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_bytes (deserialization)".to_string(),
                known_versions: vec![0, 1, 2],
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
                    Err(first_err) => {
                        // let's try decoding in V1 just to be safe
                        // Version 0 will decode all integers as I64
                        // Version 1 will decode all integers properly
                        // When version was 0 used (protocol version 1 to 8) integers other than I64
                        // existed, but were probably never used, which is why we try v1 just to be safe
                        match DocumentV0::from_bytes_v1(
                            serialized_document,
                            document_type,
                            platform_version,
                        ) {
                            Ok(document_from_version_1_deserialization) => {
                                Ok(ConsensusValidationResult::new_with_data(
                                    document_from_version_1_deserialization,
                                ))
                            }
                            Err(_) => Ok(ConsensusValidationResult::new_with_error(
                                ConsensusError::BasicError(BasicError::ContractError(first_err)),
                            )),
                        }
                    }
                }
            }
            1 => {
                match DocumentV0::from_bytes_v1(
                    serialized_document,
                    document_type,
                    platform_version,
                ) {
                    Ok(document) => Ok(ConsensusValidationResult::new_with_data(document)),
                    Err(first_err) => {
                        // Fallback: reconstruct sized types from schema and retry
                        match document_type.clone_with_sized_integer_types() {
                            Ok(sized_doc_type) => {
                                match DocumentV0::from_bytes_v1(
                                    serialized_document,
                                    sized_doc_type.as_ref(),
                                    platform_version,
                                ) {
                                    Ok(document) => {
                                        Ok(ConsensusValidationResult::new_with_data(document))
                                    }
                                    Err(_) => Ok(ConsensusValidationResult::new_with_error(
                                        ConsensusError::BasicError(BasicError::ContractError(
                                            first_err,
                                        )),
                                    )),
                                }
                            }
                            Err(_) => Ok(ConsensusValidationResult::new_with_error(
                                ConsensusError::BasicError(BasicError::ContractError(first_err)),
                            )),
                        }
                    }
                }
            }
            2 => {
                match DocumentV0::from_bytes_v2(
                    serialized_document,
                    document_type,
                    platform_version,
                ) {
                    Ok(document) => Ok(ConsensusValidationResult::new_with_data(document)),
                    Err(first_err) => {
                        // Fallback: reconstruct sized types from schema and retry
                        match document_type.clone_with_sized_integer_types() {
                            Ok(sized_doc_type) => {
                                match DocumentV0::from_bytes_v2(
                                    serialized_document,
                                    sized_doc_type.as_ref(),
                                    platform_version,
                                ) {
                                    Ok(document) => {
                                        Ok(ConsensusValidationResult::new_with_data(document))
                                    }
                                    Err(_) => Ok(ConsensusValidationResult::new_with_error(
                                        ConsensusError::BasicError(BasicError::ContractError(
                                            first_err,
                                        )),
                                    )),
                                }
                            }
                            Err(_) => Ok(ConsensusValidationResult::new_with_error(
                                ConsensusError::BasicError(BasicError::ContractError(first_err)),
                            )),
                        }
                    }
                }
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_bytes (deserialization)".to_string(),
                known_versions: vec![0, 1, 2],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::v0::DataContractConfigV0;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::serialized_version::DataContractInSerializationFormat;
    use crate::data_contract::DataContract;
    use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use crate::identity::accessors::IdentityGettersV0;
    use crate::identity::Identity;
    use crate::tests::fixtures::{get_data_contract_fixture, get_documents_fixture};
    use platform_version::version::PlatformVersion;
    use platform_version::TryIntoPlatformVersioned;

    #[test]
    fn test_version_byte_1_fallback_with_config_downgrade() {
        let platform_version = PlatformVersion::latest();
        let identity = Identity::random_identity(5, Some(100), platform_version)
            .expect("expected a random identity");
        let owner_id = identity.id();

        // Create contract with ConfigV1 (sized_integer_types=true)
        let data_contract =
            get_data_contract_fixture(Some(owner_id), 1, platform_version.protocol_version)
                .data_contract_owned();

        let document_type = data_contract
            .document_type_for_name("niceDocument")
            .expect("expected document type");

        let documents = get_documents_fixture(&data_contract, platform_version.protocol_version)
            .expect("expected documents");

        // Get the first niceDocument
        let document = &documents[0];

        // Serialize with current config (version byte 1 for sized types)
        let serialized = document
            .serialize(document_type, &data_contract, platform_version)
            .expect("expected to serialize");

        // Verify version byte is 1 or 2 (sized integer serialization)
        assert!(
            serialized[0] == 1 || serialized[0] == 2,
            "Expected version byte 1 or 2 for sized integer serialization, got {}",
            serialized[0]
        );

        // Now simulate config downgrade by creating a document type with all I64 properties
        // This is what happens when config changes from V1 to V0
        let mut contract_in_format: DataContractInSerializationFormat = (&data_contract)
            .try_into_platform_versioned(platform_version)
            .expect("expected to convert");

        // Change config to V0 (sized_integer_types=false, all integers become I64)
        match &mut contract_in_format {
            DataContractInSerializationFormat::V0(ref mut v0) => {
                v0.config = DataContractConfig::V0(DataContractConfigV0::default());
            }
            DataContractInSerializationFormat::V1(ref mut v1) => {
                v1.config = DataContractConfig::V0(DataContractConfigV0::default());
            }
        }

        let downgraded_contract = DataContract::try_from_platform_versioned(
            contract_in_format,
            true,
            &mut vec![],
            platform_version,
        )
        .expect("expected to create downgraded contract");

        let downgraded_doc_type = downgraded_contract
            .document_type_for_name("niceDocument")
            .expect("expected document type");

        // Deserialize with downgraded document type — should succeed via fallback
        let result =
            DocumentV0::from_bytes(serialized.as_slice(), downgraded_doc_type, platform_version);

        assert!(
            result.is_ok(),
            "Deserialization with config-downgraded doc type should succeed via fallback. Error: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_version_byte_1_normal_deserialization() {
        let platform_version = PlatformVersion::latest();
        let identity = Identity::random_identity(5, Some(100), platform_version)
            .expect("expected a random identity");
        let owner_id = identity.id();

        let data_contract =
            get_data_contract_fixture(Some(owner_id), 1, platform_version.protocol_version)
                .data_contract_owned();

        let document_type = data_contract
            .document_type_for_name("niceDocument")
            .expect("expected document type");

        let documents = get_documents_fixture(&data_contract, platform_version.protocol_version)
            .expect("expected documents");

        let document = &documents[0];

        let serialized = document
            .serialize(document_type, &data_contract, platform_version)
            .expect("expected to serialize");

        // Normal deserialization should work without fallback
        let result = DocumentV0::from_bytes(serialized.as_slice(), document_type, platform_version);

        assert!(
            result.is_ok(),
            "Normal deserialization should succeed. Error: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_version_byte_0_unaffected() {
        let platform_version = PlatformVersion::first();
        let identity = Identity::random_identity(5, Some(100), platform_version)
            .expect("expected a random identity");
        let owner_id = identity.id();

        let data_contract =
            get_data_contract_fixture(Some(owner_id), 1, platform_version.protocol_version)
                .data_contract_owned();

        let document_type = data_contract
            .document_type_for_name("niceDocument")
            .expect("expected document type");

        let documents = get_documents_fixture(&data_contract, platform_version.protocol_version)
            .expect("expected documents");

        let document = &documents[0];

        let serialized = document
            .serialize(document_type, &data_contract, platform_version)
            .expect("expected to serialize");

        // Version byte 0 docs should deserialize fine even with latest platform version
        let latest = PlatformVersion::latest();
        let result = DocumentV0::from_bytes(serialized.as_slice(), document_type, latest);

        assert!(
            result.is_ok(),
            "Version byte 0 deserialization should work. Error: {:?}",
            result.err()
        );
    }
}
