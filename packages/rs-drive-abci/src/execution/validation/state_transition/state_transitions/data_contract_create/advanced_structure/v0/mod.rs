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
use dpp::validation::ConsensusValidationResult;
use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreatedStateTransitionAdvancedStructureValidationV0 {
    fn validate_advanced_structure_v0(&self, execution_context: &mut StateTransitionExecutionContext) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractCreatedStateTransitionAdvancedStructureValidationV0
    for DataContractCreateTransition
{
    fn validate_advanced_structure_v0(
        &self,
        execution_context: &mut StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        // Validate data contract id
        let generated_id = DataContract::generate_data_contract_id_v0(
            self.data_contract().owner_id(),
            self.identity_nonce(),
        );

        // This hash will only take 1 block (64 bytes)
        execution_context.add_operation(ValidationOperation::DoubleSha256(1));

        if generated_id != self.data_contract().id() {
            let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(self),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![
                    BasicError::InvalidDataContractIdError(InvalidDataContractIdError::new(
                        generated_id.to_vec(),
                        self.data_contract().id().to_vec(),
                    ))
                    .into(),
                ],
            ));
        }

        Ok(ConsensusValidationResult::default())
    }
}
