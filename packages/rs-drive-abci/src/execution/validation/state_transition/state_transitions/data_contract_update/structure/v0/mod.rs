use crate::error::Error;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::{PlatformVersion, TryIntoPlatformVersioned};

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionStructureValidationV0 {
    fn validate_structure_v0(&self, platform_version: &PlatformVersion) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractUpdateStateTransitionStructureValidationV0 for DataContractUpdateTransition {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let data_contact: DataContract = self
            .data_contract()
            .clone()
            .try_into_platform_versioned(platform_version)?;
        data_contact
            .validate_structure(platform_version)
            .map_err(Error::Protocol)
    }
}
