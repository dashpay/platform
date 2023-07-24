use crate::error::Error;
use crate::execution::validation::state_transition::common::validate_schema::v0::validate_schema_v0;
use dpp::data_contract::state_transition::data_contract_update_transition::validation::basic::DATA_CONTRACT_UPDATE_SCHEMA_VALIDATOR;
use dpp::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;

pub(crate) trait StateTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl StateTransitionStructureValidationV0 for DataContractUpdateTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema_v0(&DATA_CONTRACT_UPDATE_SCHEMA_VALIDATOR, self);
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
