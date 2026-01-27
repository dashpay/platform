use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::NonceOutOfBoundsError;
use dpp::consensus::basic::BasicError;
use dpp::identity::identity_nonce::{
    validate_identity_nonce_update, validate_new_identity_nonce, MISSING_IDENTITY_REVISIONS_FILTER,
};
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;

use dpp::validation::SimpleConsensusValidationResult;

use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::platform_types::platform::PlatformStateRef;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

pub(in crate::execution::validation::state_transition::state_transitions) trait DataContractUpdateStateTransitionIdentityContractNonceV0
{
    fn validate_identity_contract_nonce_v0(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractUpdateStateTransitionIdentityContractNonceV0 for DataContractUpdateTransition {
    fn validate_identity_contract_nonce_v0(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let revision_nonce = self.identity_contract_nonce();

        if revision_nonce & MISSING_IDENTITY_REVISIONS_FILTER > 0 {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                BasicError::NonceOutOfBoundsError(NonceOutOfBoundsError::new(revision_nonce))
                    .into(),
            ));
        }

        // Get identity_id and contract_id based on transition version
        let (identity_id, contract_id) = match self {
            DataContractUpdateTransition::V0(v0) => {
                (v0.data_contract.owner_id(), v0.data_contract.id())
            }
            DataContractUpdateTransition::V1(v1) => (v1.owner_id, v1.id),
        };

        let (existing_nonce, fee) = platform.drive.fetch_identity_contract_nonce_with_fees(
            identity_id.to_buffer(),
            contract_id.to_buffer(),
            block_info,
            true,
            tx,
            platform_version,
        )?;

        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

        let result = if let Some(existing_nonce) = existing_nonce {
            validate_identity_nonce_update(existing_nonce, revision_nonce, identity_id)
        } else {
            validate_new_identity_nonce(revision_nonce, identity_id)
        };

        Ok(result)
    }
}
