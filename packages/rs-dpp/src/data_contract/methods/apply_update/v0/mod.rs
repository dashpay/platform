use crate::block::block_info::BlockInfo;
use crate::consensus::basic::data_contract::NonContiguousContractTokenPositionsError;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::update_values::DataContractUpdateValues;
use crate::data_contract::{DataContract, TokenContractPosition};
use crate::validation::operations::ProtocolValidationOperation;
use crate::validation::ConsensusValidationResult;
use crate::ProtocolError;
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
        let merged = match update_values.merge_onto_v0(self, block_info) {
            Ok(merged) => merged,
            Err(consensus_error) => {
                return Ok(ConsensusValidationResult::new_with_error(consensus_error))
            }
        };

        if !update_values.new_groups.is_empty() {
            // Positions must stay contiguous across the stored and the new
            // groups, which only the merged map can show.
            let validation_result =
                DataContract::validate_groups(&merged.groups, platform_version)?;
            if !validation_result.is_valid() {
                return Ok(ConsensusValidationResult::new_with_errors(
                    validation_result.errors,
                ));
            }
        }

        for (expected_position, position) in merged.tokens.keys().enumerate() {
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
                token.validate_token_config_groups_exist(&merged.groups, platform_version)?;
            if !validation_result.is_valid() {
                return Ok(ConsensusValidationResult::new_with_errors(
                    validation_result.errors,
                ));
            }
        }

        let updated_contract = DataContract::try_from_platform_versioned(
            DataContractInSerializationFormat::V1(merged),
            full_validation,
            validation_operations,
            platform_version,
        )?;

        Ok(ConsensusValidationResult::new_with_data(updated_contract))
    }
}
