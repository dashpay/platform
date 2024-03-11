use crate::error::Error;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreatedStateTransitionAdvancedStructureValidationV0 {
    fn validate_advanced_structure_v0(&self, platform_version: &PlatformVersion) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractCreatedStateTransitionAdvancedStructureValidationV0
    for DataContractCreateTransition
{
    fn validate_advanced_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        // Validate data contract
        let result = DataContract::try_from_platform_versioned(
            self.data_contract().clone(),
            true,
            platform_version,
        );

        // Return validation result if any consensus errors happened
        // during data contract validation
        match result {
            Err(ProtocolError::ConsensusError(consensus_error)) => {
                let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                    BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(self)?,
                );

                Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    vec![*consensus_error],
                ))
            }
            Err(protocol_error) => Err(protocol_error.into()),
            Ok(_) => Ok(ConsensusValidationResult::new()),
        }
    }
}
