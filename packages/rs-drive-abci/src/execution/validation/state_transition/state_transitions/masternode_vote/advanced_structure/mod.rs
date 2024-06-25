use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::masternode_vote::advanced_structure::v0::MasternodeVoteStateTransitionAdvancedStructureValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionStructureKnownInStateValidationV0;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

pub(crate) mod v0;

impl StateTransitionStructureKnownInStateValidationV0 for MasternodeVoteTransition {
    fn validate_advanced_structure_from_state(
        &self,
        action: &StateTransitionAction,
        identity: Option<&PartialIdentity>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .masternode_vote_state_transition
            .advanced_structure
        {
            Some(0) => {
                let identity =
                    identity.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "The identity must be known on advanced structure validation",
                    )))?;
                let StateTransitionAction::MasternodeVoteAction(masternode_vote_action) = action
                else {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "action must be a masternode vote action",
                    )));
                };
                self.validate_advanced_structure_from_state_v0(
                    masternode_vote_action,
                    identity,
                    execution_context,
                )
            }
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "masternode vote transition: validate_advanced_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "masternode vote transition: validate_advanced_structure".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    fn has_advanced_structure_validation_with_state(&self) -> bool {
        true
    }

    fn requires_advanced_structure_validation_with_state_on_check_tx(&self) -> bool {
        true
    }
}
