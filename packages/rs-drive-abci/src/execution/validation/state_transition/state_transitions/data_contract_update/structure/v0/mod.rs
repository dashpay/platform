use crate::error::Error;
use dpp::data_contract::DataContract;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::ProtocolError;

use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionStructureValidationV0 {
    fn validate_base_structure_v0(&self, platform_version: &PlatformVersion) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractUpdateStateTransitionStructureValidationV0 for DataContractUpdateTransition {
    fn validate_base_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match DataContract::try_from_platform_versioned(
            self.data_contract().clone(),
            true,
            platform_version,
        ) {
            Ok(_) => Ok(SimpleConsensusValidationResult::default()),
            Err(ProtocolError::ConsensusError(e)) => Ok(
                SimpleConsensusValidationResult::new_with_error(e.as_ref().clone()),
            ),
            Err(e) => Err(e.into()),
        }
    }
}
