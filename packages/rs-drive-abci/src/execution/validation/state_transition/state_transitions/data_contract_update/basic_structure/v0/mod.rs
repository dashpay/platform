use crate::error::Error;
use dpp::dashcore::Network;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionBasicStructureValidationV0
{
    fn validate_basic_structure_v0(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractUpdateStateTransitionBasicStructureValidationV0 for DataContractUpdateTransition {
    fn validate_basic_structure_v0(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // V0 basic structure validation only applies to V0 transitions
        // V1 transitions should use validate_basic_structure_v1
        let v0 = match self {
            DataContractUpdateTransition::V0(v0) => v0,
            DataContractUpdateTransition::V1(_) => {
                // V1 transitions don't have a full data contract embedded,
                // so this validation doesn't apply
                return Ok(SimpleConsensusValidationResult::new());
            }
        };

        let groups = v0.data_contract.groups();
        if !groups.is_empty() {
            let validation_result = DataContract::validate_groups(groups, false, platform_version)?;

            if !validation_result.is_valid() {
                return Ok(validation_result);
            }
        }

        let tokens = v0.data_contract.tokens();
        if !tokens.is_empty() {
            // Validate token structure (positions, base supply, etc.)
            let validation_result = DataContract::validate_tokens(
                v0.data_contract.id(),
                tokens,
                false,
                network_type,
                platform_version,
            )?;

            if !validation_result.is_valid() {
                return Ok(validation_result);
            }

            // V0 has full contract, so validate token config groups exist here
            for (_, token_configuration) in tokens.iter() {
                let validation_result = token_configuration.validate_token_config_groups_exist(
                    v0.data_contract.groups(),
                    platform_version,
                )?;
                if !validation_result.is_valid() {
                    return Ok(validation_result);
                }
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
