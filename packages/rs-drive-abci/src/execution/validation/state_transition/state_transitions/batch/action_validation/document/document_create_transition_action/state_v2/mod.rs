use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{
    DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0,
};

use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::state::document::document_already_present_error::DocumentAlreadyPresentError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;

use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::state::v0::fetch_documents::fetch_keep_history_document_lifecycle;
use crate::execution::validation::state_transition::batch::action_validation::document::document_create_transition_action::state_v1::DocumentCreateTransitionActionStateValidationV1;
use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::DocumentReferenceValidation;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentCreateTransitionActionStateValidationV2
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

impl DocumentCreateTransitionActionStateValidationV2 for DocumentCreateTransitionAction {
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

        // A keep-history document that has been deleted keeps its revisions and
        // keeps its id: nothing in the primary-key tree marks the id as taken,
        // so v1's by-id read cannot see the reservation. Creating over it would
        // append a new document's revision to a deleted one's record. The id
        // becomes free again once an erase has removed the last revision.
        let contract_fetch_info = self.base().data_contract_fetch_info();
        let contract = &contract_fetch_info.contract;
        let document_type_name = self.base().document_type_name();
        let Some(document_type) = contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), contract.id()).into(),
            ));
        };
        if document_type.documents_keep_history() {
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
        }

        let reference_result = self.base().validate_document_references(
            self.data(),
            None,
            platform,
            block_info,
            transaction,
            execution_context,
            platform_version,
        )?;
        if !reference_result.is_valid() {
            return Ok(reference_result);
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
