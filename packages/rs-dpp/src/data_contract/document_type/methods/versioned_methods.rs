use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::v1::DocumentTypeV1;
use crate::data_contract::document_type::{
    DocumentPropertyType, DocumentType, DocumentTypeRef, Index, DEFAULT_HASH_SIZE, MAX_INDEX_SIZE,
};
use crate::data_contract::errors::DataContractError;
use crate::document::property_names::{
    CREATED_AT, CREATED_AT_BLOCK_HEIGHT, CREATED_AT_CORE_BLOCK_HEIGHT, TRANSFERRED_AT,
    TRANSFERRED_AT_BLOCK_HEIGHT, TRANSFERRED_AT_CORE_BLOCK_HEIGHT, UPDATED_AT,
    UPDATED_AT_BLOCK_HEIGHT, UPDATED_AT_CORE_BLOCK_HEIGHT,
};
use crate::document::{Document, DocumentV0, DocumentV0Getters, INITIAL_REVISION};
use crate::fee::Credits;
use crate::identity::TimestampMillis;
use crate::prelude::{BlockHeight, CoreBlockHeight};
use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use crate::voting::vote_polls::VotePoll;
use crate::ProtocolError;
use chrono::Utc;
use itertools::Itertools;
use platform_value::btreemap_extensions::{
    BTreeValueMapHelper, BTreeValueMapPathHelper, BTreeValueMapReplacementPathHelper,
};
use platform_value::{Identifier, ReplacementType, Value};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

pub trait DocumentTypeV0MethodsVersioned: DocumentTypeV0Getters + DocumentTypeBasicMethods {
    fn create_document_from_data_v0(
        &self,
        data: Value,
        owner_id: Identifier,
        block_height: BlockHeight,
        core_block_height: CoreBlockHeight,
        document_entropy: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        let document_id = Document::generate_document_id_v0(
            &self.data_contract_id(),
            &owner_id,
            self.name(),
            &document_entropy,
        );

        let revision = if self.requires_revision() {
            Some(INITIAL_REVISION)
        } else {
            None
        };

        // Set timestamps if they are required and not exist

        let mut created_at: Option<TimestampMillis> = data
            .get_optional_integer(CREATED_AT)
            .map_err(ProtocolError::ValueError)?;

        let mut updated_at: Option<TimestampMillis> = data
            .get_optional_integer(UPDATED_AT)
            .map_err(ProtocolError::ValueError)?;

        let mut transferred_at: Option<TimestampMillis> = data
            .get_optional_integer(TRANSFERRED_AT)
            .map_err(ProtocolError::ValueError)?;

        let mut created_at_block_height: Option<BlockHeight> = data
            .get_optional_integer(CREATED_AT_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let mut updated_at_block_height: Option<BlockHeight> = data
            .get_optional_integer(UPDATED_AT_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let mut transferred_at_block_height: Option<BlockHeight> = data
            .get_optional_integer(TRANSFERRED_AT_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let mut created_at_core_block_height: Option<CoreBlockHeight> = data
            .get_optional_integer(CREATED_AT_CORE_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let mut updated_at_core_block_height: Option<CoreBlockHeight> = data
            .get_optional_integer(UPDATED_AT_CORE_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let mut transferred_at_core_block_height: Option<CoreBlockHeight> = data
            .get_optional_integer(TRANSFERRED_AT_CORE_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let is_created_at_required = self.required_fields().contains(CREATED_AT);
        let is_updated_at_required = self.required_fields().contains(UPDATED_AT);
        let is_transferred_at_required = self.required_fields().contains(TRANSFERRED_AT);

        let is_created_at_block_height_required =
            self.required_fields().contains(CREATED_AT_BLOCK_HEIGHT);
        let is_updated_at_block_height_required =
            self.required_fields().contains(UPDATED_AT_BLOCK_HEIGHT);
        let is_transferred_at_block_height_required =
            self.required_fields().contains(TRANSFERRED_AT_BLOCK_HEIGHT);

        let is_created_at_core_block_height_required = self
            .required_fields()
            .contains(CREATED_AT_CORE_BLOCK_HEIGHT);
        let is_updated_at_core_block_height_required = self
            .required_fields()
            .contains(UPDATED_AT_CORE_BLOCK_HEIGHT);
        let is_transferred_at_core_block_height_required = self
            .required_fields()
            .contains(TRANSFERRED_AT_CORE_BLOCK_HEIGHT);

        if (is_created_at_required && created_at.is_none())
            || (is_updated_at_required && updated_at.is_none()
                || (is_transferred_at_required && transferred_at.is_none()))
        {
            //we want only one call to get current time
            let now = Utc::now().timestamp_millis() as TimestampMillis;

            if is_created_at_required {
                created_at = created_at.or(Some(now));
            };

            if is_updated_at_required {
                updated_at = updated_at.or(Some(now));
            };

            if is_transferred_at_required {
                transferred_at = transferred_at.or(Some(now));
            };
        };

        if is_created_at_block_height_required {
            created_at_block_height = created_at_block_height.or(Some(block_height));
        };

        if is_updated_at_block_height_required {
            updated_at_block_height = updated_at_block_height.or(Some(block_height));
        };

        if is_transferred_at_block_height_required {
            transferred_at_block_height = transferred_at_block_height.or(Some(block_height));
        };

        if is_created_at_core_block_height_required {
            created_at_core_block_height = created_at_core_block_height.or(Some(core_block_height));
        };

        if is_updated_at_core_block_height_required {
            updated_at_core_block_height = updated_at_core_block_height.or(Some(core_block_height));
        };

        if is_transferred_at_core_block_height_required {
            transferred_at_core_block_height =
                transferred_at_core_block_height.or(Some(core_block_height));
        };

        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => {
                let mut document = DocumentV0 {
                    id: document_id,
                    owner_id,
                    properties: data
                        .into_btree_string_map()
                        .map_err(ProtocolError::ValueError)?,
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
                };

                document
                    .properties
                    .replace_at_paths(self.identifier_paths(), ReplacementType::Identifier)?;

                Ok(document.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "convert_value_to_document_v0 inner match to document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Creates a document at the current time based on document type information
    /// Properties set here must be pre validated
    fn create_document_with_prevalidated_properties_v0(
        &self,
        id: Identifier,
        owner_id: Identifier,
        block_height: BlockHeight,
        core_block_height: CoreBlockHeight,
        properties: BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        // Set timestamps if they are required and not exist
        let mut created_at: Option<TimestampMillis> = properties
            .get_optional_integer(CREATED_AT)
            .map_err(ProtocolError::ValueError)?;

        let mut updated_at: Option<TimestampMillis> = properties
            .get_optional_integer(UPDATED_AT)
            .map_err(ProtocolError::ValueError)?;

        let mut transferred_at: Option<TimestampMillis> = properties
            .get_optional_integer(TRANSFERRED_AT)
            .map_err(ProtocolError::ValueError)?;

        let mut created_at_block_height: Option<BlockHeight> = properties
            .get_optional_integer(CREATED_AT_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let mut updated_at_block_height: Option<BlockHeight> = properties
            .get_optional_integer(UPDATED_AT_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let mut transferred_at_block_height: Option<BlockHeight> = properties
            .get_optional_integer(TRANSFERRED_AT_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let mut created_at_core_block_height: Option<CoreBlockHeight> = properties
            .get_optional_integer(CREATED_AT_CORE_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let mut updated_at_core_block_height: Option<CoreBlockHeight> = properties
            .get_optional_integer(UPDATED_AT_CORE_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let mut transferred_at_core_block_height: Option<CoreBlockHeight> = properties
            .get_optional_integer(TRANSFERRED_AT_CORE_BLOCK_HEIGHT)
            .map_err(ProtocolError::ValueError)?;

        let is_created_at_required = self.required_fields().contains(CREATED_AT);
        let is_updated_at_required = self.required_fields().contains(UPDATED_AT);
        let is_transferred_at_required = self.required_fields().contains(TRANSFERRED_AT);

        let is_created_at_block_height_required =
            self.required_fields().contains(CREATED_AT_BLOCK_HEIGHT);
        let is_updated_at_block_height_required =
            self.required_fields().contains(UPDATED_AT_BLOCK_HEIGHT);
        let is_transferred_at_block_height_required =
            self.required_fields().contains(TRANSFERRED_AT_BLOCK_HEIGHT);

        let is_created_at_core_block_height_required = self
            .required_fields()
            .contains(CREATED_AT_CORE_BLOCK_HEIGHT);
        let is_updated_at_core_block_height_required = self
            .required_fields()
            .contains(UPDATED_AT_CORE_BLOCK_HEIGHT);
        let is_transferred_at_core_block_height_required = self
            .required_fields()
            .contains(TRANSFERRED_AT_CORE_BLOCK_HEIGHT);

        if (is_created_at_required && created_at.is_none())
            || (is_updated_at_required && updated_at.is_none()
                || (is_transferred_at_required && transferred_at.is_none()))
        {
            //we want only one call to get current time
            let now = Utc::now().timestamp_millis() as TimestampMillis;

            if is_created_at_required {
                created_at = created_at.or(Some(now));
            };

            if is_updated_at_required {
                updated_at = updated_at.or(Some(now));
            };

            if is_transferred_at_required {
                transferred_at = transferred_at.or(Some(now));
            };
        };

        if is_created_at_block_height_required {
            created_at_block_height = created_at_block_height.or(Some(block_height));
        };

        if is_updated_at_block_height_required {
            updated_at_block_height = updated_at_block_height.or(Some(block_height));
        };

        if is_transferred_at_block_height_required {
            transferred_at_block_height = transferred_at_block_height.or(Some(block_height));
        };

        if is_created_at_core_block_height_required {
            created_at_core_block_height = created_at_core_block_height.or(Some(core_block_height));
        };

        if is_updated_at_core_block_height_required {
            updated_at_core_block_height = updated_at_core_block_height.or(Some(core_block_height));
        };

        if is_transferred_at_core_block_height_required {
            transferred_at_core_block_height =
                transferred_at_core_block_height.or(Some(core_block_height));
        };

        let revision = if self.requires_revision() {
            Some(INITIAL_REVISION)
        } else {
            None
        };

        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(DocumentV0 {
                id,
                owner_id,
                properties,
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
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "create_document_with_prevalidated_properties_v0 (for document version)"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Figures out the prefunded voting balance (v0) for a document in a document type
    fn contested_vote_poll_for_document_v0(&self, document: &Document) -> Option<VotePoll> {
        self.contested_vote_poll_for_document_properties_v0(document.properties())
    }

    fn contested_vote_poll_for_document_properties_v0(
        &self,
        document_properties: &BTreeMap<String, Value>,
    ) -> Option<VotePoll> {
        self.indexes()
            .values()
            .find(|index| {
                if let Some(contested_index_info) = &index.contested_index {
                    contested_index_info
                        .field_matches
                        .iter()
                        .all(|(field, field_match)| {
                            if let Some(value) = document_properties
                                .get_optional_at_path(field)
                                .ok()
                                .flatten()
                            {
                                field_match.matches(value)
                            } else {
                                false
                            }
                        })
                } else {
                    false
                }
            })
            .map(|index| {
                let index_values = index.extract_values(document_properties);
                VotePoll::ContestedDocumentResourceVotePoll(ContestedDocumentResourceVotePoll {
                    contract_id: self.data_contract_id(),
                    document_type_name: self.name().clone(),
                    index_name: index.name.clone(),
                    index_values,
                })
            })
    }

    fn index_for_types_v0(
        &self,
        index_names: &[&str],
        in_field_name: Option<&str>,
        order_by: &[&str],
    ) -> Option<(&Index, u16)> {
        let mut best_index: Option<(&Index, u16)> = None;
        let mut best_difference = u16::MAX;
        for (_, index) in self.indexes().iter() {
            let difference_option = index.matches(index_names, in_field_name, order_by);
            if let Some(difference) = difference_option {
                if difference == 0 {
                    return Some((index, 0));
                } else if difference < best_difference {
                    best_difference = difference;
                    best_index = Some((index, best_difference));
                }
            }
        }
        best_index
    }

    /// The estimated size uses the middle ceil size of all attributes
    fn estimated_size_v0(&self, platform_version: &PlatformVersion) -> Result<u16, ProtocolError> {
        let mut total_size = 0u16;

        for (_, document_property) in self.flattened_properties().iter() {
            // This call now returns a Result<Option<u16>, ProtocolError>.
            let maybe_size = document_property
                .property_type
                .middle_byte_size_ceil(platform_version)?;

            if let Some(size) = maybe_size {
                total_size = match total_size.checked_add(size) {
                    Some(new_total) => new_total,
                    None => {
                        return Ok(u16::MAX);
                    }
                };
            }
        }

        Ok(total_size)
    }

    fn max_size_v0(&self, platform_version: &PlatformVersion) -> Result<u16, ProtocolError> {
        let mut total_size = 0u16;

        for (_, document_property) in self.flattened_properties().iter() {
            let maybe_size = document_property
                .property_type
                .max_byte_size(platform_version)?;

            if let Some(size) = maybe_size {
                total_size = match total_size.checked_add(size) {
                    Some(new_total) => new_total,
                    None => {
                        return Ok(u16::MAX);
                    }
                };
            }
        }

        Ok(total_size)
    }

    /// Figures out the prefunded voting balance (v0) for a document in a document type
    fn prefunded_voting_balance_for_document_v0(
        &self,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Option<(String, Credits)> {
        self.indexes()
            .values()
            .find(|index| {
                if let Some(contested_index_info) = &index.contested_index {
                    contested_index_info
                        .field_matches
                        .iter()
                        .all(|(field, field_match)| {
                            if let Some(value) = document.get(field) {
                                field_match.matches(value)
                            } else {
                                false
                            }
                        })
                } else {
                    false
                }
            })
            .map(|index| {
                (
                    index.name.clone(),
                    platform_version
                        .fee_version
                        .vote_resolution_fund_fees
                        .contested_document_vote_resolution_fund_required_amount,
                )
            })
    }

    fn serialize_value_for_key_v0(
        &self,
        key: &str,
        value: &Value,
    ) -> Result<Vec<u8>, ProtocolError> {
        match key {
            "$ownerId" | "$id" => {
                let bytes = value
                    .to_identifier_bytes()
                    .map_err(ProtocolError::ValueError)?;
                if bytes.len() != DEFAULT_HASH_SIZE {
                    Err(ProtocolError::DataContractError(
                        DataContractError::FieldRequirementUnmet(
                            "expected system value to be 32 bytes long".to_string(),
                        ),
                    ))
                } else {
                    Ok(bytes)
                }
            }
            "$createdAt" | "$updatedAt" | "$transferredAt" => {
                Ok(DocumentPropertyType::encode_date_timestamp(
                    value.to_integer().map_err(ProtocolError::ValueError)?,
                ))
            }
            "$createdAtBlockHeight" | "$updatedAtBlockHeight" | "$transferredAtBlockHeight" => {
                Ok(DocumentPropertyType::encode_u64(
                    value.to_integer().map_err(ProtocolError::ValueError)?,
                ))
            }
            "$createdAtCoreBlockHeight"
            | "$updatedAtCoreBlockHeight"
            | "$transferredAtCoreBlockHeight" => Ok(DocumentPropertyType::encode_u32(
                value.to_integer().map_err(ProtocolError::ValueError)?,
            )),
            _ => {
                let property = self.flattened_properties().get(key).ok_or_else(|| {
                    DataContractError::DocumentTypeFieldNotFound(format!("expected contract to have field: {key}, contract fields are {} on document type {}", self.flattened_properties().keys().join(" | "), self.name()))
                })?;
                let bytes = property.property_type.encode_value_for_tree_keys(value)?;
                if bytes.len() > MAX_INDEX_SIZE {
                    Err(ProtocolError::DataContractError(
                        DataContractError::FieldRequirementUnmet(
                            "value must be less than 256 bytes long".to_string(),
                        ),
                    ))
                } else {
                    Ok(bytes)
                }
            }
        }
    }

    fn deserialize_value_for_key_v0(
        &self,
        key: &str,
        value: &[u8],
    ) -> Result<Value, ProtocolError> {
        match key {
            "$ownerId" | "$id" => {
                let bytes = Identifier::from_bytes(value)?;
                Ok(Value::Identifier(bytes.to_buffer()))
            }
            "$createdAt" | "$updatedAt" | "$transferredAt" => Ok(Value::U64(
                DocumentPropertyType::decode_date_timestamp(value).ok_or(
                    ProtocolError::DataContractError(DataContractError::FieldRequirementUnmet(
                        "value must be 8 bytes long".to_string(),
                    )),
                )?,
            )),
            "$createdAtBlockHeight" | "$updatedAtBlockHeight" | "$transferredAtBlockHeight" => {
                Ok(Value::U64(DocumentPropertyType::decode_u64(value).ok_or(
                    ProtocolError::DataContractError(DataContractError::FieldRequirementUnmet(
                        "value must be 8 bytes long".to_string(),
                    )),
                )?))
            }
            "$createdAtCoreBlockHeight"
            | "$updatedAtCoreBlockHeight"
            | "$transferredAtCoreBlockHeight" => {
                Ok(Value::U32(DocumentPropertyType::decode_u32(value).ok_or(
                    ProtocolError::DataContractError(DataContractError::FieldRequirementUnmet(
                        "value must be 4 bytes long".to_string(),
                    )),
                )?))
            }
            _ => {
                let property = self.flattened_properties().get(key).ok_or_else(|| {
                    DataContractError::DocumentTypeFieldNotFound(format!("expected contract to have field: {key}, contract fields are {} on document type {}", self.flattened_properties().keys().join(" | "), self.name()))
                })?;
                property.property_type.decode_value_for_tree_keys(value)
            }
        }
    }
}

impl DocumentTypeV0MethodsVersioned for DocumentTypeV0 {}

impl DocumentTypeV0MethodsVersioned for DocumentTypeV1 {}
impl DocumentTypeV0MethodsVersioned for DocumentType {}

impl DocumentTypeV0MethodsVersioned for DocumentTypeRef<'_> {}
