use crate::error::Error;
use dpp::consensus::basic::data_contract::InvalidDataContractIdError;
use dpp::consensus::basic::BasicError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionMethodsV0;
use dpp::validation::SimpleConsensusValidationResult;

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
        // Validate data contract
        // self.data_contract().validate()
        // let data_contract_validator =
        //     DataContractValidator::new(Arc::new(protocol_version_validator)); // ffs
        // let result = data_contract_validator
        //     .validate(&(self.data_contract.to_cleaned_object().expect("TODO")))?;
        // if !result.is_valid() {
        //     return Ok(result);
        // }

        // Validate data contract id
        let generated_id = DataContract::generate_data_contract_id_v0(
            self.data_contract().owner_id(),
            self.entropy(),
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

        self.data_contract()
            .validate_structure()
            .map_err(Error::Protocol)
    }
}
