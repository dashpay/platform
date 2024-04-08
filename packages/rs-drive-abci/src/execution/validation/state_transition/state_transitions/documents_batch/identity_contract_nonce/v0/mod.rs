use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::identity::identity_nonce::{validate_identity_nonce_update, validate_new_identity_nonce};
use dpp::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentTransitionV0Methods;

use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::state_transition::StateTransitionLike;

use dpp::validation::SimpleConsensusValidationResult;

use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::platform_types::platform::PlatformStateRef;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

pub(in crate::execution::validation::state_transition::state_transitions::documents_batch) trait DocumentsBatchStateTransitionIdentityContractNonceV0
{
    fn validate_identity_contract_nonces_v0(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentsBatchStateTransitionIdentityContractNonceV0 for DocumentsBatchTransition {
    fn validate_identity_contract_nonces_v0(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // We should validate that all newly created documents have valid ids
        for transition in self.transitions() {
            let revision_nonce = transition.identity_contract_nonce();
            let identity_id = self.owner_id();
            let (existing_nonce, fee) = platform.drive.fetch_identity_contract_nonce_with_fees(
                identity_id.to_buffer(),
                transition.data_contract_id().to_buffer(),
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
            if !result.is_valid() {
                return Ok(result);
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
