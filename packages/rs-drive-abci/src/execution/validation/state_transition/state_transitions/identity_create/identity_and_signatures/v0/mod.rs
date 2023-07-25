use crate::error::Error;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use dpp::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::serialization::serialization_traits::{PlatformMessageSignable, Signable};
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;

pub(crate) trait StateTransitionIdentityAndSignaturesValidationV0 {
    fn validate_identity_and_signatures_v0(
        &self,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error>;
}

impl StateTransitionIdentityAndSignaturesValidationV0 for IdentityCreateTransition {
    fn validate_identity_and_signatures_v0(
        &self,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        let mut validation_result = ConsensusValidationResult::<Option<PartialIdentity>>::default();
        let bytes: Vec<u8> = self.signable_bytes()?;
        for key in self.public_keys.iter() {
            let result = bytes.as_slice().verify_signature(
                key.key_type,
                key.data.as_slice(),
                key.signature.as_slice(),
            )?;
            if !result.is_valid() {
                validation_result.add_errors(result.errors);
            }
        }

        // We should validate that the identity id is created from the asset lock proof

        let identifier_from_outpoint = match self.get_asset_lock_proof().create_identifier() {
            Ok(identifier) => identifier,
            Err(_) => {
                return Ok(ConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(
                        BasicError::IdentityAssetLockTransactionOutputNotFoundError(
                            IdentityAssetLockTransactionOutputNotFoundError::new(
                                self.asset_lock_proof.instant_lock_output_index().unwrap(),
                            ),
                        ),
                    ),
                ))
            }
        };

        if identifier_from_outpoint != self.identity_id {
            return Ok(ConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::InvalidIdentifierError(
                    InvalidIdentifierError::new(
                        "identity_id".to_string(),
                        "does not match created identifier from asset lock".to_string(),
                    ),
                )),
            ));
        }

        // We need to set the data, even though we are setting to None,
        // We are really setting to Some(None) internally,
        validation_result.set_data(None);
        Ok(validation_result)
    }
}
