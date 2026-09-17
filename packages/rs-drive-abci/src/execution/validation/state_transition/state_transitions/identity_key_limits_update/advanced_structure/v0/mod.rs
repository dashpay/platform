use crate::error::execution::ExecutionError::CorruptedCodeExecution;
use crate::error::Error;

use dpp::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;
use dpp::consensus::state::state_error::StateError;

use dpp::identity::PartialIdentity;

use dpp::state_transition::identity_key_limits_update_transition::accessors::IdentityKeyLimitsUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_key_limits_update_transition::IdentityKeyLimitsUpdateTransition;
use dpp::validation::ConsensusValidationResult;
use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition) trait IdentityKeyLimitsUpdateStateTransitionAdvancedStructureValidationV0
{
    fn validate_advanced_structure_v0(
        &self,
        partial_identity: &PartialIdentity,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityKeyLimitsUpdateStateTransitionAdvancedStructureValidationV0
    for IdentityKeyLimitsUpdateTransition
{
    /// The transition claims the next revision of the identity, as an identity update does: a
    /// stale view of the identity is refused, and paid for, before any key is read.
    fn validate_advanced_structure_v0(
        &self,
        partial_identity: &PartialIdentity,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let Some(revision) = partial_identity.revision else {
            return Err(Error::Execution(CorruptedCodeExecution(
                "revision should exist",
            )));
        };

        if revision + 1 != self.revision() {
            let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                BumpIdentityNonceAction::from_borrowed_identity_key_limits_update_transition(self),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![
                    StateError::InvalidIdentityRevisionError(InvalidIdentityRevisionError::new(
                        self.identity_id(),
                        revision,
                    ))
                    .into(),
                ],
            ));
        }

        Ok(ConsensusValidationResult::new())
    }
}
