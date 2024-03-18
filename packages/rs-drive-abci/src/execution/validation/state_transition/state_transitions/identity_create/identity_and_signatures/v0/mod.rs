use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use dpp::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::identity::state_transition::AssetLockProved;

use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::prelude::ConsensusValidationResult;
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

        // We should validate that the identity id is created from the asset lock proof

        let identifier_from_outpoint = match self.asset_lock_proof().create_identifier() {
            Ok(identifier) => identifier,
            Err(_) => {
                return ConsensusValidationResult::new_with_error(ConsensusError::BasicError(
                    BasicError::IdentityAssetLockTransactionOutputNotFoundError(
                        IdentityAssetLockTransactionOutputNotFoundError::new(
                            self.asset_lock_proof().output_index() as usize,
                        ),
                    ),
                ))
            }
        };

        if identifier_from_outpoint != self.identity_id() {
            return ConsensusValidationResult::new_with_error(ConsensusError::BasicError(
                BasicError::InvalidIdentifierError(InvalidIdentifierError::new(
                    "identity_id".to_string(),
                    "does not match created identifier from asset lock".to_string(),
                )),
            ));
        }

        SimpleConsensusValidationResult::new()
    }
}
