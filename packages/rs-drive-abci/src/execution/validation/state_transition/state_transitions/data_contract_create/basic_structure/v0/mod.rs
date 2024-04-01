use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::consensus::basic::data_contract::InvalidDataContractIdError;
use dpp::consensus::basic::BasicError;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::validation::SimpleConsensusValidationResult;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreatedStateTransitionBasicStructureValidationV0 {
    fn validate_basic_structure_v0(&self, execution_context: &mut StateTransitionExecutionContext) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractCreatedStateTransitionBasicStructureValidationV0 for DataContractCreateTransition {
    fn validate_basic_structure_v0(
        &self,
        execution_context: &mut StateTransitionExecutionContext,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // Validate data contract id
        let generated_id = DataContract::generate_data_contract_id_v0(
            self.data_contract().owner_id(),
            self.identity_nonce(),
        );

        // This hash will only take 1 block (64 bytes)
        execution_context.add_operation(ValidationOperation::DoubleSha256(1));

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
