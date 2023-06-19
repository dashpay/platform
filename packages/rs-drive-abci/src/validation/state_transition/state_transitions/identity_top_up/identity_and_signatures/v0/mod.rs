use crate::error::Error;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use dpp::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::signature::{IdentityNotFoundError, SignatureError};
use dpp::consensus::ConsensusError;
use dpp::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::serialization_traits::{PlatformMessageSignable, Signable};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub(in crate::validation::state_transition) trait StateTransitionIdentityAndSignaturesValidationV0 {
    fn validate_identity_and_signatures_v0(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error>;
}

impl StateTransitionIdentityAndSignaturesValidationV0 for IdentityTopUpTransition {
    fn validate_identity_and_signatures_v0(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        let mut validation_result = ConsensusValidationResult::<Option<PartialIdentity>>::default();

        let maybe_partial_identity =
            drive.fetch_identity_with_balance(self.identity_id.to_buffer(), tx)?;

        let partial_identity = match maybe_partial_identity {
            None => {
                //slightly weird to have a signature error, maybe should be changed
                validation_result.add_error(SignatureError::IdentityNotFoundError(
                    IdentityNotFoundError::new(self.identity_id),
                ));
                return Ok(validation_result);
            }
            Some(partial_identity) => partial_identity,
        };

        validation_result.set_data(Some(partial_identity));
        Ok(validation_result)
    }
}
