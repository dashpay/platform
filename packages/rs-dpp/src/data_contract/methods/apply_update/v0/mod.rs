use std::collections::BTreeMap;

use crate::block::block_info::BlockInfo;
use crate::consensus::basic::data_contract::InvalidDataContractVersionError;
use crate::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::consensus::state::state_error::StateError;
use crate::consensus::state::token::PreProgrammedDistributionTimestampInPastError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::associated_token::token_pre_programmed_distribution::accessors::v0::TokenPreProgrammedDistributionV0Methods;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::serialized_version::v1::DataContractInSerializationFormatV1;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::update_values::DataContractUpdateValues;
use crate::data_contract::DataContract;
use crate::validation::operations::ProtocolValidationOperation;
use crate::validation::ConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;

impl DataContract {
    /// Applies update values to this contract to produce a new contract.
    ///
    /// This method takes the current contract and applies the changes specified in the
    /// update values to produce a new updated contract. It merges document schemas,
    /// groups, tokens, and keywords from the update into the existing contract.
    ///
    /// # Arguments
    /// - `update_values`: The update values containing the changes to apply.
    /// - `block_info`: Block information containing timestamp, epoch, and height for the update.
    /// - `full_validation`: Whether to perform full validation on the resulting contract.
    /// - `validation_operations`: A vector to collect validation operations.
    /// - `platform_version`: The current platform version.
    ///
    /// # Returns
    /// - `Ok(ConsensusValidationResult<DataContract>)`: The validation result containing the new contract if successful.
    /// - `Err(ProtocolError)`: If a protocol error occurs during transformation.
    pub(super) fn apply_update_v0(
        &self,
        update_values: DataContractUpdateValues<'_>,
        block_info: &BlockInfo,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<DataContract>, ProtocolError> {
        // Check version is bumped by exactly 1
        let new_version = update_values.revision;
        let old_version = self.version();
        if new_version < old_version || new_version - old_version != 1 {
            return Ok(ConsensusValidationResult::new_with_error(
                InvalidDataContractVersionError::new(old_version + 1, new_version).into(),
            ));
        }

        // Validate that updated document schemas correspond to existing document types
        for document_type_name in update_values.updated_document_schemas.keys() {
            if self
                .document_type_optional_for_name(document_type_name)
                .is_none()
            {
                return Ok(ConsensusValidationResult::new_with_error(
                    DocumentTypeUpdateError::new(
                        self.id(),
                        document_type_name,
                        "document type does not exist and cannot be updated",
                    )
                    .into(),
                ));
            }
        }

        // Validate schema_defs update (existence and compatibility)
        let schema_defs_validation_result = DataContract::validate_schema_defs_update(
            self.id(),
            self.schema_defs(),
            update_values.updated_schema_defs,
            update_values.new_schema_defs,
            platform_version,
        )?;
        if !schema_defs_validation_result.is_valid() {
            return Ok(ConsensusValidationResult::new_with_errors(
                schema_defs_validation_result.errors,
            ));
        }

        // Start with the old contract's document schemas
        let mut document_schemas: BTreeMap<String, Value> = self
            .document_schemas()
            .into_iter()
            .map(|(name, schema)| (name, schema.clone()))
            .collect();

        // Apply updated document schemas
        for (name, schema) in update_values.updated_document_schemas {
            document_schemas.insert(name.clone(), schema.clone());
        }

        // Add new document schemas
        for (name, schema) in update_values.new_document_schemas {
            document_schemas.insert(name.clone(), schema.clone());
        }

        // Merge groups
        let mut groups = self.groups().clone();
        for (pos, group) in update_values.new_groups {
            groups.insert(*pos, group.clone());
        }

        // Validate combined groups if new groups were added
        if !update_values.new_groups.is_empty() {
            let validation_result =
                DataContract::validate_groups(&groups, false, platform_version)?;
            if !validation_result.is_valid() {
                return Ok(ConsensusValidationResult::new_with_errors(
                    validation_result.errors,
                ));
            }
        }

        // Validate that new tokens are contiguous with existing tokens
        if !update_values.new_tokens.is_empty() {
            let existing_tokens = self.tokens();
            let highest_existing_position = existing_tokens.keys().max().copied();
            let first_new_position = update_values.new_tokens.keys().min().copied();

            if let Some(first_new) = first_new_position {
                let expected_first = highest_existing_position.map(|h| h + 1).unwrap_or(0);
                if first_new != expected_first {
                    return Ok(ConsensusValidationResult::new_with_error(
                        crate::consensus::basic::data_contract::NonContiguousContractTokenPositionsError::new(
                            expected_first, // missing_position
                            first_new,      // followed_position
                        )
                        .into(),
                    ));
                }
            }
        }

        // Merge tokens
        let mut tokens = self.tokens().clone();
        for (pos, token) in update_values.new_tokens {
            tokens.insert(*pos, token.clone());
        }

        // Validate that new tokens reference groups that exist in the combined groups
        for token_configuration in update_values.new_tokens.values() {
            let validation_result = token_configuration
                .validate_token_config_groups_exist(&groups, platform_version)?;
            if !validation_result.is_valid() {
                return Ok(ConsensusValidationResult::new_with_errors(
                    validation_result.errors,
                ));
            }
        }

        // Update keywords
        let mut keywords: Vec<String> = self
            .keywords()
            .iter()
            .filter(|k| !update_values.remove_keywords.contains(k))
            .cloned()
            .collect();
        keywords.extend(update_values.add_keywords.iter().cloned());

        // Validate combined keywords (structure already validated in basic validation)
        let validation_result =
            DataContract::validate_keywords(self.id(), &keywords, false, platform_version)?;
        if !validation_result.is_valid() {
            return Ok(ConsensusValidationResult::new_with_errors(
                validation_result.errors,
            ));
        }

        // Update description
        let description = match update_values.update_description {
            None => self.description().cloned(),
            Some(new_desc) => new_desc.clone(),
        };

        // Update schema_defs
        let schema_defs = if update_values.updated_schema_defs.is_empty()
            && update_values.new_schema_defs.is_empty()
        {
            // No changes to schema_defs
            self.schema_defs().cloned()
        } else {
            // Start with old schema_defs or empty map
            let mut defs = self.schema_defs().cloned().unwrap_or_default();
            // Apply updates
            for (name, def) in update_values.updated_schema_defs {
                defs.insert(name.clone(), def.clone());
            }
            // Add new defs
            for (name, def) in update_values.new_schema_defs {
                defs.insert(name.clone(), def.clone());
            }
            Some(defs)
        };

        // Validate pre-programmed distribution timestamps for new tokens
        for (token_contract_position, token_configuration) in update_values.new_tokens {
            if let Some(distribution) = token_configuration
                .distribution_rules()
                .pre_programmed_distribution()
            {
                if let Some((timestamp, _)) = distribution.distributions().iter().next() {
                    if timestamp < &block_info.time_ms {
                        return Ok(ConsensusValidationResult::new_with_error(
                            StateError::PreProgrammedDistributionTimestampInPastError(
                                PreProgrammedDistributionTimestampInPastError::new(
                                    self.id(),
                                    *token_contract_position,
                                    *timestamp,
                                    block_info.time_ms,
                                ),
                            )
                            .into(),
                        ));
                    }
                }
            }
        }

        // Create the new contract using serialization format
        let serialization_format =
            DataContractInSerializationFormat::V1(DataContractInSerializationFormatV1 {
                id: self.id(),
                config: *self.config(),
                version: update_values.revision,
                owner_id: self.owner_id(),
                schema_defs,
                document_schemas,
                created_at: self.created_at(),
                updated_at: None, // Will be set below
                created_at_block_height: self.created_at_block_height(),
                updated_at_block_height: None, // Will be set below
                created_at_epoch: self.created_at_epoch(),
                updated_at_epoch: None, // Will be set below
                groups,
                tokens,
                keywords,
                description,
            });

        let mut new_contract = DataContract::try_from_platform_versioned(
            serialization_format,
            full_validation,
            validation_operations,
            platform_version,
        )?;

        // Validate document type update rules for updated schemas
        for document_type_name in update_values.updated_document_schemas.keys() {
            let old_document_type = self
                .document_type_optional_for_name(document_type_name)
                .expect("document type existence was already validated");

            let Some(new_document_type) =
                new_contract.document_type_optional_for_name(document_type_name)
            else {
                return Ok(ConsensusValidationResult::new_with_error(
                    DocumentTypeUpdateError::new(
                        self.id(),
                        document_type_name,
                        "document type failed to be created from updated schema",
                    )
                    .into(),
                ));
            };

            let validate_update_result =
                old_document_type.validate_update(new_document_type, platform_version)?;

            if !validate_update_result.is_valid() {
                return Ok(ConsensusValidationResult::new_with_errors(
                    validate_update_result.errors,
                ));
            }
        }

        new_contract.set_updated_at(Some(block_info.time_ms));
        new_contract.set_updated_at_epoch(Some(block_info.epoch.index));
        new_contract.set_updated_at_block_height(Some(block_info.height));

        Ok(ConsensusValidationResult::new_with_data(new_contract))
    }
}
