use crate::error::Error;
use dpp::data_contract::DataContract;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::ProtocolError;

use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionAdvancedStructureValidationV0 {
    fn validate_advanced_structure_v0(&self, platform_version: &PlatformVersion) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractUpdateStateTransitionAdvancedStructureValidationV0
    for DataContractUpdateTransition
{
    fn validate_advanced_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match DataContract::try_from_platform_versioned(
            self.data_contract().clone(),
            true,
            platform_version,
        ) {
            Err(ProtocolError::ConsensusError(consensus_error)) => {
                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(self)?,
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
