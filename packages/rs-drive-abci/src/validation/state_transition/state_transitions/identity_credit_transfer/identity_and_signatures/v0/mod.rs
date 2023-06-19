use crate::error::Error;
use crate::validation::state_transition::key_validation::validate_state_transition_identity_signature_v0;
use dpp::consensus::signature::{IdentityNotFoundError, SignatureError};
use dpp::identity::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub(in crate::validation::state_transition) trait StateTransitionIdentityAndSignaturesValidationV0 {
    fn validate_identity_and_signatures_v0(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error>;
}

impl StateTransitionIdentityAndSignaturesValidationV0 for IdentityCreditTransferTransition {
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
