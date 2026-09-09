//! Document serialization format 3: the default from protocol version 14.
//! Format 2 plus the contract-version stamp — a varint right after the
//! format prefix recording the data contract version the bytes conform to
//! (0 = unstamped: a pre-format-3 document re-serialized in the new
//! envelope). Introduced so contract updates can add required properties
//! via `requiredSince`: the stamp selects each annotated property's byte
//! layout (raw once the stamp reaches the annotation, presence-flagged
//! before it), letting the latest contract alone decode every stored
//! document.

use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::errors::DataContractError;

use crate::document::property_names::{
    CREATED_AT, CREATED_AT_BLOCK_HEIGHT, CREATED_AT_CORE_BLOCK_HEIGHT, PRICE, TRANSFERRED_AT,
    TRANSFERRED_AT_BLOCK_HEIGHT, TRANSFERRED_AT_CORE_BLOCK_HEIGHT, UPDATED_AT,
    UPDATED_AT_BLOCK_HEIGHT, UPDATED_AT_CORE_BLOCK_HEIGHT,
};

use crate::prelude::Revision;

use crate::ProtocolError;

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
use crate::document::v0::DocumentV0;
use crate::version::PlatformVersion;
use byteorder::{BigEndian, ReadBytesExt};
use integer_encoding::{VarInt, VarIntReader};

use platform_value::{Identifier, Value};

use std::collections::BTreeMap;

use crate::consensus::basic::decode::DecodingError;
use crate::nft::TradeMode;
use std::io::{BufReader, Read};

impl DocumentV0 {
    /// Serializes the document.
    ///
    /// Serialize v3 is v2 plus the contract version stamp: a varint right
    /// after the format prefix recording the data contract version the bytes
    /// conform to (0 = unstamped, for pre-format-3 documents that are
    /// re-serialized). A property whose `requiredSince` exceeds the stamp is
    /// encoded with a presence flag exactly like an optional property, so
    /// documents written before the property became required stay valid.
    pub(super) fn serialize_v3(
        &self,
        document_type: DocumentTypeRef,
    ) -> Result<Vec<u8>, ProtocolError> {
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

    /// Reads a serialized document and creates a Document from it.
    /// Version 3 is version 2 plus the contract version stamp, which selects
    /// each `requiredSince` property's byte layout: raw when the stamp has
    /// reached the property's `requiredSince`, presence-flagged otherwise.
    pub(super) fn from_bytes_v3(
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

        // Every property the document was serialized with must have been
        // consumed. Trailing bytes mean the document was written under a
        // newer contract version than the document type used to read it — a
        // stale reader would otherwise silently drop the fields it does not
        // know about. The stamp makes this detectable: callers should
        // refetch the contract and retry.
        let mut trailing_probe = [0u8; 1];
        let trailing = buf.read(&mut trailing_probe).map_err(|_| {
            DataContractError::CorruptedSerialization(
                "error probing for trailing bytes in serialized document".to_string(),
            )
        })?;
        if trailing > 0 {
            return Err(DataContractError::CorruptedSerialization(format!(
                "serialized document has trailing bytes: it was serialized under contract version {} with properties this document type does not know; refetch the contract",
                stamp
            )));
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
