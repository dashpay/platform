use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::identity_create::StateTransitionStructureKnownInStateValidationForIdentityCreateTransitionV0;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::serialization::Signable;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionStructureKnownInStateValidationV0 {
    /// Validates the structure of a transaction by checking its basic elements.
    ///
    /// # Arguments
    ///
    /// * `action` - An optional reference to the state transition action.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, Error>` - A result with either a SimpleConsensusValidationResult or an Error.
    fn validate_advanced_structure_from_state(
        &self,
        block_info: &BlockInfo,
        network: Network,
        action: &StateTransitionAction,
        maybe_identity: Option<&PartialIdentity>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    /// This means we should transform into the action before validation of the structure
    fn has_advanced_structure_validation_with_state(&self) -> bool;
    /// This means we should transform into the action before validation of the advanced structure,
    /// and that we must even do this on check_tx
    fn requires_advanced_structure_validation_with_state_on_check_tx(&self) -> bool;
}

impl StateTransitionStructureKnownInStateValidationV0 for StateTransition {
    fn validate_advanced_structure_from_state(
        &self,
        block_info: &BlockInfo,
        network: Network,
        action: &StateTransitionAction,
        maybe_identity: Option<&PartialIdentity>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match self {
            StateTransition::Batch(st) => st.validate_advanced_structure_from_state(
                block_info,
                network,
                action,
                maybe_identity,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityCreate(st) => {
                let signable_bytes = self.signable_bytes()?;
                let StateTransitionAction::IdentityCreateAction(identity_create_action) = action
                else {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "action must be a identity create transition action",
                    )));
                };
                st.validate_advanced_structure_from_state_for_identity_create_transition(
                    identity_create_action,
                    signable_bytes,
                    execution_context,
                    platform_version,
                )
            }
            StateTransition::MasternodeVote(st) => st.validate_advanced_structure_from_state(
                block_info,
                network,
                action,
                maybe_identity,
                execution_context,
                platform_version,
            ),
            _ => Ok(ConsensusValidationResult::new()),
        }
    }

    /// This means we should transform into the action before validation of the advanced structure
    fn has_advanced_structure_validation_with_state(&self) -> bool {
        matches!(
            self,
            StateTransition::Batch(_)
                | StateTransition::IdentityCreate(_)
                | StateTransition::MasternodeVote(_)
        )
    }

    /// This means we should transform into the action before validation of the advanced structure,
    /// and that we must even do this on check_tx
    fn requires_advanced_structure_validation_with_state_on_check_tx(&self) -> bool {
        matches!(self, StateTransition::Batch(_))
    }
}
