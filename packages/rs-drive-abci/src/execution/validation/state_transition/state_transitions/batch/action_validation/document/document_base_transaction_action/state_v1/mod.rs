use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::document::document_base_transaction_action::state_v0::DocumentBaseTransitionActionStateValidationV0;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentBaseTransitionActionStateValidationV1 {
    #[allow(clippy::too_many_arguments)]
    fn validate_state_v1(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        transition_type: &str,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentBaseTransitionActionStateValidationV1 for DocumentBaseTransitionAction {
    /// Version 1 (protocol version 15): a token cost paid out of the token's shielded pool
    /// (`TokenPaymentInfo::V1`) does not touch the owner's token balance, so the balance and
    /// frozen-account checks of v0 do not apply; the pool side (pool exists, token not paused,
    /// anchor, unspent nullifiers, pool balance and the proof) is validated by
    /// `validate_document_shielded_token_payment` once the document action itself is valid.
    /// A cost paid from the balance is validated exactly as in v0.
    fn validate_state_v1(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        transition_type: &str,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        if self.shielded_token_payment().is_some() {
            return Ok(SimpleConsensusValidationResult::new());
        }
        self.validate_state_v0(
            platform,
            owner_id,
            block_info,
            transition_type,
            execution_context,
            transaction,
            platform_version,
        )
    }
}
