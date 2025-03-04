use crate::data_contract::group::methods::v0::GroupMethodsV0;
use crate::data_contract::group::Group;
use crate::data_contract::{DataContract, GroupContractPosition};
use crate::errors::consensus::basic::data_contract::NonContiguousContractGroupPositionsError;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl DataContract {
    #[inline(always)]
    pub(super) fn validate_groups_v0(
        groups: &BTreeMap<GroupContractPosition, Group>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Check for gaps in the group contract positions
        let mut expected_position = 0;
        for &position in groups.keys() {
            if position != expected_position {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    NonContiguousContractGroupPositionsError::new(expected_position, position)
                        .into(),
                ));
            }
            expected_position += 1;
        }

        // Validate each group individually
        for group in groups.values() {
            let validation_result = group.validate(platform_version)?;
            if !validation_result.is_valid() {
                return Ok(validation_result);
            }
        }

        // If we reach here, everything is valid
        Ok(SimpleConsensusValidationResult::new())
    }
}
