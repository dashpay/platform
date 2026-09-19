use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::state::document::document_already_present_error::DocumentAlreadyPresentError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{
    DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0,
};

use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::document::document_create_transition_action::state_v2::DocumentCreateTransitionActionStateValidationV2;
use crate::execution::validation::state_transition::batch::state::fetch_keep_history_document_lifecycle::fetch_keep_history_document_lifecycle;
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
    /// V3 is V2 plus the keep-history document lifecycle: a deleted or erasing
    /// keep-history document keeps its id, so its id is not free to create.
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

        // A keep-history document that has been deleted keeps its revisions and
        // keeps its id: nothing in the primary-key tree marks the id as taken,
        // so the by-id read of the earlier generations cannot see the
        // reservation. Creating over it would append a new document's revision
        // to a deleted one's record. The id becomes free again once an erase
        // has removed the last revision.
        let contract_fetch_info = self.base().data_contract_fetch_info();
        let contract = &contract_fetch_info.contract;
        let document_type_name = self.base().document_type_name();
        let Some(document_type) = contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), contract.id()).into(),
            ));
        };
        if !document_type.documents_keep_history() {
            return Ok(SimpleConsensusValidationResult::new());
        }

        let lifecycle = fetch_keep_history_document_lifecycle(
            platform.drive,
            contract,
            document_type,
            self.base().id(),
            &block_info.epoch,
            execution_context,
            transaction,
            platform_version,
        )?;
        if lifecycle.is_present() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::DocumentAlreadyPresentError(
                    DocumentAlreadyPresentError::new(self.base().id()),
                )),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
