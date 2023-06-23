use dpp::consensus::basic::BasicError;
use dpp::consensus::basic::data_contract::InvalidDataContractIdError;
use dpp::data_contract::generate_data_contract_id;
use dpp::data_contract::state_transition::data_contract_create_transition::validation::state::validate_data_contract_create_transition_basic::DATA_CONTRACT_CREATE_SCHEMA_VALIDATOR;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use crate::error::Error;
use crate::execution::validation::state_transition::common::validate_schema::v0::validate_schema_v0;

pub(crate) trait StateTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl StateTransitionStructureValidationV0 for DataContractCreateTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        //todo: re-enable version validation
        // // Validate protocol version
        // let protocol_version_validator = ProtocolVersionValidator::default();
        // let result = protocol_version_validator
        //     .validate(self.protocol_version)
        //     .expect("TODO: again, how this will ever fail, why do we even need a validator trait");
        // if !result.is_valid() {
        //     return Ok(result);
        // }
        //
        // // Validate data contract
        // let data_contract_validator =
        //     DataContractValidator::new(Arc::new(protocol_version_validator)); // ffs
        // let result = data_contract_validator
        //     .validate(&(self.data_contract.to_cleaned_object().expect("TODO")))?;
        // if !result.is_valid() {
        //     return Ok(result);
        // }

        // Validate data contract id
        let generated_id = generate_data_contract_id(self.data_contract.owner_id, self.entropy);
        if generated_id.as_slice() != self.data_contract.id.as_ref() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                BasicError::InvalidDataContractIdError(InvalidDataContractIdError::new(
                    generated_id.to_vec(),
                    self.data_contract.id.as_ref().to_owned(),
                ))
                .into(),
            ));
        }

        self.data_contract
            .validate_structure()
            .map_err(Error::Protocol)
    }
}
