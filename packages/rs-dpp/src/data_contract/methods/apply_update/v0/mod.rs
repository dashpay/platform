use std::collections::BTreeMap;

use crate::block::block_info::BlockInfo;
use crate::consensus::basic::data_contract::{
    DataContractUpdateEntryKind, NonContiguousContractTokenPositionsError,
};
use crate::consensus::state::data_contract::data_contract_update_entry_already_exists_error::DataContractUpdateEntryAlreadyExistsError;
use crate::consensus::state::data_contract::data_contract_update_entry_not_found_error::DataContractUpdateEntryNotFoundError;
use crate::consensus::state::data_contract::data_contract_update_permission_error::DataContractUpdatePermissionError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::serialized_version::v1::DataContractInSerializationFormatV1;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::update_values::{DataContractUpdateValues, DescriptionUpdate};
use crate::data_contract::{DataContract, DocumentName, TokenContractPosition};
use crate::validation::operations::ProtocolValidationOperation;
use crate::validation::ConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;

impl DataContract {
    #[inline(always)]
    pub(super) fn apply_update_v0(
        &self,
        update_values: DataContractUpdateValues<'_>,
        block_info: &BlockInfo,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<DataContract>, ProtocolError> {
        let contract_id = self.id();

        // The delta names its owner explicitly, so the ownership check that
        // a full-contract update gets from comparing embedded and stored
        // owners happens here, before anything is merged.
        if update_values.owner_id != self.owner_id() {
            return Ok(ConsensusValidationResult::new_with_error(
                DataContractUpdatePermissionError::new(contract_id, update_values.owner_id).into(),
            ));
        }

        let not_found = |kind: DataContractUpdateEntryKind, name: String| {
            Ok(ConsensusValidationResult::new_with_error(
                DataContractUpdateEntryNotFoundError::new(contract_id, kind, name).into(),
            ))
        };
        let already_exists = |kind: DataContractUpdateEntryKind, name: String| {
            Ok(ConsensusValidationResult::new_with_error(
                DataContractUpdateEntryAlreadyExistsError::new(contract_id, kind, name).into(),
            ))
        };

        let mut document_schemas: BTreeMap<DocumentName, Value> = self
            .document_schemas()
            .into_iter()
            .map(|(name, schema)| (name, schema.clone()))
            .collect();
        for (name, schema) in update_values.updated_document_schemas {
            match document_schemas.get_mut(name) {
                Some(existing) => *existing = schema.clone(),
                None => return not_found(DataContractUpdateEntryKind::DocumentType, name.clone()),
            }
        }
        for (name, schema) in update_values.new_document_schemas {
            if document_schemas
                .insert(name.clone(), schema.clone())
                .is_some()
            {
                return already_exists(DataContractUpdateEntryKind::DocumentType, name.clone());
            }
        }

        let mut schema_defs = self.schema_defs().cloned();
        for (name, definition) in update_values.updated_schema_defs {
            match schema_defs
                .as_mut()
                .and_then(|definitions| definitions.get_mut(name))
            {
                Some(existing) => *existing = definition.clone(),
                None => return not_found(DataContractUpdateEntryKind::SchemaDef, name.clone()),
            }
        }
        for (name, definition) in update_values.new_schema_defs {
            if schema_defs
                .get_or_insert_with(BTreeMap::new)
                .insert(name.clone(), definition.clone())
                .is_some()
            {
                return already_exists(DataContractUpdateEntryKind::SchemaDef, name.clone());
            }
        }

        let mut groups = self.groups().clone();
        for (position, group) in update_values.new_groups {
            if groups.insert(*position, group.clone()).is_some() {
                return already_exists(DataContractUpdateEntryKind::Group, position.to_string());
            }
        }
        if !update_values.new_groups.is_empty() {
            // Positions must stay contiguous across the stored and the new
            // groups, which only the merged map can show.
            let validation_result = DataContract::validate_groups(&groups, platform_version)?;
            if !validation_result.is_valid() {
                return Ok(ConsensusValidationResult::new_with_errors(
                    validation_result.errors,
                ));
            }
        }

        let mut tokens = self.tokens().clone();
        for (position, token) in update_values.new_tokens {
            if tokens.insert(*position, token.clone()).is_some() {
                return already_exists(DataContractUpdateEntryKind::Token, position.to_string());
            }
        }
        for (expected_position, position) in tokens.keys().enumerate() {
            let expected_position = expected_position as TokenContractPosition;
            if *position != expected_position {
                return Ok(ConsensusValidationResult::new_with_error(
                    NonContiguousContractTokenPositionsError::new(expected_position, *position)
                        .into(),
                ));
            }
        }
        for token in update_values.new_tokens.values() {
            // A new token may reference a group that arrives in the same
            // update, so the check runs against the merged groups.
            let validation_result =
                token.validate_token_config_groups_exist(&groups, platform_version)?;
            if !validation_result.is_valid() {
                return Ok(ConsensusValidationResult::new_with_errors(
                    validation_result.errors,
                ));
            }
        }

        let mut keywords = self.keywords().clone();
        for keyword in update_values.remove_keywords {
            match keywords.iter().position(|existing| existing == keyword) {
                Some(index) => {
                    keywords.remove(index);
                }
                None => return not_found(DataContractUpdateEntryKind::Keyword, keyword.clone()),
            }
        }
        for keyword in update_values.add_keywords {
            if keywords.contains(keyword) {
                return already_exists(DataContractUpdateEntryKind::Keyword, keyword.clone());
            }
            keywords.push(keyword.clone());
        }

        let description = match update_values.description {
            DescriptionUpdate::Keep => self.description().cloned(),
            DescriptionUpdate::Clear => None,
            DescriptionUpdate::Set(description) => Some(description.clone()),
        };

        let config = update_values
            .config
            .copied()
            .unwrap_or_else(|| *self.config());

        let serialization_format =
            DataContractInSerializationFormat::V1(DataContractInSerializationFormatV1 {
                id: contract_id,
                config,
                version: update_values.version,
                owner_id: self.owner_id(),
                schema_defs,
                document_schemas,
                created_at: self.created_at(),
                updated_at: Some(block_info.time_ms),
                created_at_block_height: self.created_at_block_height(),
                updated_at_block_height: Some(block_info.height),
                created_at_epoch: self.created_at_epoch(),
                updated_at_epoch: Some(block_info.epoch.index),
                groups,
                tokens,
                keywords,
                description,
            });

        let updated_contract = DataContract::try_from_platform_versioned(
            serialization_format,
            full_validation,
            validation_operations,
            platform_version,
        )?;

        Ok(ConsensusValidationResult::new_with_data(updated_contract))
    }
}
