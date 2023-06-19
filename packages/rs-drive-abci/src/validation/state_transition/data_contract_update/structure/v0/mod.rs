use dpp::consensus::basic::BasicError;
use dpp::consensus::basic::data_contract::InvalidDataContractIdError;
use dpp::data_contract::generate_data_contract_id;
use dpp::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::data_contract::state_transition::data_contract_create_transition::validation::state::validate_data_contract_create_transition_basic::DATA_CONTRACT_CREATE_SCHEMA_VALIDATOR;
use dpp::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::data_contract::state_transition::data_contract_update_transition::validation::basic::DATA_CONTRACT_UPDATE_SCHEMA_VALIDATOR;
use dpp::validation::SimpleConsensusValidationResult;
use crate::error::Error;
use crate::validation::state_transition::common::validate_schema;

pub(in crate::validation::state_transition::data_contract_update) trait StateTransitionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl StateTransitionStructureValidationV0 for DataContractUpdateTransition {
    fn validate_structure_v0(
        &self,
    ) -> Result<SimpleConsensusValidationResult, Error>{
        let result = validate_schema(&DATA_CONTRACT_UPDATE_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        // Validate protocol version
        //todo: redo versioning
        // let protocol_version_validator = ProtocolVersionValidator::default();
        // let result = protocol_version_validator
        //     .validate(self.protocol_version)
        //     .expect("TODO: again, how this will ever fail, why do we even need a validator trait");
        // if !result.is_valid() {
        //     return Ok(result);
        // }

        self.data_contract
            .validate_structure()
            .map_err(Error::Protocol)
    }
}