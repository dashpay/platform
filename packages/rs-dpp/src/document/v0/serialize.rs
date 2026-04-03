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
            1 => DocumentV0::from_bytes_v1(serialized_document, document_type, platform_version)
                .map_err(ProtocolError::DataContractError),
            2 => DocumentV0::from_bytes_v2(serialized_document, document_type, platform_version)
                .map_err(ProtocolError::DataContractError),
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
                    Err(err) => Ok(ConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(BasicError::ContractError(err)),
                    )),
                }
            }
            2 => {
                match DocumentV0::from_bytes_v2(
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
                known_versions: vec![0, 1, 2],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::tests::json_document::json_document_to_contract;
    use integer_encoding::VarInt;
    use platform_version::version::PlatformVersion;

    // ----------------------------------------------------------------
    // Helper: load the dashpay contract and return the contract plus a
    // DocumentTypeRef for the given document type name.
    // ----------------------------------------------------------------
    fn dashpay_contract_and_type(
        platform_version: &PlatformVersion,
    ) -> (crate::prelude::DataContract, String) {
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected to load dashpay contract");
        (contract, "contactRequest".to_string())
    }

    fn family_contract(platform_version: &PlatformVersion) -> crate::prelude::DataContract {
        json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/family/family-contract.json",
            false,
            platform_version,
        )
        .expect("expected to load family contract")
    }

    fn withdrawals_contract(platform_version: &PlatformVersion) -> crate::prelude::DataContract {
        json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/withdrawals/withdrawals-contract.json",
            false,
            platform_version,
        )
        .expect("expected to load withdrawals contract")
    }

    // ================================================================
    //  Round-trip: serialize then deserialize, expect equality
    // ================================================================

    #[test]
    fn round_trip_serialize_v0_dashpay_contact_request() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let document = document_type
            .random_document(Some(42), platform_version)
            .expect("expected random document");

        let doc_v0 = match &document {
            crate::document::Document::V0(d) => d,
        };

        let serialized = doc_v0
            .serialize(document_type, &contract, platform_version)
            .expect("serialize should succeed");

        let deserialized = DocumentV0::from_bytes(&serialized, document_type, platform_version)
            .expect("from_bytes should succeed");

        assert_eq!(*doc_v0, deserialized);
    }

    #[test]
    fn round_trip_serialize_v0_family_person() {
        let platform_version = PlatformVersion::first();
        let contract = family_contract(platform_version);
        let document_type = contract
            .document_type_for_name("person")
            .expect("expected person document type");

        for seed in 0..20u64 {
            let document = document_type
                .random_document(Some(seed), platform_version)
                .expect("expected random document");
            let doc_v0 = match &document {
                crate::document::Document::V0(d) => d,
            };
            let serialized = doc_v0
                .serialize(document_type, &contract, platform_version)
                .expect("serialize should succeed");
            let deserialized = DocumentV0::from_bytes(&serialized, document_type, platform_version)
                .expect("from_bytes should succeed");
            assert_eq!(*doc_v0, deserialized, "round-trip failed for seed {seed}");
        }
    }

    #[test]
    fn round_trip_serialize_v1_family_person() {
        // Platform version that defaults to serialization v1
        let platform_version =
            PlatformVersion::get(9).unwrap_or_else(|_| PlatformVersion::latest());

        // We need a non-V0 contract for v1 serialization. Use the latest platform version
        // to load the contract and create a document type.
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/family/family-contract.json",
            false,
            platform_version,
        )
        .expect("expected to load family contract");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected person document type");

        // Only test if we can actually produce v1 serialization
        // (contract must not be V0 and config must not be V0 for v1)
        if matches!(&contract, DataContract::V0(_)) {
            // V0 contracts always force serialize_v0, so we test that path instead
            let document = document_type
                .random_document(Some(99), platform_version)
                .expect("expected random document");
            let doc_v0 = match &document {
                crate::document::Document::V0(d) => d,
            };
            let serialized = doc_v0
                .serialize(document_type, &contract, platform_version)
                .expect("serialize should succeed");
            let deserialized = DocumentV0::from_bytes(&serialized, document_type, platform_version)
                .expect("from_bytes should succeed");
            assert_eq!(*doc_v0, deserialized);
        } else {
            let document = document_type
                .random_document(Some(99), platform_version)
                .expect("expected random document");
            let doc_v0 = match &document {
                crate::document::Document::V0(d) => d,
            };
            let serialized = doc_v0
                .serialize(document_type, &contract, platform_version)
                .expect("serialize should succeed");
            let deserialized = DocumentV0::from_bytes(&serialized, document_type, platform_version)
                .expect("from_bytes should succeed");
            assert_eq!(*doc_v0, deserialized);
        }
    }

    #[test]
    fn round_trip_serialize_v2_latest_platform() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected to load dashpay contract");

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected contactRequest document type");

        let document = document_type
            .random_document(Some(7), platform_version)
            .expect("expected random document");
        let doc_v0 = match &document {
            crate::document::Document::V0(d) => d,
        };
        let serialized = doc_v0
            .serialize(document_type, &contract, platform_version)
            .expect("serialize should succeed");
        let deserialized = DocumentV0::from_bytes(&serialized, document_type, platform_version)
            .expect("from_bytes should succeed");
        assert_eq!(*doc_v0, deserialized);
    }

    #[test]
    fn round_trip_withdrawals_document() {
        let platform_version = PlatformVersion::latest();
        let contract = withdrawals_contract(platform_version);
        let document_type = contract
            .document_type_for_name("withdrawal")
            .expect("expected withdrawal document type");

        let document = document_type
            .random_document(Some(55), platform_version)
            .expect("expected random document");
        let doc_v0 = match &document {
            crate::document::Document::V0(d) => d,
        };
        let serialized = doc_v0
            .serialize(document_type, &contract, platform_version)
            .expect("serialize should succeed");
        let deserialized = DocumentV0::from_bytes(&serialized, document_type, platform_version)
            .expect("from_bytes should succeed");
        assert_eq!(*doc_v0, deserialized);
    }

    // ================================================================
    //  serialize_specific_version tests
    // ================================================================

    #[test]
    fn serialize_specific_version_v0_produces_version_0_prefix() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let document = document_type
            .random_document(Some(1), platform_version)
            .expect("expected random document");
        let doc_v0 = match &document {
            crate::document::Document::V0(d) => d,
        };

        let serialized = doc_v0
            .serialize_specific_version(document_type, &contract, 0)
            .expect("serialize_specific_version v0 should succeed");

        // The first bytes should be varint-encoded 0
        let (version, _) = u64::decode_var(&serialized).expect("expected varint");
        assert_eq!(version, 0, "serialization version prefix should be 0");
    }

    #[test]
    fn serialize_specific_version_rejects_v1_for_v0_contract() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);

        // V0 contracts should reject non-0 feature versions
        if matches!(&contract, DataContract::V0(_)) {
            let document_type = contract
                .document_type_for_name(&type_name)
                .expect("expected document type");

            let document = document_type
                .random_document(Some(1), platform_version)
                .expect("expected random document");
            let doc_v0 = match &document {
                crate::document::Document::V0(d) => d,
            };

            let result = doc_v0.serialize_specific_version(document_type, &contract, 1);
            assert!(
                result.is_err(),
                "V0 contract should reject serialize_specific_version with feature_version != 0"
            );
        }
    }

    #[test]
    fn serialize_specific_version_unknown_version_returns_error() {
        let platform_version = PlatformVersion::latest();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let document = document_type
            .random_document(Some(1), platform_version)
            .expect("expected random document");
        let doc_v0 = match &document {
            crate::document::Document::V0(d) => d,
        };

        let result = doc_v0.serialize_specific_version(document_type, &contract, 255);
        assert!(
            result.is_err(),
            "unknown feature version should produce an error"
        );
        // V0 contracts reject any non-0 version with NotSupported before reaching the
        // version dispatch. Non-V0 contracts would reach the version dispatch and return
        // UnknownVersionMismatch.
        match result.unwrap_err() {
            ProtocolError::UnknownVersionMismatch { received, .. } => {
                assert_eq!(received, 255);
            }
            ProtocolError::NotSupported(_) => {
                // V0 contract path: rejects non-0 feature version before dispatching
            }
            other => panic!(
                "expected UnknownVersionMismatch or NotSupported, got {:?}",
                other
            ),
        }
    }

    // ================================================================
    //  from_bytes deserialization error cases
    // ================================================================

    #[test]
    fn from_bytes_v0_rejects_too_small_buffer() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        // Buffer with varint(0) prefix then only 10 bytes (too small for id+owner_id = 64 bytes)
        let mut small_buf = 0u64.encode_var_vec();
        small_buf.extend_from_slice(&[0u8; 10]);

        let result = DocumentV0::from_bytes(&small_buf, document_type, platform_version);
        assert!(
            result.is_err(),
            "buffer shorter than 64 bytes after version prefix should fail"
        );
    }

    #[test]
    fn from_bytes_empty_buffer_fails() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let result = DocumentV0::from_bytes(&[], document_type, platform_version);
        assert!(result.is_err(), "empty buffer should fail deserialization");
    }

    #[test]
    fn from_bytes_unknown_serialization_version_fails() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        // Encode a version that is not 0, 1, or 2
        let mut buf = 200u64.encode_var_vec();
        buf.extend_from_slice(&[0u8; 100]); // padding

        let result = DocumentV0::from_bytes(&buf, document_type, platform_version);
        assert!(result.is_err(), "unknown version should be rejected");
        match result.unwrap_err() {
            ProtocolError::UnknownVersionMismatch { received, .. } => {
                assert_eq!(received, 200);
            }
            other => panic!("expected UnknownVersionMismatch, got {:?}", other),
        }
    }

    #[test]
    fn from_bytes_truncated_after_ids_fails() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        // Valid version prefix, valid 64-byte id+owner_id, then no more data
        // This should fail when trying to read revision or timestamp flags
        let mut buf = 0u64.encode_var_vec();
        buf.extend_from_slice(&[0xAA; 64]); // id (32) + owner_id (32)

        let result = DocumentV0::from_bytes(&buf, document_type, platform_version);
        assert!(
            result.is_err(),
            "truncated buffer after ids should fail deserialization"
        );
    }

    // ================================================================
    //  Serialization format: verify version prefix encoding
    // ================================================================

    #[test]
    fn serialization_starts_with_correct_version_varint() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let document = document_type
            .random_document(Some(100), platform_version)
            .expect("expected random document");
        let doc_v0 = match &document {
            crate::document::Document::V0(d) => d,
        };

        // serialize_v0 should prefix with varint 0
        let bytes = doc_v0
            .serialize_v0(document_type)
            .expect("serialize_v0 should succeed");
        let (ver, _) = u64::decode_var(&bytes).expect("varint decode");
        assert_eq!(ver, 0);

        // serialize_v1 should prefix with varint 1
        let bytes = doc_v0
            .serialize_v1(document_type)
            .expect("serialize_v1 should succeed");
        let (ver, _) = u64::decode_var(&bytes).expect("varint decode");
        assert_eq!(ver, 1);

        // serialize_v2 should prefix with varint 2
        let bytes = doc_v0
            .serialize_v2(document_type)
            .expect("serialize_v2 should succeed");
        let (ver, _) = u64::decode_var(&bytes).expect("varint decode");
        assert_eq!(ver, 2);
    }

    #[test]
    fn serialized_id_and_owner_id_are_embedded_after_version() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let document = document_type
            .random_document(Some(42), platform_version)
            .expect("expected random document");
        let doc_v0 = match &document {
            crate::document::Document::V0(d) => d,
        };

        let bytes = doc_v0
            .serialize_v0(document_type)
            .expect("serialize should succeed");

        // Version 0 is a single-byte varint
        let (_, varint_len) = u64::decode_var(&bytes).expect("varint decode");
        let after_version = &bytes[varint_len..];

        // Next 32 bytes = id
        assert_eq!(
            &after_version[..32],
            doc_v0.id.as_slice(),
            "id should be at offset after version"
        );
        // Following 32 bytes = owner_id
        assert_eq!(
            &after_version[32..64],
            doc_v0.owner_id.as_slice(),
            "owner_id should follow id"
        );
    }

    // ================================================================
    //  Determinism: same document serializes to the same bytes
    // ================================================================

    #[test]
    fn serialization_is_deterministic() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let document = document_type
            .random_document(Some(99), platform_version)
            .expect("expected random document");
        let doc_v0 = match &document {
            crate::document::Document::V0(d) => d,
        };

        let bytes1 = doc_v0
            .serialize(document_type, &contract, platform_version)
            .expect("first serialize");
        let bytes2 = doc_v0
            .serialize(document_type, &contract, platform_version)
            .expect("second serialize");
        assert_eq!(bytes1, bytes2, "serialization must be deterministic");
    }

    // ================================================================
    //  Multiple random documents round-trip (fuzz-like)
    // ================================================================

    #[test]
    fn round_trip_many_random_documents() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        for seed in 0..50u64 {
            let document = document_type
                .random_document(Some(seed), platform_version)
                .expect("expected random document");
            let doc_v0 = match &document {
                crate::document::Document::V0(d) => d,
            };
            let serialized = doc_v0
                .serialize(document_type, &contract, platform_version)
                .expect("serialize should succeed");
            let deserialized = DocumentV0::from_bytes(&serialized, document_type, platform_version)
                .expect("from_bytes should succeed");
            assert_eq!(*doc_v0, deserialized, "round-trip mismatch for seed {seed}");
        }
    }

    // ================================================================
    //  from_bytes_in_consensus
    // ================================================================

    #[cfg(feature = "validation")]
    #[test]
    fn from_bytes_in_consensus_valid_data_returns_valid_result() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let document = document_type
            .random_document(Some(77), platform_version)
            .expect("expected random document");
        let doc_v0 = match &document {
            crate::document::Document::V0(d) => d,
        };
        let serialized = doc_v0
            .serialize(document_type, &contract, platform_version)
            .expect("serialize should succeed");

        let result =
            DocumentV0::from_bytes_in_consensus(&serialized, document_type, platform_version)
                .expect("from_bytes_in_consensus should not return ProtocolError");

        assert!(result.is_valid(), "consensus result should be valid");
        let deserialized = result.into_data().expect("should have data");
        assert_eq!(*doc_v0, deserialized);
    }

    #[cfg(feature = "validation")]
    #[test]
    fn from_bytes_in_consensus_invalid_data_returns_consensus_error() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        // Version 0, then truncated data
        let mut buf = 0u64.encode_var_vec();
        buf.extend_from_slice(&[0u8; 10]);

        let result = DocumentV0::from_bytes_in_consensus(&buf, document_type, platform_version)
            .expect("should not return ProtocolError for consensus-level decode");

        assert!(
            !result.is_valid(),
            "consensus result should contain errors for malformed data"
        );
    }

    #[cfg(feature = "validation")]
    #[test]
    fn from_bytes_in_consensus_unknown_version_returns_protocol_error() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let mut buf = 200u64.encode_var_vec();
        buf.extend_from_slice(&[0u8; 100]);

        let result = DocumentV0::from_bytes_in_consensus(&buf, document_type, platform_version);
        assert!(
            result.is_err(),
            "unknown version should produce a ProtocolError, not a consensus error"
        );
    }

    // ================================================================
    //  Known-bytes deserialization (golden test)
    // ================================================================

    #[test]
    fn deserialize_known_withdrawal_bytes() {
        let platform_version = PlatformVersion::latest();
        let contract = withdrawals_contract(platform_version);

        let document_type = contract
            .document_type_for_name("withdrawal")
            .expect("expected withdrawal document type");

        // This is a real serialized withdrawal document (from existing test)
        let serialized = hex::decode(
            "010053626cafc76f47062f936c5938190f5f30aac997b8fc22e81c1d9a7f903bd9\
             fa8696d3f39c518784e53be79ee199e70387f9a7408254de920c1f3779de285601\
             00030000019782b96d140000019782b96d14000000000002540be40000000001\
             001976a9149e3292d2612122d81613fdb893dd36a04df3355588ac00",
        )
        .expect("expected valid hex");

        let deserialized = DocumentV0::from_bytes(&serialized, document_type, platform_version)
            .expect("expected deserialization to succeed");

        // Verify known fields
        assert_eq!(
            hex::encode(deserialized.id.as_slice()),
            "0053626cafc76f47062f936c5938190f5f30aac997b8fc22e81c1d9a7f903bd9"
        );
        assert_eq!(
            hex::encode(deserialized.owner_id.as_slice()),
            "fa8696d3f39c518784e53be79ee199e70387f9a7408254de920c1f3779de2856"
        );
        assert_eq!(deserialized.revision, Some(1));
        assert_eq!(deserialized.created_at, Some(1750244879636));
        assert_eq!(deserialized.updated_at, Some(1750244879636));
    }
}
