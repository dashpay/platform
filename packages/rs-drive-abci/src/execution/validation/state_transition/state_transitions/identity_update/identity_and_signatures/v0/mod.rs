use crate::error::execution::ExecutionError::CorruptedCodeExecution;
use crate::error::Error;

use dpp::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;
use dpp::consensus::state::state_error::StateError;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::serialization::{PlatformMessageSignable, Signable};
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub(in crate::execution::validation::state_transition) trait IdentityUpdateStateTransitionIdentityAndSignaturesValidationV0
{
    fn validate_identity_update_state_transition_signatures_v0(
        &self,
        signable_bytes: Vec<u8>,
        partial_identity: &PartialIdentity,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityUpdateStateTransitionIdentityAndSignaturesValidationV0 for IdentityUpdateTransition {
    fn validate_identity_update_state_transition_signatures_v0(
        &self,
        signable_bytes: Vec<u8>,
        partial_identity: &PartialIdentity,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let mut result = SimpleConsensusValidationResult::default();

        for key in self.public_keys_to_add().iter() {
            let validation_result = signable_bytes.as_slice().verify_signature(
                key.key_type(),
                key.data().as_slice(),
                key.signature().as_slice(),
            )?;
            if !validation_result.is_valid() {
                result.add_errors(validation_result.errors);
            }
        }

        let Some(revision) = partial_identity.revision else {
            return Err(Error::Execution(CorruptedCodeExecution("revision should exist")));
        };

        // Check revision
        if revision + 1 != self.revision() {
            result.add_error(StateError::InvalidIdentityRevisionError(
                InvalidIdentityRevisionError::new(self.identity_id(), revision),
            ));
            return Ok(result);
        }

        Ok(result)
    }
}
