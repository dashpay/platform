use crate::error::Error;

use dpp::consensus::signature::{IdentityNotFoundError, SignatureError};

use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::version::PlatformVersion;

use crate::execution::types::execution_operation::{RetrieveIdentityInfo, ValidationOperation};
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub(in crate::execution::validation::state_transition) trait IdentityTopUpStateTransitionIdentityRetrievalV0
{
    fn retrieve_topped_up_identity(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error>;
}

impl IdentityTopUpStateTransitionIdentityRetrievalV0 for IdentityTopUpTransition {
    fn retrieve_topped_up_identity(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error> {
        let mut validation_result = ConsensusValidationResult::<PartialIdentity>::default();

        execution_context.add_operation(ValidationOperation::RetrieveIdentity(
            RetrieveIdentityInfo::only_balance(),
        ));

        let maybe_partial_identity = drive.fetch_identity_with_balance(
            self.identity_id().to_buffer(),
            tx,
            platform_version,
        )?;

        let partial_identity = match maybe_partial_identity {
            None => {
                //slightly weird to have a signature error, maybe should be changed
                validation_result.add_error(SignatureError::IdentityNotFoundError(
                    IdentityNotFoundError::new(self.identity_id().to_owned()),
                ));
                return Ok(validation_result);
            }
            Some(partial_identity) => partial_identity,
        };

        validation_result.set_data(partial_identity);
        Ok(validation_result)
    }
}
