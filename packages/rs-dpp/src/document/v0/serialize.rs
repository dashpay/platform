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
                        if property.required_at(None) && !property.transient {
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
                        if !property.required_at(None) || property.transient {
                            // dbg!("we added 1", field_name);
                            buffer.push(1);
                        }
                        let value = if property.property_type.is_integer() {
                            DocumentPropertyType::I64
                                .encode_value_ref_with_size(value, property.required_at(None))
                        } else {
                            property
                                .property_type
                                .encode_value_ref_with_size(value, property.required_at(None))
                        }?;

                        // dbg!("we pushed {} with {}", field_name, hex::encode(&value));
                        buffer.extend(value.as_slice());
                        Ok(())
                    }
                } else if property.required_at(None) && !property.transient {
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
                        if property.required_at(None) && !property.transient {
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
                        if !property.required_at(None) || property.transient {
                            // dbg!("we added 1", field_name);
                            buffer.push(1);
                        }
                        let value = property
                            .property_type
                            .encode_value_ref_with_size(value, property.required_at(None))?;
                        // dbg!("we pushed {} with {}", field_name, hex::encode(&value));
                        buffer.extend(value.as_slice());
                        Ok(())
                    }
                } else if property.required_at(None) && !property.transient {
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
                        if property.required_at(None) && !property.transient {
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
                        if !property.required_at(None) || property.transient {
                            // dbg!("we added 1", field_name);
                            buffer.push(1);
                        }
                        let value = property
                            .property_type
                            .encode_value_ref_with_size(value, property.required_at(None))?;
                        // dbg!("we pushed {} with {}", field_name, hex::encode(&value));
                        buffer.extend(value.as_slice());
                        Ok(())
                    }
                } else if property.required_at(None) && !property.transient {
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
    /// Serialize v3 is v2 plus the contract version stamp: a varint right
    /// after the format prefix recording the data contract version the bytes
    /// conform to (0 = unstamped, for pre-format-3 documents that are
    /// re-serialized). A property whose `requiredSince` exceeds the stamp is
    /// encoded with a presence flag exactly like an optional property, so
    /// documents written before the property became required stay valid.
    fn serialize_v3(&self, document_type: DocumentTypeRef) -> Result<Vec<u8>, ProtocolError> {
        let mut buffer: Vec<u8> = 3u64.encode_var_vec(); //version 3

        // the contract version stamp; 0 means unstamped
        buffer.extend((self.contract_version.unwrap_or_default() as u64).encode_var_vec());

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

        // User defined properties: requiredness is evaluated at this
        // document's stamp, so a property that became required after the
        // stamp keeps the presence-flagged layout it was written with
        document_type
            .properties()
            .iter()
            .try_for_each(|(field_name, property)| {
                let required = property.required_at(self.contract_version);
                if let Some(value) = self.properties.get(field_name) {
                    if value.is_null() {
                        if required && !property.transient {
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
                    } else {
                        if !required || property.transient {
                            buffer.push(1);
                        }
                        let value = property
                            .property_type
                            .encode_value_ref_with_size(value, required)?;
                        buffer.extend(value.as_slice());
                        Ok(())
                    }
                } else if required && !property.transient {
                    Err(ProtocolError::DataContractError(
                        DataContractError::MissingRequiredKey(format!(
                            "a required field {field_name} is not present"
                        )),
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
                    return if property.required_at(None) && !property.transient {
                        Some(Err(DataContractError::CorruptedSerialization(
                            "required field after finished buffer".to_string(),
                        )))
                    } else {
                        None
                    };
                }

                // In version 0 all integers are encoded as I64 (in theory)
                let read_value = if property.property_type.is_integer() {
                    DocumentPropertyType::I64.read_optionally_from(
                        &mut buf,
                        property.required_at(None) & !property.transient,
                    )
                } else {
                    property.property_type.read_optionally_from(
                        &mut buf,
                        property.required_at(None) & !property.transient,
                    )
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
            contract_version: None,
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
                    return if property.required_at(None) && !property.transient {
                        Some(Err(DataContractError::CorruptedSerialization(
                            "required field after finished buffer".to_string(),
                        )))
                    } else {
                        None
                    };
                }
                let read_value = property.property_type.read_optionally_from(
                    &mut buf,
                    property.required_at(None) & !property.transient,
                );

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
            contract_version: None,
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
                    return if property.required_at(None) && !property.transient {
                        Some(Err(DataContractError::CorruptedSerialization(
                            "required field after finished buffer".to_string(),
                        )))
                    } else {
                        None
                    };
                }
                let read_value = property.property_type.read_optionally_from(
                    &mut buf,
                    property.required_at(None) & !property.transient,
                );

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
            contract_version: None,
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

    /// Reads a serialized document and creates a Document from it.
    /// Version 3 is version 2 plus the contract version stamp, which selects
    /// each `requiredSince` property's byte layout: raw when the stamp has
    /// reached the property's `requiredSince`, presence-flagged otherwise.
    fn from_bytes_v3(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, DataContractError> {
        let mut buf = BufReader::new(serialized_document);
        if serialized_document.len() < 65 {
            return Err(DataContractError::DecodingDocumentError(
                DecodingError::new(
                    "serialized document is too small, must have contract version, id and owner id"
                        .to_string(),
                ),
            ));
        }

        // the contract version stamp; 0 means unstamped
        let stamp: u64 = buf.read_varint().map_err(|_| {
            DataContractError::DecodingDocumentError(DecodingError::new(
                "error reading contract version stamp from serialized document".to_string(),
            ))
        })?;
        if stamp > u32::MAX as u64 {
            return Err(DataContractError::CorruptedSerialization(
                "contract version stamp does not fit in a u32".to_string(),
            ));
        }
        let contract_version = if stamp == 0 { None } else { Some(stamp as u32) };

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
                let required = property.required_at(contract_version);
                if finished_buffer {
                    return if required && !property.transient {
                        Some(Err(DataContractError::CorruptedSerialization(
                            "required field after finished buffer".to_string(),
                        )))
                    } else {
                        None
                    };
                }
                let read_value = property
                    .property_type
                    .read_optionally_from(&mut buf, required & !property.transient);

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
            contract_version,
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
                // Version 3 coincides with protocol version 14: it stamps the
                // document with the contract version its bytes conform to,
                // enabling `requiredSince` properties.
                3 => self.serialize_v3(document_type),
                version => Err(ProtocolError::UnknownVersionMismatch {
                    method: "DocumentV0::serialize".to_string(),
                    known_versions: vec![0, 1, 2, 3],
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
            3 => self.serialize_v3(document_type),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentV0::serialize".to_string(),
                known_versions: vec![0, 1, 2, 3],
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
            3 => DocumentV0::from_bytes_v3(serialized_document, document_type, platform_version)
                .map_err(ProtocolError::DataContractError),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_bytes (deserialization)".to_string(),
                known_versions: vec![0, 1, 2, 3],
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
            3 => {
                match DocumentV0::from_bytes_v3(
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
                known_versions: vec![0, 1, 2, 3],
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

        let crate::document::Document::V0(doc_v0) = &document;

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
            let crate::document::Document::V0(doc_v0) = &document;
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
            let crate::document::Document::V0(doc_v0) = &document;
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
            let crate::document::Document::V0(doc_v0) = &document;
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
        let crate::document::Document::V0(doc_v0) = &document;
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
        let crate::document::Document::V0(doc_v0) = &document;
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
        let crate::document::Document::V0(doc_v0) = &document;

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
            let crate::document::Document::V0(doc_v0) = &document;

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
        let crate::document::Document::V0(doc_v0) = &document;

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
        let crate::document::Document::V0(doc_v0) = &document;

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
        let crate::document::Document::V0(doc_v0) = &document;

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
        let crate::document::Document::V0(doc_v0) = &document;

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
            let crate::document::Document::V0(doc_v0) = &document;
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
        let crate::document::Document::V0(doc_v0) = &document;
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
    //  Missing-required-field errors in serialize_v0 / v1 / v2.
    //  The withdrawal contract requires both $createdAt and $updatedAt,
    //  so a DocumentV0 lacking those should fail serialization.
    // ================================================================

    fn doc_with_ids() -> DocumentV0 {
        DocumentV0 {
            contract_version: None,
            id: Identifier::new([1u8; 32]),
            owner_id: Identifier::new([2u8; 32]),
            properties: BTreeMap::new(),
            revision: Some(1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
    }

    #[test]
    fn serialize_v0_missing_created_at_errors_when_required() {
        let platform_version = PlatformVersion::latest();
        let contract = withdrawals_contract(platform_version);
        let document_type = contract
            .document_type_for_name("withdrawal")
            .expect("withdrawal document type");

        // Build a document missing $createdAt. Don't include any user-defined
        // required properties either — we want to trigger the $createdAt path
        // before the user-property path.
        let doc = doc_with_ids();

        let err = doc
            .serialize_v0(document_type)
            .expect_err("serialize_v0 should fail for missing $createdAt");
        match err {
            ProtocolError::DataContractError(DataContractError::MissingRequiredKey(msg)) => {
                assert!(
                    msg.contains("created at"),
                    "expected missing-created-at message, got: {msg}"
                );
            }
            other => panic!(
                "expected MissingRequiredKey for created_at, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn serialize_v0_missing_updated_at_errors_when_required() {
        let platform_version = PlatformVersion::latest();
        let contract = withdrawals_contract(platform_version);
        let document_type = contract
            .document_type_for_name("withdrawal")
            .expect("withdrawal document type");

        // Supply $createdAt but not $updatedAt — both are required.
        let mut doc = doc_with_ids();
        doc.created_at = Some(1_700_000_000_000);

        let err = doc
            .serialize_v0(document_type)
            .expect_err("serialize_v0 should fail for missing $updatedAt");
        match err {
            ProtocolError::DataContractError(DataContractError::MissingRequiredKey(msg)) => {
                assert!(
                    msg.contains("updated at"),
                    "expected missing-updated-at message, got: {msg}"
                );
            }
            other => panic!(
                "expected MissingRequiredKey for updated_at, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn serialize_v1_missing_created_at_errors_when_required() {
        let platform_version = PlatformVersion::latest();
        let contract = withdrawals_contract(platform_version);
        let document_type = contract
            .document_type_for_name("withdrawal")
            .expect("withdrawal document type");

        let doc = doc_with_ids();
        let err = doc
            .serialize_v1(document_type)
            .expect_err("serialize_v1 should fail for missing $createdAt");
        assert!(matches!(
            err,
            ProtocolError::DataContractError(DataContractError::MissingRequiredKey(_))
        ));
    }

    #[test]
    fn serialize_v2_missing_created_at_errors_when_required() {
        let platform_version = PlatformVersion::latest();
        let contract = withdrawals_contract(platform_version);
        let document_type = contract
            .document_type_for_name("withdrawal")
            .expect("withdrawal document type");

        let doc = doc_with_ids();
        let err = doc
            .serialize_v2(document_type)
            .expect_err("serialize_v2 should fail for missing $createdAt");
        assert!(matches!(
            err,
            ProtocolError::DataContractError(DataContractError::MissingRequiredKey(_))
        ));
    }

    #[test]
    fn serialize_v0_missing_required_user_property_errors() {
        // Family `person` requires `firstName`, `lastName`, `age`.
        let platform_version = PlatformVersion::first();
        let contract = family_contract(platform_version);
        let document_type = contract
            .document_type_for_name("person")
            .expect("person document type");

        // Document with only ids, no user-defined required properties set.
        let doc = doc_with_ids();

        let err = doc
            .serialize_v0(document_type)
            .expect_err("serialize_v0 should fail for missing required property");
        match err {
            ProtocolError::DataContractError(DataContractError::MissingRequiredKey(msg)) => {
                // The error message includes the field name for user-defined required fields.
                let any_expected = msg.contains("firstName")
                    || msg.contains("lastName")
                    || msg.contains("age")
                    || msg.contains("required field");
                assert!(any_expected, "unexpected error message: {msg}");
            }
            other => panic!("expected MissingRequiredKey, got {:?}", other),
        }
    }

    // ================================================================
    //  from_bytes: V1 prefix dispatches to from_bytes_v1 directly
    // ================================================================

    #[test]
    fn from_bytes_v1_prefix_dispatches_to_v1_path() {
        let platform_version = PlatformVersion::latest();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let document = document_type
            .random_document(Some(123), platform_version)
            .expect("expected random document");
        let crate::document::Document::V0(doc_v0) = &document;

        // Bypass the V0-contract gate by calling serialize_v1 directly: the
        // resulting varint-1 prefix must round-trip through from_bytes.
        let bytes = doc_v0.serialize_v1(document_type).expect("serialize_v1");
        let (ver, _) = u64::decode_var(&bytes).expect("varint");
        assert_eq!(ver, 1);
        let recovered = DocumentV0::from_bytes(&bytes, document_type, platform_version)
            .expect("from_bytes should dispatch to v1");
        assert_eq!(*doc_v0, recovered);
    }

    // ================================================================
    //  from_bytes: V2 prefix round-trip for documents with a creator_id
    //  (contactRequest is transferable, so v2 records the creator flag).
    // ================================================================

    #[test]
    fn from_bytes_v2_non_transferable_type_does_not_persist_creator_id() {
        // frozen: V0 consensus behavior — contactRequest is non-transferable
        // with TradeMode::None, so v2 intentionally skips the creator_id byte
        // in both serialize_v2 and from_bytes_v2. Assigning a creator_id on
        // the source document is therefore NOT round-tripped.
        let platform_version = PlatformVersion::latest();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let document = document_type
            .random_document(Some(321), platform_version)
            .expect("expected random document");
        let crate::document::Document::V0(mut doc_v0) = document;
        // Even setting a creator_id here has no on-wire effect for this type.
        doc_v0.creator_id = Some(Identifier::new([0xAB; 32]));

        let bytes = doc_v0.serialize_v2(document_type).expect("serialize_v2");
        let (ver, _) = u64::decode_var(&bytes).expect("varint");
        assert_eq!(ver, 2);

        let recovered = DocumentV0::from_bytes(&bytes, document_type, platform_version)
            .expect("from_bytes should dispatch to v2");
        assert_eq!(doc_v0.id, recovered.id);
        assert_eq!(doc_v0.owner_id, recovered.owner_id);
        assert_eq!(
            recovered.creator_id, None,
            "creator_id is not encoded for non-transferable / TradeMode::None types"
        );
    }

    #[test]
    fn from_bytes_v2_prefix_round_trip_with_none_creator_id() {
        let platform_version = PlatformVersion::latest();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let document = document_type
            .random_document(Some(999), platform_version)
            .expect("expected random document");
        let crate::document::Document::V0(mut doc_v0) = document;
        // creator_id is None — exercise the else-branch of v2's creator check.
        doc_v0.creator_id = None;

        let bytes = doc_v0.serialize_v2(document_type).expect("serialize_v2");
        let recovered =
            DocumentV0::from_bytes(&bytes, document_type, platform_version).expect("from_bytes v2");
        assert_eq!(recovered.creator_id, None);
    }

    // ================================================================
    //  from_bytes_v1 / v2 directly — too-small buffers should error
    //  before we read any id / owner id bytes.
    // ================================================================

    #[test]
    fn from_bytes_v1_direct_too_small_buffer_errors() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let result = DocumentV0::from_bytes_v1(&[0u8; 10], document_type, platform_version);
        assert!(
            result.is_err(),
            "from_bytes_v1 should fail for buffer < 64 bytes"
        );
    }

    #[test]
    fn from_bytes_v2_direct_too_small_buffer_errors() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let result = DocumentV0::from_bytes_v2(&[0u8; 10], document_type, platform_version);
        assert!(
            result.is_err(),
            "from_bytes_v2 should fail for buffer < 64 bytes"
        );
    }

    #[test]
    fn from_bytes_v0_direct_too_small_buffer_errors() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let result = DocumentV0::from_bytes_v0(&[0u8; 10], document_type, platform_version);
        assert!(
            result.is_err(),
            "from_bytes_v0 should fail for buffer < 64 bytes"
        );
    }

    // ================================================================
    //  from_bytes: V1 prefix with truncated post-id data errors
    // ================================================================

    #[test]
    fn from_bytes_v1_truncated_post_ids_errors() {
        let platform_version = PlatformVersion::latest();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        // V1 varint + 64 bytes (id + owner_id) — nothing after that, so the
        // revision / timestamp_flags read must fail.
        let mut buf = 1u64.encode_var_vec();
        buf.extend_from_slice(&[0xCD; 64]);

        let result = DocumentV0::from_bytes(&buf, document_type, platform_version);
        assert!(
            result.is_err(),
            "v1 with truncated post-ids should fail deserialization"
        );
    }

    #[test]
    fn from_bytes_v2_truncated_post_ids_errors() {
        let platform_version = PlatformVersion::latest();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let mut buf = 2u64.encode_var_vec();
        buf.extend_from_slice(&[0xCD; 64]);

        let result = DocumentV0::from_bytes(&buf, document_type, platform_version);
        assert!(
            result.is_err(),
            "v2 with truncated post-ids should fail deserialization"
        );
    }

    // ================================================================
    //  serialize_specific_version: V0 contract + feature_version 0
    //  should succeed (the V0-gated NotSupported branch is NOT hit).
    // ================================================================

    #[test]
    fn serialize_specific_version_v0_contract_feature_version_0_succeeds() {
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let document = document_type
            .random_document(Some(5), platform_version)
            .expect("expected random document");
        let crate::document::Document::V0(doc_v0) = &document;

        // feature_version 0 is explicitly allowed for V0 contracts.
        let bytes = doc_v0
            .serialize_specific_version(document_type, &contract, 0)
            .expect("serialize_specific_version v0 should succeed on a V0 contract");
        let (ver, _) = u64::decode_var(&bytes).expect("varint decode");
        assert_eq!(ver, 0);
    }

    // ================================================================
    //  serialize_specific_version: feature_version 2 with a non-V0
    //  contract (latest platform version) should succeed.
    // ================================================================

    #[test]
    fn serialize_specific_version_rejects_v2_for_v0_contract() {
        // V0 contracts always force serialize_v0, so feature_version 2 is
        // rejected with NotSupported before reaching the version dispatch.
        // Use `PlatformVersion::first()` so the fixture is guaranteed to load
        // as a V0 contract — without this, the test could pass vacuously if
        // the dashpay contract began deserializing as a non-V0 variant.
        let platform_version = PlatformVersion::first();
        let (contract, type_name) = dashpay_contract_and_type(platform_version);
        let document_type = contract
            .document_type_for_name(&type_name)
            .expect("expected document type");

        let document = document_type
            .random_document(Some(17), platform_version)
            .expect("expected random document");
        let crate::document::Document::V0(doc_v0) = &document;

        // Precondition: the fixture must actually be a V0 contract, otherwise
        // the NotSupported branch we intend to exercise would never be hit.
        assert!(
            matches!(&contract, DataContract::V0(_)),
            "fixture must be a V0 contract to exercise the V0-gated NotSupported branch"
        );

        let err = doc_v0
            .serialize_specific_version(document_type, &contract, 2)
            .expect_err("V0 contract should reject v2");
        match err {
            ProtocolError::NotSupported(_) => {}
            other => panic!("expected NotSupported, got {:?}", other),
        }
    }

    // ================================================================
    //  from_bytes V0-then-V1 fallback: a valid V1 buffer with a V0
    //  varint prefix should still round-trip via the fallback path.
    //  Construct bytes by serializing v1 and then overwriting the
    //  varint prefix to 0.
    // ================================================================

    #[test]
    fn from_bytes_v0_falls_back_to_v1_on_decoding_error() {
        // Use a contract whose properties are all integers so v0 (I64) and v1
        // (actual type) produce different encoded lengths / types. family's
        // `person` has one integer field `age`, suitable for fallback testing.
        let platform_version = PlatformVersion::latest();
        let contract = family_contract(platform_version);
        let document_type = contract
            .document_type_for_name("person")
            .expect("person document type");

        // Random document serialized in v1 format (integers kept as native).
        let document = document_type
            .random_document(Some(55), platform_version)
            .expect("random document");
        let crate::document::Document::V0(doc_v0) = document;

        // Serialize in v1 explicitly.
        let mut v1_bytes = doc_v0
            .serialize_v1(document_type)
            .expect("serialize_v1 should succeed");
        // Overwrite the varint-1 prefix with varint-0.
        v1_bytes[0] = 0;

        // from_bytes dispatches to v0, which fails on the mismatched layout,
        // then retries via v1 — the fallback must recover the original document.
        let recovered = DocumentV0::from_bytes(&v1_bytes, document_type, platform_version)
            .expect("v0-prefixed v1 payload must fall back to v1 deserialization");
        assert_eq!(recovered, doc_v0);
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

    // ================================================================
    //  Format 3: the contract-version stamp and requiredSince layouts
    // ================================================================

    /// A document type with:
    ///   - `a`: required at every version
    ///   - `b`: required since contract version 2
    ///   - `c`: plain optional
    fn required_since_document_type() -> crate::data_contract::document_type::DocumentType {
        use crate::data_contract::config::DataContractConfig;
        use crate::data_contract::document_type::DocumentType;
        use platform_value::platform_value;
        use std::collections::BTreeMap;

        let platform_version = PlatformVersion::latest();
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                "b": {"type": "string", "position": 1, "maxLength": 60_u32, "requiredSince": 2},
                "c": {"type": "string", "position": 2, "maxLength": 60_u32},
            },
            "required": ["a", "b"],
            "additionalProperties": false,
        });
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config");
        DocumentType::try_from_schema(
            platform_value::Identifier::new([1; 32]),
            1,
            config.version(),
            "test",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut Vec::new(),
            platform_version,
        )
        .expect("failed to create document type")
    }

    fn stamped_document(
        contract_version: Option<u32>,
        properties: BTreeMap<String, Value>,
        document_type: DocumentTypeRef,
    ) -> DocumentV0 {
        DocumentV0 {
            contract_version,
            id: Identifier::new([3; 32]),
            owner_id: Identifier::new([4; 32]),
            properties,
            revision: document_type.initial_revision(),
            ..Default::default()
        }
    }

    #[test]
    fn serialize_v3_round_trips_document_stamped_at_required_since() {
        let platform_version = PlatformVersion::latest();
        let document_type = required_since_document_type();
        let document_type_ref = document_type.as_ref();

        let mut properties = BTreeMap::new();
        properties.insert("a".to_string(), Value::Text("alpha".to_string()));
        properties.insert("b".to_string(), Value::Text("beta".to_string()));

        let document = stamped_document(Some(2), properties, document_type_ref);

        let serialized = document
            .serialize_v3(document_type_ref)
            .expect("stamped document with the required-since field should serialize");

        let (version, _) = u64::decode_var(&serialized).expect("expected varint");
        assert_eq!(version, 3, "serialization version prefix should be 3");

        let deserialized = DocumentV0::from_bytes(&serialized, document_type_ref, platform_version)
            .expect("expected deserialization to succeed");

        assert_eq!(deserialized.contract_version, Some(2));
        assert_eq!(deserialized, document);
    }

    #[test]
    fn serialize_v3_grandfathered_document_may_omit_required_since_field() {
        let platform_version = PlatformVersion::latest();
        let document_type = required_since_document_type();
        let document_type_ref = document_type.as_ref();

        let mut properties = BTreeMap::new();
        properties.insert("a".to_string(), Value::Text("alpha".to_string()));

        // Stamped at version 1, before `b` became required at version 2
        let document = stamped_document(Some(1), properties, document_type_ref);

        let serialized = document
            .serialize_v3(document_type_ref)
            .expect("grandfathered document without the required-since field should serialize");

        let deserialized = DocumentV0::from_bytes(&serialized, document_type_ref, platform_version)
            .expect("expected deserialization to succeed");

        assert_eq!(deserialized.contract_version, Some(1));
        assert!(!deserialized.properties.contains_key("b"));
        assert_eq!(deserialized, document);
    }

    #[test]
    fn serialize_v3_unstamped_document_treats_required_since_fields_as_optional() {
        let platform_version = PlatformVersion::latest();
        let document_type = required_since_document_type();
        let document_type_ref = document_type.as_ref();

        let mut properties = BTreeMap::new();
        properties.insert("a".to_string(), Value::Text("alpha".to_string()));

        // No stamp: a pre-format-3 document being re-serialized (e.g. on
        // transfer). Every requiredSince annotation postdates its bytes.
        let document = stamped_document(None, properties, document_type_ref);

        let serialized = document
            .serialize_v3(document_type_ref)
            .expect("unstamped document without the required-since field should serialize");

        let deserialized = DocumentV0::from_bytes(&serialized, document_type_ref, platform_version)
            .expect("expected deserialization to succeed");

        assert_eq!(deserialized.contract_version, None);
        assert_eq!(deserialized, document);
    }

    #[test]
    fn serialize_v3_stamped_at_required_since_missing_field_errors() {
        let document_type = required_since_document_type();
        let document_type_ref = document_type.as_ref();

        let mut properties = BTreeMap::new();
        properties.insert("a".to_string(), Value::Text("alpha".to_string()));

        // Stamped at version 2, where `b` is required — but `b` is absent
        let document = stamped_document(Some(2), properties, document_type_ref);

        let result = document.serialize_v3(document_type_ref);
        assert!(
            matches!(
                result,
                Err(ProtocolError::DataContractError(
                    DataContractError::MissingRequiredKey(_)
                ))
            ),
            "a document stamped at requiredSince must contain the field, got {result:?}"
        );
    }

    #[test]
    fn format_2_bytes_stay_readable_under_a_required_since_schema() {
        use crate::data_contract::config::DataContractConfig;
        use crate::data_contract::document_type::DocumentType;
        use platform_value::platform_value;

        let platform_version = PlatformVersion::latest();

        // The schema as it was at contract version 1, before `b` (required
        // since version 2) and `c` (optional) were appended
        let old_schema = platform_value!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60_u32},
            },
            "required": ["a"],
            "additionalProperties": false,
        });
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config");
        let old_document_type = DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "test",
            old_schema,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut Vec::new(),
            platform_version,
        )
        .expect("failed to create old document type");

        let mut properties = BTreeMap::new();
        properties.insert("a".to_string(), Value::Text("alpha".to_string()));

        // A pre-stamp document serialized in format 2 under the old schema
        // (as every document written before protocol v14 was, at the
        // latest): its buffer ends before `b` and `c`, which must read back
        // as absent under the updated schema, not as errors
        let document = stamped_document(None, properties, old_document_type.as_ref());

        let serialized = document
            .serialize_v2(old_document_type.as_ref())
            .expect("format 2 serialization should succeed");

        let (version, _) = u64::decode_var(&serialized).expect("expected varint");
        assert_eq!(version, 2);

        let new_document_type = required_since_document_type();
        let deserialized =
            DocumentV0::from_bytes(&serialized, new_document_type.as_ref(), platform_version)
                .expect("format 2 bytes must stay readable under a requiredSince schema");

        assert_eq!(deserialized.contract_version, None);
        assert!(!deserialized.properties.contains_key("b"));
        assert!(!deserialized.properties.contains_key("c"));
        assert_eq!(deserialized, document);
    }

    #[test]
    fn stamp_survives_the_wire_for_documents_stamped_past_required_since() {
        let platform_version = PlatformVersion::latest();
        let document_type = required_since_document_type();
        let document_type_ref = document_type.as_ref();

        // The same content stamped before and at the requiredSince boundary
        // must produce different byte layouts (flagged vs raw), and each must
        // round-trip through the layout its own stamp selects
        let mut properties = BTreeMap::new();
        properties.insert("a".to_string(), Value::Text("alpha".to_string()));
        properties.insert("b".to_string(), Value::Text("beta".to_string()));

        let stamped_before = stamped_document(Some(1), properties.clone(), document_type_ref);
        let stamped_at = stamped_document(Some(2), properties, document_type_ref);

        let serialized_before = stamped_before
            .serialize_v3(document_type_ref)
            .expect("expected serialization");
        let serialized_at = stamped_at
            .serialize_v3(document_type_ref)
            .expect("expected serialization");

        // The flagged layout carries one extra presence byte for `b`, and the
        // two stamps differ in the prefix varint
        assert_ne!(serialized_before, serialized_at);

        let before_back =
            DocumentV0::from_bytes(&serialized_before, document_type_ref, platform_version)
                .expect("expected deserialization");
        let at_back = DocumentV0::from_bytes(&serialized_at, document_type_ref, platform_version)
            .expect("expected deserialization");

        assert_eq!(before_back, stamped_before);
        assert_eq!(at_back, stamped_at);
    }
}
