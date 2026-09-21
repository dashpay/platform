use crate::error::Error;
use dpp::dashcore::Network;
use dpp::data_contract::associated_token::token_configuration::validate_token_configurations;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use super::v2::DataContractUpdateStateTransitionBasicStructureValidationV2;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionBasicStructureValidationV3 {
    fn validate_basic_structure_v3(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractUpdateStateTransitionBasicStructureValidationV3 for DataContractUpdateTransition {
    /// Protocol 15 retains the previous rules and validates token shielded pool opt-ins.
    fn validate_basic_structure_v3(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = self.validate_basic_structure_v2(network_type, platform_version)?;
        if !result.is_valid() {
            return Ok(result);
        }
        Ok(validate_token_configurations(
            self.data_contract().tokens(),
            platform_version,
        ))
    }
}
