use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::serialization::PlatformMessageSignable;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::validation::SimpleConsensusValidationResult;

/// Verification of the proofs of possession of the keys an identity update adds.
pub(in crate::execution::validation::state_transition) trait IdentityUpdateAddedKeyProofsOfPossessionValidation
{
    /// Verifies, in order, that each added key's signature over `signable_bytes` verifies for its
    /// public key data, and stops at the first that does not. A key type that carries no proof of
    /// possession (a hash key) must have no signature. Records one signature verification
    /// operation per key verified, the failing one included.
    ///
    /// This is consensus code: identity update advanced structure validation 1 refuses unpaid
    /// what this refuses and charges a valid update the operations it records, and check_tx
    /// refuses it at every protocol version and counts those operations in its fee estimate.
    /// Changing what it refuses or the operations it records needs a new advanced structure
    /// generation, with check_tx following it.
    fn validate_added_key_proofs_of_possession(
        &self,
        signable_bytes: &[u8],
        execution_context: &mut StateTransitionExecutionContext,
    ) -> SimpleConsensusValidationResult;
}

impl IdentityUpdateAddedKeyProofsOfPossessionValidation for IdentityUpdateTransition {
    fn validate_added_key_proofs_of_possession(
        &self,
        signable_bytes: &[u8],
        execution_context: &mut StateTransitionExecutionContext,
    ) -> SimpleConsensusValidationResult {
        for key in self.public_keys_to_add().iter() {
            let result = signable_bytes.verify_signature(
                key.key_type(),
                key.data().as_slice(),
                key.signature().as_slice(),
            );
            execution_context.add_operation(ValidationOperation::SignatureVerification(
                SignatureVerificationOperation::new(key.key_type()),
            ));
            if !result.is_valid() {
                return result;
            }
        }

        SimpleConsensusValidationResult::new()
    }
}
