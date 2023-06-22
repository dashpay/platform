use crate::error::Error;

use crate::execution::validation::state_transition::common::validate_state_transition_identity_signature::v0::validate_state_transition_identity_signature_v0;
use dpp::document::DocumentsBatchTransition;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub(crate) trait StateTransitionIdentityAndSignaturesValidationV0 {
    fn validate_identity_and_signatures_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error>;
}

impl StateTransitionIdentityAndSignaturesValidationV0 for DocumentsBatchTransition {
    fn validate_identity_and_signatures_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        Ok(
            validate_state_transition_identity_signature_v0(drive, self, false, transaction)?
                .map(Some),
        )
    }
}
