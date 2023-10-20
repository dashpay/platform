use crate::error::Error;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use dpp::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::identity::state_transition::AssetLockProved;

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
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreateStateTransitionIdentityAndSignaturesValidationV0 for IdentityCreateTransition {
    fn validate_identity_create_state_transition_signatures_v0(
        &self,
        signable_bytes: Vec<u8>,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let mut validation_result = SimpleConsensusValidationResult::default();
        for key in self.public_keys().iter() {
            let result = signable_bytes.as_slice().verify_signature(
                key.key_type(),
                key.data().as_slice(),
                key.signature().as_slice(),
            )?;
            if !result.is_valid() {
                validation_result.add_errors(result.errors);
            }
        }

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // We should validate that the identity id is created from the asset lock proof

        let identifier_from_outpoint = match self.asset_lock_proof().create_identifier() {
            Ok(identifier) => identifier,
            Err(_) => {
                return Ok(ConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(
                        BasicError::IdentityAssetLockTransactionOutputNotFoundError(
                            IdentityAssetLockTransactionOutputNotFoundError::new(
                                self.asset_lock_proof().instant_lock_output_index().unwrap(),
                            ),
                        ),
                    ),
                ))
            }
        };

        if identifier_from_outpoint != self.identity_id() {
            return Ok(ConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::InvalidIdentifierError(
                    InvalidIdentifierError::new(
                        "identity_id".to_string(),
                        "does not match created identifier from asset lock".to_string(),
                    ),
                )),
            ));
        }

        Ok(validation_result)
    }
}
