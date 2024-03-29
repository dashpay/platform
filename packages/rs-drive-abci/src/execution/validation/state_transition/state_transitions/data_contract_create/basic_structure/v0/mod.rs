use crate::error::Error;
use dpp::consensus::basic::data_contract::InvalidDataContractIdError;
use dpp::consensus::basic::BasicError;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreatedStateTransitionBasicStructureValidationV0 {
    fn validate_basic_structure_v0(&self, platform_version: &PlatformVersion) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractCreatedStateTransitionBasicStructureValidationV0 for DataContractCreateTransition {
    fn validate_basic_structure_v0(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // Validate data contract id
        let generated_id = DataContract::generate_data_contract_id_v0(
            self.data_contract().owner_id(),
            self.identity_nonce(),
        );

        if generated_id != self.data_contract().id() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                BasicError::InvalidDataContractIdError(InvalidDataContractIdError::new(
                    generated_id.to_vec(),
                    self.data_contract().id().to_vec(),
                ))
                .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::default())
    }
}
