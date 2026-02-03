use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::data_contract_update::basic_structure::v0::DataContractUpdateStateTransitionBasicStructureValidationV0;
use dpp::consensus::basic::data_contract::{
    DataContractUpdateTransitionConflictingKeywordError,
    DataContractUpdateTransitionOverlappingFieldsError, InvalidDescriptionLengthError,
};
use dpp::dashcore::Network;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionBasicStructureValidationV1
{
    fn validate_basic_structure_v1(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractUpdateStateTransitionBasicStructureValidationV1 for DataContractUpdateTransition {
    fn validate_basic_structure_v1(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // V0 transitions use V0 validation
        match self {
            DataContractUpdateTransition::V0(_) => {
                self.validate_basic_structure_v0(network_type, platform_version)
            }
            DataContractUpdateTransition::V1(v1) => {
                // Validate that updated_schema_defs and new_schema_defs don't overlap
                for key in v1.updated_schema_defs.keys() {
                    if v1.new_schema_defs.contains_key(key) {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            DataContractUpdateTransitionOverlappingFieldsError::new(
                                v1.id,
                                "schema_defs".to_string(),
                                key.clone(),
                            )
                            .into(),
                        ));
                    }
                }

                // Validate that updated_document_schemas and new_document_schemas don't overlap
                for key in v1.updated_document_schemas.keys() {
                    if v1.new_document_schemas.contains_key(key) {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            DataContractUpdateTransitionOverlappingFieldsError::new(
                                v1.id,
                                "document_schemas".to_string(),
                                key.clone(),
                            )
                            .into(),
                        ));
                    }
                }

                // Validate that add_keywords and remove_keywords don't overlap
                for keyword in &v1.add_keywords {
                    if v1.remove_keywords.contains(keyword) {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            DataContractUpdateTransitionConflictingKeywordError::new(
                                v1.id,
                                keyword.clone(),
                            )
                            .into(),
                        ));
                    }
                }

                // Validate new groups (allow offset start for updates)
                if !v1.new_groups.is_empty() {
                    let validation_result =
                        DataContract::validate_groups(&v1.new_groups, true, platform_version)?;

                    if !validation_result.is_valid() {
                        return Ok(validation_result);
                    }
                }

                // Validate new tokens (allow offset start for updates)
                // Note: validate_token_config_groups_exist is done in apply_update
                // where we have access to all groups (existing + new)
                if !v1.new_tokens.is_empty() {
                    let validation_result = DataContract::validate_tokens(
                        v1.id,
                        &v1.new_tokens,
                        true,
                        network_type,
                        platform_version,
                    )?;

                    if !validation_result.is_valid() {
                        return Ok(validation_result);
                    }
                }

                // Validate added keywords structure
                // Note: Full keyword validation (including combined keywords) is done in apply_update
                if !v1.add_keywords.is_empty() {
                    let validation_result = DataContract::validate_keywords(
                        v1.id,
                        &v1.add_keywords,
                        true,
                        platform_version,
                    )?;

                    if !validation_result.is_valid() {
                        return Ok(validation_result);
                    }
                }

                // Validate description if being updated
                if let Some(Some(description)) = &v1.update_description {
                    let char_count = description.chars().count();
                    if !(3..=100).contains(&char_count) {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            InvalidDescriptionLengthError::new(v1.id, description.clone()).into(),
                        ));
                    }
                }

                Ok(SimpleConsensusValidationResult::new())
            }
        }
    }
}
