use crate::error::Error;

use dpp::consensus::signature::{IdentityNotFoundError, SignatureError};

use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::version::PlatformVersion;

use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub(crate) trait StateTransitionIdentityAndSignaturesValidationV0 {
    fn validate_identity_and_signatures_v0(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error>;
}

impl StateTransitionIdentityAndSignaturesValidationV0 for IdentityTopUpTransition {
    fn validate_identity_and_signatures_v0(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        let mut validation_result = ConsensusValidationResult::<Option<PartialIdentity>>::default();

        let maybe_partial_identity = drive.fetch_identity_with_balance(
            self.identity_id.to_buffer(),
            tx,
            platform_version,
        )?;

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
