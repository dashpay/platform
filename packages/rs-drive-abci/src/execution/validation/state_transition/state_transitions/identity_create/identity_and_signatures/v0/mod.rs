use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::serialization::PlatformMessageSignable;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::validation::SimpleConsensusValidationResult;

pub(crate) trait IdentityCreateStateTransitionIdentityAndSignaturesValidationV0 {
    fn validate_identity_create_state_transition_signatures_v0(
        &self,
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
    ) -> SimpleConsensusValidationResult;
}

impl IdentityCreateStateTransitionIdentityAndSignaturesValidationV0 for IdentityCreateTransition {
    fn validate_identity_create_state_transition_signatures_v0(
        &self,
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
    ) -> SimpleConsensusValidationResult {
        for key in self.public_keys().iter() {
            let result = signable_bytes.as_slice().verify_signature(
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
