use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::document::document_contest_document_with_same_id_already_present_error::DocumentContestDocumentWithSameIdAlreadyPresentError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{
    DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0,
};

use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::batch::action_validation::document::document_create_transition_action::state_v2::DocumentCreateTransitionActionStateValidationV2;
use crate::execution::validation::state_transition::batch::state::v0::fetch_documents::has_contested_document_with_document_id;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentCreateTransitionActionStateValidationV3
{
    fn validate_state_v3(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentCreateTransitionActionStateValidationV3 for DocumentCreateTransitionAction {
    fn validate_state_v3(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = self.validate_state_v2(
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

        if self.prefunded_voting_balance().is_none() {
            let contract_fetch_info = self.base().data_contract_fetch_info_ref();
            let document_type = self.base().document_type()?;

            if document_type.find_contested_index().is_some() {
                let (contested_document_already_exists, fee_result) =
                    has_contested_document_with_document_id(
                        platform.drive,
                        &contract_fetch_info.contract,
                        document_type,
                        self.base().id(),
                        Some(&block_info.epoch),
                        transaction,
                        platform_version,
                    )?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                if contested_document_already_exists {
                    return Ok(ConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(
                            StateError::DocumentContestDocumentWithSameIdAlreadyPresentError(
                                DocumentContestDocumentWithSameIdAlreadyPresentError::new(
                                    self.base().id(),
                                ),
                            ),
                        ),
                    ));
                }
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
