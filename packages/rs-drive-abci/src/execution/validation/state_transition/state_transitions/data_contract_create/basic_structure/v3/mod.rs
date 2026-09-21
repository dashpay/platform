use crate::error::Error;
use dpp::dashcore::Network;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use super::v2::DataContractCreateStateTransitionBasicStructureValidationV2;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreateStateTransitionBasicStructureValidationV3 {
    fn validate_basic_structure_v3(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractCreateStateTransitionBasicStructureValidationV3 for DataContractCreateTransition {
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
        for (position, configuration) in self.data_contract().tokens() {
            let result = configuration.validate_format_version(platform_version);
            if !result.is_valid() {
                return Ok(result);
            }
            let result = configuration.validate_shielded_pool_rules(*position);
            if !result.is_valid() {
                return Ok(result);
            }
        }
        Ok(SimpleConsensusValidationResult::new())
    }
}
