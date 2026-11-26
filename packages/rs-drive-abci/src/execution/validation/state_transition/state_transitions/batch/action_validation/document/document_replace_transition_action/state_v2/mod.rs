use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::{
    DocumentReplaceTransitionAction, DocumentReplaceTransitionActionAccessorsV0,
};

use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::document::document_replace_transition_action::state_v1::DocumentReplaceTransitionActionStateValidationV1;
use crate::execution::validation::state_transition::batch::action_validation::document::document_type_sum_validation::DocumentTypeSumValidation;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentReplaceTransitionActionStateValidationV2
{
    fn validate_state_v2(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentReplaceTransitionActionStateValidationV2 for DocumentReplaceTransitionAction {
    /// Generation 1, then a replace that raises the document's summed property is held to the
    /// total its type keeps of that property (protocol version 15): the stored value leaves
    /// the total and the written one enters it.
    fn validate_state_v2(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = self.validate_state_v1(
            platform,
            owner_id,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        self.base().validate_document_type_sum(
            self.data(),
            Some(self.stored_changed_values()),
            platform,
            block_info,
            transaction,
            execution_context,
            platform_version,
        )
    }
}
