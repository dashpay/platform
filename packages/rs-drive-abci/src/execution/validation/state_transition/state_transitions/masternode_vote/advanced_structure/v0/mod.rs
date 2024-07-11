use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::consensus::state::voting::masternode_incorrect_voter_identity_id_error::MasternodeIncorrectVoterIdentityIdError;
use dpp::consensus::state::voting::masternode_incorrect_voting_address_error::MasternodeIncorrectVotingAddressError;
use dpp::identifier::MasternodeIdentifiers;
use dpp::identity::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::PartialIdentity;
use dpp::prelude::{ConsensusValidationResult, Identifier};
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::state_transition::StateTransitionIdentitySigned;
use drive::state_transition_action::identity::masternode_vote::MasternodeVoteTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::masternode_vote) trait MasternodeVoteStateTransitionAdvancedStructureValidationV0
{
    fn validate_advanced_structure_from_state_v0(
        &self,
        action: &MasternodeVoteTransitionAction,
        identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl MasternodeVoteStateTransitionAdvancedStructureValidationV0 for MasternodeVoteTransition {
    fn validate_advanced_structure_from_state_v0(
        &self,
        action: &MasternodeVoteTransitionAction,
        identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let key = identity
            .loaded_public_keys
            .get(&self.signature_public_key_id())
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "public key must be known",
            )))?;

        let voting_address = key.public_key_hash()?;
        if action.voting_address() != voting_address {
            return Ok(ConsensusValidationResult::new_with_error(
                MasternodeIncorrectVotingAddressError::new(
                    self.pro_tx_hash(),
                    action.voting_address().into(),
                    voting_address.into(),
                )
                .into(),
            ));
        }

        // We also need to verify that the provided voter_id is correct

        execution_context.add_operation(ValidationOperation::SingleSha256(2));

        let expected_voter_identity_id =
            Identifier::create_voter_identifier(self.pro_tx_hash().as_bytes(), &voting_address);

        if expected_voter_identity_id != self.voter_identity_id() {
            return Ok(ConsensusValidationResult::new_with_error(
                MasternodeIncorrectVoterIdentityIdError::new(
                    self.pro_tx_hash(),
                    expected_voter_identity_id,
                    self.voter_identity_id(),
                )
                .into(),
            ));
        }

        Ok(ConsensusValidationResult::new())
    }
}
