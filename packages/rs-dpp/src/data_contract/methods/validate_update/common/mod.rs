//! Helpers shared by every generation of `DataContract::validate_update`
//! (`v0`, `v1`, …). Only the parts of the update-validation flow that differ
//! between generations live in the per-version modules; the checks below are
//! generation independent. Each helper returns a
//! `SimpleConsensusValidationResult`; generations run them in sequence and
//! short-circuit on the first invalid result, so extraction preserves the
//! original early-return semantics exactly.

use std::collections::HashSet;

use crate::block::block_info::BlockInfo;
use crate::consensus::state::state_error::StateError;
use crate::consensus::state::token::PreProgrammedDistributionTimestampInPastError;
use crate::data_contract::accessors::v0::DataContractV0Getters;

use crate::consensus::basic::data_contract::{
    DuplicateKeywordsError, IncompatibleDataContractSchemaError, InvalidDataContractVersionError,
    InvalidDescriptionLengthError, InvalidKeywordCharacterError, InvalidKeywordLengthError,
    TooManyKeywordsError,
};
use crate::consensus::state::data_contract::data_contract_update_action_not_allowed_error::DataContractUpdateActionNotAllowedError;
use crate::consensus::state::data_contract::data_contract_update_permission_error::DataContractUpdatePermissionError;
use crate::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::associated_token::token_pre_programmed_distribution::accessors::v0::TokenPreProgrammedDistributionV0Methods;
use crate::data_contract::document_type::schema::validate_schema_compatibility;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use serde_json::json;

impl DataContract {
    /// The update must come from the contract owner, and the new contract's
    /// version must be exactly the old version plus one.
    ///
    /// Failure (version != previous version + 1): Keep ST and transform it to
    /// a nonce bump action. How: A user pushed an update that was not the
    /// next version.
    pub(super) fn validate_update_ownership_and_version(
        &self,
        new_data_contract: &DataContract,
    ) -> SimpleConsensusValidationResult {
        // Check if the contract is owned by the same identity
        if self.owner_id() != new_data_contract.owner_id() {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractUpdatePermissionError::new(self.id(), new_data_contract.owner_id())
                    .into(),
            );
        }

        // Check version is bumped
        let new_version = new_data_contract.version();
        let old_version = self.version();
        if new_version < old_version || new_version - old_version != 1 {
            return SimpleConsensusValidationResult::new_with_error(
                InvalidDataContractVersionError::new(old_version + 1, new_version).into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }

    /// Validate that the config was not updated
    /// * Includes verifications that:
    ///     - Old contract is not read_only
    ///     - New contract is not read_only
    ///     - Keeps history did not change
    ///     - Can be deleted did not change
    ///     - Documents keep history did not change
    ///     - Documents can be deleted contract default did not change
    ///     - Documents mutable contract default did not change
    ///     - Requires identity encryption bounded key did not change
    ///     - Requires identity decryption bounded key did not change
    /// * Failure (contract does not exist): Keep ST and transform it to a nonce bump action.
    /// * How: A user pushed an update to a contract that changed its configuration.
    pub(super) fn validate_update_config(
        &self,
        new_data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let config_validation_result = self.config().validate_update(
            new_data_contract.config(),
            self.id(),
            platform_version,
        )?;

        if !config_validation_result.is_valid() {
            return Ok(SimpleConsensusValidationResult::new_with_errors(
                config_validation_result.errors,
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }

    /// Validate updates for existing document types to make sure that
    /// previously created documents will be still valid with a new version
    /// of the data contract. Document types can never be removed. The new
    /// contract version is passed into the per-type dispatcher: its
    /// generation 1 admits required-set additions annotated with
    /// `requiredSince` equal to that version, while generation 0 — the only
    /// one selected by pre-v14 platform versions — ignores it.
    pub(super) fn validate_update_existing_document_types(
        &self,
        new_data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        for (document_type_name, old_document_type) in self.document_types() {
            // Make sure that existing document aren't removed
            let Some(new_document_type) =
                new_data_contract.document_type_optional_for_name(document_type_name)
            else {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    DocumentTypeUpdateError::new(
                        self.id(),
                        document_type_name,
                        "document type can't be removed",
                    )
                    .into(),
                ));
            };

            // Validate document type update rules
            let validate_update_result = old_document_type.as_ref().validate_update(
                new_document_type,
                new_data_contract.version(),
                platform_version,
            )?;

            if !validate_update_result.is_valid() {
                return Ok(SimpleConsensusValidationResult::new_with_errors(
                    validate_update_result.errors,
                ));
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }

    /// Schema $defs should be compatible: `$defs` may not be removed, and a
    /// changed `$defs` must remain compatible with previously created data.
    pub(super) fn validate_update_schema_defs(
        &self,
        new_data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if let Some(old_defs_map) = self.schema_defs() {
            // If new contract doesn't have $defs, it means that it's $defs was removed and compatibility is broken
            let Some(new_defs_map) = new_data_contract.schema_defs() else {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    IncompatibleDataContractSchemaError::new(
                        self.id(),
                        "remove".to_string(),
                        "/$defs".to_string(),
                    )
                    .into(),
                ));
            };

            // If $defs is updated we need to make sure that our data contract is still compatible
            // with previously created data
            if old_defs_map != new_defs_map {
                // both new and old $defs already validated as a part of new and old contract
                let old_defs_json = Value::from(old_defs_map)
                    .try_into_validating_json()
                    .map_err(ProtocolError::ValueError)?;

                let new_defs_json = Value::from(new_defs_map)
                    .try_into_validating_json()
                    .map_err(ProtocolError::ValueError)?;

                let old_defs_schema = json!({
                    "$defs": old_defs_json
                });

                let new_defs_schema = json!({
                    "$defs": new_defs_json
                });

                // We do not allow to remove or modify $ref in document type schemas
                // it means that compatible changes in $defs won't break the overall compatibility
                // Make sure that updated $defs schema is compatible
                let compatibility_validation_result = validate_schema_compatibility(
                    &old_defs_schema,
                    &new_defs_schema,
                    platform_version,
                )?;

                if !compatibility_validation_result.is_valid() {
                    let errors = compatibility_validation_result
                        .errors
                        .into_iter()
                        .map(|operation| {
                            IncompatibleDataContractSchemaError::new(
                                self.id(),
                                operation.name,
                                operation.path,
                            )
                            .into()
                        })
                        .collect();

                    return Ok(SimpleConsensusValidationResult::new_with_errors(errors));
                }
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }

    /// Groups can be neither removed nor changed by an update.
    pub(super) fn validate_update_groups(
        &self,
        new_data_contract: &DataContract,
    ) -> SimpleConsensusValidationResult {
        if self.groups() != new_data_contract.groups() {
            // No groups can have been removed
            for old_group_position in self.groups().keys() {
                if !new_data_contract.groups().contains_key(old_group_position) {
                    return SimpleConsensusValidationResult::new_with_error(
                        DataContractUpdateActionNotAllowedError::new(
                            self.id(),
                            "remove group".to_string(),
                        )
                        .into(),
                    );
                }
            }

            // Ensure no group has been changed
            for (old_group_position, old_group) in self.groups() {
                if let Some(new_group) = new_data_contract.groups().get(old_group_position) {
                    if old_group != new_group {
                        return SimpleConsensusValidationResult::new_with_error(
                            DataContractUpdateActionNotAllowedError::new(
                                self.id(),
                                format!(
                                    "change group at position {} is not allowed",
                                    old_group_position
                                ),
                            )
                            .into(),
                        );
                    }
                }
            }
        }

        SimpleConsensusValidationResult::new()
    }

    /// Existing tokens can be neither removed nor reconfigured; a newly
    /// added token may not carry a pre-programmed distribution timestamp in
    /// the past.
    pub(super) fn validate_update_tokens(
        &self,
        new_data_contract: &DataContract,
        block_info: &BlockInfo,
    ) -> SimpleConsensusValidationResult {
        if self.tokens() != new_data_contract.tokens() {
            for (token_position, old_token_config) in self.tokens() {
                // Check if a token has been removed
                if !new_data_contract.tokens().contains_key(token_position) {
                    return SimpleConsensusValidationResult::new_with_error(
                        DataContractUpdateActionNotAllowedError::new(
                            self.id(),
                            format!("remove token at position {}", token_position),
                        )
                        .into(),
                    );
                }

                // Check if a token configuration has been changed
                if let Some(new_token_config) = new_data_contract.tokens().get(token_position) {
                    if old_token_config != new_token_config {
                        return SimpleConsensusValidationResult::new_with_error(
                            DataContractUpdateActionNotAllowedError::new(
                                self.id(),
                                format!("update token at position {}", token_position),
                            )
                            .into(),
                        );
                    }
                }
            }

            // Validate any newly added tokens
            for (token_contract_position, token_configuration) in new_data_contract.tokens() {
                if !self.tokens().contains_key(token_contract_position) {
                    if let Some(distribution) = token_configuration
                        .distribution_rules()
                        .pre_programmed_distribution()
                    {
                        if let Some((timestamp, _)) = distribution.distributions().iter().next() {
                            if timestamp < &block_info.time_ms {
                                return SimpleConsensusValidationResult::new_with_error(
                                    StateError::PreProgrammedDistributionTimestampInPastError(
                                        PreProgrammedDistributionTimestampInPastError::new(
                                            new_data_contract.id(),
                                            *token_contract_position,
                                            *timestamp,
                                            block_info.time_ms,
                                        ),
                                    )
                                    .into(),
                                );
                            }
                        }
                    }
                }
            }
        }

        SimpleConsensusValidationResult::new()
    }

    /// Changed keywords must number at most 50, each between 3 and 50
    /// visible characters, all unique.
    pub(super) fn validate_update_keywords(
        &self,
        new_data_contract: &DataContract,
    ) -> SimpleConsensusValidationResult {
        if self.keywords() != new_data_contract.keywords() {
            // Validate there are no more than 50 contract keywords
            if new_data_contract.keywords().len() > 50 {
                return SimpleConsensusValidationResult::new_with_error(
                    TooManyKeywordsError::new(self.id(), new_data_contract.keywords().len() as u8)
                        .into(),
                );
            }

            // Validate the keywords are all unique and between 3 and 50 characters
            let mut seen_keywords = HashSet::new();
            for keyword in new_data_contract.keywords() {
                // First check keyword length
                if keyword.len() < 3 || keyword.len() > 50 {
                    return SimpleConsensusValidationResult::new_with_error(
                        InvalidKeywordLengthError::new(self.id(), keyword.to_string()).into(),
                    );
                }

                if !keyword
                    .chars()
                    .all(|c| !c.is_control() && !c.is_whitespace())
                {
                    // This would mean we have an invalid character
                    return SimpleConsensusValidationResult::new_with_error(
                        InvalidKeywordCharacterError::new(
                            new_data_contract.id(),
                            keyword.to_string(),
                        )
                        .into(),
                    );
                }

                // Then check uniqueness
                if !seen_keywords.insert(keyword) {
                    return SimpleConsensusValidationResult::new_with_error(
                        DuplicateKeywordsError::new(self.id(), keyword.to_string()).into(),
                    );
                }
            }
        }

        SimpleConsensusValidationResult::new()
    }

    /// A changed description must be between 3 and 100 characters.
    pub(super) fn validate_update_description(
        &self,
        new_data_contract: &DataContract,
    ) -> SimpleConsensusValidationResult {
        if self.description() != new_data_contract.description() {
            // Validate the description is between 3 and 100 characters
            if let Some(description) = new_data_contract.description() {
                let char_count = description.chars().count();
                if !(3..=100).contains(&char_count) {
                    return SimpleConsensusValidationResult::new_with_error(
                        InvalidDescriptionLengthError::new(self.id(), description.to_string())
                            .into(),
                    );
                }
            }
        }

        SimpleConsensusValidationResult::new()
    }
}
