use crate::error::execution::ExecutionError::CorruptedCodeExecution;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::identity_update::added_key_proofs_of_possession::IdentityUpdateAddedKeyProofsOfPossessionValidation;
use dpp::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;
use dpp::consensus::state::state_error::StateError;
use dpp::identity::PartialIdentity;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::validation::ConsensusValidationResult;
use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition) trait IdentityUpdateStateTransitionIdentityAndSignaturesValidationV1
{
    fn validate_identity_update_state_transition_signatures_v1(
        &self,
        signable_bytes: Vec<u8>,
        partial_identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityUpdateStateTransitionIdentityAndSignaturesValidationV1 for IdentityUpdateTransition {
    /// Like v0, except that an added key whose proof of possession does not verify is refused
    /// unpaid.
    ///
    /// The identity signs the transition's signable bytes, which leave out the signatures of the
    /// keys it adds, so a proof of possession is in a field the identity's signature does not
    /// cover. Such a failure is not charged to the identity: the transition is refused like one
    /// whose identity signature fails, without a fee and without using the nonce, and no key is
    /// added.
    ///
    /// The data of an added key is signed, yet a key whose public key data does not decode fails
    /// its proof of possession too, and is refused unpaid like any other proof failure. A revision
    /// mismatch is still charged, because the revision is signed; it is checked first.
    fn validate_identity_update_state_transition_signatures_v1(
        &self,
        signable_bytes: Vec<u8>,
        partial_identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let Some(revision) = partial_identity.revision else {
            return Err(Error::Execution(CorruptedCodeExecution(
                "revision should exist",
            )));
        };

        if revision.checked_add(1) != Some(self.revision()) {
            let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                BumpIdentityNonceAction::from_borrowed_identity_update_transition(self),
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

        let validation_result =
            self.validate_added_key_proofs_of_possession(&signable_bytes, execution_context);

        if validation_result.is_valid() {
            Ok(ConsensusValidationResult::new())
        } else {
            // No action: the failure is refused unpaid
            Ok(ConsensusValidationResult::new_with_errors(
                validation_result.errors,
            ))
        }
    }
}
